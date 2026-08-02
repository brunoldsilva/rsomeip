//! Deserialization according to the SOME/IP protocol.
//!
//! Provides the [`Deserialize`] trait for deserializing data, and several implementations of this
//! trait for types of the standard library.

use crate::{Buf, Bytes};
use alloc::{borrow::Cow, boxed::Box, rc::Rc, sync::Arc};
use core::array;

/// Deserialize data from a SOME/IP byte stream.
pub trait Deserialize {
    /// Type of the deserialized data.
    type Output: Sized;

    /// Deserializes an instance of [`Deserialize::Output`] from the buffer.
    ///
    /// # Errors
    ///
    /// Returns a [`DeserializeError`] if the deserialization fails.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// use rsomeip_bytes::{Deserialize as _};
    ///
    /// // The buffer can be any type that implements `Buf`.
    /// let buffer = [0x12_u8, 0x34];
    ///
    /// // Deserialize a type from the buffer.
    /// let value = u16::deserialize(&mut buffer.as_slice())?;
    /// assert_eq!(value, 0x1234);
    /// # Ok(()) }
    /// ```
    ///
    /// # Implementation notes
    ///
    /// Care should be taken so that the serialized data matches the interface definition and that
    /// it's compatible with the SOME/IP on-wire format to prevent issues during deserialization.
    fn deserialize<Buffer>(buffer: &mut Buffer) -> Result<Self::Output, DeserializeError>
    where
        Buffer: Buf + ?Sized;

    /// Returns the minimum size of a correctly serialized [`Self::Output`].
    ///
    /// Deserialization is guaranteed to fail if the buffer doesn't contain at least this amount
    /// of bytes.
    ///
    /// Having more than this amount of bytes in the buffer doesn't guarantee that deserialization
    /// does succeed however, as the correct size of some types can only be known at runtime, but
    /// it's guaranteed to be at least this value.
    ///
    /// Returns [`None`] if the size is larger than [`usize::MAX`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rsomeip_bytes::{Deserialize as _};
    ///
    /// // Basic types match the in-memory size.
    /// assert_eq!(u8::size_hint(), Some(1));
    /// assert_eq!(u16::size_hint(), Some(2));
    /// assert_eq!(u32::size_hint(), Some(4));
    /// ```
    ///
    /// # Implementation notes
    ///
    /// The value returned by this method should include the size of all statically known elements
    /// of [`Self::Output`].
    #[must_use]
    fn size_hint() -> Option<usize>;
}

/// Error when deserializing data.
#[derive(Debug, Clone, PartialEq, Eq, Hash, thiserror::Error)]
#[non_exhaustive]
pub enum DeserializeError {
    /// There is insufficient data in the buffer to deserialize the complete type.
    #[error("insufficient data in buffer")]
    InsufficientData,
    /// An invariant of the deserialized type wasn't upheld.
    #[error("invariant failed: {0}")]
    InvariantFailed(Cow<'static, str>),
    /// Value of the length field exceeds [`usize::MAX`].
    #[error("length exceeds usize::MAX")]
    LengthOverflow,
}

impl DeserializeError {
    /// Creates a new [`DeserializeError::InvariantFailed`] with the given `message`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rsomeip_bytes::DeserializeError;
    ///
    /// // Can use static error messages.
    /// let borrowed = DeserializeError::invariant("generic error");
    /// assert_eq!(borrowed.to_string(), "invariant failed: generic error");
    ///
    /// // Or dynamic error messages.
    /// let owned = DeserializeError::invariant(format!("specific error: {}", 42));
    /// assert_eq!(owned.to_string(), "invariant failed: specific error: 42");
    /// ```
    #[inline]
    #[must_use]
    pub fn invariant(message: impl Into<Cow<'static, str>>) -> Self {
        Self::InvariantFailed(message.into())
    }
}

/// Implements [`Deserialize`] for smart pointers.
macro_rules! impl_deserialize_ptr {
    ($name:ty) => {
        impl<T> Deserialize for $name
        where
            T: Deserialize<Output = T>,
            $name: From<T>,
        {
            type Output = $name;

            fn deserialize<Buffer>(buffer: &mut Buffer) -> Result<Self::Output, DeserializeError>
            where
                Buffer: Buf + ?Sized,
            {
                T::deserialize(buffer).map(<$name>::from)
            }

            fn size_hint() -> Option<usize> {
                T::size_hint()
            }
        }
    };
}

impl_deserialize_ptr!(Box<T>);
impl_deserialize_ptr!(Rc<T>);
impl_deserialize_ptr!(Arc<T>);

/// Implements [`Deserialize`] for a tuple.
///
/// Elements are deserialized by the order that they appear in the tuple. The size hint is the sum
/// of the size hint of each individual element.
macro_rules! impl_deserialize_tuple {
    ( $( $name:ident )+ ) => {
        impl<$($name: Deserialize<Output=$name>),+> Deserialize for ($($name,)+) {
            type Output = Self;
            fn deserialize<Buffer>(buffer: &mut Buffer) -> Result<Self::Output, DeserializeError>
                where Buffer: Buf + ?Sized
            {
                Ok((
                    $($name::deserialize(buffer)?,)+
                ))
            }

            fn size_hint() -> Option<usize> {
                Some(0_usize)
                $(
                    .and_then(|total| {
                        <$name>::size_hint().and_then(|size| total.checked_add(size))
                    })
                )+
            }
        }
    };
}

impl_deserialize_tuple! { A }
impl_deserialize_tuple! { A B }
impl_deserialize_tuple! { A B C }
impl_deserialize_tuple! { A B C D }
impl_deserialize_tuple! { A B C D E }
impl_deserialize_tuple! { A B C D E F }
impl_deserialize_tuple! { A B C D E F G }
impl_deserialize_tuple! { A B C D E F G H }
impl_deserialize_tuple! { A B C D E F G H I }
impl_deserialize_tuple! { A B C D E F G H I J }
impl_deserialize_tuple! { A B C D E F G H I J K }
impl_deserialize_tuple! { A B C D E F G H I J K L }

/// Implements [`Deserialize`] for a basic type.
macro_rules! impl_deserialize_basic_type {
    ($name:ty, $try_get:ident) => {
        impl Deserialize for $name {
            type Output = Self;

            fn deserialize<Buffer>(buffer: &mut Buffer) -> Result<Self::Output, DeserializeError>
            where
                Buffer: Buf + ?Sized,
            {
                buffer
                    .$try_get()
                    .map_err(|_err| DeserializeError::InsufficientData)
            }

            /// Returns the size of `self` when serialized.
            ///
            /// Never returns [`None`].
            fn size_hint() -> Option<usize> {
                Some(size_of::<$name>())
            }
        }
    };
}

impl_deserialize_basic_type!(u8, try_get_u8);
impl_deserialize_basic_type!(u16, try_get_u16);
impl_deserialize_basic_type!(u32, try_get_u32);
impl_deserialize_basic_type!(u64, try_get_u64);
impl_deserialize_basic_type!(i8, try_get_i8);
impl_deserialize_basic_type!(i16, try_get_i16);
impl_deserialize_basic_type!(i32, try_get_i32);
impl_deserialize_basic_type!(i64, try_get_i64);
impl_deserialize_basic_type!(f32, try_get_f32);
impl_deserialize_basic_type!(f64, try_get_f64);

impl Deserialize for bool {
    type Output = Self;

    fn deserialize<Buffer>(buffer: &mut Buffer) -> Result<Self::Output, DeserializeError>
    where
        Buffer: Buf + ?Sized,
    {
        // Only check the first bit.
        u8::deserialize(buffer).map(|value| (value & 0x01) == 0x01)
    }

    fn size_hint() -> Option<usize> {
        Some(1)
    }
}

// TODO(brunoldsilva): drop the `Default` bound when `array::try_from_fn` stabilizes.
impl<T, const N: usize> Deserialize for [T; N]
where
    T: Deserialize<Output = T> + Default,
{
    type Output = Self;

    fn deserialize<Buffer>(buffer: &mut Buffer) -> Result<Self::Output, DeserializeError>
    where
        Buffer: Buf + ?Sized,
    {
        // First error found during deserialization.
        let mut error: Option<DeserializeError> = None;
        // Deserialize the elements into an array. Use defaults in case of an error.
        let array = array::from_fn(|_| {
            if error.is_none() {
                T::deserialize(buffer)
                    .map_err(|err| {
                        error = Some(err);
                    })
                    .unwrap_or_default()
            } else {
                T::default()
            }
        });
        // Return the array or the first error that occurred.
        error.map_or_else(|| Ok(array), Err)
    }

    fn size_hint() -> Option<usize> {
        T::size_hint().and_then(|size| size.checked_mul(N))
    }
}

impl Deserialize for Bytes {
    type Output = Self;

    fn deserialize<Buffer>(buffer: &mut Buffer) -> Result<Self::Output, DeserializeError>
    where
        Buffer: Buf + ?Sized,
    {
        Ok(buffer.copy_to_bytes(buffer.remaining()))
    }

    fn size_hint() -> Option<usize> {
        Some(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;

    /// Tests the [`Deserialize`] implementation of a basic type.
    ///
    /// It checks if the value `1` can be deserialized from a buffer, and if an error is returned
    /// from an empty buffer.
    macro_rules! test_deserialize_basic_type {
        ($t:ty, $name:tt) => {
            #[test]
            fn $name() {
                assert_eq!(<$t>::size_hint(), Some(size_of::<$t>()));
                let value = <$t>::try_from(1_u8).expect("1_u8 fits in every other basic type");
                let buffer = value.to_be_bytes();
                let mut cursor = buffer.as_slice();
                assert_eq!(<$t>::deserialize(&mut cursor), Ok(value));
                assert_eq!(
                    <$t>::deserialize(&mut cursor),
                    Err(DeserializeError::InsufficientData)
                );
            }
        };
    }

    test_deserialize_basic_type!(u8, deserialize_u8);
    test_deserialize_basic_type!(u16, deserialize_u16);
    test_deserialize_basic_type!(u32, deserialize_u32);
    test_deserialize_basic_type!(u64, deserialize_u64);
    test_deserialize_basic_type!(i8, deserialize_i8);
    test_deserialize_basic_type!(i16, deserialize_i16);
    test_deserialize_basic_type!(i32, deserialize_i32);
    test_deserialize_basic_type!(i64, deserialize_i64);
    test_deserialize_basic_type!(f32, deserialize_f32);
    test_deserialize_basic_type!(f64, deserialize_f64);

    #[test]
    fn deserialize_bool() {
        assert_eq!(bool::size_hint(), Some(size_of::<u8>()));
        let mut buffer = Bytes::copy_from_slice(&[0_u8, 1_u8]);
        assert_eq!(bool::deserialize(&mut buffer), Ok(false));
        assert_eq!(bool::deserialize(&mut buffer), Ok(true));
        assert_eq!(
            bool::deserialize(&mut buffer),
            Err(DeserializeError::InsufficientData)
        );
    }

    #[test]
    fn deserialize_array() {
        assert_eq!(<[u8; 2]>::size_hint(), Some(size_of::<u16>()));
        let mut buffer = Bytes::copy_from_slice(&[1_u8, 2_u8]);
        assert_eq!(<[u8; 2]>::deserialize(&mut buffer), Ok([1_u8, 2_u8]));
        assert_eq!(
            <[u8; 2]>::deserialize(&mut buffer),
            Err(DeserializeError::InsufficientData)
        );
    }

    #[test]
    fn deserialize_tuple() {
        assert_eq!(<(u8, u8)>::size_hint(), Some(size_of::<u16>()));
        let mut buffer = Bytes::copy_from_slice(&[1_u8, 2_u8]);
        assert_eq!(<(u8, u8)>::deserialize(&mut buffer), Ok((1_u8, 2_u8)));
        assert_eq!(
            <(u8, u8)>::deserialize(&mut buffer),
            Err(DeserializeError::InsufficientData)
        );
    }

    #[test]
    fn deserialize_bytes() {
        assert_eq!(Bytes::size_hint(), Some(0));
        let mut buffer = Bytes::copy_from_slice(&[1_u8, 2]);
        let output = Bytes::deserialize(&mut buffer).expect("should deserialize the bytes");
        assert_eq!(output, [1_u8, 2].as_slice());
    }

    #[test]
    fn deserialize_box() {
        assert_eq!(Box::<u16>::size_hint(), u16::size_hint());
        let mut buffer = Bytes::copy_from_slice(&[1_u8, 2]);
        let output = Box::<u16>::deserialize(&mut buffer).expect("should deserialize the Box");
        assert_eq!(*output, 0x0102_u16);
    }
}
