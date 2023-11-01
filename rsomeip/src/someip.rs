use crate::bytes::{self, Deserialize, Serialize};
use types::Payload;

mod tests;
mod types;

/// A payload addressed to a given service and method.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Message {
    pub service_id: u16,
    pub method_id: u16,
    pub payload: Payload,
}

impl Message {
    /// Creates a default [`Message`] instance.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a [`MessageBuilder`] to help build a new message.
    #[must_use]
    pub fn builder() -> MessageBuilder {
        MessageBuilder::new()
    }
}

impl std::fmt::Display for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[{:04x}.{:04x}]{}",
            self.service_id, self.method_id, self.payload
        )
    }
}

impl Serialize for Message {
    fn serialize(&self, ser: &mut bytes::Serializer) -> bytes::Result<()> {
        self.service_id.serialize(ser)?;
        self.method_id.serialize(ser)?;
        self.payload.serialize_len(ser, bytes::LengthField::U32)
    }
}

impl Deserialize for Message {
    fn deserialize(de: &mut bytes::Deserializer) -> bytes::Result<Self> {
        Ok(Self {
            service_id: u16::deserialize(de)?,
            method_id: u16::deserialize(de)?,
            payload: Payload::deserialize_len(de, bytes::LengthField::U32)?,
        })
    }
}

/// A helper for constructing a [`Message`] step-by-step.
///
/// # Examples
///
/// Basic usage:
///
/// ```ignore
/// let message = Message::builder()
///     .service_id(0x1234)
///     .method_id(0x5678)
///     .build();
/// ```
#[derive(Debug, Default)]
pub struct MessageBuilder {
    msg: Message,
}

impl MessageBuilder {
    /// Creates a new [`MessageBuilder`] with a default message.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Consumes the builder to return the constructed message.
    #[must_use]
    pub fn build(self) -> Message {
        self.msg
    }

    /// Sets the service id.
    #[must_use]
    pub fn service_id(mut self, value: u16) -> Self {
        self.msg.service_id = value;
        self
    }

    /// Sets the method id.
    #[must_use]
    pub fn method_id(mut self, value: u16) -> Self {
        self.msg.method_id = value;
        self
    }

    /// Sets the payload.
    #[must_use]
    pub fn payload(mut self, value: Payload) -> Self {
        self.msg.payload = value;
        self
    }
}
