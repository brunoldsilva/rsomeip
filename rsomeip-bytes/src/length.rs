//! Length field.
//!
//! This module provides the definition of the SOME/IP length that is used to encode the length of
//! dynamically sized payloads.

use crate::{Deserialize, DeserializeError, Serialize, SerializeError};
use bytes::{Buf, BufMut};
use core::marker::PhantomData;

/// Length of a value.
///
/// Encodes the length of a dynamically sized type in the SOME/IP on-wire format.
pub trait Length: Sealed {
    /// Serializes the `value` into the given `buffer` with a length field.
    ///
    /// Returns the size of the serialized data in bytes, including the length field itself.
    ///
    /// # Errors
    ///
    /// Returns a [`SerializeError`] if the serialization fails. Some data may still be written to
    /// the buffer if an error occurs.
    ///
    /// # Panics
    ///
    /// Panics if the buffer doesn't have enough capacity for the serialized type. It's advised to
    /// ensure that the buffer has at least [`Serialize::size`] capacity.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// use rsomeip_bytes::{LengthU32, Length as _, BytesMut};
    ///
    /// let mut buffer = BytesMut::with_capacity(6);
    ///
    /// // Serialize a value with a 32-bit length field into the buffer.
    /// let size = LengthU32::serialize(&0x0102_u16, &mut buffer)?;
    ///
    /// // Size includes length and value.
    /// assert_eq!(size, 6);
    ///
    /// // Length field comes before the value.
    /// assert_eq!(&buffer.freeze(), [0_u8, 0, 0, 2, 1, 2].as_slice());
    /// # Ok(()) }
    /// ```
    fn serialize<Buffer, Value>(
        value: &Value,
        buffer: &mut Buffer,
    ) -> Result<usize, SerializeError>
    where
        Buffer: BufMut + ?Sized,
        Value: Serialize;

    /// Serializes the `value` into the `buffer` with a length field using the given functions.
    ///
    /// Returns the size of the serialized data in bytes, including the length field itself.
    ///
    /// # Errors
    ///
    /// Returns a [`SerializeError`] if the serialization fails. Some data may still be written to
    /// the buffer if an error occurs.
    ///
    /// # Panics
    ///
    /// Panics if the buffer doesn't have enough capacity for the serialized type. It's advised to
    /// ensure that the buffer has at least [`Serialize::size`] capacity.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// use rsomeip_bytes::{LengthU32, Length as _, BytesMut, Serialize as _};
    ///
    /// // Type requiring custom serialization.
    /// struct Foo {
    ///     bar: u8,
    ///     baz: u16,
    /// }
    ///
    /// let mut buffer = BytesMut::with_capacity(7);
    ///
    /// // Serialize a value with a 32-bit length field into the buffer.
    /// let size = LengthU32::serialize_with(
    ///     |value, mut buffer| Ok(value.bar.serialize(buffer)? + value.baz.serialize(buffer)?),
    ///     |value| Some(value.bar.size()? + value.baz.size()?),
    ///     Foo { bar: 1_u8, baz: 0x0203_u16 },
    ///     &mut buffer)?;
    ///
    /// // Size includes length and value.
    /// assert_eq!(size, 7);
    ///
    /// // Length field comes before the value.
    /// assert_eq!(&buffer.freeze(), [0_u8, 0, 0, 3, 1, 2, 3].as_slice());
    /// # Ok(()) }
    /// ```
    fn serialize_with<Buffer, Value, SerializeFn, SizeFn>(
        serialize: SerializeFn,
        size: SizeFn,
        value: Value,
        buffer: &mut Buffer,
    ) -> Result<usize, SerializeError>
    where
        Buffer: BufMut + ?Sized,
        for<'any> SerializeFn: Fn(&Value, &mut dyn BufMut) -> Result<usize, SerializeError>,
        for<'any> SizeFn: Fn(&Value) -> Option<usize>;

    /// Deserializes a value from the given `buffer` with a preceding length field.
    ///
    /// The value in the length field is used to limit the deserialization of the value from the
    /// buffer.
    ///
    /// # Errors
    ///
    /// Returns a [`DeserializeError`] if the deserialization fails.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// use rsomeip_bytes::{Deserialize as _, LengthU32, Length as _};
    ///
    /// // The buffer can be any type that implements `Buf`.
    /// let buffer = [0_u8, 0, 0, 2, 1, 2];
    ///
    /// // Deserialize a value with a 32-bit length field from the buffer.
    /// let value = LengthU32::deserialize::<_, u16>(&mut buffer.as_slice())?;
    /// assert_eq!(value, 0x0102_u16);
    /// # Ok(()) }
    /// ```
    fn deserialize<Buffer, Value>(buffer: &mut Buffer) -> Result<Value::Output, DeserializeError>
    where
        Buffer: Buf + ?Sized,
        Value: Deserialize;

    /// Deserializes a value from the `buffer` with a preceding length field using the given
    /// `function`.
    ///
    /// The value in the length field is used to limit the deserialization of the value from the
    /// buffer.
    ///
    /// # Errors
    ///
    /// Returns a [`DeserializeError`] if the deserialization fails.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// use rsomeip_bytes::{Deserialize as _, LengthU32, Length as _};
    ///
    /// // Type requiring custom deerialization.
    /// #[derive(Debug, PartialEq, Eq)]
    /// struct Foo {
    ///     bar: u8,
    ///     baz: u16,
    /// }
    ///
    /// // The buffer can be any type that implements `Buf`.
    /// let buffer = [0_u8, 0, 0, 3, 1, 2, 3];
    ///
    /// // Deserialize a value with a 32-bit length field from the buffer.
    /// let value = LengthU32::deserialize_with::<_, Foo, _>(
    ///     |mut buffer| {
    ///         Ok(Foo {
    ///             bar: u8::deserialize(buffer)?,
    ///             baz: u16::deserialize(buffer)?,
    ///         })
    ///     },
    ///     &mut buffer.as_slice())?;
    /// assert_eq!(value, Foo { bar: 1_u8, baz: 0x0203_u16 });
    /// # Ok(()) }
    /// ```
    fn deserialize_with<Buffer, Value, Function>(
        function: Function,
        buffer: &mut Buffer,
    ) -> Result<Value, DeserializeError>
    where
        Buffer: Buf + ?Sized,
        for<'any> Function: Fn(&mut dyn Buf) -> Result<Value, DeserializeError>;

    /// Returns the size of the length field in bytes.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rsomeip_bytes::{Length as _, LengthZero, LengthU8, LengthU16, LengthU32};
    ///
    /// assert_eq!(LengthZero::size(), 0);
    /// assert_eq!(LengthU8::size(), 1);
    /// assert_eq!(LengthU16::size(), 2);
    /// assert_eq!(LengthU32::size(), 4);
    /// ```
    #[must_use]
    fn size() -> usize;

    /// Returns the capacity of the length field.
    ///
    /// This is the maximum value that the field can represent.
    ///
    /// Returns [`None`] is the capacity is greater than [`usize::MAX`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rsomeip_bytes::{Length as _, LengthZero, LengthU8, LengthU16, LengthU32};
    ///
    /// assert_eq!(LengthZero::capacity(), Some(usize::MAX));
    /// assert_eq!(LengthU8::capacity(), Some(0xff));
    /// assert_eq!(LengthU16::capacity(), Some(0xffff));
    /// assert_eq!(LengthU32::capacity(), Some(0xffff_ffff));
    /// ```
    #[must_use]
    fn capacity() -> Option<usize>;
}

/// Sealed trait to prevent external implementations.
pub trait Sealed {}

/// Implementation details common among length fields.
macro_rules! impl_length_field {
    ($name:ident, $repr:ident) => {
        impl $name {
            /// Minimum value of the length field.
            pub const MIN: $repr = <$repr>::MIN;

            /// Maximum value of the length field.
            pub const MAX: $repr = <$repr>::MAX;
        }

        impl Length for $name {
            fn serialize<Buffer, Value>(
                value: &Value,
                buffer: &mut Buffer,
            ) -> Result<usize, $crate::SerializeError>
            where
                Value: $crate::Serialize,
                Buffer: ::bytes::BufMut + ?Sized,
            {
                value
                    .size()
                    .ok_or($crate::SerializeError::SizeOverflow)
                    .and_then(|size| {
                        // Check if the size fits inside of the length field.
                        let Ok(length) = <$repr>::try_from(size) else {
                            return Err($crate::SerializeError::LengthOverflow);
                        };
                        // Serialize the length.
                        length.serialize(buffer)?;
                        // Serialize the value.
                        let actual_size = value.serialize(buffer)?;
                        // Checks if the sizes differ.
                        if actual_size != size {
                            return Err($crate::SerializeError::SizeMismatch);
                        }
                        // Add the length field to the total length.
                        actual_size
                            .checked_add(Self::size())
                            .ok_or($crate::SerializeError::SizeOverflow)
                    })
            }

            fn serialize_with<Buffer, Value, SerializeFn, SizeFn>(
                serialize: SerializeFn,
                size: SizeFn,
                value: Value,
                mut buffer: &mut Buffer,
            ) -> Result<usize, SerializeError>
            where
                Buffer: BufMut + ?Sized,
                for<'any> SerializeFn: Fn(&Value, &mut dyn BufMut) -> Result<usize, SerializeError>,
                for<'any> SizeFn: Fn(&Value) -> Option<usize>,
            {
                size(&value)
                    .ok_or($crate::SerializeError::SizeOverflow)
                    .and_then(|size| {
                        // Check if the size fits inside of the length field.
                        let Ok(length) = <$repr>::try_from(size) else {
                            return Err($crate::SerializeError::LengthOverflow);
                        };
                        // Serialize the length.
                        length.serialize(buffer)?;
                        // Serialize the value.
                        let actual_size = serialize(&value, &mut buffer)?;
                        // Checks if the sizes differ.
                        if actual_size != size {
                            return Err($crate::SerializeError::SizeMismatch);
                        }
                        // Add the length field to the total length.
                        actual_size
                            .checked_add(Self::size())
                            .ok_or($crate::SerializeError::SizeOverflow)
                    })
            }

            fn deserialize<Buffer, Value>(
                buffer: &mut Buffer,
            ) -> Result<Value::Output, DeserializeError>
            where
                Buffer: Buf + ?Sized,
                Value: Deserialize,
            {
                <$repr>::deserialize(buffer).and_then(|length| {
                    usize::try_from(length)
                        .map_err(|_| DeserializeError::LengthOverflow)
                        .and_then(|length| {
                            if length > buffer.remaining() {
                                return Err(DeserializeError::InsufficientData);
                            }
                            let mut take = buffer.take(length);
                            Value::deserialize(&mut take)
                        })
                })
            }

            fn deserialize_with<Buffer, Value, Function>(
                function: Function,
                buffer: &mut Buffer,
            ) -> Result<Value, DeserializeError>
            where
                Buffer: Buf + ?Sized,
                for<'any> Function: Fn(&mut dyn Buf) -> Result<Value, DeserializeError>,
            {
                <$repr>::deserialize(buffer).and_then(|length| {
                    usize::try_from(length)
                        .map_err(|_| DeserializeError::LengthOverflow)
                        .and_then(|length| {
                            if length > buffer.remaining() {
                                return Err(DeserializeError::InsufficientData);
                            }
                            let mut take = buffer.take(length);
                            function(&mut take)
                        })
                })
            }

            #[inline]
            fn size() -> usize {
                ::core::mem::size_of::<$repr>()
            }

            #[inline]
            fn capacity() -> Option<usize> {
                usize::try_from(Self::MAX).ok()
            }
        }

        impl Sealed for $name {}
    };
}

/// 8-bit length field.
///
/// Used to encode lengths between 0 and 255 bytes long.
///
/// Normally used together with [`DynamicArray`] and [`DynamicString`].
///
/// [`DynamicArray`]: crate::DynamicArray
/// [`DynamicString`]: crate::DynamicString
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LengthU8;

impl_length_field!(LengthU8, u8);

/// 16-bit length field.
///
/// Used to encode lengths between 0 and 65'535 bytes long.
///
/// Normally used together with [`DynamicArray`] and [`DynamicString`].
///
/// [`DynamicArray`]: crate::DynamicArray
/// [`DynamicString`]: crate::DynamicString
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LengthU16;

impl_length_field!(LengthU16, u16);

/// 32-bit length field.
///
/// Used to encode lengths between 0 and 4'294'967'295 bytes long.
///
/// Normally used together with [`DynamicArray`] and [`DynamicString`].
///
/// [`DynamicArray`]: crate::DynamicArray
/// [`DynamicString`]: crate::DynamicString
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LengthU32;

impl_length_field!(LengthU32, u32);

/// 0-bit length field.
///
/// Used to skip length encoding.
///
/// Normally used together with [`DynamicArray`] and [`DynamicString`].
///
/// [`DynamicArray`]: crate::DynamicArray
/// [`DynamicString`]: crate::DynamicString
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LengthZero;

impl Length for LengthZero {
    #[inline]
    fn serialize<Buffer, Value>(value: &Value, buffer: &mut Buffer) -> Result<usize, SerializeError>
    where
        Buffer: BufMut + ?Sized,
        Value: Serialize,
    {
        value.serialize(buffer)
    }

    #[inline]
    fn serialize_with<Buffer, Value, SerializeFn, SizeFn>(
        serialize: SerializeFn,
        _size: SizeFn,
        value: Value,
        mut buffer: &mut Buffer,
    ) -> Result<usize, SerializeError>
    where
        Buffer: BufMut + ?Sized,
        for<'any> SerializeFn: Fn(&Value, &mut dyn BufMut) -> Result<usize, SerializeError>,
        for<'any> SizeFn: Fn(&Value) -> Option<usize>,
    {
        (serialize)(&value, &mut buffer)
    }

    #[inline]
    fn deserialize<Buffer, Value>(buffer: &mut Buffer) -> Result<Value::Output, DeserializeError>
    where
        Buffer: Buf + ?Sized,
        Value: Deserialize,
    {
        Value::deserialize(buffer)
    }

    #[inline]
    fn deserialize_with<Buffer, Value, Function>(
        function: Function,
        mut buffer: &mut Buffer,
    ) -> Result<Value, DeserializeError>
    where
        Buffer: Buf + ?Sized,
        for<'any> Function: Fn(&mut dyn Buf) -> Result<Value, DeserializeError>,
    {
        (function)(&mut buffer)
    }

    #[inline]
    fn size() -> usize {
        0
    }

    #[inline]
    fn capacity() -> Option<usize> {
        Some(usize::MAX)
    }
}

impl Sealed for LengthZero {}

/// Wrapper for serializing a value with a preceding length field.
///
/// See also [`Length::serialize`].
///
/// # Examples
///
/// ```rust
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// use rsomeip_bytes::{SerializeWithLength, LengthU32, Serialize as _};
///
/// let wrapper = SerializeWithLength::<LengthU32, _>::from(&1_u16);
/// assert_eq!(Some(6), wrapper.size());
///
/// let bytes = wrapper.to_bytes()?;
/// assert_eq!(&bytes, [0_u8, 0, 0, 2, 0, 1].as_slice());
/// # Ok(()) }
/// ```
pub struct SerializeWithLength<'value, Length, Value> {
    /// Value to serialize.
    value: &'value Value,
    /// Length to serialize before the value.
    _length: PhantomData<Length>,
}

impl<'value, Length, Value> From<&'value Value> for SerializeWithLength<'value, Length, Value>
where
    Length: crate::Length,
    Value: Serialize,
{
    fn from(value: &'value Value) -> Self {
        Self {
            value,
            _length: PhantomData,
        }
    }
}

impl<Length, Value> Serialize for SerializeWithLength<'_, Length, Value>
where
    Length: crate::Length,
    Value: Serialize,
{
    fn serialize<Buffer>(&self, buffer: &mut Buffer) -> Result<usize, SerializeError>
    where
        Buffer: BufMut + ?Sized,
    {
        Length::serialize(self.value, buffer)
    }

    fn size(&self) -> Option<usize> {
        self.value
            .size()
            .and_then(|size| size.checked_add(Length::size()))
    }
}

/// Wrapper for deserializing a value with a preceding length field.
///
/// See also [`Length::deserialize`].
///
/// # Examples
///
/// ```rust
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// use rsomeip_bytes::{DeserializeWithLength, LengthU32, Deserialize as _};
///
/// // Size hint includes the length field.
/// assert_eq!(Some(4), DeserializeWithLength::<LengthU32, u16>::size_hint());
///
/// let mut buffer = [0_u8, 0, 0, 2, 0, 1];
/// let value = DeserializeWithLength::<LengthU32, u16>::deserialize(&mut buffer.as_slice())?;
/// assert_eq!(value, 1_u16);
/// # Ok(()) }
/// ```
pub struct DeserializeWithLength<Length, Value> {
    /// Length to deserialize before the value.
    _length: PhantomData<Length>,
    /// Value to deserialize.
    _value: PhantomData<Value>,
}

impl<Length, Value> Deserialize for DeserializeWithLength<Length, Value>
where
    Length: crate::Length,
    Value: Deserialize,
{
    type Output = Value::Output;

    fn deserialize<Buffer>(buffer: &mut Buffer) -> Result<Self::Output, DeserializeError>
    where
        Buffer: Buf + ?Sized,
    {
        Length::deserialize::<_, Value>(buffer)
    }

    fn size_hint() -> Option<usize> {
        Some(Length::size())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Serialize;
    use bytes::{Bytes, BytesMut};

    #[test]
    fn serialize_length_overflow() {
        let mut buffer = BytesMut::new();
        let value = BytesMut::zeroed(0x1_0000_0000_usize).freeze();
        assert_eq!(
            LengthU8::serialize(&value, &mut buffer),
            Err(SerializeError::LengthOverflow)
        );
        assert_eq!(
            LengthU16::serialize(&value, &mut buffer),
            Err(SerializeError::LengthOverflow)
        );
        assert_eq!(
            LengthU32::serialize(&value, &mut buffer),
            Err(SerializeError::LengthOverflow)
        );
        assert_eq!(
            LengthU8::serialize_with(
                |value, buffer| value.serialize(buffer),
                Serialize::size,
                value.clone(),
                &mut buffer
            ),
            Err(SerializeError::LengthOverflow)
        );
        assert_eq!(
            LengthU16::serialize_with(
                |value, buffer| value.serialize(buffer),
                Serialize::size,
                value.clone(),
                &mut buffer
            ),
            Err(SerializeError::LengthOverflow)
        );
        assert_eq!(
            LengthU32::serialize_with(
                |value, buffer| value.serialize(buffer),
                Serialize::size,
                value,
                &mut buffer
            ),
            Err(SerializeError::LengthOverflow)
        );
    }

    #[test]
    fn serialize_size_mismatch() {
        // Custom type with incorrect Serialize impl.
        struct Foo;
        impl Serialize for Foo {
            fn serialize<Buffer>(&self, _buffer: &mut Buffer) -> Result<usize, SerializeError>
            where
                Buffer: BufMut + ?Sized,
            {
                Ok(2)
            }

            fn size(&self) -> Option<usize> {
                Some(1)
            }
        }

        let mut buffer = BytesMut::new();
        assert_eq!(
            LengthU8::serialize(&Foo, &mut buffer),
            Err(SerializeError::SizeMismatch)
        );
        assert_eq!(
            LengthU16::serialize(&Foo, &mut buffer),
            Err(SerializeError::SizeMismatch)
        );
        assert_eq!(
            LengthU32::serialize(&Foo, &mut buffer),
            Err(SerializeError::SizeMismatch)
        );
        assert_eq!(
            LengthU8::serialize_with(|_value, _buffer| Ok(2), |_value| Some(1), 0_u8, &mut buffer),
            Err(SerializeError::SizeMismatch)
        );
        assert_eq!(
            LengthU16::serialize_with(|_value, _buffer| Ok(2), |_value| Some(1), 0_u8, &mut buffer),
            Err(SerializeError::SizeMismatch)
        );
        assert_eq!(
            LengthU32::serialize_with(|_value, _buffer| Ok(2), |_value| Some(1), 0_u8, &mut buffer),
            Err(SerializeError::SizeMismatch)
        );
    }

    #[test]
    fn deserialize_insufficient_data() {
        let mut buffer = Bytes::copy_from_slice(&[0xff_u8, 0xff, 0xff, 0xff, 1]);
        assert_eq!(
            LengthU8::deserialize::<_, u8>(&mut buffer.clone()),
            Err(DeserializeError::InsufficientData)
        );
        assert_eq!(
            LengthU16::deserialize::<_, u8>(&mut buffer.clone()),
            Err(DeserializeError::InsufficientData)
        );
        assert_eq!(
            LengthU32::deserialize::<_, u8>(&mut buffer.clone()),
            Err(DeserializeError::InsufficientData)
        );
        assert_eq!(
            LengthU8::deserialize_with::<_, u8, _>(
                |buffer| u8::deserialize(buffer),
                &mut buffer.clone()
            ),
            Err(DeserializeError::InsufficientData)
        );
        assert_eq!(
            LengthU16::deserialize_with::<_, u8, _>(
                |buffer| u8::deserialize(buffer),
                &mut buffer.clone()
            ),
            Err(DeserializeError::InsufficientData)
        );
        assert_eq!(
            LengthU32::deserialize_with::<_, u8, _>(|buffer| u8::deserialize(buffer), &mut buffer),
            Err(DeserializeError::InsufficientData)
        );
    }

    #[test]
    fn serialize_length_zero() {
        let mut buffer = BytesMut::with_capacity(2);
        assert_eq!(Ok(1), LengthZero::serialize(&1_u8, &mut buffer));
        assert_eq!(
            Ok(1),
            LengthZero::serialize_with(
                |value, buffer| value.serialize(buffer),
                Serialize::size,
                2_u8,
                &mut buffer
            )
        );
        assert_eq!(&buffer.freeze(), [1_u8, 2_u8].as_slice());
    }

    #[test]
    fn deserialize_length_zero() {
        let mut buffer = Bytes::copy_from_slice(&[1_u8, 2]);
        assert_eq!(Ok(1_u8), LengthZero::deserialize::<_, u8>(&mut buffer));
        assert_eq!(
            Ok(2_u8),
            LengthZero::deserialize_with::<_, u8, _>(|buffer| u8::deserialize(buffer), &mut buffer)
        );
    }
}
