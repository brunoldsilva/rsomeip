# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `DynamicString` and `StaticString` types.

    These implement `Serialize` and `Deserialize` and provide the means of specifying the length and
    encoding of strings during both serialization and deserialization.

- `DynamicArray` type.

    Like the `DynamicString` type, provides a way to serialize and deserialize iterator types using
    a specific length field.

- `Length` trait and `LengthU*` types.

    These are sealed traits and marker types that are used to encode a length field before
    dynamically sized payloads.

- `Encoding` trait and `Utf*` types.

    These are sealed traits and marker types that are used to specify the encoding of strings when
    writing or reading data.

- `SerializeWithFn`, `SerializeWithLength` and `DeserializeWithLength`.

    Convenient wrappers to help serialize and deserialize certain types.

- `std` feature and `no_std` compatibility.

    Adds support for standard library types. Enabled by default.

    Disabling it allows the crate to be compiled in `no_std` environments.

### Changed

- Refactored `Serialize` and `Deserialize` traits.

    Removed the length handling methods to dedicated `Length` trait and markers, and introduced
    `DynamicArray` and `DynamicString` types to make use of them.

    Moved `size_hint` to the `Deserialize` trait and added a dedicated `size` method to `Serialize`.

    Added a `to_bytes` method to `Serialize` that returns the value serialized into a `Bytes`
    buffer.

- Refactored `SerializeError` and `DeserializeError` variants.

    Simplified some error variants that weren't providing very useful information and added a few
    more to cover new specific error conditions.

### Removed

- Removed `Serialize` and `Deserialize` trait impls from some standard library types.

    Mainly from `Vec` and `String` which should not use the `*Array` and `*String` helpers.

- `serialize_into!` and `deserialize_from!` macros.

    These didn't provide much value over using tuples directly and would require large refactors to
    match the capabilities of the new features.

- `SerializeString` trait.

    Replaced with `StaticString`, `DynamicString` and `Encoding`.

## [0.1.0] - 2025-08-13

### Added

- `Serialize` and `Deserialize` traits.

    These abstract the process of writing and reading data from SOME/IP on-wire data streams.

    The crate provides implementations of these traits for several types of the standard library
    that roughly match the types in the SOME/IP specification.

- `serialize_into!` and `deserialize_from` macros.

    These simplify the process of implementing the `Serialize` and `Deserialize` traits for custom
    types.

- `SerializeString` trait.

    Handles serialization of strings using different encodings, such as UTF-8, UTF-16 Big Endian and
    UTF-16 Little Endian.

    There isn't a `DeserrializeString` trait because this can already be handled with the
    regular `Deserialize` trait.

- `SerializeError` and `DeserializeError` types for handling errors.

    These represent several error conditions that can be encountered during the serialization and
    deserialization process, respectively.

[Unreleased]: https://github.com/brunoldsilva/rsomeip/compare/rsomeip-bytes-v0.1.0...HEAD
[0.1.0]: https://github.com/brunoldsilva/rsomeip/releases/tag/rsomeip-bytes-v0.1.0
