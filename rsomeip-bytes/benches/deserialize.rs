//! Deserialization benchmarks.
//!
//! This module contains benchmarks that measure the performance of [`Deserialize`] implementations.

#![expect(clippy::expect_used, reason = "used to uphold invariants")]

use bytes::{Bytes, BytesMut};
use criterion::{BatchSize, Criterion, Throughput, criterion_group, criterion_main};
use rsomeip_bytes::{Deserialize as _, LengthField, Serialize as _, SerializeString as _};
use std::{array, f32, f64, hint::black_box, iter};

/// Benchmarks basic type deserialization.
fn deserialize_basic_type(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("deserialize_basic_type");
    macro_rules! benchmark_basic_type {
        ($name:literal, $value:expr, $class:ty) => {{
            group.bench_function($name, |bench| {
                let buffer = {
                    let mut buffer = BytesMut::with_capacity($value.size_hint());
                    $value.serialize(&mut buffer).expect("valid serialization");
                    buffer.freeze()
                };
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

/// Benchmarks nested tuple deserialization.
fn deserialize_nested_tuple(criterion: &mut Criterion) {
    type NestedTuple = ((((i8, i8), (i8, i8), i16), i32), i64);
    let mut group = criterion.benchmark_group("deserialize_nested_tuple");

    // Deserialize without a length field.
    group.bench_function("no_len", |bench| {
        let buffer = {
            let value: NestedTuple = ((((-1, -1), (-1, -1), -1), -1), -1);
            let mut buffer = BytesMut::with_capacity(value.size_hint());
            value.serialize(&mut buffer).expect("valid serialization");
            buffer.freeze()
        };
        bench.iter_batched(
            || buffer.clone(),
            |mut buffer| NestedTuple::deserialize(black_box(&mut buffer)),
            BatchSize::SmallInput,
        );
    });

    // Deserialize with a length field.
    group.bench_function("len", |bench| {
        let buffer = {
            let value: NestedTuple = ((((-1, -1), (-1, -1), -1), -1), -1);
            let mut buffer = BytesMut::with_capacity(value.size_hint());
            value
                .serialize_len(LengthField::U8, &mut buffer)
                .expect("valid serialization");
            buffer.freeze()
        };
        bench.iter_batched(
            || buffer.clone(),
            |mut buffer| NestedTuple::deserialize_len(LengthField::U8, black_box(&mut buffer)),
            BatchSize::SmallInput,
        );
    });
}

/// Benchmarks flat tuple deserialization.
fn deserialize_flat_tuple(criterion: &mut Criterion) {
    type FlatTuple = (i8, i8, i8, i8, i16, i32, i64);
    let mut group = criterion.benchmark_group("deserialize_flat_tuple");

    // Deserialize without a length field.
    group.bench_function("no_len", |bench| {
        let buffer = {
            let value: FlatTuple = (-1, -1, -1, -1, -1, -1, -1);
            let mut buffer = BytesMut::with_capacity(value.size_hint());
            value.serialize(&mut buffer).expect("valid serialization");
            buffer.freeze()
        };
        bench.iter_batched(
            || buffer.clone(),
            |mut buffer| FlatTuple::deserialize(black_box(&mut buffer)),
            BatchSize::SmallInput,
        );
    });

    // Deserialize with a length field.
    group.bench_function("len", |bench| {
        let buffer = {
            let value: FlatTuple = (-1, -1, -1, -1, -1, -1, -1);
            let mut buffer = BytesMut::with_capacity(value.size_hint());
            value
                .serialize_len(LengthField::U8, &mut buffer)
                .expect("valid serialization");
            buffer.freeze()
        };
        bench.iter_batched(
            || buffer.clone(),
            |mut buffer| FlatTuple::deserialize_len(LengthField::U8, black_box(&mut buffer)),
            BatchSize::SmallInput,
        );
    });
}

/// Benchmarks [`Vec<u8>`] deserialization.
fn deserialize_vec(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("deserialize_vec");

    // Deserialize a `Vec` of varying length without a length field.
    for size in [0_usize, 0x00FF, 0x0FFF, 0xFFFF] {
        group
            .throughput(Throughput::Bytes(
                u64::try_from(size).expect("safe conversion"),
            ))
            .bench_with_input(format!("{size}"), &size, |bench, &size| {
                let buffer = {
                    let value: Vec<u8> = (0..size)
                        .map(|current| u8::try_from(current.rem_euclid(255)).unwrap_or(0))
                        .collect();
                    let mut buffer = BytesMut::with_capacity(size);
                    value.serialize(&mut buffer).expect("valid serialization");
                    buffer.freeze()
                };
                bench.iter_batched(
                    || buffer.clone(),
                    |mut buffer| Vec::<u8>::deserialize(black_box(&mut buffer)),
                    BatchSize::SmallInput,
                );
            });
    }

    // Deserialize a `Vec` of varying length with a length field.
    for size in [0_usize, 0x00FF, 0x0FFF, 0xFFFF] {
        group
            .throughput(Throughput::Bytes(
                u64::try_from(size).expect("safe conversion"),
            ))
            .bench_with_input(format!("len/{size}"), &size, |bench, &size| {
                let buffer = {
                    let value: Vec<u8> = (0..size)
                        .map(|current| u8::try_from(current.rem_euclid(255)).unwrap_or(0))
                        .collect();
                    let mut buffer = BytesMut::with_capacity(size);
                    value
                        .serialize_len(LengthField::U16, &mut buffer)
                        .expect("valid serialization");
                    buffer.freeze()
                };
                bench.iter_batched(
                    || buffer.clone(),
                    |mut buffer| {
                        Vec::<u8>::deserialize_len(LengthField::U16, black_box(&mut buffer))
                    },
                    BatchSize::SmallInput,
                );
            });
    }
}

/// Benchmarks array deserialization.
fn deserialize_array(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("deserialize_array");

    macro_rules! benchmark_array {
        ($array:ident, $class:ty) => {{
            group
                .throughput(Throughput::Bytes(
                    u64::try_from($array.len()).expect("safe conversion"),
                ))
                .bench_with_input(format!("{}", $array.len()), &$array, |bench, &array| {
                    let buffer = {
                        let mut buffer = BytesMut::with_capacity(array.len());
                        array.serialize(&mut buffer).expect("valid serialization");
                        buffer.freeze()
                    };
                    bench.iter_batched(
                        || buffer.clone(),
                        |mut buffer| <$class>::deserialize(black_box(&mut buffer)),
                        BatchSize::SmallInput,
                    );
                });
        }};
    }

    macro_rules! benchmark_array_len {
        ($array:ident, $class:ty) => {{
            group
                .throughput(Throughput::Bytes(
                    u64::try_from($array.len()).expect("safe conversion"),
                ))
                .bench_with_input(format!("len/{}", $array.len()), &$array, |bench, &array| {
                    let buffer = {
                        let mut buffer = BytesMut::with_capacity(array.len());
                        array
                            .serialize_len(LengthField::U16, &mut buffer)
                            .expect("valid serialization");
                        buffer.freeze()
                    };
                    bench.iter_batched(
                        || buffer.clone(),
                        |mut buffer| {
                            <$class>::deserialize_len(LengthField::U16, black_box(&mut buffer))
                        },
                        BatchSize::SmallInput,
                    );
                });
        }};
    }

    let array_ff: [u8; 0x00FF] =
        array::from_fn(|index| u8::try_from(index.rem_euclid(255)).unwrap_or(0));
    let array_fff: [u8; 0x0FFF] =
        array::from_fn(|index| u8::try_from(index.rem_euclid(255)).unwrap_or(0));
    let array_ffff: [u8; 0xFFFF] =
        array::from_fn(|index| u8::try_from(index.rem_euclid(255)).unwrap_or(0));

    // Deserialize arrays of varying length without a length field.
    benchmark_array!(array_ff, [u8; 0x00FF]);
    benchmark_array!(array_fff, [u8; 0x0FFF]);
    benchmark_array!(array_ffff, [u8; 0xFFFF]);

    // Deserialize arrays of varying length with a length field.
    benchmark_array_len!(array_ff, [u8; 0x00FF]);
    benchmark_array_len!(array_fff, [u8; 0x0FFF]);
    benchmark_array_len!(array_ffff, [u8; 0xFFFF]);
}

/// Benchmarks [`Bytes`] deserialization.
fn deserialize_bytes(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("deserialize_bytes");

    // Deserialize a `Bytes` of varying length without a length field.
    for size in [0_usize, 0x00FF, 0x0FFF, 0xFFFF] {
        group
            .throughput(Throughput::Bytes(
                u64::try_from(size).expect("safe conversion"),
            ))
            .bench_with_input(format!("{size}"), &size, |bench, &size| {
                let buffer: Bytes = (0..size)
                    .map(|current| u8::try_from(current.rem_euclid(255)).unwrap_or(0))
                    .collect();
                bench.iter_batched(
                    || buffer.clone(),
                    |mut buffer| Bytes::deserialize(black_box(&mut buffer)),
                    BatchSize::SmallInput,
                );
            });
    }

    // Deserialize a `Bytes` of varying length with a length field.
    for size in [0_usize, 0x00FF, 0x0FFF, 0xFFFF] {
        group
            .throughput(Throughput::Bytes(
                u64::try_from(size).expect("safe conversion"),
            ))
            .bench_with_input(format!("len/{size}"), &size, |bench, &size| {
                let buffer = {
                    let value: Bytes = (0..size)
                        .map(|current| u8::try_from(current.rem_euclid(255)).unwrap_or(0))
                        .collect();
                    let mut buffer = BytesMut::with_capacity(size);
                    value
                        .serialize_len(LengthField::U16, &mut buffer)
                        .expect("valid serialziation");
                    buffer.freeze()
                };
                bench.iter_batched(
                    || buffer.clone(),
                    |mut buffer| Bytes::deserialize_len(LengthField::U16, black_box(&mut buffer)),
                    BatchSize::SmallInput,
                );
            });
    }
}

/// Benchmarks [`String`] deserialization.
fn deserialize_string(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("deserialize_string");

    macro_rules! benchmark_string {
        ($format:literal, $method:ident, $length:expr, $throughput:expr, $de:expr) => {{
            for size in [0_usize, 0x00FF, 0x0FFF, 0xFFFF] {
                group
                    .throughput(Throughput::Bytes(
                        u64::try_from(size.saturating_mul($throughput)).expect("safe conversion"),
                    ))
                    .bench_with_input(format!("{}/{size}", $format), &size, |bench, &size| {
                        let buffer = {
                            let value =
                                String::from_utf8(iter::repeat_n(42, size).collect::<Vec<u8>>())
                                    .expect("valid String");
                            let mut buffer =
                                BytesMut::with_capacity(size.saturating_mul($throughput));
                            value
                                .as_str()
                                .$method(&mut buffer, $length)
                                .expect("valid serialization");
                            buffer.freeze()
                        };
                        bench.iter_batched(|| buffer.clone(), $de, BatchSize::SmallInput);
                    });
            }
        }};
    }

    // Deserialize a `String` of varying length without a length field.
    benchmark_string!("UTF-8", serialize_utf8, None, 1, |mut buffer| {
        String::deserialize(black_box(&mut buffer))
    });
    benchmark_string!("UTF-16BE", serialize_utf16_be, None, 2, |mut buffer| {
        String::deserialize(black_box(&mut buffer))
    });
    benchmark_string!("UTF-16LE", serialize_utf16_le, None, 2, |mut buffer| {
        String::deserialize(black_box(&mut buffer))
    });

    // Deserialize a `String` of varying length with a length field.
    benchmark_string!(
        "UTF-8/len",
        serialize_utf8,
        Some(LengthField::U32),
        1,
        |mut buffer| String::deserialize_len(LengthField::U32, black_box(&mut buffer))
    );
    benchmark_string!(
        "UTF-16BE/len",
        serialize_utf16_be,
        Some(LengthField::U32),
        2,
        |mut buffer| String::deserialize_len(LengthField::U32, black_box(&mut buffer))
    );
    benchmark_string!(
        "UTF-16LE/len",
        serialize_utf16_le,
        Some(LengthField::U32),
        2,
        |mut buffer| String::deserialize_len(LengthField::U32, black_box(&mut buffer))
    );
}

criterion_group!(
    benches,
    deserialize_basic_type,
    deserialize_nested_tuple,
    deserialize_flat_tuple,
    deserialize_vec,
    deserialize_array,
    deserialize_bytes,
    deserialize_string
);
criterion_main!(benches);
