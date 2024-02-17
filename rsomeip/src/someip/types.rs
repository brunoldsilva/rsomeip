//! Common types of the SOME/IP protocol.

use crate::bytes::{self};

/// A payload addressed to a method of a service.
#[derive(Debug, PartialEq, Eq)]
pub struct Message<T> {
    service: ServiceId,
    method: MethodId,
    payload: T,
}

impl<T> Message<T> {
    /// Creates a new [`Message`].
    #[must_use]
    pub const fn new(service: ServiceId, method: MethodId, payload: T) -> Self {
        Self {
            service,
            method,
            payload,
        }
    }

    /// Returns the [`MessageId`] of this [`Message`].
    pub fn id(&self) -> MessageId {
        #![allow(clippy::cast_lossless)] // Lossy conversion is fine.
        ((self.service() as MessageId) << 16) | (self.method() as MessageId)
    }

    /// Sets the [`MessageId`] of this [`Message`].
    pub fn set_id(&mut self, id: MessageId) {
        #![allow(clippy::cast_possible_truncation)] // Truncation is intended.
        self.set_service((id >> 16) as ServiceId);
        self.set_method((id & 0xffff) as MethodId);
    }

    /// Returns the [`ServiceId`] of this [`Message`].
    pub fn service(&self) -> ServiceId {
        self.service
    }

    /// Sets the [`ServiceId`] of this [`Message`].
    pub fn set_service(&mut self, service: ServiceId) {
        self.service = service;
    }

    /// Returns the [`MethodId`] of this [`Message`].
    pub fn method(&self) -> MethodId {
        self.method
    }

    /// Sets the [`MethodId`] of this [`Message`].
    pub fn set_method(&mut self, method: MethodId) {
        self.method = method;
    }

    /// Returns a reference to the payload of this [`Message`].
    pub fn payload(&self) -> &T {
        &self.payload
    }

    /// Sets the payload of this [`Message`].
    pub fn set_payload(&mut self, payload: T) {
        self.payload = payload;
    }
}

impl<T> bytes::Serialize for Message<T>
where
    T: bytes::Serialize,
{
    fn serialize(&self, ser: &mut bytes::Serializer) -> bytes::Result<()> {
        self.id().serialize(ser)?;
        self.payload().serialize_len(ser, bytes::LengthField::U32)
    }
}

impl<T> bytes::Deserialize for Message<T>
where
    T: bytes::Deserialize,
{
    fn deserialize(de: &mut bytes::Deserializer) -> bytes::Result<Self> {
        Ok(Self {
            service: ClientId::deserialize(de)?,
            method: MethodId::deserialize(de)?,
            payload: T::deserialize_len(de, bytes::LengthField::U32)?,
        })
    }
}

impl<T> Default for Message<T>
where
    T: Default,
{
    fn default() -> Self {
        Self {
            service: Default::default(),
            method: Default::default(),
            payload: Default::default(),
        }
    }
}

impl<T> std::fmt::Display for Message<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[M.{:04x?}.{:04x?}]", self.service(), self.method())
    }
}

impl<T> Clone for Message<T>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        Self {
            service: self.service,
            method: self.method,
            payload: self.payload.clone(),
        }
    }
}

/// Additional payload metadata for a SOME/IP [`Message`].
#[derive(Debug, PartialEq, Eq)]
pub struct Request<T> {
    client: ClientId,
    session: SessionId,
    protocol: ProtocolVersion,
    interface: InterfaceVersion,
    message_type: MessageType,
    return_code: ReturnCode,
    payload: T,
}

impl<T> Request<T> {
    /// Creates a new [`Request`].
    #[must_use]
    pub fn new(payload: T) -> Self {
        Self {
            client: 0,
            session: 0,
            protocol: 1,
            interface: 0,
            message_type: MessageType::Request,
            return_code: ReturnCode::Ok,
            payload,
        }
    }

    /// Returns a new [`RequestBuilder`].
    #[must_use]
    pub fn build(payload: T) -> RequestBuilder<T> {
        RequestBuilder::new(payload)
    }

    /// Returns the [`RequestId`] of this [`Request`].
    pub fn id(&self) -> RequestId {
        #![allow(clippy::cast_lossless)] // Lossy conversion is fine.
        ((self.client() as RequestId) << 16) | (self.session() as RequestId)
    }

    /// Sets the [`RequestId`] of this [`Request`].
    pub fn set_id(&mut self, request: RequestId) {
        #![allow(clippy::cast_possible_truncation)] // Truncation is intended.
        self.set_client((request >> 16) as ClientId);
        self.set_session((request & 0xffff) as SessionId);
    }

    /// Returns the [`ClientId`] of this [`Request`].
    pub fn client(&self) -> ClientId {
        self.client
    }

    /// Sets the [`ClientId`] of this [`Request`].
    pub fn set_client(&mut self, client: ClientId) {
        self.client = client;
    }

    /// Returns the [`SessionId`] of this [`Request`].
    pub fn session(&self) -> SessionId {
        self.session
    }

    /// Sets the [`SessionId`] of this [`Request`].
    pub fn set_session(&mut self, session: SessionId) {
        self.session = session;
    }

    /// Returns the [`ProtocolVersion`] of this [`Request`].
    pub fn protocol(&self) -> ProtocolVersion {
        self.protocol
    }

    /// Sets the [`ProtocolVersion`] of this [`Request`].
    pub fn set_protocol(&mut self, protocol: ProtocolVersion) {
        self.protocol = protocol;
    }

    /// Returns the [`InterfaceVersion`] of this [`Request`].
    pub fn interface(&self) -> InterfaceVersion {
        self.interface
    }

    /// Sets the [`InterfaceVersion`] of this [`Request`].
    pub fn set_interface(&mut self, interface: InterfaceVersion) {
        self.interface = interface;
    }

    /// Returns the [`MessageType`] of this [`Request`].
    pub fn message_type(&self) -> MessageType {
        self.message_type
    }

    /// Sets the [`MessageType`] of this [`Request`].
    pub fn set_message_type(&mut self, message_type: MessageType) {
        self.message_type = message_type;
    }

    /// Returns the [`ReturnCode`] of this [`Request`].
    pub fn return_code(&self) -> ReturnCode {
        self.return_code
    }

    /// Sets the [`ReturnCode`] of this [`Request`].
    pub fn set_return_code(&mut self, return_code: ReturnCode) {
        self.return_code = return_code;
    }

    /// Returns a reference to the payload of this [`Request`].
    pub fn payload(&self) -> &T {
        &self.payload
    }

    /// Sets the payload of this [`Request`].
    pub fn set_payload(&mut self, payload: T) {
        self.payload = payload;
    }
}

impl<T> bytes::Serialize for Request<T>
where
    T: bytes::Serialize,
{
    fn serialize(&self, ser: &mut bytes::Serializer) -> bytes::Result<()> {
        self.id().serialize(ser)?;
        self.protocol().serialize(ser)?;
        self.interface().serialize(ser)?;
        self.message_type().serialize(ser)?;
        self.return_code().serialize(ser)?;
        self.payload().serialize(ser)
    }
}

impl<T> bytes::Deserialize for Request<T>
where
    T: bytes::Deserialize,
{
    fn deserialize(de: &mut bytes::Deserializer) -> bytes::Result<Self> {
        Ok(Self {
            client: ClientId::deserialize(de)?,
            session: SessionId::deserialize(de)?,
            protocol: ProtocolVersion::deserialize(de)?,
            interface: InterfaceVersion::deserialize(de)?,
            message_type: MessageType::deserialize(de)?,
            return_code: ReturnCode::deserialize(de)?,
            payload: T::deserialize(de)?,
        })
    }
}

impl<T> Default for Request<T>
where
    T: Default,
{
    fn default() -> Self {
        Self::new(T::default())
    }
}

impl<T> std::fmt::Display for Request<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[R.{:04x?}.{:04x?}.{:02x?}.{:02x?}]",
            self.client(),
            self.session(),
            u8::from(self.message_type()),
            u8::from(self.return_code())
        )
    }
}

impl<T> Clone for Request<T>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        Self {
            payload: self.payload.clone(),
            ..(*self)
        }
    }
}

/// Utility for build [`Request`]s using the builder pattern.
#[derive(Debug)]
pub struct RequestBuilder<T> {
    inner: Request<T>,
}

impl<T> RequestBuilder<T> {
    /// Creates a new [`RequestBuilder`].
    pub fn new(payload: T) -> Self {
        Self {
            inner: Request::new(payload),
        }
    }

    /// Returns the [`Request`] that is being built, consuming the [`RequestBuilder`].
    pub fn get(self) -> Request<T> {
        self.inner
    }

    /// Sets the [`RequestId`] of the [`Request`].
    #[must_use]
    pub fn id(mut self, value: RequestId) -> Self {
        self.inner.set_id(value);
        self
    }

    /// Sets the [`ClientId`] of the [`Request`].
    #[must_use]
    pub fn client(mut self, value: ClientId) -> Self {
        self.inner.set_client(value);
        self
    }

    /// Sets the [`SessionId`] of the [`Request`].
    #[must_use]
    pub fn session(mut self, value: SessionId) -> Self {
        self.inner.set_session(value);
        self
    }

    /// Sets the [`ProtocolVersion`] of the [`Request`].
    #[must_use]
    pub fn protocol(mut self, value: ProtocolVersion) -> Self {
        self.inner.set_protocol(value);
        self
    }

    /// Sets the [`InterfaceVersion`] of the [`Request`].
    #[must_use]
    pub fn interface(mut self, value: InterfaceVersion) -> Self {
        self.inner.set_interface(value);
        self
    }

    /// Sets the [`MessageType`] of the [`Request`].
    #[must_use]
    pub fn message_type(mut self, value: MessageType) -> Self {
        self.inner.set_message_type(value);
        self
    }

    /// Sets the [`ReturnCode`] of the [`Request`].
    #[must_use]
    pub fn return_code(mut self, value: ReturnCode) -> Self {
        self.inner.set_return_code(value);
        self
    }

    /// Sets the payload of the [`Request`].
    #[must_use]
    pub fn payload(mut self, value: T) -> Self {
        self.inner.set_payload(value);
        self
    }
}

/// Identifies the type of message.
#[non_exhaustive]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum MessageType {
    #[default]
    Request,
    RequestNoReturn,
    Notification,
    Response,
    Error,
    TpRequest,
    TpRequestNoReturn,
    TpNotification,
    TpResponse,
    TpError,
    Unknown(u8),
}

impl bytes::Serialize for MessageType {
    fn serialize(&self, ser: &mut bytes::Serializer) -> bytes::Result<()> {
        u8::from(*self).serialize(ser)
    }
}

impl bytes::Deserialize for MessageType {
    fn deserialize(de: &mut bytes::Deserializer) -> bytes::Result<Self> {
        Ok(Self::from(u8::deserialize(de)?))
    }
}

impl From<u8> for MessageType {
    fn from(value: u8) -> Self {
        match value {
            0x00 => Self::Request,
            0x01 => Self::RequestNoReturn,
            0x02 => Self::Notification,
            0x80 => Self::Response,
            0x81 => Self::Error,
            0x20 => Self::TpRequest,
            0x21 => Self::TpRequestNoReturn,
            0x22 => Self::TpNotification,
            0xa0 => Self::TpResponse,
            0xa1 => Self::TpError,
            x => Self::Unknown(x),
        }
    }
}

impl From<MessageType> for u8 {
    fn from(value: MessageType) -> Self {
        match value {
            MessageType::Request => 0x00,
            MessageType::RequestNoReturn => 0x01,
            MessageType::Notification => 0x02,
            MessageType::Response => 0x80,
            MessageType::Error => 0x81,
            MessageType::TpRequest => 0x20,
            MessageType::TpRequestNoReturn => 0x21,
            MessageType::TpNotification => 0x22,
            MessageType::TpResponse => 0xa0,
            MessageType::TpError => 0xa1,
            MessageType::Unknown(x) => x,
        }
    }
}

/// Identifies the result of the request.
#[non_exhaustive]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum ReturnCode {
    #[default]
    Ok,
    NotOk,
    UnknownService,
    UnknownMethod,
    NotReady,
    NotReachable,
    Timeout,
    WrongProtocolVersion,
    WrongInterfaceVersion,
    MalformedMessage,
    WrongMessageType,
    E2eRepeated,
    E2eWrongSequence,
    E2e,
    E2eNotAvailable,
    E2eNoNewData,
    Reserved(u8),
    Unknown(u8),
}

impl bytes::Serialize for ReturnCode {
    fn serialize(&self, ser: &mut bytes::Serializer) -> bytes::Result<()> {
        u8::from(*self).serialize(ser)
    }
}

impl bytes::Deserialize for ReturnCode {
    fn deserialize(de: &mut bytes::Deserializer) -> bytes::Result<Self> {
        Ok(Self::from(u8::deserialize(de)?))
    }
}

impl From<u8> for ReturnCode {
    fn from(value: u8) -> Self {
        match value {
            0x00 => Self::Ok,
            0x01 => Self::NotOk,
            0x02 => Self::UnknownService,
            0x03 => Self::UnknownMethod,
            0x04 => Self::NotReady,
            0x05 => Self::NotReachable,
            0x06 => Self::Timeout,
            0x07 => Self::WrongProtocolVersion,
            0x08 => Self::WrongInterfaceVersion,
            0x09 => Self::MalformedMessage,
            0x0a => Self::WrongMessageType,
            0x0b => Self::E2eRepeated,
            0x0c => Self::E2eWrongSequence,
            0x0d => Self::E2e,
            0x0e => Self::E2eNotAvailable,
            0x0f => Self::E2eNoNewData,
            n if (0x10..=0x5e).contains(&n) => Self::Reserved(n),
            n => Self::Unknown(n),
        }
    }
}

impl From<ReturnCode> for u8 {
    fn from(value: ReturnCode) -> Self {
        match value {
            ReturnCode::Ok => 0x00,
            ReturnCode::NotOk => 0x01,
            ReturnCode::UnknownService => 0x02,
            ReturnCode::UnknownMethod => 0x03,
            ReturnCode::NotReady => 0x04,
            ReturnCode::NotReachable => 0x05,
            ReturnCode::Timeout => 0x06,
            ReturnCode::WrongProtocolVersion => 0x07,
            ReturnCode::WrongInterfaceVersion => 0x08,
            ReturnCode::MalformedMessage => 0x09,
            ReturnCode::WrongMessageType => 0x0a,
            ReturnCode::E2eRepeated => 0x0b,
            ReturnCode::E2eWrongSequence => 0x0c,
            ReturnCode::E2e => 0x0d,
            ReturnCode::E2eNotAvailable => 0x0e,
            ReturnCode::E2eNoNewData => 0x0f,
            ReturnCode::Reserved(x) | ReturnCode::Unknown(x) => x,
        }
    }
}

/// Identifier of a method of a service.
pub type MessageId = u32;

/// Identifier of a service interface.
pub type ServiceId = u16;

/// Identifier of a method or event.
pub type MethodId = u16;

/// Identifier of a request.
pub type RequestId = u32;

/// Identifier of the client that sent the request.
pub type ClientId = u16;

/// Identifier of the request in a sequence.
pub type SessionId = u16;

/// Major version of the SOME/IP protocol being used.
pub type ProtocolVersion = u8;

/// Major version of the service referenced by the [`ServiceId`].
pub type InterfaceVersion = u8;

#[cfg(test)]
mod tests;
