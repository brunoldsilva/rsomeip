/// Serializes a series of variables into the given `buffer`.
///
/// An optional `length` parameter can be specified to serialize the variables using [`serialize_len`].
///
/// # Performance Considerations
///
/// This macro is provided simply as a convenience for implementing the [`Serialize`] trait.
///
/// It uses tuples to serialize the variables into the buffer, which may not work well in every
/// situation.
///
/// # Examples
///
/// ```rust
/// use rsomeip_bytes::{BytesMut, Serialize, serialize_into};
///
/// let [a, b, c, d] = [1u8, 2, 3, 4];
/// let mut buffer = BytesMut::new();
///
/// let mut size = 0;
/// size += serialize_into!(&mut buffer, a, b).expect("should serialize");
/// size += serialize_into!(&mut buffer, length = U8, c, d).expect("should serialize");
///
/// assert_eq!(size, 5);
/// assert_eq!(buffer[..], [1u8, 2, 2, 3, 4]);
/// ```
///
/// [`Serialize`]: crate::Serialize
/// [`serialize_len`]: crate::Serialize::serialize_len
#[macro_export]
macro_rules! serialize_into {
    ($buffer:expr, length=$length:ident, $($member:expr),+) => {
        (
            $($member,)+
        ).serialize_len($crate::LengthField::$length, $buffer)
    };
    ($buffer:expr, $($member:expr),+) => {
        (
            $($member,)+
        ).serialize($buffer)
    };
}

/// Returns the combined size hint of a series of variables.
///
/// An optional `length` parameter can be specified to include a length field in the size
/// calculations.
///
/// # Examples
///
/// ```rust
/// use rsomeip_bytes::{Serialize, size_hint};
///
/// let [a, b, c, d] = [1u8, 2, 3, 4];
///
/// let mut size = 0;
/// size += size_hint!(a, b);
/// size += size_hint!(length = U32, c, d);
/// assert_eq!(size, 8);
/// ```
#[macro_export]
macro_rules! size_hint {
    (length=$length:ident, $($member:expr),+) => {{
        let mut size = 0;
        size += (|length: $crate::LengthField| {
                match length {
                    $crate::LengthField::U8 => 0u8.size_hint(),
                    $crate::LengthField::U16 => 0u16.size_hint(),
                    $crate::LengthField::U32 => 0u32.size_hint(),
                }
            })($crate::LengthField::$length);
        $(size += $member.size_hint();)+
        size
    }};
    ($($member:expr),+) => {{
        let mut size = 0;
        $(size += $member.size_hint();)+
        size
    }};
}

/// Deserializes a series of variables from the given `buffer`.
///
/// An optional `length` parameter can be specified to deserialize the variables using
/// [`deserialize_len`].
///
/// # Performance Considerations
///
/// This macro is provided simply as a convenience for implementing the [`Deserialize`] trait.
///
/// It uses tuples to deserialize the variables from the buffer, which may not be the most performant
/// solution for every use case, but is one that works well in most situations.
///
/// # Examples
///
/// ```rust
/// use rsomeip_bytes::{Bytes, Deserialize, deserialize_from};
///
/// let mut buffer = Bytes::copy_from_slice(&[1u8, 2, 2, 3, 4][..]);
///
/// let (a, b) = deserialize_from!(&mut buffer, u8, u8).expect("should deserialize");
/// let (c, d) = deserialize_from!(&mut buffer, length = U8, u8, u8).expect("should deserialize");
///
/// assert_eq!([a, b, c, d], [1u8, 2, 3, 4]);
/// ```
///
/// [`Deserialize`]: crate::Deserialize
/// [`deserialize_len`]: crate::Deserialize::deserialize_len
#[macro_export]
macro_rules! deserialize_from {
    ($buffer:expr, length=$length:ident, $($repr:ty),+) => {
        <(
            $($repr),+
        )>::deserialize_len($crate::LengthField::$length, $buffer)
    };
    ($buffer:expr, $($repr:ty),+) => {
        <(
            $($repr),+
        )>::deserialize($buffer)
    };
}

#[cfg(test)]
mod tests {
    use crate::{Deserialize, Serialize};
    use bytes::{Bytes, BytesMut};

    #[test]
    fn serialize_macro() {
        let [a, b, c, d] = [1u8, 2, 3, 4];
        let mut buffer = BytesMut::new();
        let mut size = 0;
        size += serialize_into!(&mut buffer, &a, b).expect("should serialize");
        size += serialize_into!(&mut buffer, length = U8, &c, d).expect("should serialize");
        assert_eq!(size, 5);
        assert_eq!(buffer[..], [1u8, 2, 2, 3, 4]);
    }

    #[test]
    fn size_hint_macro() {
        let [a, b, c, d] = [1u8, 2, 3, 4];
        let mut size = 0;
        size += size_hint!(&a, b);
        size += size_hint!(length = U32, &c, d);
        assert_eq!(size, 8);
    }

    #[test]
    fn deserialize_macro() {
        let mut buffer = Bytes::copy_from_slice(&[1u8, 2, 2, 3, 4][..]);
        let (a, b) = deserialize_from!(&mut buffer, u8, u8).expect("should deserialize");
        let (c, d) =
            deserialize_from!(&mut buffer, length = U8, u8, u8).expect("should deserialize");
        assert_eq!([a, b, c, d], [1u8, 2, 3, 4]);
    }
}
