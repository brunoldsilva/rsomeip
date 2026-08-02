//! Dynamic arrays.
//!
//! This module provides the [`DynamicArray<Length, Collection>`] type which is used to serialize
//! and deserialize arbitrary collections with specific length fields.

use crate::{Deserialize, DeserializeError, Serialize, SerializeError};
use core::{iter, marker::PhantomData};

/// Dynamically sized array.
///
/// Used to serialize and deserialize arbitrary collections using specific length fields.
///
/// Works on any type that implements [`IntoIterator<Item = Serialize>`] which includes [`Vec<T>`],
/// [`BTreeMap<K, V>`], and [`Option<T>`].
///
/// The length can be [`LengthZero`], [`LengthU8`], [`LengthU16`] or [`LengthU32`] for 0, 8, 16, and
/// 32 bit length fields, respectively.
///
/// # Examples
///
/// ```rust
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// use rsomeip_bytes::{Deserialize as _, DynamicArray, LengthU32, Serialize as _};
///
/// // Works on any value that implements `IntoIterator`.
/// let value = vec![1, 2, 3, 4, 5];
///
/// // Specify the size of the length field in the generic parameters. It will be included when
/// // serializing the value.
/// let array = DynamicArray::<LengthU32, _>::from(&value);
///
/// // The array also implements the `Serialize` trait.
/// let mut bytes = array.to_bytes()?;
///
/// // And `Deserialize`, as well.
/// let output = DynamicArray::<LengthU32, Vec<i32>>::deserialize(&mut bytes)?;
/// assert_eq!(value, output);
/// # Ok(()) }
/// ```
///
/// [`Vec<T>`]: alloc::vec::Vec
/// [`BTreeMap<K, V>`]: alloc::collections::BTreeMap
/// [`LengthZero`]: crate::LengthZero
/// [`LengthU8`]: crate::LengthU8
/// [`LengthU16`]: crate::LengthU16
/// [`LengthU32`]: crate::LengthU32
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[must_use = "must call `serialize` on this array"]
pub struct DynamicArray<Length, Collection> {
    /// Collection to serialize.
    inner: Collection,
    /// Length to include before the value.
    _length: PhantomData<Length>,
}

impl<Length, Collection> From<Collection> for DynamicArray<Length, Collection>
where
    Length: crate::Length,
    Collection: IntoIterator + Copy,
    Collection::Item: Serialize,
{
    fn from(collection: Collection) -> Self {
        Self {
            inner: collection,
            _length: PhantomData,
        }
    }
}

impl<Length, Collection> Serialize for DynamicArray<Length, Collection>
where
    Length: crate::Length,
    Collection: IntoIterator + Copy,
    Collection::Item: Serialize,
{
    fn serialize<Buffer>(&self, buffer: &mut Buffer) -> Result<usize, SerializeError>
    where
        Buffer: bytes::BufMut + ?Sized,
    {
        Length::serialize_with(
            |_value, buffer| {
                // Serialize each item of the array individually and add up the results.
                self.inner.into_iter().try_fold(0_usize, |acc, elem| {
                    elem.serialize(buffer)
                        .and_then(|size| acc.checked_add(size).ok_or(SerializeError::SizeOverflow))
                })
            },
            |_value| {
                // Add up the size of each element in the array.
                self.inner.into_iter().try_fold(0_usize, |acc, elem| {
                    elem.size().and_then(|size| acc.checked_add(size))
                })
            },
            0,
            buffer,
        )
    }

    fn size(&self) -> Option<usize> {
        // Add up the size of each element in the array plus the size of the length field.
        self.inner
            .into_iter()
            .try_fold(Length::size(), |acc, elem| {
                elem.size().and_then(|size| acc.checked_add(size))
            })
    }
}

impl<Length, Collection> Deserialize for DynamicArray<Length, Collection>
where
    Length: crate::Length,
    Collection: IntoIterator + FromIterator<<Collection::Item as Deserialize>::Output>,
    Collection::Item: Deserialize,
{
    type Output = Collection;

    fn deserialize<Buffer>(buffer: &mut Buffer) -> Result<Self::Output, DeserializeError>
    where
        Buffer: bytes::Buf + ?Sized,
    {
        Length::deserialize_with(
            |buffer| {
                // First error encountered during deserialization.
                let mut error: Option<DeserializeError> = None;
                let collection = iter::from_fn(|| {
                    // Deserialize one item at a time.
                    if error.is_none() && buffer.has_remaining() {
                        Collection::Item::deserialize(buffer)
                            .map_err(|err| {
                                // Record the error.
                                error = Some(err);
                            })
                            .ok()
                    } else {
                        None
                    }
                })
                .collect();
                // Return the first error or the collection if none was encountered.
                error.map_or_else(|| Ok(collection), Err)
            },
            buffer,
        )
    }

    fn size_hint() -> Option<usize> {
        // Only the size of the length field is known at compile time.
        Some(Length::size())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LengthU32;
    use alloc::{vec, vec::Vec};

    #[test]
    fn serialize() {
        let value = vec![1_u16, 2, 3, 4];
        let array = <DynamicArray<LengthU32, _>>::from(&value);
        let bytes = <DynamicArray<LengthU32, _>>::from(&value)
            .to_bytes()
            .expect("should serialize the Vec");
        assert_eq!(Some(bytes.len()), array.size());
        assert_eq!(bytes, [0_u8, 0, 0, 8, 0, 1, 0, 2, 0, 3, 0, 4].as_slice());
    }

    #[test]
    fn serialize_empty() {
        let value: Vec<u16> = vec![];
        let array = <DynamicArray<LengthU32, _>>::from(&value);
        let bytes = <DynamicArray<LengthU32, _>>::from(&value)
            .to_bytes()
            .expect("should serialize the Vec");
        assert_eq!(Some(bytes.len()), array.size());
        assert_eq!(bytes, [0_u8, 0, 0, 0].as_slice());
    }

    #[test]
    fn deserialize() {
        let buffer = [0_u8, 0, 0, 8, 0, 1, 0, 2, 0, 3, 0, 4, 0, 5];
        let output = DynamicArray::<LengthU32, Vec<u16>>::deserialize(&mut buffer.as_slice())
            .expect("should deserialize the Vec");
        assert_eq!(output, vec![1_u16, 2, 3, 4]);
        assert_eq!(DynamicArray::<LengthU32, Vec<u16>>::size_hint(), Some(4));
    }

    #[test]
    fn deserialize_empty() {
        let buffer = [0_u8, 0, 0, 0, 0, 1];
        let output = DynamicArray::<LengthU32, Vec<u16>>::deserialize(&mut buffer.as_slice())
            .expect("should deserialize the Vec");
        assert_eq!(output, Vec::<u16>::new());
        assert_eq!(DynamicArray::<LengthU32, Vec<u16>>::size_hint(), Some(4));
    }

    #[test]
    fn deserialize_error() {
        let buffer = [0_u8, 0, 0, 3, 0, 1, 0];
        let error = DynamicArray::<LengthU32, Vec<u16>>::deserialize(&mut buffer.as_slice())
            .expect_err("shouldn't deserialize the Vec");
        assert_eq!(error, DeserializeError::InsufficientData);
    }

    #[test]
    fn dynamic_vec() {
        let value = vec![1_u16, 2, 3, 4];
        let mut bytes = DynamicArray::<LengthU32, _>::from(&value)
            .to_bytes()
            .expect("should serialize the Vec");
        let output = DynamicArray::<LengthU32, Vec<u16>>::deserialize(&mut bytes)
            .expect("should deserialize the Vec");
        assert_eq!(value, output);
    }

    #[test]
    #[cfg(feature = "std")]
    fn dynamic_hash_map() {
        use std::collections::HashMap;

        let value = HashMap::<i32, i32>::from_iter(vec![(1_i32, 2_i32), (3_i32, 4_i32)]);
        let mut bytes = DynamicArray::<LengthU32, _>::from(&value)
            .to_bytes()
            .expect("should serialize the HashMap");
        let output = DynamicArray::<LengthU32, HashMap<i32, i32>>::deserialize(&mut bytes)
            .expect("should deserialize the HashMap");
        assert_eq!(value, output);
    }
}
