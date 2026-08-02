//! Serialization according to the SOME/IP protocol.
//!
//! Provides the [`Serialize`] trait and several implementations for types of the standard library.

use crate::{BufMut, Bytes, BytesMut};
use alloc::{borrow::Cow, boxed::Box, rc::Rc, sync::Arc};

/// Serialize according to the SOME/IP on-wire format.
pub trait Serialize {
    /// Serializes `self` into the given `buffer`.
    ///
    /// Returns the length of the serialized data.
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
    /// use rsomeip_bytes::Serialize;
    ///
    /// let mut buffer = [0_u8; 4];
    /// let size = 0x1234_5678_u32.serialize(&mut buffer.as_mut_slice())?;
    /// assert_eq!(size, 4);
    /// assert_eq!(buffer.as_slice(), [0x12_u8, 0x34, 0x56, 0x78].as_slice());
    /// # Ok(()) }
    /// ```
    ///
    /// # Implementation notes
    ///
    /// Care should be taken so that the serialized data matches the interface definition and that
    /// it's compatible with the SOME/IP on-wire format to prevent issues during deserialization.
    fn serialize<Buffer>(&self, buffer: &mut Buffer) -> Result<usize, SerializeError>
    where
        Buffer: BufMut + ?Sized;

    /// Returns the size of `self` when serialized.
    ///
    /// Returns [`None`] if the size is out of bounds.
    ///
    /// This method is suitable for calculating the value of a length field or the capacity of a
    /// buffer prior to serializing `self`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rsomeip_bytes::Serialize as _;
    ///
    /// assert_eq!(1_u8.size(), Some(1));
    /// assert_eq!(1_u16.size(), Some(2));
    /// assert_eq!(1_u32.size(), Some(4));
    /// ```
    ///
    /// # Implementation notes
    ///
    /// Care should be taken so that the output of this method exactly matches the output of the
    /// [`serialize`] method.
    ///
    /// [`serialize`]: [`Serialize::serialize`]
    fn size(&self) -> Option<usize>;

    /// Returns `self` serialized into [`Bytes`].
    ///
    /// # Errors
    ///
    /// Returns a [`SerializeError`] if the serialization fails.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// use rsomeip_bytes::Serialize;
    ///
    /// let bytes = 0x1234_5678_u32.to_bytes()?;
    /// assert_eq!(bytes, [0x12_u8, 0x34, 0x56, 0x78].as_slice());
    /// # Ok(()) }
    /// ```
    fn to_bytes(&self) -> Result<Bytes, SerializeError> {
        let Some(size) = self.size() else {
            return Err(SerializeError::SizeOverflow);
        };
        let mut buffer = BytesMut::with_capacity(size);
        self.serialize(&mut buffer).map(|_total| buffer.freeze())
    }
}

/// Error when serializing data.
#[derive(Debug, Clone, PartialEq, Eq, Hash, thiserror::Error)]
#[non_exhaustive]
pub enum SerializeError {
    /// The target buffer doesn't have enough capacity for `self`.
    #[error("buffer would overflow")]
    BufferOverflow,
    /// An invariant of the serialized type wasn't upheld.
    #[error("invariant failed: {0}")]
    InvariantFailed(Cow<'static, str>),
    /// Length exceeds the capacity of the length field.
    #[error("length exceeds capacity of length field")]
    LengthOverflow,
    /// Size of the serialized data doesn't match value returned by [`Serialize::size`].
    #[error("size doesn't match expected value")]
    SizeMismatch,
    /// Size exceeds the capacity of [`usize`].
    #[error("size exceeds capacity of `usize`")]
    SizeOverflow,
}

impl SerializeError {
    /// Creates a new [`SerializeError::InvariantFailed`] with the given `message`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rsomeip_bytes::SerializeError;
    ///
    /// // Can use static error messages.
    /// let borrowed = SerializeError::invariant("generic error");
    /// assert_eq!(borrowed.to_string(), "invariant failed: generic error");
    ///
    /// // Or dynamic error messages.
    /// let owned = SerializeError::invariant(format!("specific error: {}", 42));
    /// assert_eq!(owned.to_string(), "invariant failed: specific error: 42");
    /// ```
    #[inline]
    #[must_use]
    pub fn invariant(message: impl Into<Cow<'static, str>>) -> Self {
        Self::InvariantFailed(message.into())
    }
}

/// Implements [`Serialize`] for references and pointers using a forwarding call.
macro_rules! impl_serialize_forward {
    ($name:ty) => {
        impl<T: Serialize + ?Sized> Serialize for $name {
            fn serialize<Buffer>(&self, buffer: &mut Buffer) -> Result<usize, SerializeError>
            where
                Buffer: BufMut + ?Sized,
            {
                (**self).serialize(buffer)
            }

            fn size(&self) -> Option<usize> {
                (**self).size()
            }
        }
    };
}

impl_serialize_forward!(&T);
impl_serialize_forward!(&mut T);
impl_serialize_forward!(Box<T>);
impl_serialize_forward!(Rc<T>);
impl_serialize_forward!(Arc<T>);

/// Implements the [`Serialize`] trait for tuples.
///
/// Each method calls itself on each member of the tuple.
macro_rules! impl_serialize_tuple {
    ($( $name:ident )+) => {
        #[expect(non_snake_case, reason = "generic parameters")]
        #[expect(clippy::min_ident_chars, reason = "generic parameters")]
        impl<$($name: Serialize),+> Serialize for ($($name,)+) {
            fn serialize<Buffer>(&self, buffer: &mut Buffer) -> Result<usize, SerializeError>
            where
                Buffer: BufMut + ?Sized
            {
                let &($(ref $name,)+) = self;
                Ok(0_usize)
                $(
                    .and_then(|total| {
                        $name.serialize(buffer).and_then(|size|
                            total.checked_add(size).ok_or(SerializeError::SizeOverflow)
                        )
                    })
                )+
            }

            fn size(&self) -> Option<usize> {
                let &($(ref $name,)+) = self;
                Some(0_usize)
                $(
                    .and_then(|total| {
                        $name.size().and_then(|size| total.checked_add(size))
                    })
                )+
            }
        }
    };
}

impl_serialize_tuple! { A }
impl_serialize_tuple! { A B }
impl_serialize_tuple! { A B C }
impl_serialize_tuple! { A B C D }
impl_serialize_tuple! { A B C D E }
impl_serialize_tuple! { A B C D E F }
impl_serialize_tuple! { A B C D E F G }
impl_serialize_tuple! { A B C D E F G H }
impl_serialize_tuple! { A B C D E F G H I }
impl_serialize_tuple! { A B C D E F G H I J }
impl_serialize_tuple! { A B C D E F G H I J K }
impl_serialize_tuple! { A B C D E F G H I J K L }

/// Implements the [`Serialize`] trait for basic types.
///
/// In order to improve performance, the [`serialize`] method doesn't do any checks before writing
/// to the buffer. Depending on the type of the actual buffer, this might cause a panic if it
/// doesn't have enough capacity.
///
/// [`serialize`]: [`Serialize::serialize`]
macro_rules! impl_serialize_basic_type {
    ($name:ty, $method:ident) => {
        impl Serialize for $name {
            /// Serializes `self` into the given `buffer`.
            ///
            /// Returns the length of the serialized data.
            ///
            /// # Panics
            ///
            /// Panics if `buffer` doesn't have enough capacity for `self`.
            fn serialize<Buffer>(&self, buffer: &mut Buffer) -> Result<usize, SerializeError>
            where
                Buffer: BufMut + ?Sized,
            {
                buffer.$method(*self);
                Ok(size_of::<$name>())
            }

            /// Returns the size of `self` when serialized.
            ///
            /// Never returns [`None`].
            fn size(&self) -> Option<usize> {
                Some(size_of::<$name>())
            }
        }
    };
}
impl_serialize_basic_type!(u8, put_u8);
impl_serialize_basic_type!(u16, put_u16);
impl_serialize_basic_type!(u32, put_u32);
impl_serialize_basic_type!(u64, put_u64);
impl_serialize_basic_type!(i8, put_i8);
impl_serialize_basic_type!(i16, put_i16);
impl_serialize_basic_type!(i32, put_i32);
impl_serialize_basic_type!(i64, put_i64);
impl_serialize_basic_type!(f32, put_f32);
impl_serialize_basic_type!(f64, put_f64);

impl Serialize for bool {
    fn serialize<Buffer>(&self, buffer: &mut Buffer) -> Result<usize, SerializeError>
    where
        Buffer: BufMut + ?Sized,
    {
        if *self {
            buffer.put_u8(1);
        } else {
            buffer.put_u8(0);
        }
        Ok(size_of::<u8>())
    }

    fn size(&self) -> Option<usize> {
        Some(1)
    }
}

impl<T: Serialize, const N: usize> Serialize for [T; N] {
    fn serialize<Buffer>(&self, buffer: &mut Buffer) -> Result<usize, SerializeError>
    where
        Buffer: BufMut + ?Sized,
    {
        let mut iterator = self.iter();
        iterator.try_fold(0_usize, |acc, elem| {
            elem.serialize(buffer)
                .and_then(|elem| acc.checked_add(elem).ok_or(SerializeError::SizeOverflow))
        })
    }

    fn size(&self) -> Option<usize> {
        let mut iterator = self.iter();
        iterator.try_fold(0_usize, |acc, elem| {
            elem.size().and_then(|elem| acc.checked_add(elem))
        })
    }
}

impl Serialize for Bytes {
    fn serialize<Buffer>(&self, buffer: &mut Buffer) -> Result<usize, SerializeError>
    where
        Buffer: BufMut + ?Sized,
    {
        buffer.put_slice(self);
        Ok(self.len())
    }

    fn size(&self) -> Option<usize> {
        Some(self.len())
    }
}

/// Wrapper for serializing a value with a pair of functions.
///
/// # Examples
///
/// ```rust
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// use rsomeip_bytes::{SerializeWithFn, BufMut, Serialize as _};
///
/// // Type that needs custom serialization.
/// struct Foo {
///     bar: u8,
///     baz: u16,
/// }
///
/// let wrapper = SerializeWithFn::new(
///         &Foo{ bar: 1_u8, baz: 2_u16 },
///         |value, buffer| (&value.bar, &value.baz).serialize(buffer),
///         |value| (&value.bar, &value.baz).size(),
///     );
///
/// let bytes = wrapper.to_bytes()?;
/// assert_eq!(&bytes, [1_u8, 0, 2].as_slice());
/// # Ok(()) }
/// ```
pub struct SerializeWithFn<'value, Value, SerializeFn, SizeFn> {
    /// Value to serialize.
    value: &'value Value,
    /// Serialization function. Equivalent to [`Serialize::serialize`].
    serialize: SerializeFn,
    /// Size function. Equivalent to [`Serialize::size`].
    size: SizeFn,
}

impl<'value, Value, SerializeFn, SizeFn> SerializeWithFn<'value, Value, SerializeFn, SizeFn>
where
    for<'any> SerializeFn: Fn(&Value, &mut dyn BufMut) -> Result<usize, SerializeError>,
    for<'any> SizeFn: Fn(&Value) -> Option<usize>,
{
    /// Creates a new [`SerializeWithFn`].
    #[inline]
    #[must_use]
    pub const fn new(value: &'value Value, serialize: SerializeFn, size: SizeFn) -> Self {
        Self {
            value,
            serialize,
            size,
        }
    }
}

impl<Value, SerializeFn, SizeFn> Serialize for SerializeWithFn<'_, Value, SerializeFn, SizeFn>
where
    for<'any> SerializeFn: Fn(&Value, &mut dyn BufMut) -> Result<usize, SerializeError>,
    for<'any> SizeFn: Fn(&Value) -> Option<usize>,
{
    fn serialize<Buffer>(&self, mut buffer: &mut Buffer) -> Result<usize, SerializeError>
    where
        Buffer: BufMut + ?Sized,
    {
        (self.serialize)(self.value, &mut buffer)
    }

    fn size(&self) -> Option<usize> {
        (self.size)(self.value)
    }
}

#[cfg(test)]
#[expect(clippy::inline_modules, reason = "rust-clippy#17342")]
mod tests {
    use super::*;

    macro_rules! test_serialize_basic_type {
        ($t:ty, $name:ident) => {
            #[test]
            fn $name() {
                let mut buffer = BytesMut::with_capacity(size_of::<$t>());
                let result = <$t>::MAX.serialize(&mut buffer);
                assert_eq!(result, Ok(size_of::<$t>()));
                assert_eq!(result.ok(), <$t>::MAX.size());
                assert_eq!(buffer.freeze(), <$t>::MAX.to_be_bytes().as_slice());
            }
        };
    }

    test_serialize_basic_type!(u8, serialize_u8);
    test_serialize_basic_type!(u16, serialize_u16);
    test_serialize_basic_type!(u32, serialize_u32);
    test_serialize_basic_type!(u64, serialize_u64);
    test_serialize_basic_type!(i8, serialize_i8);
    test_serialize_basic_type!(i16, serialize_i16);
    test_serialize_basic_type!(i32, serialize_i32);
    test_serialize_basic_type!(i64, serialize_i64);
    test_serialize_basic_type!(f32, serialize_f32);
    test_serialize_basic_type!(f64, serialize_f64);

    #[test]
    fn serialize_bool() {
        let mut buffer = BytesMut::with_capacity(2);
        for value in [true, false] {
            let size = value
                .serialize(&mut buffer)
                .expect("should serialize the bool");
            assert_eq!(size, 1);
            assert_eq!(value.size(), Some(1));
        }
        assert_eq!(buffer.freeze(), [1_u8, 0_u8].as_slice());
    }

    #[test]
    fn serialize_array() {
        let mut buffer = BytesMut::with_capacity(2);
        let array = [1_u8, 2_u8];
        let size = array
            .serialize(&mut buffer)
            .expect("should serialize the array");
        assert_eq!(size, 2);
        assert_eq!(Some(size), array.size());
        assert_eq!(buffer.freeze(), [1_u8, 2_u8].as_slice());
    }

    #[test]
    fn serialize_tuple() {
        let mut buffer = BytesMut::with_capacity(2);
        let tuple = (1_u8, 2_u8);
        let size = tuple
            .serialize(&mut buffer)
            .expect("should serialize the tuple");
        assert_eq!(size, 2);
        assert_eq!(Some(size), tuple.size());
        assert_eq!(buffer.freeze(), [1_u8, 2_u8].as_slice());
    }

    #[test]
    fn serialize_bytes() {
        let mut buffer = BytesMut::with_capacity(2);
        let bytes = Bytes::copy_from_slice(&[1_u8, 2_u8]);
        let size = bytes
            .serialize(&mut buffer)
            .expect("should serialize the buffer");
        assert_eq!(size, 2);
        assert_eq!(Some(size), bytes.size());
        assert_eq!(buffer.freeze(), [1_u8, 2].as_slice());
    }

    #[test]
    fn serialize_box() {
        let mut buffer = BytesMut::with_capacity(2);
        let value = Box::new(0x0102_u16);
        let size = value
            .serialize(&mut buffer)
            .expect("should serialize the buffer");
        assert_eq!(size, 2);
        assert_eq!(Some(size), value.size());
        assert_eq!(buffer.freeze(), [1_u8, 2].as_slice());
    }
}
