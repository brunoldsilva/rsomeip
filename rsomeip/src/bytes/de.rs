use super::{Error, LengthField, Result};

mod tests;

pub struct Deserializer<'de> {
    buffer: &'de [u8],
    cursor: usize,
    limit: Option<usize>,
}

impl<'de> Deserializer<'de> {
    /// Create a new [`Deserializer`] with the given byte buffer.
    #[must_use]
    pub fn new(buffer: &'de [u8]) -> Self {
        Self {
            buffer,
            cursor: 0,
            limit: None,
        }
    }

    /// Creates a new [`Deserializer`] with an optional byte limit.
    ///
    /// This method creates a new [`Deserializer`] that shares the same underlying buffer
    /// with the current instance but imposes an optional limit on the number of bytes
    /// that can be read. The limit is specified as an offset from the current cursor
    /// position.
    ///
    /// # Errors
    ///
    /// This function will return an error if the new limit would exceed the buffers
    /// capacity.
    pub fn set_limit(&mut self, limit: Option<usize>) -> Result<()> {
        if let Some(len) = limit.map(|len| self.cursor + len) {
            if len > self.buffer.len() {
                return Err(Error::BufferOverflow);
            }
        }
        self.limit = limit.map(|len| self.cursor + len);
        Ok(())
    }

    /// Reads a value of type `T` from the buffer.
    ///
    /// This method reads a value of type `T` from the buffer, advancing the cursor
    /// accordingly. It performs bounds checking if a limit has been set, ensuring
    /// that the read operation does not exceed the specified limit.
    ///
    /// # Errors
    ///
    /// This function will return an error if reading a value would exceed the buffer's
    /// capacity or the set limit, or if converting the raw data into the value fails.
    pub fn read<T>(&mut self) -> Result<T>
    where
        T: for<'a> std::convert::TryFrom<&'a [u8]>,
    {
        if let Some(limit) = self.limit {
            if self.cursor + std::mem::size_of::<T>() > limit {
                return Err(Error::ExceedsLimit);
            }
        }
        self.buffer
            .get(self.cursor..self.cursor + std::mem::size_of::<T>())
            .ok_or(Error::BufferOverflow)
            .and_then(|v| {
                self.cursor += std::mem::size_of::<T>();
                v.try_into().map_err(|_| Error::Failure)
            })
    }
}

pub trait Deserialize: Sized {
    /// Deserializes an instance of the implementing type from a `Deserializer`.
    ///
    /// This function is used to deserialize an instance of the implementing type from the provided
    /// `Deserializer`. The implementation of this function should specify how the data should be
    /// deserialized and return the deserialized instance on success.
    ///
    /// # Errors
    ///
    /// This function will return an error if the deserialization process fails for any reason,
    /// such as encountering unexpected data or running out of data in the `Deserializer`.
    fn deserialize(de: &mut Deserializer) -> Result<Self>;

    /// Deserialize an instance of the implementing type with a specified length field.
    ///
    /// This function provides an extended version of `deserialize`, allowing you to deserialize
    /// an instance of the implementing type with a specified length field (`LengthField`)
    /// indicating the size of the data to be deserialized. It first reads the length field and
    /// uses it to limit the amount of data read from the `Deserializer`. Then, it calls the
    /// regular `deserialize` method with the limited data.
    ///
    /// # Errors
    /// This function will return an error if the deserialization process fails for any reason,
    /// such as encountering unexpected data, running out of data in the `Deserializer`, or
    /// exceeding the specified length.
    fn deserialize_len(de: &mut Deserializer, len: LengthField) -> Result<Self> {
        let length: usize = match len {
            LengthField::U8 => u8::deserialize(de)?.into(),
            LengthField::U16 => u16::deserialize(de)?.into(),
            LengthField::U32 => u32::deserialize(de)?
                .try_into()
                .map_err(|_| Error::Failure)?,
        };
        de.set_limit(Some(length))?;
        let res = Self::deserialize(de);
        de.set_limit(None)?;
        res
    }
}

macro_rules! deserialize_basic_type {
    ($t:ty) => {
        impl Deserialize for $t {
            fn deserialize(de: &mut Deserializer) -> Result<Self> {
                de.read().map(<$t>::from_be_bytes)
            }

            fn deserialize_len(_de: &mut Deserializer, _len: LengthField) -> Result<Self> {
                Err(Error::Message(String::from(
                    "basic types should not include a length field",
                )))
            }
        }
    };
}

deserialize_basic_type!(u8);
deserialize_basic_type!(u16);
deserialize_basic_type!(u32);
deserialize_basic_type!(u64);
deserialize_basic_type!(i8);
deserialize_basic_type!(i16);
deserialize_basic_type!(i32);
deserialize_basic_type!(i64);
deserialize_basic_type!(f32);
deserialize_basic_type!(f64);

impl Deserialize for bool {
    fn deserialize(de: &mut Deserializer) -> Result<Self> {
        u8::deserialize(de).map(|v| (v & 0x01) == 1u8)
    }

    fn deserialize_len(_de: &mut Deserializer, _len: LengthField) -> Result<Self> {
        Err(Error::Message(String::from(
            "basic types should not include a length field",
        )))
    }
}

impl<T, const N: usize> Deserialize for [T; N]
where
    T: Deserialize + Default,
{
    fn deserialize(de: &mut Deserializer) -> Result<Self> {
        Ok(std::array::from_fn(|_| {
            T::deserialize(de).unwrap_or_default()
        }))
    }
}

impl<T> Deserialize for Vec<T>
where
    T: Deserialize,
{
    fn deserialize(de: &mut Deserializer) -> Result<Self> {
        let mut vec = Self::new();
        loop {
            match T::deserialize(de) {
                Ok(element) => vec.push(element),
                Err(Error::ExceedsLimit) => break,
                Err(err) => return Err(err),
            }
        }
        Ok(vec)
    }
}

impl Deserialize for String {
    fn deserialize(de: &mut Deserializer) -> Result<Self> {
        /// Deserializes an UTF-8 encoded, null terminated string.
        fn deserialize_utf8(de: &mut Deserializer) -> Result<String> {
            if u8::deserialize(de)? != 0xbf_u8 {
                return Err(Error::Failure);
            }
            let mut raw_string = Vec::<u8>::new();
            loop {
                let value = u8::deserialize(de)?;
                if value == 0x00 {
                    break;
                }
                raw_string.push(value);
            }
            String::from_utf8(raw_string).map_err(|_| Error::Failure)
        }
        /// Deserializes an UTF-16 encoded, null terminated string.
        fn deserialize_utf16(de: &mut Deserializer, is_be: bool) -> Result<String> {
            let mut raw_string = Vec::<u16>::new();
            loop {
                let value = if is_be {
                    u16::deserialize(de)?
                } else {
                    u16::deserialize(de).map(u16::from_be)?
                };
                if value == 0x0000 {
                    break;
                }
                raw_string.push(value);
            }
            String::from_utf16(&raw_string).map_err(|_| Error::Failure)
        }
        match dbg!(u16::deserialize(de)?) {
            0xefbb => deserialize_utf8(de),
            0xfeff => deserialize_utf16(de, true),
            0xfffe => deserialize_utf16(de, false),
            _ => Err(Error::Failure),
        }
    }
}
