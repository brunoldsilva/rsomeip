//! SOME/IP primitive types.
//!
//! This module provides strongly-typed definitions for the primitives used by the SOME/IP protocol.

use rsomeip_bytes::{Buf, BufMut, Deserialize, DeserializeError, Serialize, SerializeError};

/// Implements basic functionality for new-types that wrap a single primitive.
///
/// Includes const (`new` and `as_`) and non-const (`from` and `into`) methods for converting between
/// the new-type and its internal representation, and implements [`Serialize`], [`Deserialize`] and
/// [`std::fmt::Display`] for the type.
macro_rules! impl_basic_type {
    ($name:ident, $repr:ty, $getter:ident, $fmt:literal) => {
        impl $name {
            #[doc=concat!("Creates a new [`", stringify!($name), "`] with the given `value`.")]
            ///
            /// # Examples
            ///
            /// ```rust
            #[doc=concat!("use rsomeip_proto::", stringify!($name), ";")]
            ///
            #[doc=concat!("let value = ", stringify!($name), "::new(1 as ", stringify!($repr), ");")]
            #[doc=concat!("assert_eq!(value.", stringify!($getter), "(), 1 as ", stringify!($repr), ");")]
            /// ```
            #[inline]
            #[must_use]
            pub const fn new(value: $repr) -> Self {
                Self(value)
            }

            #[doc=concat!("Returns the [`", stringify!($repr), "`] representation of this [`", stringify!($name), "`].")]
            ///
            /// # Examples
            ///
            /// ```rust
            #[doc=concat!("use rsomeip_proto::", stringify!($name), ";")]
            ///
            #[doc=concat!("let value = ", stringify!($name), "::new(1 as ", stringify!($repr), ");")]
            #[doc=concat!("assert_eq!(value.", stringify!($getter), "(), 1 as ", stringify!($repr), ");")]
            /// ```
            #[inline]
            #[must_use]
            pub const fn $getter(self) -> $repr {
                self.0
            }
        }

        impl From<$repr> for $name {
            fn from(value: $repr) -> Self {
                Self::new(value)
            }
        }

        impl From<$name> for $repr {
            fn from(value: $name) -> Self {
                value.0
            }
        }

        impl rsomeip_bytes::Serialize for $name {
            fn serialize(&self, buffer: &mut impl BufMut) -> Result<usize, rsomeip_bytes::SerializeError> {
                self.0.serialize(buffer)
            }

            fn size_hint(&self) -> usize {
                self.0.size_hint()
            }
        }

        impl rsomeip_bytes::Deserialize for $name {
            type Output = Self;

            fn deserialize(buffer: &mut impl Buf) -> Result<Self::Output, rsomeip_bytes::DeserializeError> {
                <$repr>::deserialize(buffer).map(Self::new)
            }
        }

        impl ::std::fmt::Display for $name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                write!(f, $fmt, self.0)
            }
        }
    };
}

macro_rules! impl_basic_type_u16 {
    ($name:ident) => {
        impl_basic_type!($name, u16, as_u16, "{:04x?}");
    };
}

macro_rules! impl_basic_type_u8 {
    ($name:ident) => {
        impl_basic_type!($name, u8, as_u8, "{:02x?}");
    };
}

/// Unique identifier of a service interface.
///
/// A service is a logical combination of zero or more [methods], zero or more events, and zero or
/// more fields.
///
/// [methods]: MethodId
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ServiceId(u16);

impl_basic_type_u16!(ServiceId);

/// Unique identifier of a method, field or event on a service interface.
///
/// A method is a callable function, procedure or subroutine which can be invoked by consumers of a
/// service.
///
/// A field represents a status or valid value on which getters, setters and notifiers act upon.
///
/// An event is a uni-directional data transmission that is invoked on changes or cyclically and
/// is sent to consumers of a service.
///
/// # Practices
///
/// It's common practice to split the ID space of [`MethodId`] between methods and
/// events/notifications.
///
/// Methods would use the range `0x0000-0x7fff` and events/notifications would use the range
/// `0x8fff-0xffff`.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MethodId(u16);

impl_basic_type_u16!(MethodId);

/// Unique identifier of a method or event on a service interface.
///
/// Consists of [`ServiceId`] followed by a [`MethodId`].
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MessageId {
    /// Service portion of the id.
    pub service: ServiceId,
    /// Method portion of the id.
    pub method: MethodId,
}

impl MessageId {
    /// Creates a new [`MessageId`] with the given `service` and `method` ids.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rsomeip_proto::{MethodId, MessageId, ServiceId};
    ///
    /// let id = MessageId::new(ServiceId::new(0x1234), MethodId::new(0x5678));
    /// assert_eq!(id.service.as_u16(), 0x1234);
    /// assert_eq!(id.method.as_u16(), 0x5678);
    /// ```
    #[inline]
    #[must_use]
    pub const fn new(service: ServiceId, method: MethodId) -> Self {
        Self { service, method }
    }

    /// Returns `self` with its [`ServiceId`] set to `value`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rsomeip_proto::{MessageId, ServiceId};
    ///
    /// let id = MessageId::default().with_service(ServiceId::new(0x1234));
    /// assert_eq!(id.service.as_u16(), 0x1234);
    /// ```
    #[inline]
    #[must_use]
    pub const fn with_service(mut self, value: ServiceId) -> Self {
        self.service = value;
        self
    }

    /// Returns `self` with its [`MethodId`] set to `value`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rsomeip_proto::{MethodId, MessageId};
    ///
    /// let id = MessageId::default().with_method(MethodId::new(0x1234));
    /// assert_eq!(id.method.as_u16(), 0x1234);
    /// ```
    #[inline]
    #[must_use]
    pub const fn with_method(mut self, value: MethodId) -> Self {
        self.method = value;
        self
    }

    /// Creates a new [`MessageId`] from the given [`u32`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rsomeip_proto::MessageId;
    ///
    /// let id = MessageId::from_u32(0x1234_5678);
    /// assert_eq!(id.service.as_u16(), 0x1234);
    /// assert_eq!(id.method.as_u16(), 0x5678);
    /// ```
    #[inline]
    #[must_use]
    pub const fn from_u32(value: u32) -> Self {
        let service = ServiceId::new((value >> 16) as u16);
        let method = MethodId::new((value & 0xffff) as u16);
        Self::new(service, method)
    }

    /// Returns the [`u32`] representation of `self`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rsomeip_proto::{MethodId, MessageId, ServiceId};
    ///
    /// let id = MessageId::new(ServiceId::new(0x1234), MethodId::new(0x5678));
    /// assert_eq!(id.as_u32(), 0x1234_5678);
    /// ```
    #[inline]
    #[must_use]
    pub const fn as_u32(self) -> u32 {
        let service = (self.service.as_u16() as u32) << 16;
        service | (self.method.as_u16() as u32)
    }
}

impl From<u32> for MessageId {
    fn from(value: u32) -> Self {
        Self::from_u32(value)
    }
}

impl From<MessageId> for u32 {
    fn from(value: MessageId) -> Self {
        value.as_u32()
    }
}

impl Serialize for MessageId {
    fn serialize(&self, buffer: &mut impl BufMut) -> Result<usize, SerializeError> {
        self.as_u32().serialize(buffer)
    }

    fn size_hint(&self) -> usize {
        size_of::<u32>()
    }
}

impl Deserialize for MessageId {
    type Output = Self;

    fn deserialize(buffer: &mut impl Buf) -> Result<Self::Output, DeserializeError> {
        u32::deserialize(buffer).map(Self::from_u32)
    }
}

impl std::fmt::Display for MessageId {
    /// Formats `self` using the given formatter.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rsomeip_proto::{ServiceId, MethodId, MessageId};
    ///
    /// let service = ServiceId::new(0x1234);
    /// let method = MethodId::new(0x5678);
    /// let message = MessageId::new(service, method);
    /// assert_eq!(format!("{message}"), "1234.5678");
    /// ```
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}", self.service, self.method)
    }
}

/// Unique identifier of the client of a service interface.
///
/// This is used to differentiate calls from multiple clients to the same method.
///
/// Should be unique in the network.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ClientId(u16);

impl_basic_type_u16!(ClientId);

impl ClientId {
    /// Returns the prefix of this id.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rsomeip_proto::ClientId;
    ///
    /// let id = ClientId::new(0x0034).with_prefix(0x12);
    /// assert_eq!(id.as_u16(), 0x1234);
    /// assert_eq!(id.prefix(), 0x12);
    /// ```
    #[inline]
    #[must_use]
    pub const fn prefix(&self) -> u8 {
        self.0.to_be_bytes()[0]
    }

    /// Returns `self` with the given `prefix`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rsomeip_proto::ClientId;
    ///
    /// let id = ClientId::new(0x0034).with_prefix(0x12);
    /// assert_eq!(id.as_u16(), 0x1234);
    /// assert_eq!(id.prefix(), 0x12);
    /// ```
    #[inline]
    #[must_use]
    pub const fn with_prefix(mut self, prefix: u8) -> Self {
        self.0 |= (prefix as u16) << 8;
        self
    }
}

/// Unique identifier of a sequential message or request.
///
/// Used to differentiate messages to the same method or event.
///
/// # Session Handling
///
/// When session handling is active (`0x0001-0xffff`), the session ID should be
/// incremented according to the respective use case.
///
/// After reaching `0xffff`, the session ID should wrap back around to `1`.
///
/// ## Request/Response
///
/// Each new request should increment the ID by `1` and each response should copy over
/// the same ID from the request.
///
/// ## SOME/IP-TP
///
/// Each SOME/IP-TP segment should carry over the ID of the original message.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SessionId(u16);

impl_basic_type_u16!(SessionId);

impl SessionId {
    /// Default value when session handling is disabled.
    pub const DISABLED: Self = Self::new(0);

    /// Default value when session handling is enabled.
    pub const ENABLED: Self = Self::new(1);

    /// Whether session handling is enabled.
    ///
    /// Session handling is considered enabled if the [`SessionId`] is greater than `0`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rsomeip_proto::SessionId;
    ///
    /// assert!(SessionId::ENABLED.is_enabled());
    /// assert!(!SessionId::DISABLED.is_enabled());
    /// ```
    #[inline]
    #[must_use]
    pub const fn is_enabled(self) -> bool {
        self.0 > 0
    }

    /// Increments this [`SessionId`] by one and returns the old value.
    ///
    /// After `0xffff`, incrementing the ID wraps it back to `1`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rsomeip_proto::SessionId;
    ///
    /// let mut session = SessionId::new(0x0001);
    /// assert_eq!(session.increment().as_u16(), 0x0001);
    /// assert_eq!(session.as_u16(), 0x0002);
    /// ```
    #[expect(clippy::return_self_not_must_use)]
    pub const fn increment(&mut self) -> Self {
        let old = self.0;
        self.0 = match old.wrapping_add(1) {
            0 => 1,
            x => x,
        };
        Self::new(old)
    }
}

/// Unique identifier of message to a service interface.
///
/// Consists of a [`ClientId`] followed by a [`SessionId`].
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RequestId {
    /// Client portion of the id.
    pub client: ClientId,
    /// Session portion of the id.
    pub session: SessionId,
}

impl RequestId {
    /// Creates a new [`RequestId`] with the given `client` and `session` ids.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rsomeip_proto::{ClientId, RequestId, SessionId};
    ///
    /// let id = RequestId::new(ClientId::new(0x1234), SessionId::new(0x5678));
    /// assert_eq!(id.client.as_u16(), 0x1234);
    /// assert_eq!(id.session.as_u16(), 0x5678);
    /// ```
    #[inline]
    #[must_use]
    pub const fn new(client: ClientId, session: SessionId) -> Self {
        Self { client, session }
    }

    /// Returns `self` with its [`ClientId`] set to `value`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rsomeip_proto::{ClientId, RequestId};
    ///
    /// let id = RequestId::default().with_client(ClientId::new(0x1234));
    /// assert_eq!(id.client.as_u16(), 0x1234);
    /// ```
    #[inline]
    #[must_use]
    pub const fn with_client(mut self, value: ClientId) -> Self {
        self.client = value;
        self
    }

    /// Returns `self` with its [`SessionId`] set to `value`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rsomeip_proto::{RequestId, SessionId};
    ///
    /// let id = RequestId::default().with_session(SessionId::new(0x1234));
    /// assert_eq!(id.session.as_u16(), 0x1234);
    /// ```
    #[inline]
    #[must_use]
    pub const fn with_session(mut self, value: SessionId) -> Self {
        self.session = value;
        self
    }

    /// Creates a new [`RequestId`] from the given [`u32`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rsomeip_proto::RequestId;
    ///
    /// let id = RequestId::from_u32(0x1234_5678);
    /// assert_eq!(id.client.as_u16(), 0x1234);
    /// assert_eq!(id.session.as_u16(), 0x5678);
    /// ```
    #[must_use]
    pub const fn from_u32(value: u32) -> Self {
        let client = ClientId::new((value >> 16) as u16);
        let session = SessionId::new((value & 0xffff) as u16);
        Self::new(client, session)
    }

    /// Returns the [`u32`] representation of `self`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rsomeip_proto::{ClientId, RequestId, SessionId};
    ///
    /// let id = RequestId::new(ClientId::new(0x1234), SessionId::new(0x5678));
    /// assert_eq!(id.as_u32(), 0x1234_5678);
    /// ```
    #[must_use]
    pub const fn as_u32(self) -> u32 {
        let client = (self.client.as_u16() as u32) << 16;
        client | (self.session.as_u16() as u32)
    }
}

impl From<u32> for RequestId {
    fn from(value: u32) -> Self {
        Self::from_u32(value)
    }
}

impl From<RequestId> for u32 {
    fn from(value: RequestId) -> Self {
        value.as_u32()
    }
}

impl Serialize for RequestId {
    fn serialize(&self, buffer: &mut impl BufMut) -> Result<usize, SerializeError> {
        self.as_u32().serialize(buffer)
    }

    fn size_hint(&self) -> usize {
        size_of::<u32>()
    }
}

impl Deserialize for RequestId {
    type Output = Self;

    fn deserialize(buffer: &mut impl Buf) -> Result<Self::Output, DeserializeError> {
        u32::deserialize(buffer).map(Self::from_u32)
    }
}

impl std::fmt::Display for RequestId {
    /// Formats `self` using the given formatter.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rsomeip_proto::{ClientId, RequestId, SessionId};
    ///
    /// let client = ClientId::new(0x1234);
    /// let session = SessionId::new(0x5678);
    /// let request = RequestId::new(client, session);
    /// assert_eq!(format!("{request}"), "1234.5678");
    /// ```
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}", self.client, self.session)
    }
}

/// Major version of the SOME/IP protocol.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProtocolVersion(u8);

impl_basic_type_u8!(ProtocolVersion);

/// Major version of a service interface.
///
/// # Compatibility
///
/// The [`InterfaceVersion`] identifies the format of the SOME/IP message body.
///
/// It should be incremented for any of the following reasons:
///
/// - incompatible changes to the payload format
/// - incompatible changes to the service behavior
/// - required by application design
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InterfaceVersion(u8);

impl_basic_type_u8!(InterfaceVersion);

/// [`MessageType`] and associated flags packed into a single byte.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct MessageTypeField(u8);

impl_basic_type_u8!(MessageTypeField);

impl MessageTypeField {
    /// Transport Protocol flag. Determines if the message is a segment of a larger message.
    pub const TP_FLAG: u8 = 0x20;

    /// A mask of all [`MessageTypeField`] flags.
    pub const ALL_FLAGS: u8 = Self::TP_FLAG;

    /// Whether the Transport Protocol flag is set.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rsomeip_proto::MessageTypeField;
    ///
    /// assert!(!MessageTypeField::new(0).is_tp());
    /// assert!(MessageTypeField::new(MessageTypeField::TP_FLAG).is_tp());
    /// ```
    #[inline]
    #[must_use]
    pub const fn is_tp(&self) -> bool {
        self.0 & Self::TP_FLAG == Self::TP_FLAG
    }

    /// Returns the mask of all currently enabled flags.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rsomeip_proto::MessageTypeField;
    ///
    /// let mask = MessageTypeField::new(MessageTypeField::TP_FLAG).flags();
    /// assert_eq!(mask, MessageTypeField::TP_FLAG);
    /// ```
    #[inline]
    #[must_use]
    pub const fn flags(self) -> u8 {
        self.0 & Self::ALL_FLAGS
    }

    /// Returns this [`MessageTypeField`] with the given `flags` set to `enabled`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rsomeip_proto::MessageTypeField;
    ///
    /// let field = MessageTypeField::default()
    ///     .with_flags(MessageTypeField::TP_FLAG, true);
    /// assert_eq!(field.flags(), MessageTypeField::TP_FLAG);
    ///
    /// let field = field.with_flags(MessageTypeField::TP_FLAG, false);
    /// assert_eq!(field.flags(), 0);
    /// ```
    #[inline]
    #[must_use]
    pub const fn with_flags(mut self, flags: u8, enabled: bool) -> Self {
        if enabled {
            self.0 |= flags;
        } else {
            self.0 &= !flags;
        }
        self
    }

    /// Returns the [`MessageType`] representation of this [`MessageTypeField`].
    ///
    /// Ignores all flags.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rsomeip_proto::{MessageType, MessageTypeField};
    ///
    /// let field = MessageTypeField::new(0x80);
    /// assert_eq!(field.as_type(), MessageType::Response);
    /// ```
    #[inline]
    #[must_use]
    pub const fn as_type(self) -> MessageType {
        MessageType::from_u8(self.0)
    }

    /// Creates a new [`MessageTypeField`] from the given [`MessageType`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rsomeip_proto::{MessageType, MessageTypeField};
    ///
    /// let field = MessageTypeField::from_type(MessageType::Response);
    /// assert_eq!(field.as_u8(), 0x80);
    /// ```
    #[inline]
    #[must_use]
    pub const fn from_type(value: MessageType) -> Self {
        Self::new(value.as_u8())
    }
}

/// Type of a SOME/IP message.
///
/// It's used both by servers and clients to specify how the message should be interpreted by the
/// peer.
#[non_exhaustive]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum MessageType {
    /// A request expecting a response.
    #[default]
    Request,
    /// A fire-and-forget request.
    RequestNoReturn,
    /// A notification or event callback expecting no response.
    Notification,
    /// A response message.
    Response,
    /// A response containing an error.
    Error,
    /// Unknown message type.
    Unknown(u8),
}

impl MessageType {
    /// Creates a new [`MessageType`] from the given `value`.
    #[inline]
    #[must_use]
    pub const fn from_u8(value: u8) -> Self {
        match value & !MessageTypeField::ALL_FLAGS {
            0x00 => Self::Request,
            0x01 => Self::RequestNoReturn,
            0x02 => Self::Notification,
            0x80 => Self::Response,
            0x81 => Self::Error,
            x => Self::Unknown(x),
        }
    }

    /// Returns the [`u8`] representation of `self`.
    #[inline]
    #[must_use]
    pub const fn as_u8(self) -> u8 {
        match self {
            Self::Request => 0x00,
            Self::RequestNoReturn => 0x01,
            Self::Notification => 0x02,
            Self::Response => 0x80,
            Self::Error => 0x81,
            Self::Unknown(x) => x,
        }
    }
}

impl From<u8> for MessageType {
    fn from(value: u8) -> Self {
        Self::from_u8(value)
    }
}

impl From<MessageType> for u8 {
    fn from(value: MessageType) -> Self {
        value.as_u8()
    }
}

impl Serialize for MessageType {
    fn serialize(&self, buffer: &mut impl BufMut) -> Result<usize, SerializeError> {
        u8::from(*self).serialize(buffer)
    }

    fn size_hint(&self) -> usize {
        size_of::<u8>()
    }
}

impl Deserialize for MessageType {
    type Output = Self;

    fn deserialize(buffer: &mut impl Buf) -> Result<Self::Output, DeserializeError> {
        u8::deserialize(buffer).map(Self::from)
    }
}

impl std::fmt::Display for MessageType {
    /// Formats `self` using the given formatter.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rsomeip_proto::MessageType;
    ///
    /// assert_eq!(format!("{}", MessageType::Request), "Request");
    /// ```
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

/// Result of processing a SOME/IP request.
#[non_exhaustive]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum ReturnCode {
    /// No error occurred.
    #[default]
    Ok,
    /// An unspecified error occurred.
    NotOk,
    /// The requested [`ServiceId`] is unknown.
    UnknownService,
    /// The requested [`MethodId`] is unknown.
    UnknownMethod,
    /// Application not running.
    NotReady,
    /// System running the service not reachable (internal only).
    NotReachable,
    /// A timeout occurred (internal only).
    Timeout,
    /// Version of the SOME/IP protocol not supported.
    WrongProtocolVersion,
    /// Interface version mismatch.
    WrongInterfaceVersion,
    /// Payload could not be deserialized.
    MalformedMessage,
    /// Wrong [`MessageType`] was received.
    WrongMessageType,
    /// Repeated E2E calculation error.
    E2eRepeated,
    /// Wrong E2E sequence error.
    E2eWrongSequence,
    /// Unspecified E2E error.
    E2e,
    /// E2E not available.
    E2eNotAvailable,
    /// No new data for E2E calculation present.
    E2eNoNewData,
    /// Reserved for generic errors to be specified in future versions of SOME/IP.
    Reserved(u8),
    /// Reserved for errors specified by the service interface.
    Other(u8),
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
            n => Self::Other(n),
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
            ReturnCode::Reserved(x) | ReturnCode::Other(x) => x,
        }
    }
}

impl Serialize for ReturnCode {
    fn serialize(&self, buffer: &mut impl BufMut) -> Result<usize, SerializeError> {
        u8::from(*self).serialize(buffer)
    }

    fn size_hint(&self) -> usize {
        size_of::<u8>()
    }
}

impl Deserialize for ReturnCode {
    type Output = Self;

    fn deserialize(buffer: &mut impl Buf) -> Result<Self::Output, DeserializeError> {
        u8::deserialize(buffer).map(Self::from)
    }
}

impl std::fmt::Display for ReturnCode {
    /// Formats `self` using the given formatter.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rsomeip_proto::ReturnCode;
    ///
    /// assert_eq!(format!("{}", ReturnCode::Ok), "Ok");
    /// ```
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rsomeip_bytes::{Bytes, BytesMut};

    /// Tests the serialization and deserialization of new-types to and from their internal representation.
    macro_rules! test_new_type_serialization {
        ($name:ty, $repr:ty, $ser_test_name:ident, $de_test_name:ident) => {
            #[test]
            fn $ser_test_name() {
                let value = <$name>::from(<$repr>::MAX);
                let mut buffer = BytesMut::new();
                assert_eq!(value.size_hint(), size_of::<$repr>());
                assert_eq!(value.serialize(&mut buffer), Ok(size_of::<$repr>()));
                assert_eq!(buffer[..], <$repr>::MAX.to_be_bytes()[..]);
            }

            #[test]
            fn $de_test_name() {
                let mut buffer = Bytes::copy_from_slice(&(<$repr>::MAX).to_be_bytes()[..]);
                let value =
                    <$name>::deserialize(&mut buffer).expect("should deserialize the value");
                assert_eq!(<$repr>::from(value), <$repr>::MAX);
            }
        };
    }

    test_new_type_serialization!(
        ServiceId,
        u16,
        service_id_is_serializable,
        service_id_is_deserializable
    );
    test_new_type_serialization!(
        MethodId,
        u16,
        method_id_is_serializable,
        method_id_is_deserializable
    );
    test_new_type_serialization!(
        MessageId,
        u32,
        message_id_is_serializable,
        message_id_is_deserializable
    );
    test_new_type_serialization!(
        ClientId,
        u16,
        client_id_is_serializable,
        client_id_is_deserializable
    );
    test_new_type_serialization!(
        SessionId,
        u16,
        session_id_is_serializable,
        session_id_is_deserializable
    );
    test_new_type_serialization!(
        RequestId,
        u32,
        request_id_is_serializable,
        request_id_is_deserializable
    );
    test_new_type_serialization!(
        ProtocolVersion,
        u8,
        protocol_version_is_serializable,
        protocol_version_is_deserializable
    );
    test_new_type_serialization!(
        InterfaceVersion,
        u8,
        interface_version_is_serializable,
        interface_version_is_deserializable
    );
    test_new_type_serialization!(
        MessageTypeField,
        u8,
        message_type_is_serializable,
        message_type_is_deserializable
    );
    test_new_type_serialization!(
        ReturnCode,
        u8,
        return_code_is_serializable,
        return_code_is_deserializable
    );

    #[test]
    fn message_id_converts_from_u32() {
        let message_id = MessageId::from(0x1234_5678_u32);
        assert_eq!(message_id.service.as_u16(), 0x1234);
        assert_eq!(message_id.method.as_u16(), 0x5678);
    }

    #[test]
    fn message_id_converts_to_u32() {
        let service = ServiceId::new(0x1234);
        let method = MethodId::new(0x5678);
        assert_eq!(u32::from(MessageId::new(service, method)), 0x1234_5678_u32);
    }

    #[test]
    fn request_id_converts_from_u32() {
        let request_id = RequestId::from(0x1234_5678_u32);
        assert_eq!(request_id.client.as_u16(), 0x1234);
        assert_eq!(request_id.session.as_u16(), 0x5678);
    }

    #[test]
    fn request_id_converts_to_u32() {
        let client = ClientId::new(0x1234);
        let session = SessionId::new(0x5678);
        assert_eq!(u32::from(RequestId::new(client, session)), 0x1234_5678_u32);
    }

    #[test]
    fn is_session_handling_enabled() {
        assert!(!SessionId::DISABLED.is_enabled());
        assert!(SessionId::ENABLED.is_enabled());
    }

    #[test]
    fn session_id_wraps_to_1() {
        let mut session = SessionId::new(0xffff);
        assert_eq!(session.increment().as_u16(), 0xffff);
        assert_eq!(session.as_u16(), 1);
    }

    #[test]
    fn message_type_converts_from_u8() {
        assert_eq!(MessageType::from(0x00), MessageType::Request);
        assert_eq!(MessageType::from(0x01), MessageType::RequestNoReturn);
        assert_eq!(MessageType::from(0x02), MessageType::Notification);
        assert_eq!(MessageType::from(0x80), MessageType::Response);
        assert_eq!(MessageType::from(0x81), MessageType::Error);
        assert_eq!(MessageType::from(0x20), MessageType::Request);
        assert_eq!(MessageType::from(0x21), MessageType::RequestNoReturn);
        assert_eq!(MessageType::from(0x22), MessageType::Notification);
        assert_eq!(MessageType::from(0xa0), MessageType::Response);
        assert_eq!(MessageType::from(0xa1), MessageType::Error);
        (u8::MIN..=u8::MAX)
            .filter(|x| ![0x00, 0x01, 0x02, 0x80, 0x81, 0x20, 0x21, 0x22, 0xa0, 0xa1].contains(x))
            .for_each(|x| {
                assert_eq!(
                    MessageType::Unknown(x & !MessageTypeField::ALL_FLAGS),
                    MessageType::from(x)
                );
            });
    }

    #[test]
    fn message_type_converts_to_u8() {
        assert_eq!(0x00, u8::from(MessageType::Request));
        assert_eq!(0x01, u8::from(MessageType::RequestNoReturn));
        assert_eq!(0x02, u8::from(MessageType::Notification));
        assert_eq!(0x80, u8::from(MessageType::Response));
        assert_eq!(0x81, u8::from(MessageType::Error));
        (u8::MIN..=u8::MAX)
            .filter(|x| ![0x00, 0x01, 0x02, 0x80, 0x81, 0x20, 0x21, 0x22, 0xa0, 0xa1].contains(x))
            .for_each(|x| {
                assert_eq!(u8::from(MessageType::Unknown(x)), x);
            });
    }

    #[test]
    fn return_code_converts_from_u8() {
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
        (0x5f..=u8::MAX).for_each(|x| assert_eq!(ReturnCode::Other(x), ReturnCode::from(x)));
    }

    #[test]
    fn return_code_converts_to_u8() {
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
        (0x5f..=u8::MAX).for_each(|x| assert_eq!(u8::from(ReturnCode::Other(x)), x));
    }
}
