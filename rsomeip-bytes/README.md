# rsomeip-bytes

[![GitHub][github-badge]][github-url]
[![Crates.io][crates-io-badge]][crates-io-url]
[![Docs.rs][docsrs-badge]][docsrs-url]
![license-badge]

Serialization according to the SOME/IP on-wire format.

This crate provides traits and types to assist in correctly implementing the serialization and
deserialization of data types according to the [Open SOME/IP Specification][open-someip-spec].

## Getting started

1. Add `rsomeip-bytes` as a dependency to your project.

    ```toml
    # Cargo.toml

    [dependencies]
    rsomeip-bytes = "0.2.0"
    ```

2. Implement `Serialize` and `Deserialize` for your data types.

    ```rust
    use rsomeip_bytes::{
        Serialize, SerializeError, Deserialize, DeserializeError, bytes::{Buf, BufMut}
    };

    /// Example of a composite type that is used in a SOME/IP message payload.
    #[derive(Debug, PartialEq, Eq)]
    struct Foo {
        bar: u8,
        baz: u16,
    }

    impl Serialize for Foo {
        // This method is used to write the data to the buffer.
        fn serialize<Buffer>(&self, buffer: &mut Buffer) -> Result<usize, SerializeError>
        where
            Buffer: BufMut + ?Sized,
        {
            // Most basic types already implement `Serialize`.
            let mut size = 0;
            size += self.bar.serialize(buffer)?;
            size += self.baz.serialize(buffer)?;
            Ok(size)
        }

        // This method is used for calculating the value of length fields, for example.
        fn size(&self) -> Option<usize> {
            // It's important that this value matches the size returned by the `serialize` method.
            let mut size = 0;
            size += self.bar.size()?;
            size += self.baz.size()?;
            Some(size)
        }
    }

    impl Deserialize for Foo {
        type Output = Self;

        // This method is used to read data from the buffer.
        fn deserialize<Buffer>(buffer: &mut Buffer) -> Result<Self::Output, DeserializeError>
        where
            Buffer: Buf + ?Sized,
        {
            // Like before, most basic types also implement `Deserialize`.
            let value = Self {
                bar: u8::deserialize(buffer)?,
                baz: u16::deserialize(buffer)?,
            };
            Ok(value)
        }

        // This method is used as an estimate of the minimum amount of data required to deserialize
        // the output from the buffer.
        fn size_hint() -> Option<usize> {
            let mut size = 0;
            size += u8::size_hint()?;
            size += u16::size_hint()?;
            Some(size)
        }
    }

    // A buffer can be any type that implements `Buf` and `BufMut`.
    let mut buffer = [0u8; 3];

    // Use the `Serialize` trait to write data to the buffer.
    let value = Foo { bar: 0x01_u8, baz: 0x0203_u16 };
    assert_eq!(Some(3), value.size());
    assert_eq!(Ok(3), value.serialize(&mut buffer.as_mut_slice()));

    // By default, data is serialized in Big Endian byte order.
    assert_eq!(buffer, [0x01_u8, 0x02, 0x03]);

    // Use the `Deserialize` trait to read data from the buffer.
    assert_eq!(Some(3), Foo::size_hint());
    assert_eq!(Ok(value), Foo::deserialize(&mut buffer.as_slice()));
    ```

## License

This project is licensed under either the [Apache-2.0 License] or [MIT License],
at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual
licensed as above, without any additional terms or conditions.

[Apache-2.0 License]: http://www.apache.org/licenses/LICENSE-2.0
[crates-io-badge]: https://img.shields.io/crates/v/rsomeip_bytes
[crates-io-url]: https://crates.io/crates/rsomeip-bytes
[docsrs-badge]: https://img.shields.io/docsrs/rsomeip-bytes
[docsrs-url]: https://docs.rs/rsomeip-bytes/latest/rsomeip_bytes/
[github-badge]: https://img.shields.io/badge/GitHub-rsomeip-blue
[github-url]: https://github.com/brunoldsilva/rsomeip
[license-badge]: https://img.shields.io/crates/l/rsomeip_bytes
[MIT License]: http://opensource.org/licenses/MIT
[open-someip-spec]: https://some-ip.com/standards.shtml
