#![doc = include_str!("../README.md")]

pub(crate) mod message;
pub use message::{GenericMessage, Header};

/// SOME/IP message, Protocol Version 1.
///
/// This includes a [`Header`] after the [`MessageId`] and length field which contains additional data
/// about the message.
///
/// This is used by [`Endpoint`] and [`Interface`] to further check messages for correctness.
pub type Message<T> = GenericMessage<Header, T>;

pub(crate) mod primitives;
pub use primitives::{
    ClientId, InterfaceVersion, MessageId, MessageType, MessageTypeField, MethodId,
    ProtocolVersion, RequestId, ReturnCode, ServiceId, SessionId,
};

pub(crate) mod endpoint;
pub use endpoint::{Endpoint, EndpointError};

pub(crate) mod interface;
pub use interface::{Interface, InterfaceType, MethodType};
