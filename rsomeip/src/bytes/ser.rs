use super::{Error, LengthField, Result};
use std::ops::Range;

mod tests;

/// Writes data to byte buffers, according to the SOME/IP specification.
pub struct Serializer<'a> {
    buffer: &'a mut [u8],
    cursor: usize,
}

impl<'a> Serializer<'a> {
    /// Returns a new [Serializer] that writes to the given buffer.
    pub fn new(buffer: &'a mut [u8]) -> Self {
        Self { buffer, cursor: 0 }
    }

    /// Copies the contents of the source slice into the buffer, at the cursor's current
    /// position.
    ///
    /// If successful, it will advance the cursor position by the amount of bytes written,
    /// so that the next call to `write` will place the new bytes after the last ones.
    ///
    /// # Errors
    ///
    /// Returns an error if writing to the cursor's current position would exceed
    /// the buffer's capacity.
    pub fn write(&mut self, src: &[u8]) -> Result<usize> {
        self.write_to(self.cursor..self.cursor + src.len(), src)
    }

    /// Copies the contents of the source slice into the buffer, over the given range.
    ///
    /// If the range goes past the cursors old position, it will move the cursor to the new
    /// end position, so that the next call to `write` will place the new bytes after the
    /// range.
    ///
    /// # Errors
    ///
    /// Returns an error if writing at the given range would exceed the buffer's capacity,
    /// in which case, nothing is written to the buffer nor the cursor updated.
    pub fn write_to(&mut self, range: Range<usize>, src: &[u8]) -> Result<usize> {
        self.buffer
            .get_mut(range.clone())
            .map(|dst| {
                dst.copy_from_slice(src);
                self.cursor = self.cursor.max(range.end);
                range.len()
            })
            .ok_or(Error::BufferOverflow)
    }

    /// Advances the cursor position without writing anything to the buffer, returning the
    /// range that was skipped.
    ///
    /// This function may be useful for inserting padding in the serialized data.
    ///
    /// # Errors
    ///
    /// Returns an error if the new cursor position would exceed the buffer's capacity,
    /// in which case, the cursor is not moved.
    pub fn skip(&mut self, len: usize) -> Result<Range<usize>> {
        let range = self.cursor..self.cursor + len;
        self.buffer
            .get_mut(range.clone())
            .map(|_| {
                self.cursor += len;
                range
            })
            .ok_or(Error::BufferOverflow)
    }
}

/// A trait for serializing data structures into a byte stream.
pub trait Serialize {
    /// Serialize the data structure into the provided `Serializer`.
    ///
    /// This function is called to convert the data structure into a byte stream
    /// using the provided `Serializer`. The implementation of this function
    /// should specify how the data should be serialized.
    ///
    /// # Errors
    ///
    /// This function will return an error if serialization fails.
    fn serialize(&self, ser: &mut Serializer) -> Result<()>;

    /// Serialize the data structure with a specified length field.
    ///
    /// This function provides an extended version of `serialize`, allowing you
    /// to specify a length field to represent the size of the serialized data.
    /// It will first serialize the data using the `serialize` method, and then
    /// prepend the length of the serialized data as specified by the `LengthField`.
    ///
    /// # Errors
    ///
    /// This function will return an error if serialization fails or the length exceeds the
    /// capacity of the length field.
    fn serialize_len(&self, ser: &mut Serializer, len: LengthField) -> Result<()> {
        let skipped = match len {
            LengthField::U8 => ser.skip(std::mem::size_of::<u8>()),
            LengthField::U16 => ser.skip(std::mem::size_of::<u16>()),
            LengthField::U32 => ser.skip(std::mem::size_of::<u32>()),
        }?;
        self.serialize(ser)?;
        let length = ser.skip(0)?.end - skipped.end;
        match len {
            LengthField::U8 => u8::try_from(length)
                .map_err(|_| Error::LengthOverflow)
                .and_then(|length| ser.write_to(skipped, &length.to_be_bytes())),
            LengthField::U16 => u16::try_from(length)
                .map_err(|_| Error::LengthOverflow)
                .and_then(|length| ser.write_to(skipped, &length.to_be_bytes())),
            LengthField::U32 => u32::try_from(length)
                .map_err(|_| Error::LengthOverflow)
                .and_then(|length| ser.write_to(skipped, &length.to_be_bytes())),
        }?;
        Ok(())
    }
}

macro_rules! serialize_basic_type {
    ($t:ty) => {
        impl Serialize for $t {
            fn serialize(&self, ser: &mut Serializer) -> Result<()> {
                ser.write(&self.to_be_bytes()).map(|_| ())
            }
        }
    };
}

serialize_basic_type!(u8);
serialize_basic_type!(u16);
serialize_basic_type!(u32);
serialize_basic_type!(u64);
serialize_basic_type!(i8);
serialize_basic_type!(i16);
serialize_basic_type!(i32);
serialize_basic_type!(i64);
serialize_basic_type!(f32);
serialize_basic_type!(f64);

impl Serialize for bool {
    fn serialize(&self, ser: &mut Serializer) -> Result<()> {
        if *self {
            1u8.serialize(ser)
        } else {
            0u8.serialize(ser)
        }
    }
}

impl<T> Serialize for [T]
where
    T: Serialize,
{
    fn serialize(&self, ser: &mut Serializer) -> Result<()> {
        self.iter().try_for_each(|e| e.serialize(ser))
    }
}

impl<T> Serialize for Vec<T>
where
    T: Serialize,
{
    fn serialize(&self, ser: &mut Serializer) -> Result<()> {
        self.as_slice().serialize(ser)
    }
}

impl<T, const N: usize> Serialize for [T; N]
where
    T: Serialize,
{
    fn serialize(&self, ser: &mut Serializer) -> Result<()> {
        self.as_slice().serialize(ser)
    }
}

impl<T> Serialize for T
where
    T: Fn(&mut Serializer) -> Result<()>,
{
    fn serialize(&self, ser: &mut Serializer) -> Result<()> {
        self(ser)
    }
}

/// A trait for serializing strings into various encodings and with optional length fields.
pub trait SerializeString {
    /// Serialize the string into UTF-8 encoding.
    ///
    /// This function serializes the string into UTF-8 encoding and writes it to the
    /// provided `Serializer`. If a `LengthField` is specified, the length of the
    /// serialized string can be prepended before the actual data.
    ///
    /// # Errors
    ///
    /// This function will return an error if serialization fails.
    fn serialize_utf8(&self, ser: &mut Serializer, len: Option<LengthField>) -> Result<()>;

    /// Serialize the string into UTF-16 encoding with big-endian byte order.
    ///
    /// This function serializes the string into UTF-16 encoding with big-endian byte
    /// order and writes it to the provided `Serializer`. If a `LengthField` is
    /// specified, the length of the serialized string can be prepended before the
    /// actual data.
    ///
    /// # Errors
    ///
    /// This function will return an error if serialization fails.
    fn serialize_utf16_be(&self, ser: &mut Serializer, len: Option<LengthField>) -> Result<()>;

    /// Serialize the string into UTF-16 encoding with little-endian byte order.
    ///
    /// This function serializes the string into UTF-16 encoding with little-endian byte
    /// order and writes it to the provided `Serializer`. If a `LengthField` is
    /// specified, the length of the serialized string can be prepended before the
    /// actual data.
    ///
    /// # Errors
    ///
    /// This function will return an error if serialization fails.
    fn serialize_utf16_le(&self, ser: &mut Serializer, len: Option<LengthField>) -> Result<()>;
}

impl SerializeString for str {
    fn serialize_utf8(&self, ser: &mut Serializer, len: Option<LengthField>) -> Result<()> {
        let serialize_op = |ser: &mut Serializer| {
            [0xef_u8, 0xbb, 0xbf].serialize(ser)?; // Byte Order Mark.
            self.as_bytes().serialize(ser)?;
            0x00_u8.serialize(ser) // Delimiter.
        };
        match len {
            Some(len) => serialize_op.serialize_len(ser, len),
            None => serialize_op.serialize(ser),
        }
    }

    fn serialize_utf16_be(&self, ser: &mut Serializer, len: Option<LengthField>) -> Result<()> {
        let serialize_op = |ser: &mut Serializer| {
            0xfeff_u16.serialize(ser)?; // Byte Order Mark.
            self.encode_utf16().try_for_each(|e| e.serialize(ser))?;
            0x0000_u16.serialize(ser) // Delimiter.
        };
        match len {
            Some(len) => serialize_op.serialize_len(ser, len),
            None => serialize_op.serialize(ser),
        }
    }

    fn serialize_utf16_le(&self, ser: &mut Serializer, len: Option<LengthField>) -> Result<()> {
        let serialize_op = |ser: &mut Serializer| {
            0xfffe_u16.serialize(ser)?; // Byte Order Mark.
            self.encode_utf16()
                .try_for_each(|e| e.to_le_bytes().serialize(ser))?;
            0x0000_u16.serialize(ser) // Delimiter.
        };
        match len {
            Some(len) => serialize_op.serialize_len(ser, len),
            None => serialize_op.serialize(ser),
        }
    }
}
