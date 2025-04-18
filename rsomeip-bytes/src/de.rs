//! Deserialization according to the SOME/IP protocol.
//!
//! Provides the [`Deserialize`] trait for deserializing data, and several implementations of this
//! trait for types of the standard library.

use super::{Buf, Bytes, LengthField};

/// Deserialize data from a SOME/IP byte stream.
///
/// This trait provides methods for deserializing data structures from a byte stream ([`Bytes`])
/// encoded in the SOME/IP on-wire format.
///
/// [`deserialize`] is used to deserialize statically sized types, while [`deserialize_len`] is
/// used to deserialize dynamically sized types from the stream.
///
/// [`deserialize`]: Deserialize::deserialize
/// [`deserialize_len`]: Deserialize::deserialize_len
pub trait Deserialize {
    /// Type of the data that will be deserialized.
    type Output: Sized;

    /// Deserializes an instance of [`Deserialize::Output`] from the buffer.
    ///
    /// # Errors
    ///
    /// This function will return an error if the deserialization process fails for any reason,
    /// such as encountering unexpected data or running out of data in the buffer.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rsomeip_bytes::{Bytes, Deserialize, DeserializeError};
    /// let mut buffer = Bytes::copy_from_slice(&[1u8, 2u8]);
    /// assert_eq!(u8::deserialize(&mut buffer), Ok(1u8));
    /// assert_eq!(u8::deserialize(&mut buffer), Ok(2u8));
    /// assert_eq!(u8::deserialize(&mut buffer), Err(DeserializeError));
    /// ```
    fn deserialize(buffer: &mut Bytes) -> Result<Self::Output, DeserializeError>;

    /// Deserializes an instance of [`Deserialize::Output`] from the buffer.
    ///
    /// This method specifies a length field which is used to indicate the size of the data to be
    /// deserialized. This is necessary in case of dynamically sized data structures, like [`Vec`]
    /// or [`String`].
    ///
    /// # Errors
    ///
    /// This function will return an error if the deserialization process fails for any reason,
    /// such as encountering unexpected data, running out of data in the buffer, or exceeding the
    /// specified length.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rsomeip_bytes::{Bytes, Deserialize, DeserializeError, LengthField};
    ///
    /// let mut buffer = Bytes::copy_from_slice(&[2u8, 1u8, 2u8, 3u8]);
    /// let vec: Vec<u8> = Vec::deserialize_len(LengthField::U8, &mut buffer).unwrap();
    /// assert_eq!(&vec[..], &[1u8, 2u8][..]);
    ///
    /// assert_eq!(u8::deserialize(&mut buffer), Ok(3u8));
    /// assert_eq!(u8::deserialize(&mut buffer), Err(DeserializeError));
    /// ```
    fn deserialize_len(
        length: LengthField,
        buffer: &mut Bytes,
    ) -> Result<Self::Output, DeserializeError> {
        let length: usize = match length {
            LengthField::U8 => u8::deserialize(buffer)?.into(),
            LengthField::U16 => u16::deserialize(buffer)?.into(),
            LengthField::U32 => u32::deserialize(buffer)?
                .try_into()
                .map_err(|_| DeserializeError)?,
        };
        let mut payload = buffer.split_to(length);
        Self::deserialize(&mut payload)
    }
}

macro_rules! deserialize_basic_type {
    ($t:ty, $f:ident) => {
        impl Deserialize for $t {
            type Output = Self;

            fn deserialize(buffer: &mut Bytes) -> Result<Self::Output, DeserializeError> {
                if buffer.remaining() < size_of::<Self>() {
                    return Err(DeserializeError);
                }
                Ok(buffer.$f())
            }
        }
    };
}

deserialize_basic_type!(u8, get_u8);
deserialize_basic_type!(u16, get_u16);
deserialize_basic_type!(u32, get_u32);
deserialize_basic_type!(u64, get_u64);
deserialize_basic_type!(i8, get_i8);
deserialize_basic_type!(i16, get_i16);
deserialize_basic_type!(i32, get_i32);
deserialize_basic_type!(i64, get_i64);
deserialize_basic_type!(f32, get_f32);
deserialize_basic_type!(f64, get_f64);

impl Deserialize for bool {
    type Output = Self;

    fn deserialize(buffer: &mut Bytes) -> Result<Self::Output, DeserializeError> {
        if buffer.remaining() < size_of::<Self>() {
            return Err(DeserializeError);
        }
        Ok((buffer.get_u8() & 0x01) == 0x01)
    }
}

impl<T, const N: usize> Deserialize for [T; N]
where
    T: Deserialize<Output = T> + Default,
{
    type Output = Self;

    fn deserialize(buffer: &mut Bytes) -> Result<Self::Output, DeserializeError> {
        Ok(std::array::from_fn(|_| {
            T::deserialize(buffer).unwrap_or_default()
        }))
    }
}

impl<T> Deserialize for Vec<T>
where
    T: Deserialize<Output = T>,
{
    type Output = Self;

    fn deserialize(buffer: &mut Bytes) -> Result<Self::Output, DeserializeError> {
        let mut vec = vec![];
        while buffer.has_remaining() {
            vec.push(T::deserialize(buffer)?);
        }
        Ok(vec)
    }
}

macro_rules! deserialize_tuple {
    ( $( $name:ident )+ ) => {
        impl<$($name: Deserialize<Output=$name>),+> Deserialize for ($($name,)+) {
            type Output = Self;
            fn deserialize(buffer: &mut Bytes) -> Result<Self::Output, DeserializeError> {
                Ok((
                    $($name::deserialize(buffer)?,)+
                ))
            }
        }
    };
}

deserialize_tuple! { A }
deserialize_tuple! { A B }
deserialize_tuple! { A B C }
deserialize_tuple! { A B C D }
deserialize_tuple! { A B C D E }
deserialize_tuple! { A B C D E F }
deserialize_tuple! { A B C D E F G }
deserialize_tuple! { A B C D E F G H }
deserialize_tuple! { A B C D E F G H I }
deserialize_tuple! { A B C D E F G H I J }
deserialize_tuple! { A B C D E F G H I J K }
deserialize_tuple! { A B C D E F G H I J K L }

impl Deserialize for String {
    type Output = Self;

    fn deserialize(buffer: &mut Bytes) -> Result<Self::Output, DeserializeError> {
        /// Deserializes an UTF-8 encoded, null terminated string.
        fn deserialize_utf8(buffer: &mut Bytes) -> Result<String, DeserializeError> {
            if u8::deserialize(buffer)? != 0xbf_u8 {
                return Err(DeserializeError);
            }
            let mut raw_string = Vec::<u8>::new();
            loop {
                let value = u8::deserialize(buffer)?;
                if value == 0x00 {
                    break;
                }
                raw_string.push(value);
            }
            String::from_utf8(raw_string).map_err(|_| DeserializeError)
        }
        /// Deserializes an UTF-16 encoded, null terminated string.
        fn deserialize_utf16(buffer: &mut Bytes, is_be: bool) -> Result<String, DeserializeError> {
            let mut raw_string = Vec::<u16>::new();
            loop {
                let value = if is_be {
                    u16::deserialize(buffer)?
                } else {
                    u16::deserialize(buffer).map(u16::from_be)?
                };
                if value == 0x0000 {
                    break;
                }
                raw_string.push(value);
            }
            String::from_utf16(&raw_string).map_err(|_| DeserializeError)
        }
        match u16::deserialize(buffer)? {
            0xefbb => deserialize_utf8(buffer),
            0xfeff => deserialize_utf16(buffer, true),
            0xfffe => deserialize_utf16(buffer, false),
            _ => Err(DeserializeError),
        }
    }
}

impl Deserialize for Bytes {
    type Output = Self;

    fn deserialize(buffer: &mut Bytes) -> Result<Self::Output, DeserializeError> {
        Ok(buffer.split_to(buffer.remaining()))
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct DeserializeError;

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;

    macro_rules! test_deserialize_basic_type {
        ($t:ty, $name:tt) => {
            #[test]
            #[allow(clippy::cast_precision_loss, clippy::cast_lossless)]
            fn $name() {
                let mut buffer = Bytes::from((1 as $t).to_be_bytes().to_vec());
                assert_eq!(<$t>::deserialize(&mut buffer), Ok(1 as $t));
                assert_eq!(<$t>::deserialize(&mut buffer), Err(DeserializeError));
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
        let mut buffer = Bytes::copy_from_slice(&[0u8, 1u8]);
        assert_eq!(bool::deserialize(&mut buffer), Ok(false));
        assert_eq!(bool::deserialize(&mut buffer), Ok(true));
        assert_eq!(bool::deserialize(&mut buffer), Err(DeserializeError));
    }

    #[test]
    fn deserialize_len() {
        let mut buffer = Bytes::copy_from_slice(&[2u8, 1u8, 2u8, 3u8]);
        let vec: Vec<u8> =
            Vec::deserialize_len(LengthField::U8, &mut buffer).expect("should deserialize the vec");
        assert_eq!(&vec[..], &[1u8, 2u8][..]);
    }

    #[test]
    fn deserialize_array() {
        let mut buffer = Bytes::copy_from_slice(&[1u8, 2u8]);
        assert_eq!(<[u8; 2]>::deserialize(&mut buffer), Ok([1u8, 2u8]));
        assert_eq!(<[u8; 2]>::deserialize(&mut buffer), Ok([0u8, 0u8]));
    }

    #[test]
    fn deserialize_vec() {
        let mut buffer = Bytes::copy_from_slice(&[1u8, 2u8]);
        let vec: Vec<u8> = Vec::deserialize(&mut buffer).expect("should deserialize the vec");
        assert_eq!(&vec[..], &[1u8, 2u8][..]);
    }

    #[test]
    fn deserialize_malformed_vec() {
        let mut buffer = Bytes::copy_from_slice(&[1u8, 2u8, 3u8]);
        let error =
            Vec::<u16>::deserialize(&mut buffer).expect_err("should not deserialize the vec");
        assert_eq!(error, DeserializeError);
    }

    #[test]
    fn deserialize_tuple() {
        let mut buffer = Bytes::copy_from_slice(&[1u8, 2u8]);
        assert_eq!(<(u8, u8)>::deserialize(&mut buffer), Ok((1u8, 2u8)));
        assert_eq!(<(u8, u8)>::deserialize(&mut buffer), Err(DeserializeError));
    }

    #[test]
    fn deserialize_bytes() {
        let mut buffer = Bytes::copy_from_slice(&[2u8, 1u8, 2u8, 3u8, 4u8]);

        let bytes = Bytes::deserialize_len(LengthField::U8, &mut buffer)
            .expect("should deserialize the bytes");
        assert_eq!(&bytes[..], &[1u8, 2u8][..]);

        let bytes = Bytes::deserialize(&mut buffer).expect("should deserialize the bytes");
        assert_eq!(&bytes[..], &[3u8, 4u8][..]);
    }

    #[test]
    fn deserialize_utf8() {
        let raw_string = [
            0xef_u8, // Byte Order Mark
            0xbb, 0xbf, 0x72, 0x75, 0x73, 0x74, // "rust"
            0x00, // Delimiter
        ];
        let mut buffer = Bytes::copy_from_slice(&raw_string);
        assert_eq!(String::deserialize(&mut buffer), Ok(String::from("rust")));
    }

    #[test]
    fn deserialize_utf16_be() {
        let raw_string = [
            0xfe_u8, 0xff, // Big-Endian Byte Order Mark
            0x95, 0x08, // "锈"
            0x00, 0x00, // Delimiter
        ];
        let mut buffer = Bytes::copy_from_slice(&raw_string);
        assert_eq!(String::deserialize(&mut buffer), Ok(String::from("锈")));
    }

    #[test]
    fn deserialize_utf16_le() {
        let raw_string = [
            0xff_u8, 0xfe, // Little-Endian Byte Order Mark
            0x08, 0x95, // "锈"
            0x00, 0x00, // Delimiter
        ];
        let mut buffer = Bytes::copy_from_slice(&raw_string);
        assert_eq!(String::deserialize(&mut buffer), Ok(String::from("锈")));
    }
}
