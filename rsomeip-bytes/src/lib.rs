#![doc = include_str!("../README.md")]

// Re-export for convenience.
pub use bytes::{Buf, BufMut, Bytes, BytesMut};

mod de;
pub use de::{Deserialize, DeserializeError};

mod ser;
pub use ser::{Serialize, SerializeError, SerializeString};

mod macros;

/// Size of the length field.
///
/// The size of a dynamic payload is encoded in a preceding length field which can be 1, 2, or 4
/// bytes long.
#[derive(Debug, Clone, Copy)]
pub enum LengthField {
    U8,
    U16,
    U32,
}
