//! Dynamic and static strings.
//!
//! This module provides the [`DynamicString`] and [`StaticString`] types from serializing and
//! deserializing strings from the SOME/IP on-wire format. It also provides the [`Encoding`] trait
//! and types for specifying the string encoding.

use crate::{Deserialize, DeserializeError, Serialize, SerializeError};
use alloc::{string::String, vec::Vec};
use bytes::{Buf, BufMut};
use core::marker::PhantomData;

/// Size of the string encoding in bytes.
///
/// Includes the size of the Byte Order Mark and the Delimiter.
///
/// It's the same for UTF-8 and UTF-16.
const ENCODING_SIZE: usize = 4;

/// String encoding according to the SOME/IP on-wire format.
pub trait Encoding: Sealed {
    /// Serializes the `value` into the given `buffer`.
    ///
    /// Includes a Byte Order Mark and a Delimiter before and after the actual string, respectively.
    ///
    /// The string must not contain any null characters.
    ///
    /// Returns the length of the serialized data.
    ///
    /// # Errors
    ///
    /// Returns a [`SerializeError`] if the serialization fails. Some data may still be written to
    /// the buffer if an error occurs.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// use rsomeip_bytes::{Encoding as _, Utf8};
    ///
    /// // The buffer can be any type that implements `BufMut`.
    /// let mut buffer = [0_u8; 11];
    /// let size = Utf8::serialize("rsomeip", &mut buffer.as_mut_slice())?;
    ///
    /// // Size includes the size of the Byte Order Mark, string and delimiter.
    /// assert_eq!(size, 11);
    /// assert_eq!(buffer.as_slice(), [
    ///         0xef_u8, 0xbb, 0xbf, // UTF-8 BOM
    ///         0x72, 0x73, 0x6f, 0x6d, 0x65, 0x69, 0x70, // "rsomeip"
    ///         0x00, // Delimiter
    ///     ].as_slice());
    /// # Ok(()) }
    /// ```
    fn serialize<Value, Buffer>(value: Value, buffer: &mut Buffer) -> Result<usize, SerializeError>
    where
        Value: AsRef<str>,
        Buffer: BufMut + ?Sized;

    /// Returns the size of the `value` when serialized.
    ///
    /// Includes the size of the Byte Order Mark and the Delimiter.
    ///
    /// Returns [`None`] if the size is out of bounds.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rsomeip_bytes::{Encoding as _, Utf8, Utf16BE, Utf16LE};
    ///
    /// // Size includes the size of the Byte Order Mark, string and delimiter.
    /// assert_eq!(Utf8::size("rsomeip"), Some(11));
    /// assert_eq!(Utf16BE::size("rsomeip"), Some(18));
    /// assert_eq!(Utf16LE::size("rsomeip"), Some(18));
    /// ```
    fn size<Value>(value: &Value) -> Option<usize>
    where
        Value: AsRef<str> + ?Sized;

    /// Deserializes a null-terminated [`String`] from the given `buffer`.
    ///
    /// Expects a Byte Order Mark and a Delimiter at the start and end of the string, respectively.
    ///
    /// # Errors
    ///
    /// Returns a [`DeserializeError`] if the deserialization fails.
    ///
    /// Specifically, deserialization fails if the string is not null-terminated or if there is a
    /// null character before the end of the string.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// use rsomeip_bytes::{Encoding as _, Utf8};
    ///
    /// // The buffer can be any type that implements `Buf`.
    /// let buffer = [
    ///     0xef_u8, 0xbb, 0xbf, // UTF-8 BOM
    ///     0x72, 0x73, 0x6f, 0x6d, 0x65, 0x69, 0x70, // "rsomeip"
    ///     0x00, // Delimiter
    /// ];
    ///
    /// let string = Utf8::deserialize(&mut buffer.as_slice())?;
    /// assert_eq!(&string, "rsomeip");
    /// # Ok(()) }
    /// ```
    fn deserialize<Buffer>(buffer: &mut Buffer) -> Result<String, DeserializeError>
    where
        Buffer: Buf + ?Sized;
}

/// Sealed trait to prevent external implementations.
pub trait Sealed {}

/// UTF-8 encoding for strings.
///
/// Normally, used with [`StaticString`] and [`DynamicString`].
///
/// # Examples
///
/// ```rust
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// use rsomeip_bytes::{StaticString, Utf8, Serialize as _, Deserialize as _};
///
/// // A `StaticString` can be used to always write the same amount of data regardless
/// // of the size of the string.
/// let value = String::from("rsomeip");
/// let string = StaticString::<11, Utf8, _>::from(&value);
///
/// // Buffer can be any type that implements `BufMut`.
/// let mut buffer = [0_u8; 11];
///
/// // Serialized data includes Byte Order Mark and Delimiter.
/// assert_eq!(Ok(11), string.serialize(&mut buffer.as_mut_slice()));
/// assert_eq!(buffer, [
///         0xef_u8, 0xbb, 0xbf, // UTF-8 BOM
///         0x72, 0x73, 0x6f, 0x6d, 0x65, 0x69, 0x70, // "rsomeip"
///         0x00, // Delimiter
///     ]);
///
/// // Deserialization automatically strips the BOM and Delimiter.
/// let output = StaticString::<11, Utf8, String>::deserialize(&mut buffer.as_slice())?;
/// assert_eq!(output, value);
/// # Ok(()) }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Utf8;

impl Encoding for Utf8 {
    fn serialize<Value, Buffer>(value: Value, buffer: &mut Buffer) -> Result<usize, SerializeError>
    where
        Value: AsRef<str>,
        Buffer: BufMut + ?Sized,
    {
        let string = value.as_ref();

        // UTF-8 Byte Order Mark + String + Delimiter
        buffer.put_slice(&[0xef_u8, 0xbb, 0xbf]);
        buffer.put_slice(string.as_bytes()); // The string is already UTF-8 encoded.
        buffer.put_u8(0x00);

        // Total size.
        Self::size(&string).ok_or(SerializeError::SizeOverflow)
    }

    fn size<Value>(value: &Value) -> Option<usize>
    where
        Value: AsRef<str> + ?Sized,
    {
        // The string is already UTF-8 encoded.
        value.as_ref().len().checked_add(ENCODING_SIZE)
    }

    fn deserialize<Buffer>(buffer: &mut Buffer) -> Result<String, DeserializeError>
    where
        Buffer: Buf + ?Sized,
    {
        // Extract the Byte Order Mark.
        if <[u8; 3]>::deserialize(buffer)? != [0xef_u8, 0xbb, 0xbf] {
            return Err(DeserializeError::invariant("invalid UTF-8 Byte Order Mark"));
        }
        // Extract the remainder of the buffer to a vector.
        let mut raw = Vec::new();
        raw.put(buffer);
        // Check if the string is correctly null terminated.
        let trimmed = remove_delimiters(raw, 0)?;
        // Convert the vector into an UTF-8 string.
        String::from_utf8(trimmed)
            .map_err(|_err| DeserializeError::invariant("invalid UTF-8 string"))
    }
}

impl Sealed for Utf8 {}

/// UTF-16 Little Endian encoding for strings.
///
/// Normally, used with [`StaticString`] and [`DynamicString`].
///
/// # Examples
///
/// ```rust
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// use rsomeip_bytes::{StaticString, Utf16LE, Serialize as _, Deserialize as _};
///
/// // A `StaticString` can be used to always write the same amount of data regardless
/// // of the size of the string.
/// let value = String::from("rsomeip");
/// let string = StaticString::<18, Utf16LE, _>::from(&value);
///
/// // Buffer can be any type that implements `BufMut`.
/// let mut buffer = [0_u8; 18];
///
/// // Serialized data includes Byte Order Mark and Delimiter.
/// assert_eq!(Ok(18), string.serialize(&mut buffer.as_mut_slice()));
/// assert_eq!(buffer, [
///         0xff, 0xfe, // UTF-16LE BOM
///         0x72, 0, 0x73, 0, 0x6f, 0, 0x6d, 0, 0x65, 0, 0x69, 0, 0x70, 0, // "rsomeip"
///         0x00, 0x00 // Delimiter
///     ]);
///
/// // Deserialization automatically strips the BOM and Delimiter.
/// let output = StaticString::<18, Utf16LE, String>::deserialize(&mut buffer.as_slice())?;
/// assert_eq!(output, value);
/// # Ok(()) }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Utf16LE;

impl Encoding for Utf16LE {
    fn serialize<Value, Buffer>(value: Value, buffer: &mut Buffer) -> Result<usize, SerializeError>
    where
        Value: AsRef<str>,
        Buffer: BufMut + ?Sized,
    {
        let string = value.as_ref();
        let mut size = ENCODING_SIZE; // BOM + Delimiter

        // UTF-16 Byte Order Mark + String + Delimiter
        buffer.put_u16_le(0xfeff);
        for charater in string.encode_utf16() {
            buffer.put_u16_le(charater);
            // Update the total size of the string.
            size = size
                .checked_add(size_of::<u16>())
                .ok_or(SerializeError::SizeOverflow)?;
        }
        buffer.put_u16_le(0x0000);

        // Return the total size.
        Ok(size)
    }

    fn size<Value>(value: &Value) -> Option<usize>
    where
        Value: AsRef<str> + ?Sized,
    {
        value
            .as_ref()
            .encode_utf16()
            .count()
            .checked_mul(size_of::<u16>())
            .and_then(|size| size.checked_add(ENCODING_SIZE))
    }

    fn deserialize<Buffer>(buffer: &mut Buffer) -> Result<String, DeserializeError>
    where
        Buffer: Buf + ?Sized,
    {
        // Extract the Byte Order Mark.
        if 0xfffe != u16::deserialize(buffer)? {
            return Err(DeserializeError::invariant(
                "invalid UTF-16LE Byte Order Mark",
            ));
        }
        // Extract the remainder of the buffer to a vector.
        let raw = deserialize_raw_utf16(buffer, |buffer| {
            buffer
                .try_get_u16_le()
                .map_err(|_err| DeserializeError::InsufficientData)
        })?;
        // Check if the string is correctly null terminated.
        let trimmed = remove_delimiters(raw, 0)?;
        // Convert the vector into an UTF-16 string.
        String::from_utf16(&trimmed)
            .map_err(|_err| DeserializeError::invariant("invalid UTF-16LE string"))
    }
}

impl Sealed for Utf16LE {}

/// UTF-16 Big Endian encoding for strings.
///
/// Normally, used with [`StaticString`] and [`DynamicString`].
///
/// # Examples
///
/// ```rust
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// use rsomeip_bytes::{StaticString, Utf16BE, Serialize as _, Deserialize as _};
///
/// // A `StaticString` can be used to always write the same amount of data regardless
/// // of the size of the string.
/// let value = String::from("rsomeip");
/// let string = StaticString::<18, Utf16BE, _>::from(&value);
///
/// // Buffer can be any type that implements `BufMut`.
/// let mut buffer = [0_u8; 18];
///
/// // Serialized data includes Byte Order Mark and Delimiter.
/// assert_eq!(Ok(18), string.serialize(&mut buffer.as_mut_slice()));
/// assert_eq!(buffer, [
///         0xfe, 0xff, // UTF-16BE BOM
///         0, 0x72, 0, 0x73, 0, 0x6f, 0, 0x6d, 0, 0x65, 0, 0x69, 0, 0x70, // "rsomeip"
///         0x00, 0x00 // Delimiter
///     ]);
///
/// // Deserialization automatically strips the BOM and Delimiter.
/// let output = StaticString::<18, Utf16BE, String>::deserialize(&mut buffer.as_slice())?;
/// assert_eq!(output, value);
/// # Ok(()) }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Utf16BE;

impl Encoding for Utf16BE {
    fn serialize<Value, Buffer>(value: Value, buffer: &mut Buffer) -> Result<usize, SerializeError>
    where
        Value: AsRef<str>,
        Buffer: BufMut + ?Sized,
    {
        let string = value.as_ref();
        let mut size = 4_usize; // BOM + Delimiter

        // UTF-16 Byte Order Mark + String + Delimiter
        buffer.put_u16(0xfeff);
        for charater in string.encode_utf16() {
            buffer.put_u16(charater);
            // Update the total size of the string.
            size = size
                .checked_add(size_of::<u16>())
                .ok_or(SerializeError::SizeOverflow)?;
        }
        buffer.put_u16(0x0000);

        // Return the total size.
        Ok(size)
    }

    fn size<Value>(value: &Value) -> Option<usize>
    where
        Value: AsRef<str> + ?Sized,
    {
        // Same as Little Endian.
        Utf16LE::size(value)
    }

    fn deserialize<Buffer>(buffer: &mut Buffer) -> Result<String, DeserializeError>
    where
        Buffer: Buf + ?Sized,
    {
        // Extract the Byte Order Mark.
        if 0xfeff != u16::deserialize(buffer)? {
            return Err(DeserializeError::invariant(
                "invalid UTF-16BE Byte Order Mark",
            ));
        }
        // Extract the remainder of the buffer to a vector.
        let raw = deserialize_raw_utf16(buffer, u16::deserialize)?;
        // Check if the string is correctly null terminated.
        let trimmed = remove_delimiters(raw, 0)?;
        // Convert the vector into an UTF-16 string.
        String::from_utf16(&trimmed)
            .map_err(|_err| DeserializeError::invariant("invalid UTF-16BE string"))
    }
}

impl Sealed for Utf16BE {}

/// Extracts a raw UTF-16 string from the given `buffer`.
///
/// Removes odd padding from the end of the string.
///
/// # Errors
///
/// Returns a [`DeserializeError`] if the deserialization fails or if the odd byte isn't null.
fn deserialize_raw_utf16<Buffer, Deserializer>(
    buffer: &mut Buffer,
    deserialize: Deserializer,
) -> Result<Vec<u16>, DeserializeError>
where
    Buffer: Buf + ?Sized,
    Deserializer: Fn(&mut Buffer) -> Result<u16, DeserializeError>,
{
    let mut raw = Vec::new();
    loop {
        match buffer.remaining() {
            // Nothing to extract.
            0 => break,
            // Only one byte remaining.
            1 => {
                // Must be padding.
                if u8::deserialize(buffer)? != 0 {
                    return Err(DeserializeError::invariant("not null terminated"));
                }
            }
            // Two or more bytes remaining. Put them in the Vec.
            _ => raw.push(deserialize(buffer)?),
        }
    }
    Ok(raw)
}

/// Extracts the raw string from the given `input` and removes any trailing `delimiter`.
///
/// # Errors
///
/// Returns a [`DeserializeError::InvariantFailed`] if the string doesn't have a delimiter or if
/// there is a delimiter in the middle of the string.
fn remove_delimiters<T>(mut input: Vec<T>, delimiter: T) -> Result<Vec<T>, DeserializeError>
where
    T: PartialEq + Copy,
{
    input
        .iter()
        .position(|&elem| elem == delimiter)
        .ok_or_else(|| DeserializeError::invariant("not null terminated"))
        .and_then(|position| {
            // Trim the delimiter.
            let remainder = input.split_off(position);
            // Check for non-null data after the delimiter.
            if !remainder.is_empty() && remainder.iter().any(|&elem| elem != delimiter) {
                return Err(DeserializeError::invariant(
                    "null byte before end of string",
                ));
            }
            Ok(input)
        })
}

/// Dynamically sized string.
///
/// Encodes the size of the string in a preceding length field.
///
/// # Examples
///
/// ```rust
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// use rsomeip_bytes::{DynamicString, LengthU32, Utf8, Serialize as _, Deserialize as _};
///
/// // A `DynamicString` encodes the size of the string in a preceeding length field.
/// let value = String::from("rsomeip");
/// let string = DynamicString::<LengthU32, Utf8, _>::from(&value);
///
/// // Size includes the length field, BOM, string, and delimiter.
/// assert_eq!(Some(15), string.size());
/// // Size hint only includes the length field.
/// assert_eq!(Some(4), DynamicString::<LengthU32, Utf8, String>::size_hint());
///
/// // Buffer can be any type that implements `BufMut`.
/// let mut buffer = [0_u8; 15];
///
/// // Serialized data includes Length, Byte Order Mark and Delimiter.
/// assert_eq!(Ok(15), string.serialize(&mut buffer.as_mut_slice()));
/// assert_eq!(buffer, [
///         0x00_u8, 0x00, 0x00, 0x0b, // Length
///         0xef, 0xbb, 0xbf, // UTF-8 BOM
///         0x72, 0x73, 0x6f, 0x6d, 0x65, 0x69, 0x70, // "rsomeip"
///         0x00, // Delimiter
///     ]);
///
/// // Deserialization automatically strips the length, BOM, and delimiters.
/// let output = DynamicString::<LengthU32, Utf8, String>::deserialize(&mut buffer.as_slice())?;
/// assert_eq!(output, value);
/// # Ok(()) }
/// ```
pub struct DynamicString<Length, Encoding, Value> {
    /// String to serialize.
    inner: Value,
    /// Length to include before the string.
    _length: PhantomData<Length>,
    /// Encoding to use when serializing/deserializing the string.
    _encoding: PhantomData<Encoding>,
}

impl<Length, Encoding, Value> From<Value> for DynamicString<Length, Encoding, Value>
where
    Length: crate::Length,
    Encoding: crate::Encoding,
    Value: AsRef<str>,
{
    fn from(value: Value) -> Self {
        Self {
            inner: value,
            _length: PhantomData,
            _encoding: PhantomData,
        }
    }
}

impl<Length, Encoding, Value> Serialize for DynamicString<Length, Encoding, Value>
where
    Length: crate::Length,
    Encoding: crate::Encoding,
    Value: AsRef<str>,
{
    fn serialize<Buffer>(&self, buffer: &mut Buffer) -> Result<usize, crate::SerializeError>
    where
        Buffer: bytes::BufMut + ?Sized,
    {
        let wrapper = crate::SerializeWithFn::new(
            &self.inner,
            |value: &Value, buf: &mut dyn bytes::BufMut| Encoding::serialize(value, buf),
            |value: &Value| Encoding::size(value),
        );
        Length::serialize(&wrapper, buffer)
    }

    fn size(&self) -> Option<usize> {
        Encoding::size(&self.inner).and_then(|size| size.checked_add(Length::size()))
    }
}

impl<Length, Encoding, Value> Deserialize for DynamicString<Length, Encoding, Value>
where
    Length: crate::Length,
    Encoding: crate::Encoding,
    Value: From<String>,
{
    type Output = Value;

    fn deserialize<Buffer>(buffer: &mut Buffer) -> Result<Self::Output, crate::DeserializeError>
    where
        Buffer: bytes::Buf + ?Sized,
    {
        Length::deserialize_with(
            |buffer| Encoding::deserialize(buffer).map(|value| Value::from(value)),
            buffer,
        )
    }

    fn size_hint() -> Option<usize> {
        Some(Length::size())
    }
}

/// Statically sizes string.
///
/// Serialized static strings always have the same length regardless of the size of the actual
/// string.
///
/// Extra space is padded with delimiters.
///
/// # Examples
///
/// ```rust
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// use rsomeip_bytes::{StaticString, Utf8, Serialize as _, Deserialize as _};
///
/// // A `StaticString` can be used to always write the same amount of data regardless
/// // of the size of the string.
/// let value = String::from("rsomeip");
/// let string = StaticString::<15, Utf8, _>::from(&value);
///
/// // Size is the capacity of the underlying array.
/// assert_eq!(Some(15), string.size());
/// assert_eq!(Some(15), StaticString::<15, Utf8, String>::size_hint());
///
/// // Buffer can be any type that implements `BufMut`.
/// let mut buffer = [0_u8; 15];
///
/// // Serialized data includes Byte Order Mark, delimiter, and padding.
/// assert_eq!(Ok(15), string.serialize(&mut buffer.as_mut_slice()));
/// assert_eq!(buffer, [
///         0xef_u8, 0xbb, 0xbf, // UTF-8 BOM
///         0x72, 0x73, 0x6f, 0x6d, 0x65, 0x69, 0x70, // "rsomeip"
///         0x00, // Delimiter
///         0x00, 0x00, 0x00, 0x00, // Padding
///     ]);
///
/// // Deserialization automatically strips the BOM, delimiter, and padding.
/// let output = StaticString::<15, Utf8, String>::deserialize(&mut buffer.as_slice())?;
/// assert_eq!(output, value);
/// # Ok(()) }
/// ```
pub struct StaticString<const N: usize, Encoding, Value> {
    /// String to serialize.
    value: Value,
    /// Size of the array.
    _length: PhantomData<[(); N]>,
    /// Encoding to use when serializing/deserializing the string.
    _encoding: PhantomData<Encoding>,
}

impl<Encoding, Value, const N: usize> From<Value> for StaticString<N, Encoding, Value>
where
    Encoding: crate::Encoding,
    Value: AsRef<str>,
{
    fn from(value: Value) -> Self {
        Self {
            value,
            _length: PhantomData,
            _encoding: PhantomData,
        }
    }
}

impl<Encoding, Value, const N: usize> Serialize for StaticString<N, Encoding, Value>
where
    Encoding: crate::Encoding,
    Value: AsRef<str>,
{
    fn serialize<Buffer>(&self, buffer: &mut Buffer) -> Result<usize, SerializeError>
    where
        Buffer: BufMut + ?Sized,
    {
        // Check if the string fits in the array.
        if Encoding::size(&self.value).ok_or(SerializeError::SizeOverflow)? > N {
            return Err(SerializeError::invariant(
                "string exceeds static array capacity",
            ));
        }
        // Limit the amount that can be written into the buffer.
        let mut limit = buffer.limit(N);
        // Write the string into the buffer.
        _ = Encoding::serialize(&self.value, &mut limit)?;
        // Fill the remaining space with '0'.
        limit.put_bytes(0, limit.remaining_mut());
        // Return the size of the array.
        Ok(N)
    }

    fn size(&self) -> Option<usize> {
        Some(N)
    }
}

impl<Encoding, Value, const N: usize> Deserialize for StaticString<N, Encoding, Value>
where
    Encoding: crate::Encoding,
    Value: From<String>,
{
    type Output = Value;

    fn deserialize<Buffer>(buffer: &mut Buffer) -> Result<Self::Output, DeserializeError>
    where
        Buffer: Buf + ?Sized,
    {
        Encoding::deserialize(buffer).map(Value::from)
    }

    fn size_hint() -> Option<usize> {
        Some(N)
    }
}

#[cfg(test)]
#[expect(clippy::inline_modules, reason = "rust-clippy#17342")]
mod tests {
    use super::*;
    use bytes::BytesMut;

    #[test]
    fn utf8_delimiter_checks() {
        let buffer = [
            0xef_u8, 0xbb, 0xbf, // UTF-8 BOM
            0x72, 0x73, 0x6f, 0x6d, 0x65, 0x69, 0x70, // "rsomeip"
            0x00, // Delimiter
            0x62, 0x79, 0x74, 0x65, 0x73, // "bytes"
            0x00, // Delimiter
        ];
        assert_eq!(
            <Utf8>::deserialize(&mut buffer.get(0..10).expect("should slice the buffer")),
            Err(DeserializeError::invariant("not null terminated"))
        );
        assert_eq!(
            <Utf8>::deserialize(&mut buffer.as_slice()),
            Err(DeserializeError::invariant(
                "null byte before end of string"
            ))
        );
    }

    #[test]
    fn utf16be_delimiter_checks() {
        let buffer = [
            0xfe_u8, 0xff, // UTF-16 BE BOM
            0, 0x72, 0, 0x73, 0, 0x6f, 0, 0x6d, 0, 0x65, 0, 0x69, 0, 0x70, // "rsomeip"
            0x00, 0x00, // Delimiter
            0, 0x62, 0, 0x79, 0, 0x74, 0, 0x65, 0, 0x73, // "bytes"
            0x00, 0x00, // Delimiter
        ];
        assert_eq!(
            <Utf16BE>::deserialize(&mut buffer.get(0..16).expect("should slice the buffer")),
            Err(DeserializeError::invariant("not null terminated"))
        );
        assert_eq!(
            <Utf16BE>::deserialize(&mut buffer.as_slice()),
            Err(DeserializeError::invariant(
                "null byte before end of string"
            ))
        );
    }

    #[test]
    fn utf16le_delimiter_checks() {
        let buffer = [
            0xff_u8, 0xfe, // UTF-16LE BOM
            0x72, 0, 0x73, 0, 0x6f, 0, 0x6d, 0, 0x65, 0, 0x69, 0, 0x70, 0, // "rsomeip"
            0x00, 0x00, // Delimiter
            0x62, 0, 0x79, 0, 0x74, 0, 0x65, 0, 0x73, 0, // "bytes"
            0x00, 0x00, // Delimiter
        ];
        assert_eq!(
            <Utf16LE>::deserialize(&mut buffer.get(0..16).expect("should slice the buffer")),
            Err(DeserializeError::invariant("not null terminated"))
        );
        assert_eq!(
            <Utf16LE>::deserialize(&mut buffer.as_slice()),
            Err(DeserializeError::invariant(
                "null byte before end of string"
            ))
        );
    }

    #[test]
    fn utf16be_odd_padding() {
        let buffer = [
            0xfe_u8, 0xff, // UTF-16 BE BOM
            0, 0x72, 0, 0x73, 0, 0x6f, 0, 0x6d, 0, 0x65, 0, 0x69, 0, 0x70, // "rsomeip"
            0x00, 0x00, // Delimiter
            0x00, // Padding
        ];
        assert_eq!(
            Utf16BE::deserialize(&mut buffer.as_slice()),
            Ok(String::from("rsomeip"))
        );
    }

    #[test]
    fn utf16le_odd_padding() {
        let buffer = [
            0xff_u8, 0xfe, // UTF-16LE BOM
            0x72, 0, 0x73, 0, 0x6f, 0, 0x6d, 0, 0x65, 0, 0x69, 0, 0x70, 0, // "rsomeip"
            0x00, 0x00, // Delimiter
            0x00, // Padding
        ];
        assert_eq!(
            Utf16LE::deserialize(&mut buffer.as_slice()),
            Ok(String::from("rsomeip"))
        );
    }

    #[test]
    fn utf16be_invalid_padding() {
        let buffer = [
            0xfe_u8, 0xff, // UTF-16 BE BOM
            0, 0x72, 0, 0x73, 0, 0x6f, 0, 0x6d, 0, 0x65, 0, 0x69, 0, 0x70, // "rsomeip"
            0x00, 0x00, // Delimiter
            0x01, // Padding
        ];
        assert_eq!(
            Utf16BE::deserialize(&mut buffer.as_slice()),
            Err(DeserializeError::invariant("not null terminated"))
        );
    }

    #[test]
    fn utf16le_invalid_padding() {
        let buffer = [
            0xff_u8, 0xfe, // UTF-16LE BOM
            0x72, 0, 0x73, 0, 0x6f, 0, 0x6d, 0, 0x65, 0, 0x69, 0, 0x70, 0, // "rsomeip"
            0x00, 0x00, // Delimiter
            0x01, // Padding
        ];
        assert_eq!(
            Utf16LE::deserialize(&mut buffer.as_slice()),
            Err(DeserializeError::invariant("not null terminated"))
        );
    }

    #[test]
    fn utf8_bom_check() {
        let buffer = [
            0xef_u8, 0x00, 0xbf, // Invalid BOM
            0x00, // Delimiter
        ];
        assert_eq!(
            Utf8::deserialize(&mut buffer.as_slice()),
            Err(DeserializeError::invariant("invalid UTF-8 Byte Order Mark"))
        );
    }

    #[test]
    fn utf16be_bom_check() {
        let buffer = [
            0xff_u8, 0x00, // Invalid BOM
            0x00, 0x00, // Delimiter
        ];
        assert_eq!(
            Utf16BE::deserialize(&mut buffer.as_slice()),
            Err(DeserializeError::invariant(
                "invalid UTF-16BE Byte Order Mark"
            ))
        );
    }

    #[test]
    fn utf16le_bom_check() {
        let buffer = [
            0xfe_u8, 0x00, // Invalid BOM
            0x00, 0x00, // Delimiter
        ];
        assert_eq!(
            Utf16LE::deserialize(&mut buffer.as_slice()),
            Err(DeserializeError::invariant(
                "invalid UTF-16LE Byte Order Mark"
            ))
        );
    }

    #[test]
    fn utf8_empty_string() {
        let buffer = [
            0xef_u8, 0xbb, 0xbf, // UTF-8 BOM
            0x00, // Delimiter
        ];
        assert_eq!(Utf8::deserialize(&mut buffer.as_slice()), Ok(String::new()));
    }

    #[test]
    fn utf16be_empty_string() {
        let buffer = [
            0xfe_u8, 0xff, // UTF-16 BE BOM
            0x00, 0x00, // Delimiter
        ];
        assert_eq!(
            Utf16BE::deserialize(&mut buffer.as_slice()),
            Ok(String::new())
        );
    }

    #[test]
    fn utf16le_empty_string() {
        let buffer = [
            0xff_u8, 0xfe, // UTF-16LE BOM
            0x00, 0x00, // Delimiter
        ];
        assert_eq!(
            Utf16LE::deserialize(&mut buffer.as_slice()),
            Ok(String::new())
        );
    }

    #[test]
    fn static_string_exceeds_capacity() {
        assert_eq!(
            StaticString::<4, Utf8, _>::from("rsomeip").serialize(&mut BytesMut::with_capacity(11)),
            Err(SerializeError::invariant(
                "string exceeds static array capacity"
            ))
        );
    }
}
