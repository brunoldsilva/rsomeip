//! Serialization benchmarks.
//!
//! This module contains benchmarks that measure the performance of [`Serialize`] implementations.

#![expect(clippy::expect_used, reason = "acceptable in benchmarks")]

use bytes::{Bytes, BytesMut};
use criterion::{
    BenchmarkGroup, Criterion, Throughput, criterion_group, criterion_main,
    measurement::Measurement,
};
use rsomeip_bytes::{
    DynamicArray, DynamicString, LengthU8, LengthU16, LengthU32, LengthZero, Serialize as _,
    StaticString, Utf8, Utf16BE, Utf16LE,
};
use std::{array, f32, f64, hint::black_box, iter};

/// Sizes of length encoded types.
const DYNAMIC_SIZES: [usize; 4] = [0, 0x00ff, 0x0fff, 0xffff];

/// Benchmarks basic type serialization.
fn serialize_basic_type(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("serialize/basic_type");
    macro_rules! benchmark_basic_type {
        ($name:literal, $value:expr) => {
            group.bench_function($name, |bench| {
                let mut buffer = BytesMut::with_capacity($value.size().expect("should fit usize"));
                // Test serialization.
                _ = $value.to_bytes().expect("should serialize the value");
                // Benchmark serialization.
                bench.iter(|| {
                    buffer.clear();
                    $value.serialize(black_box(&mut buffer))
                });
            });
        };
    }
    benchmark_basic_type!("bool", true);
    benchmark_basic_type!("u8", 1_u8);
    benchmark_basic_type!("u16", 1_u16);
    benchmark_basic_type!("u32", 1_u32);
    benchmark_basic_type!("u64", 1_u64);
    benchmark_basic_type!("i8", -1_i8);
    benchmark_basic_type!("i16", -1_i16);
    benchmark_basic_type!("i32", -1_i32);
    benchmark_basic_type!("i64", -1_i64);
    benchmark_basic_type!("f32", f32::consts::PI);
    benchmark_basic_type!("f64", f64::consts::PI);
}

/// Benchmarks nested tuple serialization.
fn serialize_tuple(criterion: &mut Criterion) {
    type FlatTuple = (i8, i8, i8, i8, i16, i32, i64);
    type NestedTuple = ((((i8, i8), (i8, i8), i16), i32), i64);

    // Group the two benchmarks.
    let mut group = criterion.benchmark_group("serialize/tuple");

    group.bench_function("flat", |bench| {
        // Create a value to serialize.
        let value: FlatTuple = (-1, -1, -1, -1, -1, -1, -1);
        let mut buffer = BytesMut::with_capacity(value.size().expect("should fit in usize"));

        // Check if serialization succeeds.
        _ = value.to_bytes().expect("should serialize the value");

        // Benchmark the serialization.
        bench.iter(|| {
            buffer.clear();
            value.serialize(black_box(&mut buffer))
        });
    });

    group.bench_function("nested", |bench| {
        // Create a value to serialize.
        let value: NestedTuple = ((((-1, -1), (-1, -1), -1), -1), -1);
        let mut buffer = BytesMut::with_capacity(value.size().expect("should fit in usize"));

        // Check if serialization succeeds.
        _ = value.to_bytes().expect("should serialize the value");

        // Benchmark the serialization.
        bench.iter(|| {
            buffer.clear();
            value.serialize(black_box(&mut buffer))
        });
    });
}

/// Benchmarks dynamic array serialization.
fn serialize_dynamic_array(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("serialize/dynamic_array");

    benchmark_dynamic_array::<LengthZero>(&mut group, "LengthZero");
    benchmark_dynamic_array::<LengthU8>(&mut group, "LengthU8");
    benchmark_dynamic_array::<LengthU16>(&mut group, "LengthU16");
    benchmark_dynamic_array::<LengthU32>(&mut group, "LengthU32");
}

/// Benchmarks dynamic array serialization with a specific length field.
fn benchmark_dynamic_array<Length>(
    group: &mut BenchmarkGroup<'_, impl Measurement>,
    name: &'static str,
) where
    Length: rsomeip_bytes::Length,
{
    // Benchmark multiple sizes.
    for size in DYNAMIC_SIZES {
        // Skip if the value is greater than the capacity of the length field.
        if Length::capacity().is_none_or(|capacity| size > capacity) {
            continue;
        }
        let size = u64::try_from(size).expect("should fit in u64");
        // Measure the throughput.
        group.throughput(Throughput::Bytes(size)).bench_with_input(
            format!("{name}/{size}"),
            &size,
            |bench, &size| {
                // Create a value to serialize.
                let value: Vec<u8> = (0..size)
                    .map(|current| u8::try_from(current.rem_euclid(255)).unwrap_or(0))
                    .collect();
                let wrapper = DynamicArray::<Length, _>::from(&value);
                // Create a buffer to serialize the value.
                let mut buffer =
                    BytesMut::with_capacity(wrapper.size().expect("should fit in usize"));
                // Check if serialization succeeds.
                _ = wrapper.to_bytes().expect("should serialize the value");
                // Benchmark the serialization.
                bench.iter(|| {
                    buffer.clear();
                    wrapper.serialize(black_box(&mut buffer))
                });
            },
        );
    }
}

/// Benchmarks static array serialization.
fn serialize_static_array(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("serialize/static_array");

    benchmark_static_array::<0>(&mut group);
    benchmark_static_array::<16>(&mut group);
    benchmark_static_array::<256>(&mut group);
}

/// Benchmarks static array serialization for a specific length.
fn benchmark_static_array<const N: usize>(group: &mut BenchmarkGroup<'_, impl Measurement>) {
    // Measure the throughput.
    group
        .throughput(Throughput::Bytes(
            u64::try_from(N).expect("should fit in u64"),
        ))
        .bench_function(format!("{N}"), |bench| {
            // Create a value to serialize.
            let value: [u8; N] =
                array::from_fn(|index| u8::try_from(index.rem_euclid(255)).unwrap_or(0));
            // Create a buffer to serialize the value.
            let mut buffer = BytesMut::with_capacity(value.size().expect("should fit in usize"));
            // Check if serialization succeeds.
            _ = value.to_bytes().expect("should serialize the value");
            // Benchmark the serialization.
            bench.iter(|| {
                buffer.clear();
                value.serialize(black_box(&mut buffer))
            });
        });
}

/// Benchmarks dynamic string serialization.
fn serialize_dynamic_string(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("serialize/dynamic_string");

    // UTF-8
    let make_utf8 = |size: usize| {
        let size = size.saturating_sub(4); // BOM + Delimiter
        String::from_utf8(iter::repeat_n(42, size).collect()).expect("should be valid UTF-8")
    };
    benchmark_dynamic_string::<LengthZero, Utf8>(&mut group, "UTF-8/LengthZero", make_utf8);
    benchmark_dynamic_string::<LengthU32, Utf8>(&mut group, "UTF-8/LengthU32", make_utf8);

    // UTF-16
    let make_uft16 = |size: usize| {
        let size = size.saturating_sub(4); // BOM + Delimiter
        String::from_utf16(&iter::repeat_n(42, size.div_euclid(2)).collect::<Vec<u16>>())
            .expect("should be valid UTF-16")
    };

    // UTF-16 Big Endian
    benchmark_dynamic_string::<LengthZero, Utf16BE>(&mut group, "UTF-16BE/LengthZero", make_uft16);
    benchmark_dynamic_string::<LengthU32, Utf16BE>(&mut group, "UTF-16BE/LengthU32", make_uft16);

    // UTF-16 Little Endian
    benchmark_dynamic_string::<LengthZero, Utf16LE>(&mut group, "UTF-16LE/LengthZero", make_uft16);
    benchmark_dynamic_string::<LengthU32, Utf16LE>(&mut group, "UTF-16LE/LengthU32", make_uft16);
}

/// Benchmarks dynamic string serialization for a specific length field.
fn benchmark_dynamic_string<Length, Encoding>(
    group: &mut BenchmarkGroup<'_, impl Measurement>,
    name: &'static str,
    value_fn: fn(usize) -> String,
) where
    Length: rsomeip_bytes::Length,
    Encoding: rsomeip_bytes::Encoding,
{
    // Benchmark multiple sizes.
    for size in DYNAMIC_SIZES {
        // Skip if the value is greater than the capacity of the length field.
        if Length::capacity().is_none_or(|capacity| size > capacity) {
            continue;
        }
        // Measure the throughput.
        group
            .throughput(Throughput::Bytes(
                u64::try_from(size).expect("should fit in u64"),
            ))
            .bench_with_input(format!("{name}/{size}"), &size, |bench, &size| {
                // Create a value to serialize.
                let value = value_fn(size);
                let wrapper = DynamicString::<Length, Encoding, _>::from(&value);
                // Create a buffer to serialize the value.
                let mut buffer =
                    BytesMut::with_capacity(wrapper.size().expect("should fit in usize"));
                // Check if serialization succeeds.
                _ = wrapper.to_bytes().expect("should serialize the value");
                // Benchmark the serialization.
                bench.iter(|| {
                    buffer.clear();
                    wrapper.serialize(black_box(&mut buffer))
                });
            });
    }
}

/// Benchmarks static string serialization.
fn serialize_static_string(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("serialize/static_string");

    // UTF-8
    let make_utf8 = |size: usize| {
        let size = size.saturating_sub(4); // BOM + Delimiter
        String::from_utf8(iter::repeat_n(42, size).collect()).expect("should be valid UTF-8")
    };
    benchmark_static_string::<4, Utf8>(&mut group, "UTF-8", make_utf8);
    benchmark_static_string::<16, Utf8>(&mut group, "UTF-8", make_utf8);
    benchmark_static_string::<256, Utf8>(&mut group, "UTF-8", make_utf8);

    // UTF-16
    let make_uft16 = |size: usize| {
        let size = size.saturating_sub(4); // BOM + Delimiter
        String::from_utf16(&iter::repeat_n(42, size.div_euclid(2)).collect::<Vec<u16>>())
            .expect("should be valid UTF-16")
    };

    // UTF-16 Big Endian
    benchmark_static_string::<4, Utf16BE>(&mut group, "UTF-16BE", make_uft16);
    benchmark_static_string::<16, Utf16BE>(&mut group, "UTF-16BE", make_uft16);
    benchmark_static_string::<256, Utf16BE>(&mut group, "UTF-16BE", make_uft16);

    // UTF-16 Little Endian
    benchmark_static_string::<4, Utf16LE>(&mut group, "UTF-16LE", make_uft16);
    benchmark_static_string::<16, Utf16LE>(&mut group, "UTF-16LE", make_uft16);
    benchmark_static_string::<256, Utf16LE>(&mut group, "UTF-16LE", make_uft16);
}

/// Benchmarks static string serialization for a specific length field.
fn benchmark_static_string<const N: usize, Encoding>(
    group: &mut BenchmarkGroup<'_, impl Measurement>,
    name: &'static str,
    value_fn: fn(usize) -> String,
) where
    Encoding: rsomeip_bytes::Encoding,
{
    // Measure the throughput.
    group
        .throughput(Throughput::Bytes(
            u64::try_from(N).expect("should fit in u64"),
        ))
        .bench_function(format!("{name}/{N}"), |bench| {
            // Create a value to serialize.
            let value = value_fn(N);
            let wrapper = StaticString::<N, Encoding, _>::from(&value);
            // Create a buffer to serialize the value.
            let mut buffer =
                BytesMut::with_capacity(wrapper.size().expect("should serialize the value"));
            // Check if serialization succeeds.
            _ = wrapper.to_bytes().expect("should serialize the value");
            // Benchmark the serialization.
            bench.iter(|| {
                buffer.clear();
                wrapper.serialize(black_box(&mut buffer))
            });
        });
}

/// Benchmarks [`Bytes`] serialization.
fn serialize_bytes(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("serialize/bytes");

    // serialize a `Bytes` of varying length.
    for size in DYNAMIC_SIZES {
        // Measure the throughput.
        group
            .throughput(Throughput::Bytes(
                u64::try_from(size).expect("should fit in u64"),
            ))
            .bench_with_input(format!("{size}"), &size, |bench, &size| {
                // Create a value to serialize.
                let value: Bytes = (0..size)
                    .map(|current| u8::try_from(current.rem_euclid(255)).unwrap_or(0))
                    .collect();
                // Create a buffer to serialize the value.
                let mut buffer = BytesMut::with_capacity(size);
                // Check if serialization succeeds.
                _ = value.to_bytes().expect("should serialize the type");
                // Benchmark the serialization.
                bench.iter(|| {
                    buffer.clear();
                    value.serialize(black_box(&mut buffer))
                });
            });
    }
}

criterion_group!(
    benches,
    serialize_basic_type,
    serialize_tuple,
    serialize_dynamic_array,
    serialize_static_array,
    serialize_dynamic_string,
    serialize_static_string,
    serialize_bytes,
);
criterion_main!(benches);
