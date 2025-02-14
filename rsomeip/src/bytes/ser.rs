//! Serialization according to the SOME/IP protocol.
//!
//! Provides the [`Serialize`] and [`SerializeString`] traits for serializing data, and several
//! implementations of these traits for types of the standard library.

use super::{BufMut, Bytes, BytesMut, LengthField};

/// Serialize data into a SOME/IP byte stream.
///
/// This trait provides methods for serializing data structures into a byte stream ([`BytesMut`])
/// encoded in the SOME/IP on-wire format.
///
/// [`serialize`] is used to serialize statically sized types, while [`serialize_len`] is used to
/// serialize dynamically sized types into the stream.
///
/// [`serialize`]: Serialize::serialize
/// [`serialize_len`]: Serialize::serialize_len
pub trait Serialize {
    /// Serializes the implementing type into the buffer.
    ///
    /// Returns the size of the serialized data.
    ///
    /// # Errors
    ///
    /// This function will return an error if serialization fails for any reason, such as the
    /// buffer not having enough space.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rsomeip::bytes::{BytesMut, Serialize};
    /// let mut buffer = BytesMut::with_capacity(3);
    /// assert_eq!(1u8.serialize(&mut buffer), Ok(1));
    /// assert_eq!(2u16.serialize(&mut buffer), Ok(2));
    /// assert_eq!(&buffer.freeze()[..], &[1u8, 0u8, 2u8][..]);
    /// ```
    fn serialize(&self, buffer: &mut BytesMut) -> Result<usize, SerializeError>;

    /// Serializes the implementing type into the buffer.
    ///
    /// This method specifies a length field which is used to indicate the size of the data to be
    /// serialized. This is necessary in case of dynamically sized data structures, like [`Vec`].
    ///
    /// Returns the size of the serialized data including the length field.
    ///
    /// # Errors
    ///
    /// This function will return an error if serialization fails for any reason, such as the
    /// buffer not having enough space, or the length of the serialized data exceeding the capacity
    /// of the length field.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rsomeip::bytes::{BytesMut, Serialize, LengthField};
    /// let mut buffer = BytesMut::with_capacity(3);
    /// let vec = vec![0u8, 1u8];
    /// assert_eq!(vec.serialize_len(LengthField::U8, &mut buffer), Ok(3));
    /// assert_eq!(&buffer.freeze()[..], &[2u8, 0u8, 1u8][..]);
    /// ```
    fn serialize_len(
        &self,
        length: LengthField,
        buffer: &mut BytesMut,
    ) -> Result<usize, SerializeError> {
        // Generic function to reduce code duplication from matching on `length`.
        fn serialize_len_internal<S, T, F>(
            data: &S,
            buffer: &mut BytesMut,
            write_length: F,
        ) -> Result<usize, SerializeError>
        where
            S: Serialize + ?Sized,
            T: TryFrom<usize>,
            F: FnOnce(&mut BytesMut, T),
        {
            // Split off a buffer for writing the payload and length field separately.
            if size_of::<T>() > buffer.capacity() {
                buffer.reserve(size_of::<T>());
            }
            let mut payload = buffer.split_off(size_of::<T>());

            // Serialize the payload first to get its size.
            let length = data.serialize(&mut payload)?;

            // Try serializing the length field itself.
            T::try_from(length)
                .map_err(|_| SerializeError)
                .map(|length| write_length(buffer, length))?;

            // Finally, rejoin the payload and length buffers.
            buffer.unsplit(payload);
            Ok(size_of::<T>() + length)
        }
        match length {
            LengthField::U8 => serialize_len_internal(self, buffer, BytesMut::put_u8),
            LengthField::U16 => serialize_len_internal(self, buffer, BytesMut::put_u16),
            LengthField::U32 => serialize_len_internal(self, buffer, BytesMut::put_u32),
        }
    }
}

macro_rules! serialize_basic_type {
    ($t:ty, $f:ident) => {
        impl Serialize for $t {
            fn serialize(&self, buffer: &mut BytesMut) -> Result<usize, SerializeError> {
                buffer.reserve(size_of::<$t>());
                BytesMut::$f(buffer, *self);
                Ok(size_of::<$t>())
            }
        }
    };
}

serialize_basic_type!(u8, put_u8);
serialize_basic_type!(u16, put_u16);
serialize_basic_type!(u32, put_u32);
serialize_basic_type!(u64, put_u64);
serialize_basic_type!(i8, put_i8);
serialize_basic_type!(i16, put_i16);
serialize_basic_type!(i32, put_i32);
serialize_basic_type!(i64, put_i64);
serialize_basic_type!(f32, put_f32);
serialize_basic_type!(f64, put_f64);

impl Serialize for bool {
    fn serialize(&self, buffer: &mut BytesMut) -> Result<usize, SerializeError> {
        if *self {
            1u8.serialize(buffer)
        } else {
            0u8.serialize(buffer)
        }
    }
}

impl<T> Serialize for [T]
where
    T: Serialize,
{
    fn serialize(&self, buffer: &mut BytesMut) -> Result<usize, SerializeError> {
        let mut total = 0;
        for element in self {
            total += element.serialize(buffer)?;
        }
        Ok(total)
    }
}

impl<T> Serialize for Vec<T>
where
    T: Serialize,
{
    fn serialize(&self, buffer: &mut BytesMut) -> Result<usize, SerializeError> {
        self.as_slice().serialize(buffer)
    }
}

impl<T, const N: usize> Serialize for [T; N]
where
    T: Serialize,
{
    fn serialize(&self, buffer: &mut BytesMut) -> Result<usize, SerializeError> {
        self.as_slice().serialize(buffer)
    }
}

impl<T> Serialize for T
where
    T: Fn(&mut BytesMut) -> Result<usize, SerializeError>,
{
    fn serialize(&self, buffer: &mut BytesMut) -> Result<usize, SerializeError> {
        self(buffer)
    }
}

macro_rules! serialize_tuple {
    ( $( $name:ident )+ ) => {
        impl<$($name: Serialize),+> Serialize for ($($name,)+)
        {
            #[allow(non_snake_case)]
            fn serialize(&self, buffer: &mut BytesMut) -> Result<usize, SerializeError> {
                let ($($name,)+) = self;
                let mut total = 0;
                $(total += $name.serialize(buffer)?;)+
                Ok(total)
            }
        }
    };
}

serialize_tuple! { A }
serialize_tuple! { A B }
serialize_tuple! { A B C }
serialize_tuple! { A B C D }
serialize_tuple! { A B C D E }
serialize_tuple! { A B C D E F }
serialize_tuple! { A B C D E F G }
serialize_tuple! { A B C D E F G H }
serialize_tuple! { A B C D E F G H I }
serialize_tuple! { A B C D E F G H I J }
serialize_tuple! { A B C D E F G H I J K }
serialize_tuple! { A B C D E F G H I J K L }

impl Serialize for Bytes {
    fn serialize(&self, buffer: &mut BytesMut) -> Result<usize, SerializeError> {
        buffer.extend_from_slice(&self[..]);
        Ok(self.len())
    }
}

/// Serialize strings of various encodings into a SOME/IP byte stream.
///
/// The protocol specifies three encodings (UTF-8, UTF-16 Little-Endian, and UTF-16 Big-Endian)
/// which require adding Byte-Order-Marks and Delimiters to the start and end of the data stream,
/// respectively.
///
/// This trait provides one method for serializing strings using each of those encodings.
pub trait SerializeString {
    /// Serializes the string into the buffer using UTF-8 encoding.
    ///
    /// A length field can be specified to indicate the size of the serialized data.
    ///
    /// # Errors
    ///
    /// This function will return an error if serialization fails for any reason, such as the
    /// buffer not having enough space, or the length of the serialized data exceeding the capacity
    /// of the length field.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rsomeip::bytes::{SerializeString, BytesMut};
    /// let mut buffer = BytesMut::with_capacity(10);
    /// assert_eq!("Hello!".serialize_utf8(&mut buffer, None), Ok(10));
    /// assert_eq!(
    ///     &buffer.freeze()[..],
    ///     [
    ///         0xef_u8, 0xbb, 0xbf, // UTF-8 Byte Order Mark
    ///         0x48, 0x65, 0x6c, 0x6c, 0x6f, 0x21, // Hello!
    ///         0x00  // Delimiter
    ///     ]
    /// );
    /// ```
    fn serialize_utf8(
        &self,
        buffer: &mut BytesMut,
        len: Option<LengthField>,
    ) -> Result<usize, SerializeError>;

    /// Serializes the string into the buffer using UTF-16 Big-Endian encoding.
    ///
    /// A length field can be specified to indicate the size of the serialized data.
    ///
    /// # Errors
    ///
    /// This function will return an error if serialization fails for any reason, such as the
    /// buffer not having enough space, or the length of the serialized data exceeding the capacity
    /// of the length field.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rsomeip::bytes::{SerializeString, BytesMut};
    /// let mut buffer = BytesMut::with_capacity(12);
    /// assert_eq!("语言处理".serialize_utf16_be(&mut buffer, None), Ok(12));
    /// assert_eq!(
    ///     &buffer.freeze()[..],
    ///     [
    ///         0xfe_u8, 0xff, // UTF-16 Big Endian Byte Order Mark
    ///         0x8b, 0xed, 0x8a, 0x00, 0x59, 0x04, 0x74, 0x06, // 语言处理
    ///         0x00, 0x00, // Delimiter
    ///     ]
    /// );
    /// ```
    fn serialize_utf16_be(
        &self,
        buffer: &mut BytesMut,
        len: Option<LengthField>,
    ) -> Result<usize, SerializeError>;

    /// Serializes the string into the buffer using UTF-16 Little-Endian encoding.
    ///
    /// A length field can be specified to indicate the size of the serialized data.
    ///
    /// # Errors
    ///
    /// This function will return an error if serialization fails for any reason, such as the
    /// buffer not having enough space, or the length of the serialized data exceeding the capacity
    /// of the length field.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rsomeip::bytes::{SerializeString, BytesMut};
    /// let mut buffer = BytesMut::with_capacity(12);
    /// assert_eq!("语言处理".serialize_utf16_le(&mut buffer, None), Ok(12));
    /// assert_eq!(
    ///     &buffer.freeze()[..],
    ///     [
    ///         0xff_u8, 0xfe, // UTF-16 Little Endian Byte Order Mark
    ///         0xed, 0x8b, 0x00, 0x8a, 0x04, 0x59, 0x06, 0x74, // 语言处理
    ///         0x00, 0x00, // Delimiter
    ///     ]
    /// );
    /// ```
    fn serialize_utf16_le(
        &self,
        buffer: &mut BytesMut,
        len: Option<LengthField>,
    ) -> Result<usize, SerializeError>;
}

impl SerializeString for str {
    fn serialize_utf8(
        &self,
        buffer: &mut BytesMut,
        len: Option<LengthField>,
    ) -> Result<usize, SerializeError> {
        let serialize_op = |buffer: &mut BytesMut| {
            [0xef_u8, 0xbb, 0xbf].serialize(buffer)?; // Byte Order Mark.
            let size = self.as_bytes().serialize(buffer)?;
            0x00_u8.serialize(buffer)?; // Delimiter.
            Ok(size + 4)
        };
        match len {
            Some(len) => serialize_op.serialize_len(len, buffer),
            None => serialize_op.serialize(buffer),
        }
    }

    fn serialize_utf16_be(
        &self,
        buffer: &mut BytesMut,
        len: Option<LengthField>,
    ) -> Result<usize, SerializeError> {
        let serialize_op = |buffer: &mut BytesMut| {
            0xfeff_u16.serialize(buffer)?; // Byte Order Mark.
            let mut total = 0;
            for byte in self.encode_utf16() {
                total += byte.serialize(buffer)?;
            }
            0x0000_u16.serialize(buffer)?; // Delimiter.
            Ok(total + 4)
        };
        match len {
            Some(len) => serialize_op.serialize_len(len, buffer),
            None => serialize_op.serialize(buffer),
        }
    }

    fn serialize_utf16_le(
        &self,
        buffer: &mut BytesMut,
        len: Option<LengthField>,
    ) -> Result<usize, SerializeError> {
        let serialize_op = |buffer: &mut BytesMut| {
            0xfffe_u16.serialize(buffer)?; // Byte Order Mark.
            let mut total = 0;
            for byte in self.encode_utf16() {
                total += byte.to_le_bytes().serialize(buffer)?;
            }
            0x0000_u16.serialize(buffer)?; // Delimiter.
            Ok(total + 4)
        };
        match len {
            Some(len) => serialize_op.serialize_len(len, buffer),
            None => serialize_op.serialize(buffer),
        }
    }
}

/// Represents an error when serializing data.
#[derive(Debug, PartialEq, Eq)]
pub struct SerializeError;

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! test_serialize_basic_type {
        ($t:ty, $name:ident) => {
            #[test]
            fn $name() {
                let mut buffer = BytesMut::with_capacity(size_of::<$t>());
                let result = <$t>::MAX.serialize(&mut buffer);
                assert_eq!(result, Ok(size_of::<$t>()));
                assert_eq!(&buffer.freeze()[..], <$t>::MAX.to_be_bytes());
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
        let mut buffer = BytesMut::with_capacity(1);
        assert_eq!(true.serialize(&mut buffer), Ok(1));
        assert_eq!(&buffer.freeze()[..], &[1u8]);
    }

    #[test]
    fn serialize_len() {
        let mut buffer = BytesMut::with_capacity(3);
        let vec = vec![0u8, 1u8];
        assert_eq!(vec.serialize_len(LengthField::U8, &mut buffer), Ok(3));
        assert_eq!(&buffer.freeze()[..], &[2u8, 0u8, 1u8][..]);
    }

    #[test]
    fn serialize_vec() {
        let mut buffer = BytesMut::with_capacity(2);
        let vec = vec![1u8, 2u8];
        let size = vec
            .serialize(&mut buffer)
            .expect("should serialize the vec");
        assert_eq!(size, 2);
        assert_eq!(&buffer.freeze()[..], &[1u8, 2u8][..]);
    }

    #[test]
    fn serialize_array() {
        let mut buffer = BytesMut::with_capacity(2);
        let array = [1u8, 2u8];
        let size = array
            .serialize(&mut buffer)
            .expect("should serialize the array");
        assert_eq!(size, 2);
        assert_eq!(&buffer.freeze()[..], &[1u8, 2u8][..]);
    }

    #[test]
    fn serialize_fn() {
        let mut buffer = BytesMut::with_capacity(2);
        let fun = |buffer: &mut BytesMut| {
            1u8.serialize(buffer)?;
            2u8.serialize(buffer)?;
            Ok(2usize)
        };
        let size = fun
            .serialize(&mut buffer)
            .expect("should serialize the function");
        assert_eq!(size, 2);
        assert_eq!(&buffer.freeze()[..], &[1u8, 2u8][..]);
    }

    #[test]
    fn serialize_tuple() {
        let mut buffer = BytesMut::with_capacity(2);
        let tuple = (1u8, 2u8);
        let size = tuple
            .serialize(&mut buffer)
            .expect("should serialize the tuple");
        assert_eq!(size, 2);
        assert_eq!(&buffer.freeze()[..], &[1u8, 2u8][..]);
    }

    #[test]
    fn serialize_bytes() {
        let mut buffer = BytesMut::with_capacity(2);
        let bytes = Bytes::copy_from_slice(&[1u8, 2u8]);
        assert_eq!(bytes.serialize(&mut buffer), Ok(2));
        assert_eq!(&buffer.freeze()[..], &[1u8, 2u8][..]);
    }

    #[test]
    fn serialize_utf8() {
        let mut buffer = BytesMut::with_capacity(10);
        let size = "Hello!"
            .serialize_utf8(&mut buffer, None)
            .expect("should serialize the string");
        assert_eq!(size, 10);
        assert_eq!(
            &buffer.freeze()[..],
            [
                0xef_u8, 0xbb, 0xbf, // UTF-8 Byte Order Mark
                0x48, 0x65, 0x6c, 0x6c, 0x6f, 0x21, // Hello!
                0x00  // Delimiter
            ]
        );
    }

    #[test]
    fn serialize_utf16_be() {
        let mut buffer = BytesMut::with_capacity(12);
        let size = "语言处理"
            .serialize_utf16_be(&mut buffer, None)
            .expect("should serialize the string");
        assert_eq!(size, 12);
        assert_eq!(
            &buffer.freeze()[..],
            [
                0xfe_u8, 0xff, // UTF-16 Big Endian Byte Order Mark
                0x8b, 0xed, 0x8a, 0x00, 0x59, 0x04, 0x74, 0x06, // 语言处理
                0x00, 0x00, // Delimiter
            ]
        );
    }

    #[test]
    fn serialize_utf16_le() {
        let mut buffer = BytesMut::with_capacity(12);
        let size = "语言处理"
            .serialize_utf16_le(&mut buffer, None)
            .expect("should serialize the string");
        assert_eq!(size, 12);
        assert_eq!(
            &buffer.freeze()[..],
            [
                0xff_u8, 0xfe, // UTF-16 Little Endian Byte Order Mark
                0xed, 0x8b, 0x00, 0x8a, 0x04, 0x59, 0x06, 0x74, // 语言处理
                0x00, 0x00, // Delimiter
            ]
        );
    }
}
