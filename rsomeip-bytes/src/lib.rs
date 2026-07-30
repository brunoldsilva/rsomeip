//! [![GitHub][github-badge]][github-url]
//! [![Crates.io][crates-io-badge]][crates-io-url]
//! [![Docs.rs][docsrs-badge]][docsrs-url]
//! ![license-badge]
//!
//! Serialization according to the SOME/IP on-wire format.
//!
//! This crate provides traits and types to assist in correctly implementing the serialization and
//! deserialization of types according to the [Open SOME/IP Specification][open-someip-spec].
//!
//! # Getting started
//!
//! 1. Add `rsomeip-bytes` as a dependency to your project.
//!
//!     ```toml
//!     # Cargo.toml
//!
//!     [dependencies]
//!     rsomeip-bytes = "0.2.0"
//!     ```
//!
//! 2. Implement [`Serialize`] and [`Deserialize`] for your data types.
//!
//!     ```rust
//!     # fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     use rsomeip_bytes::{
//!         Serialize, SerializeError, Deserialize, DeserializeError, bytes::{Buf, BufMut}
//!     };
//!
//!     /// Example of a composite type that is used in a SOME/IP message payload.
//!     #[derive(Debug, PartialEq, Eq)]
//!     struct Foo {
//!         bar: u8,
//!         baz: u16,
//!     }
//!
//!     impl Serialize for Foo {
//!         // This method is used to write the data to the buffer.
//!         fn serialize<Buffer>(&self, buffer: &mut Buffer) -> Result<usize, SerializeError>
//!         where
//!             Buffer: BufMut + ?Sized,
//!         {
//!             // Most basic types already implement `Serialize`.
//!             let mut size = 0;
//!             size += self.bar.serialize(buffer)?;
//!             size += self.baz.serialize(buffer)?;
//!             Ok(size)
//!         }
//!
//!         // This method is used for calculating the value of length fields, for example.
//!         fn size(&self) -> Option<usize> {
//!             // It's important that this value matches the size returned by the `serialize` method.
//!             let mut size = 0;
//!             size += self.bar.size()?;
//!             size += self.baz.size()?;
//!             Some(size)
//!         }
//!     }
//!
//!     impl Deserialize for Foo {
//!         type Output = Self;
//!
//!         // This method is used to read data from the buffer.
//!         fn deserialize<Buffer>(buffer: &mut Buffer) -> Result<Self::Output, DeserializeError>
//!         where
//!             Buffer: Buf + ?Sized,
//!         {
//!             // Like before, most basic types also implement `Deserialize`.
//!             let value = Self {
//!                 bar: u8::deserialize(buffer)?,
//!                 baz: u16::deserialize(buffer)?,
//!             };
//!             Ok(value)
//!         }
//!
//!         // This method is used as an estimate of the minimum amount of data required to deserialize
//!         // the output from the buffer.
//!         fn size_hint() -> Option<usize> {
//!             let mut size = 0;
//!             size += u8::size_hint()?;
//!             size += u16::size_hint()?;
//!             Some(size)
//!         }
//!     }
//!
//!     // A buffer can be any type that implements `Buf` and `BufMut`.
//!     let mut buffer = [0u8; 3];
//!
//!     // Use the `Serialize` trait to write data to the buffer.
//!     let value = Foo { bar: 0x01_u8, baz: 0x0203_u16 };
//!     assert_eq!(Some(3), value.size());
//!     assert_eq!(Ok(3), value.serialize(&mut buffer.as_mut_slice()));
//!
//!     // By default, data is serialized in Big Endian byte order.
//!     assert_eq!(buffer, [0x01_u8, 0x02, 0x03]);
//!
//!     // Use the `Deserialize` trait to read data from the buffer.
//!     assert_eq!(Some(3), Foo::size_hint());
//!     assert_eq!(Ok(value), Foo::deserialize(&mut buffer.as_slice()));
//!     # Ok(()) }
//!
//! # Usage
//!
//! The main goal with this crate is to have your types implement the [`Serialize`] and
//! [`Deserialize`] traits so that the other `rsomeip` crates can abstract the serialization and
//! deserialization of your types.
//!
//! To make it easier, this crate implements these traits for several types of the Rust standard
//! library and provides some convenient wrappers for those that don't. This is enough to cover most
//! use cases foreseen by the specification.
//!
//! ## Basic types
//!
//! All basic types defined in the specification implement [`Serialize`] and [`Deserialize`]. These
//! include:
//!
//! - [`bool`]
//! - [`u8`] and [`i8`]
//! - [`u16`] and [`i16`]
//! - [`u32`] and [`i32`]
//! - [`u64`] and [`i64`]
//! - [`f32`] and [`f64`]
//!
//! ### Endianess
//!
//! The [`Serialize::serialize`] and [`Deserialize::deserialize`] implementations for these types
//! use Big Endian byte orders for writing and reading data. If another endianess is desired, then
//! you must manually implement it yourself.
//!
//! ## Tuples
//!
//! There's a blanket implementation of both traits for any tuple whose types also implement
//! [`Serialize`] and [`Deserialize`].
//!
//! This can be used as a convenient way to call these methods on a bunch of elements all at once.
//!
//! ```rust
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! use rsomeip_bytes::{Serialize, Deserialize};
//!
//! let mut buffer = [0u8; 7];
//!
//! let value = (0x01_u8, 0x0203_u16, 0x0405_0607_u32);
//! assert_eq!(Ok(7), value.serialize(&mut buffer.as_mut_slice()));
//! assert_eq!(buffer, [0x01_u8, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07]);
//! assert_eq!(Ok(value), <(u8, u16, u32)>::deserialize(&mut buffer.as_slice()));
//! # Ok(()) }
//! ```
//!
//! ## Structs
//!
//! Structs required you to manually implement the [`Serialize`] and [`Deserialize`] traits as
//! described in the [Getting started](#getting-started) section.
//!
//! ## Dynamic arrays
//!
//! This crate doesn't implement any traits for container types of the standard library. Instead, it
//! provides the [`DynamicArray`] wrapper to serialize and deserialize any type that implements
//! [`IntoIterator`] and [`FromIterator`], respectively.
//!
//! This [`DynamicArray`] type takes a [`Length`] as a generic parameter to encode the size of the
//! array in a preceding length field.
//!
//! ```rust
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! use rsomeip_bytes::{Serialize, Deserialize, LengthU32, DynamicArray};
//!
//! let mut buffer = [0u8; 7];
//!
//! let value = vec![1_u8, 2, 3];
//! let array = DynamicArray::<LengthU32, _>::from(&value);
//! assert_eq!(Ok(7), array.serialize(&mut buffer.as_mut_slice()));
//! assert_eq!(buffer, [0x00_u8, 0x00, 0x00, 0x03, 0x01, 0x02, 0x03]); // Length before the payload.
//! assert_eq!(Ok(value), DynamicArray::<LengthU32, Vec<u8>>::deserialize(&mut buffer.as_slice()));
//! # Ok(()) }
//! ```
//!
//! ## Static arrays
//!
//! Static arrays use Rust's [`prim@array`] primitive since it translates directly into the SOME/IP
//! notion of an array.
//!
//! Since their size is fixed, static arrays don't require a length field.
//!
//! ```rust
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! use rsomeip_bytes::{Serialize, Deserialize};
//!
//! let mut buffer = [0u8; 4];
//!
//! let value = [1_u8, 2, 3, 4];
//! assert_eq!(Ok(4), value.serialize(&mut buffer.as_mut_slice()));
//! assert_eq!(buffer, [0x01_u8, 0x02, 0x03, 0x04]);
//! assert_eq!(Ok(value), <[u8; 4]>::deserialize(&mut buffer.as_slice()));
//! # Ok(()) }
//! ```
//!
//! ## Dynamic strings
//!
//! Like for dynamic arrays, this crate provides the [`DynamicString`] wrapper for any type that
//! implements [`AsRef<str>`] and [`From<String>`].
//!
//! Besides also accepting a [`Length`] parameter, this wrapper takes an [`Encoding`] parameter to
//! specify the encoding of the serialized string.
//!
//! Available encodings include [`Utf8`], [`Utf16BE`], and [`Utf16LE`].
//!
//! ```rust
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! use rsomeip_bytes::{Serialize, Deserialize, LengthU32, Utf8, DynamicString};
//!
//! let mut buffer = [0_u8; 15];
//!
//! let value = String::from("rsomeip");
//! let string = DynamicString::<LengthU32, Utf8, _>::from(&value);
//! assert_eq!(Ok(15), string.serialize(&mut buffer.as_mut_slice()));
//! assert_eq!(
//!     buffer,
//!     [
//!         0x00_u8, 0x00, 0x00, 0x0b, // Length
//!         0xef, 0xbb, 0xbf, // UTF-8 BOM
//!         0x72, 0x73, 0x6f, 0x6d, 0x65, 0x69, 0x70, // "rsomeip"
//!         0x00, // Delimiter
//!     ]
//! );
//! assert_eq!(
//!     Ok(value),
//!     DynamicString::<LengthU32, Utf8, String>::deserialize(&mut buffer.as_slice())
//! );
//! # Ok(()) }
//! ```
//!
//! ## Static strings
//!
//! These fill the same role as static arrays do for generic containers. The [`StaticString`]
//! wrapper takes an [`Encoding`] parameter like the dynamic strings, but the [`Length`] parameter
//! is dropped in favor of specifying a fixed size for the string.
//!
//! ```rust
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! use rsomeip_bytes::{Serialize, Deserialize, Utf8, StaticString};
//!
//! let mut buffer = [0_u8; 15];
//!
//! let value = String::from("rsomeip");
//! let string = StaticString::<15, Utf8, _>::from(&value);
//! assert_eq!(Ok(15), string.serialize(&mut buffer.as_mut_slice()));
//! assert_eq!(
//!     buffer,
//!     [
//!         0xef_u8, 0xbb, 0xbf, // UTF-8 BOM
//!         0x72, 0x73, 0x6f, 0x6d, 0x65, 0x69, 0x70, // "rsomeip"
//!         0x00, // Delimiter
//!         0x00, 0x00, 0x00, 0x00, // Padding
//!     ]
//! );
//! assert_eq!(
//!     Ok(value),
//!     StaticString::<15, Utf8, String>::deserialize(&mut buffer.as_slice())
//! );
//! # Ok(()) }
//! ```
//!
//! ## Enums and unions
//!
//! These need to be manually implemented by you.
//!
//! # License
//!
//! This project is licensed under either the [Apache-2.0 License] or [MIT License],
//! at your option.
//!
//! [Apache-2.0 License]: http://www.apache.org/licenses/LICENSE-2.0
//! [crates-io-badge]: https://img.shields.io/crates/v/rsomeip_bytes
//! [crates-io-url]: https://crates.io/crates/rsomeip-bytes
//! [docsrs-badge]: https://img.shields.io/docsrs/rsomeip-bytes
//! [docsrs-url]: https://docs.rs/rsomeip-bytes/latest/rsomeip_bytes/
//! [github-badge]: https://img.shields.io/badge/GitHub-rsomeip-blue
//! [github-url]: https://github.com/brunoldsilva/rsomeip
//! [license-badge]: https://img.shields.io/crates/l/rsomeip_bytes
//! [MIT License]: http://opensource.org/licenses/MIT
//! [open-someip-spec]: https://some-ip.com/standards.shtml

#![cfg_attr(not(feature = "std"), no_std)]
#![warn(clippy::std_instead_of_core, clippy::std_instead_of_alloc)]

extern crate alloc;
extern crate core;

// Re-export for convenience.
pub use bytes::{self, Buf, BufMut, Bytes, BytesMut};

mod array;
pub use array::DynamicArray;

mod de;
pub use de::{Deserialize, DeserializeError};

mod length;
pub use length::{
    DeserializeWithLength, Length, LengthU8, LengthU16, LengthU32, LengthZero, SerializeWithLength,
};

mod ser;
pub use ser::{Serialize, SerializeError, SerializeWithFn};

mod string;
pub use string::{DynamicString, Encoding, StaticString, Utf8, Utf16BE, Utf16LE};

#[cfg(doc)]
#[doc(hidden)]
#[doc = include_str!("../README.md")]
pub struct ReadMeCheck;
