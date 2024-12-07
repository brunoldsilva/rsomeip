//! Data serialization according to the SOME/IP protocol.
//!
//! This module provides traits for serializing and deserializing data structures according to the
//! on-wire format of the SOME/IP protocol:
//!
//! - [`Deserialize`] provides methods for extracting data structures from bytes.
//!
//! - [`Serialize`] provides methods for writing data structures to bytes.
//!
//!   - [`SerializeString`] provides methods for writing strings to bytes using UTF-8, UTF-16-BE
//!     and UTF-16-LE encodings.
//!
//! Additionally, this module also provides implementations of these traits for all basic types and
//! some common structures of the Rust library.

// Re-export for convenience.
pub use bytes::{Buf, BufMut, Bytes, BytesMut};

mod de;
pub use de::{Deserialize, DeserializeError};

mod ser;
pub use ser::{Serialize, SerializeError, SerializeString};

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
