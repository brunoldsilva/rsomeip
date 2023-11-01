pub mod de;
pub mod error;
pub mod ser;

pub use de::{Deserialize, Deserializer};
pub use error::{Error, Result};
pub use ser::{Serialize, Serializer};

#[derive(Debug, Clone, Copy)]
pub enum LengthField {
    U8,
    U16,
    U32,
}
