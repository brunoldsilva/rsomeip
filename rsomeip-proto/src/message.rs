//! SOME/IP message.
//!
//! This module provides the definition for the SOME/IP message.

use crate::{
    ClientId, InterfaceVersion, MessageId, MessageType, MessageTypeField, MethodId,
    ProtocolVersion, RequestId, ReturnCode, ServiceId, SessionId,
};
use rsomeip_bytes::{
    Buf, BufMut, Deserialize, DeserializeError, Serialize, SerializeError, deserialize_from,
    serialize_into, size_hint,
};

/// Message addressed to a service instance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message<H, B> {
    /// Identifier of the service interface.
    pub service: ServiceId,
    /// Identifier of the method or event.
    pub method: MethodId,
    /// Additional information about the message.
    pub header: H,
    /// Contents of the message.
    pub body: B,
}

// Constructor and accessors.
impl<H, B> Message<H, B> {
    /// Creates a new [`Message<H, B>`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rsomeip_proto::Message;
    ///
    /// let message = Message::new("header", "body");
    /// assert_eq!(message.header, "header");
    /// assert_eq!(message.body, "body");
    /// ```
    #[inline]
    #[must_use]
    pub const fn new(header: H, body: B) -> Self {
        Self {
            service: ServiceId::new(0),
            method: MethodId::new(0),
            header,
            body,
        }
    }

    /// Returns the [`MessageId`] portion of this message.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rsomeip_proto::{Message, ServiceId, MethodId};
    ///
    /// let message = Message::new("header", "body")
    ///     .with_service(ServiceId::new(0x1234))
    ///     .with_method(MethodId::new(0x5678));
    /// assert_eq!(message.id().as_u32(), 0x1234_5678);
    /// ```
    #[inline]
    #[must_use]
    pub const fn id(&self) -> MessageId {
        MessageId::new(self.service, self.method)
    }

    /// Returns `self` with the given [`MessageId`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rsomeip_proto::{Message, MessageId};
    ///
    /// let message = Message::new("header", "body").with_id(MessageId::from(0x1234_5678));
    /// assert_eq!(message.service.as_u16(), 0x1234);
    /// assert_eq!(message.method.as_u16(), 0x5678);
    /// ```
    #[inline]
    #[must_use]
    pub const fn with_id(mut self, id: MessageId) -> Self {
        self.service = id.service;
        self.method = id.method;
        self
    }

    /// Returns `self` with the given [`ServiceId`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rsomeip_proto::{Message, ServiceId};
    ///
    /// let message = Message::new("header", "body").with_service(ServiceId::new(0x1234));
    /// assert_eq!(message.service.as_u16(), 0x1234);
    /// ```
    #[inline]
    #[must_use]
    pub const fn with_service(mut self, service: ServiceId) -> Self {
        self.service = service;
        self
    }

    /// Returns `self` with the given [`MethodId`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rsomeip_proto::{Message, MethodId};
    ///
    /// let message = Message::new("header", "body").with_method(MethodId::new(0x1234));
    /// assert_eq!(message.method.as_u16(), 0x1234);
    /// ```
    #[inline]
    #[must_use]
    pub const fn with_method(mut self, method: MethodId) -> Self {
        self.method = method;
        self
    }

    /// Maps a [`Message<H, B>`] to a [`Message<T, B>`] by replacing the header of the message with
    /// the given value.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rsomeip_proto::Message;
    ///
    /// let message = Message::new("header", "body").with_header(0x1234_u16);
    /// assert_eq!(message.header, 0x1234_u16);
    /// ```
    #[inline]
    #[must_use]
    pub fn with_header<T>(self, header: T) -> Message<T, B> {
        Message::new(header, self.body)
            .with_service(self.service)
            .with_method(self.method)
    }

    /// Maps a [`Message<H, B>`] to a [`Message<H, T>`] by replacing the body of the message with the
    /// given value.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rsomeip_proto::Message;
    ///
    /// let message = Message::new("header", "body").with_body(0x1234_u16);
    /// assert_eq!(message.body, 0x1234_u16);
    /// ```
    #[inline]
    #[must_use]
    pub fn with_body<T>(self, body: T) -> Message<H, T> {
        Message::new(self.header, body)
            .with_service(self.service)
            .with_method(self.method)
    }

    /// Maps a [`Message<H, B>`] to a [`Message<T, U>`] by applying a function to the header and body
    /// of the message.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rsomeip_proto::Message;
    ///
    /// let message = Message::new(1u8, 2u8)
    ///     .map(|header, body| (header + 1, body + 1));
    /// assert_eq!(message.header, 2);
    /// assert_eq!(message.body, 3);
    /// ```
    #[inline]
    #[must_use]
    pub fn map<T, U, F>(self, f: F) -> Message<T, U>
    where
        F: FnOnce(H, B) -> (T, U),
    {
        let (header, body) = f(self.header, self.body);
        Message::new(header, body)
            .with_service(self.service)
            .with_method(self.method)
    }

    /// Maps a [`Message<H, B>`] to a [`Message<T, B>`] by applying a function to the header of the
    /// message.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rsomeip_proto::Message;
    ///
    /// let message = Message::new(1u8, "body").map_header(|header| header + 1);
    /// assert_eq!(message.header, 2);
    /// ```
    #[inline]
    #[must_use]
    pub fn map_header<T, F>(self, f: F) -> Message<T, B>
    where
        F: FnOnce(H) -> T,
    {
        Message::new(f(self.header), self.body)
            .with_service(self.service)
            .with_method(self.method)
    }

    /// Maps a [`Message<H, B>`] to a [`Message<H, T>`] by applying a function to the body of the
    /// message.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rsomeip_proto::Message;
    ///
    /// let message = Message::new("header", 2u8).map_body(|body| body + 1);
    /// assert_eq!(message.body, 3);
    /// ```
    #[inline]
    #[must_use]
    pub fn map_body<T, F>(self, f: F) -> Message<H, T>
    where
        F: FnOnce(B) -> T,
    {
        Message::new(self.header, f(self.body))
            .with_service(self.service)
            .with_method(self.method)
    }

    /// Replaces the header of the message with the given `value`.
    ///
    /// Returns the old value.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rsomeip_proto::Message;
    ///
    /// let mut message = Message::new("header", "body");
    /// assert_eq!(message.replace_header("new_header"), "header");
    /// assert_eq!(message.header, "new_header");
    /// ```
    #[inline]
    #[must_use]
    pub const fn replace_header(&mut self, value: H) -> H {
        std::mem::replace(&mut self.header, value)
    }

    /// Replaces the body of the message with the given `value`.
    ///
    /// Returns the old value.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rsomeip_proto::Message;
    ///
    /// let mut message = Message::new("header", "body");
    /// assert_eq!(message.replace_body("new_body"), "body");
    /// assert_eq!(message.body, "new_body");
    /// ```
    #[inline]
    #[must_use]
    pub const fn replace_body(&mut self, value: B) -> B {
        std::mem::replace(&mut self.body, value)
    }
}

/// Specialization for the [`Header`] type.
impl<B> Message<Header, B> {
    /// Returns `self` as a response message with the given `return_code`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rsomeip_proto::{Message, Header, MessageType, ReturnCode};
    ///
    /// let message = Message::new(Header::new(), "body")
    ///     .into_response(ReturnCode::NotOk);
    /// assert_eq!(message.header.message_type.as_type(), MessageType::Response);
    /// assert_eq!(message.header.return_code, ReturnCode::NotOk);
    /// assert_eq!(message.body, "body");
    /// ```
    #[inline]
    #[must_use]
    pub fn into_response(self, return_code: ReturnCode) -> Self {
        self.map_header(|header| {
            header
                .with_message_type(MessageType::Response)
                .with_return_code(return_code)
        })
    }

    /// Returns `self` as an error message with the given `return_code`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rsomeip_proto::{Message, Header, MessageType, ReturnCode};
    ///
    /// let message = Message::new(Header::new(), "body")
    ///     .into_error(ReturnCode::NotOk);
    /// assert_eq!(message.header.message_type.as_type(), MessageType::Error);
    /// assert_eq!(message.header.return_code, ReturnCode::NotOk);
    /// assert_eq!(message.body, "body");
    /// ```
    #[inline]
    #[must_use]
    pub fn into_error(self, return_code: ReturnCode) -> Self {
        self.map_header(|header| {
            header
                .with_message_type(MessageType::Error)
                .with_return_code(return_code)
        })
    }

    /// Returns the [`ClientId`] portion of the [`Header`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rsomeip_proto::{Message, Header, ClientId};
    ///
    /// let message = Message::new(Header::new(), "body")
    ///     .with_client(ClientId::new(0x1234));
    /// assert_eq!(message.client().as_u16(), 0x1234);
    /// ```
    #[inline]
    #[must_use]
    pub const fn client(&self) -> ClientId {
        self.header.client
    }

    /// Returns `self` with the given [`ClientId`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rsomeip_proto::{Message, Header, ClientId};
    ///
    /// let message = Message::new(Header::new(), "body")
    ///     .with_client(ClientId::new(0x1234));
    /// assert_eq!(message.client().as_u16(), 0x1234);
    /// ```
    #[inline]
    #[must_use]
    pub const fn with_client(mut self, client: ClientId) -> Self {
        self.header.client = client;
        self
    }

    /// Returns the [`SessionId`] portion of the [`Header`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rsomeip_proto::{Message, Header, SessionId};
    ///
    /// let message = Message::new(Header::new(), "body")
    ///     .with_session(SessionId::new(0x1234));
    /// assert_eq!(message.session().as_u16(), 0x1234);
    /// ```
    #[inline]
    #[must_use]
    pub const fn session(&self) -> SessionId {
        self.header.session
    }

    /// Returns `self` with the given [`SessionId`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rsomeip_proto::{Message, Header, SessionId};
    ///
    /// let message = Message::new(Header::new(), "body")
    ///     .with_session(SessionId::new(0x1234));
    /// assert_eq!(message.session().as_u16(), 0x1234);
    /// ```
    #[inline]
    #[must_use]
    pub const fn with_session(mut self, session: SessionId) -> Self {
        self.header.session = session;
        self
    }

    /// Returns the [`ProtocolVersion`] portion of the [`Header`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rsomeip_proto::{Message, Header, ProtocolVersion};
    ///
    /// let message = Message::new(Header::new(), "body")
    ///     .with_protocol(ProtocolVersion::new(0x12));
    /// assert_eq!(message.protocol().as_u8(), 0x12);
    /// ```
    #[inline]
    #[must_use]
    pub const fn protocol(&self) -> ProtocolVersion {
        self.header.protocol
    }

    /// Returns `self` with the given [`ProtocolVersion`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rsomeip_proto::{Message, Header, ProtocolVersion};
    ///
    /// let message = Message::new(Header::new(), "body")
    ///     .with_protocol(ProtocolVersion::new(0x12));
    /// assert_eq!(message.protocol().as_u8(), 0x12);
    /// ```
    #[inline]
    #[must_use]
    pub const fn with_protocol(mut self, protocol: ProtocolVersion) -> Self {
        self.header.protocol = protocol;
        self
    }

    /// Returns the [`InterfaceVersion`] portion of the [`Header`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rsomeip_proto::{Message, Header, InterfaceVersion};
    ///
    /// let message = Message::new(Header::new(), "body")
    ///     .with_interface(InterfaceVersion::new(0x12));
    /// assert_eq!(message.interface().as_u8(), 0x12);
    /// ```
    #[inline]
    #[must_use]
    pub const fn interface(&self) -> InterfaceVersion {
        self.header.interface
    }

    /// Returns `self` with the given [`InterfaceVersion`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rsomeip_proto::{Message, Header, InterfaceVersion};
    ///
    /// let message = Message::new(Header::new(), "body")
    ///     .with_interface(InterfaceVersion::new(0x12));
    /// assert_eq!(message.interface().as_u8(), 0x12);
    /// ```
    #[inline]
    #[must_use]
    pub const fn with_interface(mut self, interface: InterfaceVersion) -> Self {
        self.header.interface = interface;
        self
    }

    /// Returns the [`MessageType`] of this [`Message<Header, B>`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rsomeip_proto::{Message, Header, MessageType};
    ///
    /// let message = Message::new(Header::new(), "body")
    ///     .with_message_type(MessageType::Notification);
    /// assert_eq!(message.message_type(), MessageType::Notification);
    /// ```
    #[inline]
    #[must_use]
    pub const fn message_type(&self) -> MessageType {
        self.header.message_type.as_type()
    }

    /// Returns `self` with the given [`MessageType`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rsomeip_proto::{Message, Header, MessageType};
    ///
    /// let message = Message::new(Header::new(), "body")
    ///     .with_message_type(MessageType::Notification);
    /// assert_eq!(message.message_type(), MessageType::Notification);
    /// ```
    #[inline]
    #[must_use]
    pub const fn with_message_type(mut self, message_type: MessageType) -> Self {
        self.header.message_type = MessageTypeField::from_type(message_type);
        self
    }

    /// Returns the [`ReturnCode`] portion of the [`Header`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rsomeip_proto::{Message, Header, ReturnCode};
    ///
    /// let message = Message::new(Header::new(), "body")
    ///     .with_return_code(ReturnCode::E2e);
    /// assert_eq!(message.return_code(), ReturnCode::E2e);
    /// ```
    #[inline]
    #[must_use]
    pub const fn return_code(&self) -> ReturnCode {
        self.header.return_code
    }

    /// Returns `self` with the given [`ReturnCode`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rsomeip_proto::{Message, Header, ReturnCode};
    ///
    /// let message = Message::new(Header::new(), "body")
    ///     .with_return_code(ReturnCode::E2e);
    /// assert_eq!(message.return_code(), ReturnCode::E2e);
    /// ```
    #[inline]
    #[must_use]
    pub const fn with_return_code(mut self, return_code: ReturnCode) -> Self {
        self.header.return_code = return_code;
        self
    }
}

impl<H, B> Serialize for Message<H, B>
where
    H: Serialize,
    B: Serialize,
{
    fn serialize(&self, buffer: &mut impl BufMut) -> Result<usize, SerializeError> {
        let mut size = 0;
        size += serialize_into!(buffer, self.service, self.method)?;
        size += serialize_into!(buffer, length = U32, &self.header, &self.body)?;
        Ok(size)
    }

    fn size_hint(&self) -> usize {
        let mut size = 0;
        size += size_hint!(self.service, self.method);
        size += size_hint!(length = U32, &self.header, &self.body);
        size
    }
}

impl<H, B> Deserialize for Message<H, B>
where
    H: Deserialize<Output = H>,
    B: Deserialize<Output = B>,
{
    type Output = Self;

    fn deserialize(buffer: &mut impl Buf) -> Result<Self::Output, DeserializeError> {
        let (service, method) = deserialize_from!(buffer, ServiceId, MethodId)?;
        let (header, body) = deserialize_from!(buffer, length = U32, H, B)?;
        Ok(Self {
            service,
            method,
            header,
            body,
        })
    }
}

#[cfg(test)]
impl<H, B> Message<H, B>
where
    H: Serialize + Deserialize<Output = H>,
    B: Serialize + Deserialize<Output = B>,
{
    /// Returns a [`Bytes`] buffer containing the serialized message.
    ///
    /// # Panics
    ///
    /// Panics if the serialization fails.
    #[must_use]
    pub fn to_bytes(&self) -> rsomeip_bytes::Bytes {
        let mut buffer = rsomeip_bytes::BytesMut::new();
        self.serialize(&mut buffer)
            .expect("should serialize the message");
        buffer.freeze()
    }

    /// Returns a [`Message<H, B>`] deserialized from the `buffer`.
    ///
    /// # Panics
    ///
    /// Panics if the deserialization fails.
    #[must_use]
    pub fn from_bytes(buffer: &mut impl Buf) -> Self {
        Self::deserialize(buffer).expect("should deserialize the message")
    }
}

impl<H, B> Default for Message<H, B>
where
    H: Default,
    B: Default,
{
    fn default() -> Self {
        Self {
            service: ServiceId::default(),
            method: MethodId::default(),
            header: H::default(),
            body: B::default(),
        }
    }
}

impl<H, B> std::fmt::Display for Message<H, B>
where
    H: std::fmt::Display,
{
    /// Formats `self` into a string.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rsomeip_proto::{Message, ServiceId, MethodId};
    ///
    /// let message = Message::new("header", "body")
    ///     .with_service(ServiceId::new(0x1234))
    ///     .with_method(MethodId::new(0x5678));
    /// assert_eq!(message.to_string(), "1234.5678.header")
    /// ```
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.service, self.method, self.header)
    }
}

/// A message processing error.
///
/// A SOME/IP endpoint may occasionally try to send or receive messages which are incorrectly
/// configured for the service interface that processes them.
///
/// This enum serves as a way to represent the reason for a message to not be processed, as well as
/// whether the error should be reported to the user or not.
///
/// # Error Handling
///
/// Depending on the error variant, either an error response should be sent back to the source or the
/// whole message should be dropped.
///
/// If the error is [`MessageError::Invalid`], then an error response should be sent with the
/// contained [`ReturnCode`]. Otherwise, the message should be dropped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum MessageError {
    #[error("invalid message: {0}")]
    Invalid(ReturnCode),
    #[error("message dropped: {0}")]
    Dropped(ReturnCode),
}

/// Header of a SOME/IP message.
///
/// Contains additional information about the message itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Header {
    /// Client that sent the message.
    pub client: ClientId,
    /// Order of the message in a sequence.
    pub session: SessionId,
    /// Major version of the SOME/IP protocol.
    pub protocol: ProtocolVersion,
    /// Major version of the service interface.
    pub interface: InterfaceVersion,
    /// Message type and associated flags.
    pub message_type: MessageTypeField,
    /// Result of processing the message.
    pub return_code: ReturnCode,
}

impl Header {
    /// Creates a new [`Header`].
    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self {
            client: ClientId::new(0),
            session: SessionId::DISABLED,
            protocol: ProtocolVersion::new(1),
            interface: InterfaceVersion::new(0),
            message_type: MessageTypeField::new(0),
            return_code: ReturnCode::Ok,
        }
    }

    /// Returns the [`RequestId`] portion of `self`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rsomeip_proto::{Header, ClientId, SessionId};
    ///
    /// let header = Header::new()
    ///     .with_client(ClientId::new(0x1234))
    ///     .with_session(SessionId::new(0x5678));
    /// assert_eq!(header.id().as_u32(), 0x1234_5678);
    /// ```
    #[inline]
    #[must_use]
    pub const fn id(&self) -> RequestId {
        RequestId::new(self.client, self.session)
    }

    /// Returns `self` with the given [`RequestId`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rsomeip_proto::{Header, RequestId};
    ///
    /// let header = Header::new().with_id(RequestId::from(0x1234_5678));
    /// assert_eq!(header.client.as_u16(), 0x1234);
    /// assert_eq!(header.session.as_u16(), 0x5678);
    /// ```
    #[inline]
    #[must_use]
    pub const fn with_id(mut self, id: RequestId) -> Self {
        self.client = id.client;
        self.session = id.session;
        self
    }

    /// Returns `self` with the given [`ClientId`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rsomeip_proto::{Header, ClientId};
    ///
    /// let header = Header::new().with_client(ClientId::new(0x1234));
    /// assert_eq!(header.client.as_u16(), 0x1234);
    /// ```
    #[inline]
    #[must_use]
    pub const fn with_client(mut self, client: ClientId) -> Self {
        self.client = client;
        self
    }

    /// Returns `self` with the given [`SessionId`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rsomeip_proto::{Header, SessionId};
    ///
    /// let header = Header::new().with_session(SessionId::new(0x1234));
    /// assert_eq!(header.session.as_u16(), 0x1234);
    /// ```
    #[inline]
    #[must_use]
    pub const fn with_session(mut self, session: SessionId) -> Self {
        self.session = session;
        self
    }

    /// Returns `self` with the given [`ProtocolVersion`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rsomeip_proto::{Header, ProtocolVersion};
    ///
    /// let header = Header::new().with_protocol(ProtocolVersion::new(0x12));
    /// assert_eq!(header.protocol.as_u8(), 0x12);
    /// ```
    #[inline]
    #[must_use]
    pub const fn with_protocol(mut self, protocol: ProtocolVersion) -> Self {
        self.protocol = protocol;
        self
    }

    /// Returns `self` with the given [`InterfaceVersion`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rsomeip_proto::{Header, InterfaceVersion};
    ///
    /// let header = Header::new().with_interface(InterfaceVersion::new(0x12));
    /// assert_eq!(header.interface.as_u8(), 0x12);
    /// ```
    #[inline]
    #[must_use]
    pub const fn with_interface(mut self, interface: InterfaceVersion) -> Self {
        self.interface = interface;
        self
    }

    /// Returns `self` with the given [`MessageType`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rsomeip_proto::{Header, MessageType};
    ///
    /// let header = Header::new().with_message_type(MessageType::Notification);
    /// assert_eq!(header.message_type.as_type(), MessageType::Notification);
    /// ```
    #[inline]
    #[must_use]
    pub const fn with_message_type(mut self, message_type: MessageType) -> Self {
        self.message_type = MessageTypeField::from_type(message_type);
        self
    }

    /// Returns `self` with the given [`ReturnCode`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rsomeip_proto::{Header, ReturnCode};
    ///
    /// let header = Header::new().with_return_code(ReturnCode::NotOk);
    /// assert_eq!(header.return_code, ReturnCode::NotOk);
    /// ```
    #[inline]
    #[must_use]
    pub const fn with_return_code(mut self, return_code: ReturnCode) -> Self {
        self.return_code = return_code;
        self
    }
}

impl Default for Header {
    fn default() -> Self {
        Self::new()
    }
}

impl Serialize for Header {
    fn serialize(&self, buffer: &mut impl BufMut) -> Result<usize, SerializeError> {
        serialize_into!(
            buffer,
            self.client,
            self.session,
            self.protocol,
            self.interface,
            self.message_type,
            self.return_code
        )
    }

    fn size_hint(&self) -> usize {
        size_hint!(
            self.client,
            self.session,
            self.protocol,
            self.interface,
            self.message_type,
            self.return_code
        )
    }
}

impl Deserialize for Header {
    type Output = Self;

    fn deserialize(buffer: &mut impl Buf) -> Result<Self::Output, DeserializeError> {
        Ok(Self {
            client: ClientId::deserialize(buffer)?,
            session: SessionId::deserialize(buffer)?,
            protocol: ProtocolVersion::deserialize(buffer)?,
            interface: InterfaceVersion::deserialize(buffer)?,
            message_type: MessageTypeField::deserialize(buffer)?,
            return_code: ReturnCode::deserialize(buffer)?,
        })
    }
}

impl std::fmt::Display for Header {
    /// Formats `self` using the given formatter.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rsomeip_proto::{Header, ClientId, SessionId, MessageType, ReturnCode};
    ///
    /// let header = Header::default()
    ///     .with_client(ClientId::new(0x1234))
    ///     .with_session(SessionId::new(0x5678))
    ///     .with_message_type(MessageType::Notification)
    ///     .with_return_code(ReturnCode::Ok);
    /// assert_eq!(header.to_string(), "1234.5678.02.00")
    /// ```
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}.{}.{:02x?}.{:02x?}",
            self.client,
            self.session,
            u8::from(self.message_type),
            u8::from(self.return_code)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rsomeip_bytes::{Bytes, BytesMut};

    const DESERIALIZED_MESSAGE: Message<u8, u8> = Message::new(1u8, 2u8)
        .with_service(ServiceId::new(0x1234_u16))
        .with_method(MethodId::new(0x5678_u16));

    const SERIALIZED_MESSAGE: [u8; 10] = [
        0x12, 0x34, // ServiceId
        0x56, 0x78, // MethodId
        0x00, 0x00, 0x00, 0x02, // Length
        0x01, // Header
        0x02, // Body
    ];

    #[test]
    fn message_is_serializable() {
        let mut buffer = BytesMut::new();
        assert_eq!(DESERIALIZED_MESSAGE.size_hint(), SERIALIZED_MESSAGE.len());
        assert_eq!(
            DESERIALIZED_MESSAGE.serialize(&mut buffer),
            Ok(SERIALIZED_MESSAGE.len())
        );
        assert_eq!(buffer[..], SERIALIZED_MESSAGE[..]);
    }

    #[test]
    fn message_is_deserializable() {
        let mut buffer = Bytes::copy_from_slice(&SERIALIZED_MESSAGE[..]);
        let message =
            Message::<u8, u8>::deserialize(&mut buffer).expect("should deserialize the message");
        assert_eq!(message, DESERIALIZED_MESSAGE);
    }

    const DESERIALIZED_HEADER: Header = Header::new()
        .with_client(ClientId::new(0x1234))
        .with_session(SessionId::new(0x5678))
        .with_protocol(ProtocolVersion::new(0x9a))
        .with_interface(InterfaceVersion::new(0xbc))
        .with_message_type(MessageType::Error)
        .with_return_code(ReturnCode::E2e);

    const SERIALIZED_HEADER: [u8; 8] = [
        0x12, 0x34, // ClientId
        0x56, 0x78, // SessionId
        0x9a, // ProtocolVersion
        0xbc, // InterfaceVersion
        0x81, // MessageType
        0x0d, // ReturnCode
    ];

    #[test]
    fn header_is_serializable() {
        let mut buffer = BytesMut::new();
        assert_eq!(DESERIALIZED_HEADER.size_hint(), SERIALIZED_HEADER.len());
        assert_eq!(
            DESERIALIZED_HEADER.serialize(&mut buffer),
            Ok(SERIALIZED_HEADER.len())
        );
        assert_eq!(buffer[..], SERIALIZED_HEADER[..]);
    }

    #[test]
    fn header_is_deserializable() {
        let mut buffer = Bytes::copy_from_slice(&SERIALIZED_HEADER[..]);
        let header = Header::deserialize(&mut buffer).expect("should deserialize the HEADER");
        assert_eq!(header, DESERIALIZED_HEADER);
    }
}
