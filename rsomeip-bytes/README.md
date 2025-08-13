# rsomeip-bytes

A serialization crate for the SOME/IP on-wire format.

## Overview

This crate provides traits for the serialization and deserialization of data
types according to the SOME/IP protocol.

It also provides implementation of said traits for all basic types supported by
the protocol, as well as for some commonly used types from Rust's standard
library.

## Getting Started

1. Add `rsomeip-bytes` as a dependency to your project.

   ```toml
   # Cargo.toml

   [dependencies]
   rsomeip-bytes = "0.1.0"
   ```

2. Use the [`Serialize`] and [`Deserialize`] traits to write and read data.

   ```rust
   use rsomeip_bytes::{Serialize, Deserialize, BytesMut, LengthField};

   // Write data into a buffer using the `serialize` method.
   let mut buffer = BytesMut::new();
   1u8.serialize(&mut buffer).unwrap();

   // Use `serialize_len` method for dynamically sized types.
   let dyn_data = vec![1u8, 2];
   dyn_data.serialize_len(LengthField::U8, &mut buffer).unwrap();

   // Read data from a buffer using the `deserialize` method.
   let mut buffer = buffer.freeze();
   assert_eq!(u8::deserialize(&mut buffer), Ok(1u8));

   // Use `deserialize_len` for dynamically sized types.
   let dyn_data = Vec::<u8>::deserialize_len(LengthField::U8, &mut buffer).unwrap();
   assert_eq!(&dyn_data[..], &[1u8, 2]);
   ```

3. Implement the traits for your custom types.

   ```rust
   use rsomeip_bytes::{
       Serialize, Deserialize, Buf, BufMut, SerializeError, DeserializeError,
       LengthField
   };

   struct Foo {
       a: u8,
       b: Vec<u8>,
   }

   impl Serialize for Foo {
       fn serialize(&self, buffer: &mut impl BufMut) -> Result<usize, SerializeError> {
           let mut size = 0;
           size += self.a.serialize(buffer)?;
           size += self.b.serialize_len(LengthField::U8, buffer)?;
           Ok(size)
       }

       fn size_hint(&self) -> usize {
           let mut size = 0;
           size += self.a.size_hint();
           size += 0u8.size_hint(); // Remember to include the size of the length field.
           size += self.b.size_hint();
           size
       }
   }

   impl Deserialize for Foo {
       type Output = Self;

       fn deserialize(buffer: &mut impl Buf) -> Result<Self, DeserializeError> {
           Ok(Foo{
               a: u8::deserialize(buffer)?,
               b: Vec::deserialize_len(LengthField::U8, buffer)?,
           })
       }
   }
   ```

## Motivation

The SOME/IP protocol specifies a list of supported data types and how to
represent those types on wire.

This crate aims to enable Rust projects to represent their data types using this
format.

## License

This project is licensed under either the [Apache-2.0 License] or [MIT License],
at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.

[Apache-2.0 License]: http://www.apache.org/licenses/LICENSE-2.0
[MIT License]: http://opensource.org/licenses/MIT
