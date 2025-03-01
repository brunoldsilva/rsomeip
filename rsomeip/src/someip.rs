//! SOME/IP protocol primitives.
//!
//! This module provides the building blocks for messages sent between SOME/IP endpoints on the
//! network.
//!
//! - [`Message`] encapsulates the essential components for sending data between endpoints.
//!
//! - [`MessageType`] helps specify how a message should be interpreted.
//!
//! - [`ReturnCode`] helps servers notify clients of the result of a request.

use crate::bytes::{Bytes, BytesMut, Deserialize, LengthField, Serialize};

/// Identifier of the method of a service.
pub type MessageId = u32;

/// Identifier of the service interface.
pub type ServiceId = u16;

/// Identifier of the method or event.
pub type MethodId = u16;

/// Identifier of the request.
pub type RequestId = u32;

/// Identifier of the client that sent the request.
pub type ClientId = u16;

/// Identifier of the request in a sequence.
pub type SessionId = u16;

/// Major version of the SOME/IP protocol being used.
pub type ProtocolVersion = u8;

/// Major version of the service referenced by the [`ServiceId`].
pub type InterfaceVersion = u8;

/// Identifier of single service instance on the network.
pub type InstanceId = u16;

/// Size in bytes of a SOME/IP header.
pub const HEADER_SIZE: usize = 16;

/// A SOME/IP message.
///
/// This struct encapsulates the essential components of a SOME/IP message, allowing for the
/// transmission of requests, responses, and events.
///
/// # Examples
///
/// ```rust
/// use rsomeip::someip::{Message, MessageType, ReturnCode};
/// let message = Message::new(1u8)
///     .with_id(0x1234_5678)
///     .with_request(0x9abc_def0)
///     .with_type(MessageType::Response)
///     .with_code(ReturnCode::NotOk);
/// assert_eq!(message.id(), 0x1234_5678);
/// assert_eq!(message.request(), 0x9abc_def0);
/// assert_eq!(message.message_type, MessageType::Response);
/// assert_eq!(message.return_code, ReturnCode::NotOk);
/// assert_eq!(message.payload, 1u8);
/// ```
#[derive(Debug, Clone)]
pub struct Message<T> {
    /// Identifier of the service interface.
    pub service: ServiceId,
    /// Identifier of the method or event.
    pub method: MethodId,
    /// Identifier of the client that sent the request.
    pub client: ClientId,
    /// Identifier of request in a sequence.
    pub session: SessionId,
    /// Major version of the SOME/IP protocol being used.
    pub protocol: ProtocolVersion,
    /// Major version of the service referenced by the service id.
    pub interface: InterfaceVersion,
    /// Identifies the type of message.
    pub message_type: MessageType,
    /// Identifies the result of the request.
    pub return_code: ReturnCode,
    /// Payload of the message.
    pub payload: T,
}

impl<T> Message<T> {
    /// Creates a new [`Message<T>`].
    pub fn new(payload: T) -> Self {
        Self {
            service: 0,
            method: 0,
            client: 0,
            session: 0,
            protocol: 1,
            interface: 0,
            message_type: MessageType::default(),
            return_code: ReturnCode::default(),
            payload,
        }
    }

    /// Returns a message with the given service id.
    #[must_use]
    pub fn with_service(mut self, service: ServiceId) -> Self {
        self.service = service;
        self
    }

    /// Returns a message with the given method id.
    #[must_use]
    pub fn with_method(mut self, method: MethodId) -> Self {
        self.method = method;
        self
    }

    /// Returns a message with the given message id.
    ///
    /// The message id is the combination of the service and method ids of the message.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rsomeip::someip::Message;
    /// let message = Message::new(1u8).with_id(0x1234_5678);
    /// assert_eq!(message.service, 0x1234);
    /// assert_eq!(message.method, 0x5678);
    /// ```
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub fn with_id(mut self, id: MessageId) -> Self {
        self.service = (id >> 16) as ServiceId;
        self.method = (id & 0xffff) as MethodId;
        self
    }

    /// Returns a message with the given client id.
    #[must_use]
    pub fn with_client(mut self, client: ClientId) -> Self {
        self.client = client;
        self
    }

    /// Returns a message with the given session id.
    #[must_use]
    pub fn with_session(mut self, session: SessionId) -> Self {
        self.session = session;
        self
    }

    /// Returns a message with the given request id.
    ///
    /// The request id is the combination of the client and session ids of the message.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rsomeip::someip::Message;
    /// let message = Message::new(1u8).with_request(0x1234_5678);
    /// assert_eq!(message.client, 0x1234);
    /// assert_eq!(message.session, 0x5678);
    /// ```
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub fn with_request(mut self, request: RequestId) -> Self {
        self.client = (request >> 16) as ClientId;
        self.session = (request & 0xffff) as SessionId;
        self
    }

    /// Returns a message with the given protocol version.
    #[must_use]
    pub fn with_protocol(mut self, protocol: ProtocolVersion) -> Self {
        self.protocol = protocol;
        self
    }

    /// Returns a message with the given interface version.
    #[must_use]
    pub fn with_interface(mut self, interface: InterfaceVersion) -> Self {
        self.interface = interface;
        self
    }

    /// Returns a message with the given message type.
    #[must_use]
    pub fn with_type(mut self, message_type: MessageType) -> Self {
        self.message_type = message_type;
        self
    }

    /// Returns a message with the given return code.
    #[must_use]
    pub fn with_code(mut self, return_code: ReturnCode) -> Self {
        self.return_code = return_code;
        self
    }

    /// Returns a message with the given payload.
    #[must_use]
    pub fn with_payload(mut self, payload: T) -> Self {
        self.payload = payload;
        self
    }

    /// Returns the id of the message.
    ///
    /// This id is a combination of the service and method ids of the message.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rsomeip::someip::Message;
    /// let message = Message::new(1u8)
    ///     .with_service(0x1234)
    ///     .with_method(0x5678);
    /// assert_eq!(message.id(), 0x1234_5678);
    /// ```
    #[allow(clippy::cast_lossless)]
    pub fn id(&self) -> MessageId {
        ((self.service as MessageId) << 16) | (self.method as MessageId)
    }

    /// Returns the id of the request.
    ///
    /// This id is a combination of the client and session ids of the message.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rsomeip::someip::Message;
    /// let message = Message::new(1u8)
    ///     .with_client(0x1234)
    ///     .with_session(0x5678);
    /// assert_eq!(message.request(), 0x1234_5678);
    /// ```
    #[allow(clippy::cast_lossless)]
    pub fn request(&self) -> MessageId {
        ((self.client as RequestId) << 16) | (self.session as RequestId)
    }
}

impl<T> Serialize for Message<T>
where
    T: Serialize,
{
    fn serialize(&self, buffer: &mut BytesMut) -> Result<usize, crate::bytes::SerializeError> {
        let mut size = 0;
        size += self.id().serialize(buffer)?;

        // A length field is expected between the message id and the rest of the payload, so we
        // introduce it here by encapsulating the request and serializing it with `serialize_len`.
        let request = |buffer: &mut BytesMut| {
            let mut size = 0;
            size += self.request().serialize(buffer)?;
            size += self.protocol.serialize(buffer)?;
            size += self.interface.serialize(buffer)?;
            size += self.message_type.serialize(buffer)?;
            size += self.return_code.serialize(buffer)?;
            size += self.payload.serialize(buffer)?;
            Ok(size)
        };
        size += request.serialize_len(LengthField::U32, buffer)?;

        Ok(size)
    }
}

impl<T> Deserialize for Message<T>
where
    T: Deserialize<Output = T>,
{
    type Output = Self;

    fn deserialize(buffer: &mut Bytes) -> Result<Self::Output, crate::bytes::DeserializeError> {
        type Request<T> = (
            RequestId,
            ProtocolVersion,
            InterfaceVersion,
            MessageType,
            ReturnCode,
            T,
        );
        let id = MessageId::deserialize(buffer)?;

        // A length field is expected here which we must account for when deserializing the request.
        let (request, protocol, interface, message_type, return_code, payload) =
            Request::<T>::deserialize_len(LengthField::U32, buffer)?;

        Ok(Self::new(payload)
            .with_id(id)
            .with_request(request)
            .with_protocol(protocol)
            .with_interface(interface)
            .with_type(message_type)
            .with_code(return_code))
    }
}

impl<T: Default> Default for Message<T> {
    fn default() -> Self {
        Self::new(T::default())
    }
}

impl<T> std::fmt::Display for Message<T> {
    /// Formats the [`Message<T>`] into a string.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rsomeip::someip::{Message, MessageType, ReturnCode};
    ///
    /// let message = Message::new(1u8)
    ///     .with_id(0x1234_5678)
    ///     .with_request(0x9abc_def0)
    ///     .with_type(MessageType::Response)
    ///     .with_code(ReturnCode::NotOk);
    /// assert_eq!(message.to_string(), "M.1234.5678.9abc.def0.80.01");
    /// ```
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "M.{:04x?}.{:04x?}.{:04x?}.{:04x?}.{:02x?}.{:02x?}",
            self.service,
            self.method,
            self.client,
            self.session,
            u8::from(self.message_type),
            u8::from(self.return_code)
        )
    }
}

/// Type of a SOME/IP message.
///
/// It's used both by servers and clients to specify how the message should be interpreted by the
/// peer.
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

impl Serialize for MessageType {
    fn serialize(&self, buffer: &mut BytesMut) -> Result<usize, crate::bytes::SerializeError> {
        u8::from(*self).serialize(buffer)
    }
}

impl Deserialize for MessageType {
    type Output = Self;

    fn deserialize(buffer: &mut Bytes) -> Result<Self::Output, crate::bytes::DeserializeError> {
        Ok(Self::from(u8::deserialize(buffer)?))
    }
}

/// Result of a SOME/IP request.
///
/// It's used by servers to provide meaningful information to clients about why their request
/// failed.
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

impl Serialize for ReturnCode {
    fn serialize(&self, buffer: &mut BytesMut) -> Result<usize, crate::bytes::SerializeError> {
        u8::from(*self).serialize(buffer)
    }
}

impl Deserialize for ReturnCode {
    type Output = Self;

    fn deserialize(buffer: &mut Bytes) -> Result<Self::Output, crate::bytes::DeserializeError> {
        Ok(Self::from(u8::deserialize(buffer)?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_represents_data() {
        let message = Message::new(1u8)
            .with_service(0x1234)
            .with_method(0x5678)
            .with_client(0x9abc)
            .with_session(0xdef0)
            .with_protocol(0x02)
            .with_interface(0xff)
            .with_type(MessageType::Notification)
            .with_code(ReturnCode::NotOk);
        assert_eq!(message.service, 0x1234);
        assert_eq!(message.method, 0x5678);
        assert_eq!(message.client, 0x9abc);
        assert_eq!(message.session, 0xdef0);
        assert_eq!(message.protocol, 0x02);
        assert_eq!(message.interface, 0xff);
        assert_eq!(message.message_type, MessageType::Notification);
        assert_eq!(message.return_code, ReturnCode::NotOk);
        assert_eq!(message.payload, 1u8);
    }

    // A message serialized with the SOME/IP format.
    const SERIALIZED_MESSAGE: [u8; 17] = [
        0x12, 0x34, 0x56, 0x78, // message id
        0x00, 0x00, 0x00, 0x09, // length
        0x9a, 0xbc, 0xde, 0xf0, // request id
        0x01, // protocol version
        0x00, // interface version
        0x80, // message type
        0x01, // return code
        0x01, // payload
    ];

    #[test]
    fn serialize_message() {
        let mut buffer = BytesMut::with_capacity(17);
        let message = Message::new(1u8)
            .with_id(0x1234_5678)
            .with_request(0x9abc_def0)
            .with_type(MessageType::Response)
            .with_code(ReturnCode::NotOk);
        assert_eq!(message.serialize(&mut buffer), Ok(17));
        assert_eq!(&buffer.freeze()[..], &SERIALIZED_MESSAGE);
    }

    #[test]
    fn deserialize_message() {
        let mut buffer = Bytes::copy_from_slice(&SERIALIZED_MESSAGE);
        let message =
            Message::<u8>::deserialize(&mut buffer).expect("should deserialize the message");
        assert_eq!(message.service, 0x1234);
        assert_eq!(message.method, 0x5678);
        assert_eq!(message.client, 0x9abc);
        assert_eq!(message.session, 0xdef0);
        assert_eq!(message.protocol, 0x01);
        assert_eq!(message.interface, 0x00);
        assert_eq!(message.message_type, MessageType::Response);
        assert_eq!(message.return_code, ReturnCode::NotOk);
        assert_eq!(message.payload, 1u8);
    }

    #[test]
    fn message_type_from_u8() {
        assert_eq!(MessageType::from(0x00), MessageType::Request);
        assert_eq!(MessageType::from(0x01), MessageType::RequestNoReturn);
        assert_eq!(MessageType::from(0x02), MessageType::Notification);
        assert_eq!(MessageType::from(0x80), MessageType::Response);
        assert_eq!(MessageType::from(0x81), MessageType::Error);
        assert_eq!(MessageType::from(0x20), MessageType::TpRequest);
        assert_eq!(MessageType::from(0x21), MessageType::TpRequestNoReturn);
        assert_eq!(MessageType::from(0x22), MessageType::TpNotification);
        assert_eq!(MessageType::from(0xa0), MessageType::TpResponse);
        assert_eq!(MessageType::from(0xa1), MessageType::TpError);
        (u8::MIN..=u8::MAX)
            .filter(|x| ![0x00, 0x01, 0x02, 0x80, 0x81, 0x20, 0x21, 0x22, 0xa0, 0xa1].contains(x))
            .for_each(|x| assert_eq!(MessageType::Unknown(x), MessageType::from(x)));
    }

    #[test]
    fn u8_from_message_type() {
        assert_eq!(0x00, u8::from(MessageType::Request));
        assert_eq!(0x01, u8::from(MessageType::RequestNoReturn));
        assert_eq!(0x02, u8::from(MessageType::Notification));
        assert_eq!(0x80, u8::from(MessageType::Response));
        assert_eq!(0x81, u8::from(MessageType::Error));
        assert_eq!(0x20, u8::from(MessageType::TpRequest));
        assert_eq!(0x21, u8::from(MessageType::TpRequestNoReturn));
        assert_eq!(0x22, u8::from(MessageType::TpNotification));
        assert_eq!(0xa0, u8::from(MessageType::TpResponse));
        assert_eq!(0xa1, u8::from(MessageType::TpError));
        (u8::MIN..=u8::MAX)
            .filter(|x| ![0x00, 0x01, 0x02, 0x80, 0x81, 0x20, 0x21, 0x22, 0xa0, 0xa1].contains(x))
            .for_each(|x| assert_eq!(u8::from(MessageType::Unknown(x)), x));
    }

    #[test]
    fn serialize_message_type() {
        let mut buffer = BytesMut::with_capacity(1);
        assert_eq!(MessageType::Response.serialize(&mut buffer), Ok(1));
        assert_eq!(&buffer.freeze()[..], &[0x80]);
    }

    #[test]
    fn deserialize_message_type() {
        let mut buffer = Bytes::copy_from_slice(&[0x80_u8]);
        assert_eq!(
            MessageType::deserialize(&mut buffer),
            Ok(MessageType::Response)
        );
    }

    #[test]
    fn return_code_from_u8() {
        assert_eq!(ReturnCode::from(0x00), ReturnCode::Ok);
        assert_eq!(ReturnCode::from(0x01), ReturnCode::NotOk);
        assert_eq!(ReturnCode::from(0x02), ReturnCode::UnknownService);
        assert_eq!(ReturnCode::from(0x03), ReturnCode::UnknownMethod);
        assert_eq!(ReturnCode::from(0x04), ReturnCode::NotReady);
        assert_eq!(ReturnCode::from(0x05), ReturnCode::NotReachable);
        assert_eq!(ReturnCode::from(0x06), ReturnCode::Timeout);
        assert_eq!(ReturnCode::from(0x07), ReturnCode::WrongProtocolVersion);
        assert_eq!(ReturnCode::from(0x08), ReturnCode::WrongInterfaceVersion);
        assert_eq!(ReturnCode::from(0x09), ReturnCode::MalformedMessage);
        assert_eq!(ReturnCode::from(0x0a), ReturnCode::WrongMessageType);
        assert_eq!(ReturnCode::from(0x0b), ReturnCode::E2eRepeated);
        assert_eq!(ReturnCode::from(0x0c), ReturnCode::E2eWrongSequence);
        assert_eq!(ReturnCode::from(0x0d), ReturnCode::E2e);
        assert_eq!(ReturnCode::from(0x0e), ReturnCode::E2eNotAvailable);
        assert_eq!(ReturnCode::from(0x0f), ReturnCode::E2eNoNewData);
        (0x10..=0x5e).for_each(|x| assert_eq!(ReturnCode::Reserved(x), ReturnCode::from(x)));
        (0x5f..=u8::MAX).for_each(|x| assert_eq!(ReturnCode::Unknown(x), ReturnCode::from(x)));
    }

    #[test]
    fn u8_from_return_code() {
        assert_eq!(0x00, u8::from(ReturnCode::Ok));
        assert_eq!(0x01, u8::from(ReturnCode::NotOk));
        assert_eq!(0x02, u8::from(ReturnCode::UnknownService));
        assert_eq!(0x03, u8::from(ReturnCode::UnknownMethod));
        assert_eq!(0x04, u8::from(ReturnCode::NotReady));
        assert_eq!(0x05, u8::from(ReturnCode::NotReachable));
        assert_eq!(0x06, u8::from(ReturnCode::Timeout));
        assert_eq!(0x07, u8::from(ReturnCode::WrongProtocolVersion));
        assert_eq!(0x08, u8::from(ReturnCode::WrongInterfaceVersion));
        assert_eq!(0x09, u8::from(ReturnCode::MalformedMessage));
        assert_eq!(0x0a, u8::from(ReturnCode::WrongMessageType));
        assert_eq!(0x0b, u8::from(ReturnCode::E2eRepeated));
        assert_eq!(0x0c, u8::from(ReturnCode::E2eWrongSequence));
        assert_eq!(0x0d, u8::from(ReturnCode::E2e));
        assert_eq!(0x0e, u8::from(ReturnCode::E2eNotAvailable));
        assert_eq!(0x0f, u8::from(ReturnCode::E2eNoNewData));
        (0x10..=0x5e).for_each(|x| assert_eq!(u8::from(ReturnCode::Reserved(x)), x));
        (0x5f..=u8::MAX).for_each(|x| assert_eq!(u8::from(ReturnCode::Unknown(x)), x));
    }

    #[test]
    fn serialize_return_code() {
        let mut buffer = BytesMut::with_capacity(1);
        assert_eq!(ReturnCode::NotOk.serialize(&mut buffer), Ok(1));
        assert_eq!(&buffer.freeze()[..], &[0x01]);
    }

    #[test]
    fn deserialize_return_code() {
        let mut buffer = Bytes::copy_from_slice(&[1u8]);
        assert_eq!(ReturnCode::deserialize(&mut buffer), Ok(ReturnCode::NotOk));
    }
}
