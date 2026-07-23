//! Serialization benchmarks.
//!
//! This module contains benchmarks that measure the performance of [`Serialize`] implementations.

#![expect(clippy::expect_used, reason = "same as in tests")]

use bytes::{Bytes, BytesMut};
use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use rsomeip_bytes::{LengthField, Serialize as _, SerializeString as _};
use std::{array, f32, f64, hint::black_box, iter};

/// Benchmarks basic type serialization.
fn serialize_basic_type(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("serialize_basic_type");
    macro_rules! benchmark_basic_type {
        ($name:literal, $value:expr) => {
            group.bench_function($name, |bench| {
                let mut buffer = BytesMut::with_capacity($value.size_hint());
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
fn serialize_nested_tuple(criterion: &mut Criterion) {
    type NestedTuple = ((((i8, i8), (i8, i8), i16), i32), i64);
    let mut group = criterion.benchmark_group("serialize_nested_tuple");

    // Serialize without a length field.
    group.bench_function("no_len", |bench| {
        let value: NestedTuple = ((((-1, -1), (-1, -1), -1), -1), -1);
        let mut buffer = BytesMut::with_capacity(value.size_hint());
        bench.iter(|| {
            buffer.clear();
            value.serialize(black_box(&mut buffer))
        });
    });

    // Serialize with a length field.
    group.bench_function("len", |bench| {
        let value: NestedTuple = ((((-1, -1), (-1, -1), -1), -1), -1);
        let mut buffer = BytesMut::with_capacity(value.size_hint());
        bench.iter(|| {
            buffer.clear();
            value.serialize_len(LengthField::U8, black_box(&mut buffer))
        });
    });
}

/// Benchmarks flat tuple serialization.
fn serialize_flat_tuple(criterion: &mut Criterion) {
    type FlatTuple = (i8, i8, i8, i8, i16, i32, i64);
    let mut group = criterion.benchmark_group("serialize_flat_tuple");

    // Serialize without a length field.
    group.bench_function("no_len", |bench| {
        let value: FlatTuple = (-1, -1, -1, -1, -1, -1, -1);
        let mut buffer = BytesMut::with_capacity(value.size_hint());
        bench.iter(|| {
            buffer.clear();
            value.serialize(black_box(&mut buffer))
        });
    });

    // Serialize without a length field.
    group.bench_function("len", |bench| {
        let value: FlatTuple = (-1, -1, -1, -1, -1, -1, -1);
        let mut buffer = BytesMut::with_capacity(value.size_hint());
        bench.iter(|| {
            buffer.clear();
            value.serialize_len(LengthField::U8, black_box(&mut buffer))
        });
    });
}

/// Benchmarks [`Vec<u8>`] serialization.
fn serialize_vec(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("serialize_vec");

    // Serialize a `Vec` of varying length without a length field.
    for size in [0_usize, 0x00FF, 0x0FFF, 0xFFFF] {
        group
            .throughput(Throughput::Bytes(
                u64::try_from(size).expect("safe conversion"),
            ))
            .bench_with_input(format!("{size}"), &size, |bench, &size| {
                let value: Vec<u8> = (0..size)
                    .map(|current| u8::try_from(current.rem_euclid(255)).unwrap_or(0))
                    .collect();
                let mut buffer = BytesMut::with_capacity(size);
                bench.iter(|| {
                    buffer.clear();
                    value.serialize(black_box(&mut buffer))
                });
            });
    }

    // Serialize a `Vec` of varying length with a length field.
    for size in [0_usize, 0x00FF, 0x0FFF, 0xFFFF] {
        group
            .throughput(Throughput::Bytes(
                u64::try_from(size).expect("safe conversion"),
            ))
            .bench_with_input(format!("len/{size}"), &size, |bench, &size| {
                let value: Vec<u8> = (0..size)
                    .map(|current| u8::try_from(current.rem_euclid(255)).unwrap_or(0))
                    .collect();
                let mut buffer = BytesMut::with_capacity(size);
                bench.iter(|| {
                    buffer.clear();
                    value.serialize_len(LengthField::U16, black_box(&mut buffer))
                });
            });
    }
}

/// Benchmarks array serialization.
fn serialize_array(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("serialize_array");

    macro_rules! benchmark_array {
        ($array:ident) => {{
            group
                .throughput(Throughput::Bytes(
                    u64::try_from($array.len()).expect("safe conversion"),
                ))
                .bench_with_input(format!("{}", $array.len()), &$array, |bench, &array| {
                    let mut buffer = BytesMut::with_capacity(array.len());
                    bench.iter(|| {
                        buffer.clear();
                        array.serialize(black_box(&mut buffer))
                    });
                });
        }};
    }

    macro_rules! benchmark_array_len {
        ($array:ident) => {{
            group
                .throughput(Throughput::Bytes(
                    u64::try_from($array.len()).expect("safe conversion"),
                ))
                .bench_with_input(format!("len/{}", $array.len()), &$array, |bench, &array| {
                    let mut buffer = BytesMut::with_capacity(array.len());
                    bench.iter(|| {
                        buffer.clear();
                        array.serialize_len(LengthField::U16, black_box(&mut buffer))
                    });
                });
        }};
    }

    let array_ff: [u8; 0x00FF] =
        array::from_fn(|index| u8::try_from(index.rem_euclid(255)).unwrap_or(0));
    let array_fff: [u8; 0x0FFF] =
        array::from_fn(|index| u8::try_from(index.rem_euclid(255)).unwrap_or(0));
    let array_ffff: [u8; 0xFFFF] =
        array::from_fn(|index| u8::try_from(index.rem_euclid(255)).unwrap_or(0));

    // Serialize arrays of varying length without a length field.
    benchmark_array!(array_ff);
    benchmark_array!(array_fff);
    benchmark_array!(array_ffff);

    // Serialize arrays of varying length with a length field.
    benchmark_array_len!(array_ff);
    benchmark_array_len!(array_fff);
    benchmark_array_len!(array_ffff);
}

/// Benchmarks [`Bytes`] serialization.
fn serialize_bytes(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("serialize_bytes");

    // Serialize a `Bytes` of varying length without a length field.
    for size in [0_usize, 0x00FF, 0x0FFF, 0xFFFF] {
        group
            .throughput(Throughput::Bytes(
                u64::try_from(size).expect("safe conversion"),
            ))
            .bench_with_input(format!("{size}"), &size, |bench, &size| {
                let value: Bytes = (0..size)
                    .map(|current| u8::try_from(current.rem_euclid(255)).unwrap_or(0))
                    .collect();
                let mut buffer = BytesMut::with_capacity(size);
                bench.iter(|| {
                    buffer.clear();
                    value.serialize(black_box(&mut buffer))
                });
            });
    }

    // Serialize a `Bytes` of varying length with a length field.
    for size in [0_usize, 0x00FF, 0x0FFF, 0xFFFF] {
        group
            .throughput(Throughput::Bytes(
                u64::try_from(size).expect("safe conversion"),
            ))
            .bench_with_input(format!("len/{size}"), &size, |bench, &size| {
                let value: Bytes = (0..size)
                    .map(|current| u8::try_from(current.rem_euclid(255)).unwrap_or(0))
                    .collect();
                let mut buffer = BytesMut::with_capacity(size);
                bench.iter(|| {
                    buffer.clear();
                    value.serialize_len(LengthField::U16, black_box(&mut buffer))
                });
            });
    }
}

/// Benchmarks [`String`] serialization.
fn serialize_string(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("serialize_string");

    macro_rules! benchmark_string {
        ($format:literal, $method:ident, $length:expr, $throughput:expr) => {{
            for size in [0_usize, 0x00FF, 0x0FFF, 0xFFFF] {
                group
                    .throughput(Throughput::Bytes(
                        u64::try_from(size.saturating_mul($throughput)).expect("safe conversion"),
                    ))
                    .bench_with_input(format!("{}/{size}", $format), &size, |bench, &size| {
                        let value =
                            String::from_utf8(iter::repeat_n(42, size).collect::<Vec<u8>>())
                                .expect("valid String");
                        let mut buffer = BytesMut::with_capacity(size.saturating_mul($throughput));
                        bench.iter(|| {
                            buffer.clear();
                            value.as_str().$method(black_box(&mut buffer), $length)
                        });
                    });
            }
        }};
    }

    // Serialize a `String` of varying length without a length field.
    benchmark_string!("UTF-8", serialize_utf8, None, 1);
    benchmark_string!("UTF-16BE", serialize_utf16_be, None, 2);
    benchmark_string!("UTF-16LE", serialize_utf16_le, None, 2);

    // Serialize a `String` of varying length with a length field.
    benchmark_string!("UTF-8/len", serialize_utf8, Some(LengthField::U16), 1);
    benchmark_string!(
        "UTF-16BE/len",
        serialize_utf16_be,
        Some(LengthField::U16),
        2
    );
    benchmark_string!(
        "UTF-16LE/len",
        serialize_utf16_le,
        Some(LengthField::U16),
        2
    );
}

criterion_group!(
    benches,
    serialize_basic_type,
    serialize_nested_tuple,
    serialize_flat_tuple,
    serialize_vec,
    serialize_array,
    serialize_bytes,
    serialize_string
);
criterion_main!(benches);
