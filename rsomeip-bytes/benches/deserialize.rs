//! Deserialization benchmarks.
//!
//! This module contains benchmarks that measure the performance of [`Deserialize`] implementations.

#![expect(clippy::expect_used, reason = "acceptable in benchmarks")]

use bytes::Bytes;
use criterion::{
    BatchSize, BenchmarkGroup, Criterion, Throughput, criterion_group, criterion_main,
    measurement::Measurement,
};
use rsomeip_bytes::{
    Deserialize as _, DynamicArray, DynamicString, LengthU8, LengthU16, LengthU32, LengthZero,
    Serialize as _, StaticString, Utf8, Utf16BE, Utf16LE,
};
use std::{array, f32, f64, hint::black_box, iter};

/// Sizes of length encoded types.
const DYNAMIC_SIZES: [usize; 4] = [0, 0x00ff, 0x0fff, 0xffff];

/// Benchmarks basic type deserialization.
fn deserialize_basic_type(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("deserialize/basic_type");
    macro_rules! benchmark_basic_type {
        ($name:literal, $value:expr, $class:ty) => {{
            group.bench_function($name, |bench| {
                // Create a value to deserialize.
                let buffer = $value.to_bytes().expect("should serialize the value");
                // Check if deserialization succeeds.
                _ = <$class>::deserialize(&mut buffer.clone())
                    .expect("should deserialize the value");
                // Benchmark the deserialization.
                bench.iter_batched(
                    || buffer.clone(),
                    |mut buffer| <$class>::deserialize(black_box(&mut buffer)),
                    BatchSize::SmallInput,
                );
            });
        }};
    }
    benchmark_basic_type!("bool", true, bool);
    benchmark_basic_type!("u8", 1_u8, u8);
    benchmark_basic_type!("u16", 1_u16, u16);
    benchmark_basic_type!("u32", 1_u32, u32);
    benchmark_basic_type!("u64", 1_u64, u64);
    benchmark_basic_type!("i8", -1_i8, i8);
    benchmark_basic_type!("i16", -1_i16, i16);
    benchmark_basic_type!("i32", -1_i32, i32);
    benchmark_basic_type!("i64", -1_i64, i64);
    benchmark_basic_type!("f32", f32::consts::PI, f32);
    benchmark_basic_type!("f64", f64::consts::PI, f64);
}

/// Benchmarks tuple deserialization.
fn deserialize_tuple(criterion: &mut Criterion) {
    type FlatTuple = (i8, i8, i8, i8, i16, i32, i64);
    type NestedTuple = ((((i8, i8), (i8, i8), i16), i32), i64);

    // Group the two benchmarks.
    let mut group = criterion.benchmark_group("deserialize/tuple");

    group.bench_function("flat", |bench| {
        // Create a value to deserialize.
        let value: FlatTuple = (-1, -1, -1, -1, -1, -1, -1);
        let buffer = value.to_bytes().expect("should serialize the type");

        // Check if deserialization succeeds.
        _ = FlatTuple::deserialize(&mut buffer.clone()).expect("should deserialize the value");

        // Benchmark the deserialization.
        bench.iter_batched(
            || buffer.clone(),
            |mut buffer| FlatTuple::deserialize(black_box(&mut buffer)),
            BatchSize::SmallInput,
        );
    });

    group.bench_function("nested", |bench| {
        // Create a value to deserialize.
        let value: NestedTuple = ((((-1, -1), (-1, -1), -1), -1), -1);
        let buffer = value.to_bytes().expect("should serialize the value");

        // Check if deserialization succeeds.
        _ = NestedTuple::deserialize(&mut buffer.clone()).expect("should deserialize the value");

        // Benchmark the deserialization.
        bench.iter_batched(
            || buffer.clone(),
            |mut buffer| NestedTuple::deserialize(black_box(&mut buffer)),
            BatchSize::SmallInput,
        );
    });
}

/// Benchmarks dynamic array deserialization.
fn deserialize_dynamic_array(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("deserialize/dynamic_array");

    benchmark_dynamic_array::<LengthZero>(&mut group, "LengthZero");
    benchmark_dynamic_array::<LengthU8>(&mut group, "LengthU8");
    benchmark_dynamic_array::<LengthU16>(&mut group, "LengthU16");
    benchmark_dynamic_array::<LengthU32>(&mut group, "LengthU32");
}

/// Benchmarks dynamic array deserialization for a specific length field.
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
                let buffer = DynamicArray::<Length, _>::from(&value)
                    .to_bytes()
                    .expect("should serialize the value");
                // Check if deserialization succeeds.
                _ = DynamicArray::<Length, Vec<u8>>::deserialize(&mut buffer.clone())
                    .expect("should deserialize the value");
                // Benchmark the deserialization.
                bench.iter_batched(
                    || buffer.clone(),
                    |mut buffer| {
                        DynamicArray::<Length, Vec<u8>>::deserialize(black_box(&mut buffer))
                    },
                    BatchSize::SmallInput,
                );
            },
        );
    }
}

/// Benchmarks static array deserialization.
fn deserialize_static_array(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("deserialize/static_array");

    benchmark_static_array::<0>(&mut group);
    benchmark_static_array::<16>(&mut group);
    benchmark_static_array::<256>(&mut group);
}

/// Benchmarks static array deserialization for a specific length.
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
            let buffer = value.to_bytes().expect("should serialize the value");
            // Check if deserialization succeeds.
            _ = <[u8; N]>::deserialize(&mut buffer.clone()).expect("should deserialize the value");
            // Benchmark the deserialization.
            bench.iter_batched(
                || buffer.clone(),
                |mut buffer| <[u8; N]>::deserialize(black_box(&mut buffer)),
                BatchSize::SmallInput,
            );
        });
}

/// Benchmarks dynamic string deserialization.
fn deserialize_dynamic_string(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("deserialize/dynamic_string");

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

/// Benchmarks dynamic string deserialization for a specific length field.
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
                let buffer = DynamicString::<Length, Encoding, _>::from(&value)
                    .to_bytes()
                    .expect("should serialize the value");
                // Check if deserialization succeeds.
                _ = DynamicString::<Length, Encoding, String>::deserialize(&mut buffer.clone())
                    .expect("should deserialize the value");
                // Benchmark the deserialization.
                bench.iter_batched(
                    || buffer.clone(),
                    |mut buffer| {
                        DynamicString::<Length, Encoding, String>::deserialize(black_box(
                            &mut buffer,
                        ))
                    },
                    BatchSize::SmallInput,
                );
            });
    }
}

/// Benchmarks static string deserialization.
fn deserialize_static_string(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("deserialize/static_string");

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

/// Benchmarks static string deserialization for a specific length field.
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
            let buffer = StaticString::<N, Encoding, _>::from(&value)
                .to_bytes()
                .expect("should serialize the value");
            // Check if deserialization succeeds.
            _ = StaticString::<N, Encoding, String>::deserialize(&mut buffer.clone())
                .expect("should deserialize the value");
            // Benchmark the deserialization.
            bench.iter_batched(
                || buffer.clone(),
                |mut buffer| {
                    StaticString::<N, Encoding, String>::deserialize(black_box(&mut buffer))
                },
                BatchSize::SmallInput,
            );
        });
}

/// Benchmarks [`Bytes`] deserialization.
fn deserialize_bytes(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("deserialize/bytes");

    // Deserialize a `Bytes` of varying length.
    for size in DYNAMIC_SIZES {
        // Measure the throughput.
        group
            .throughput(Throughput::Bytes(
                u64::try_from(size).expect("should fit in u64"),
            ))
            .bench_with_input(format!("{size}"), &size, |bench, &size| {
                // Create a value to deserialize.
                let buffer: Bytes = (0..size)
                    .map(|current| u8::try_from(current.rem_euclid(255)).unwrap_or(0))
                    .collect();
                // Check if deserialization succeeds.
                _ = Bytes::deserialize(&mut buffer.clone()).expect("should deserialize the type");
                // Benchmark the deserialization.
                bench.iter_batched(
                    || buffer.clone(),
                    |mut buffer| Bytes::deserialize(black_box(&mut buffer)),
                    BatchSize::SmallInput,
                );
            });
    }
}

criterion_group!(
    benches,
    deserialize_basic_type,
    deserialize_tuple,
    deserialize_dynamic_array,
    deserialize_static_array,
    deserialize_dynamic_string,
    deserialize_static_string,
    deserialize_bytes,
);
criterion_main!(benches);
