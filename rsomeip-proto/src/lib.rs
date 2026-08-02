//! Sans-IO implementation of the SOME/IP protocol.

#![cfg_attr(doc, doc = include_str!("../README.md"))]

mod message;
pub use message::{GenericMessage, Header};

/// SOME/IP message, Protocol Version 1.
///
/// This includes a [`Header`] after the [`MessageId`] and length field which contains additional data
/// about the message.
///
/// This is used by [`Endpoint`] and [`Interface`] to further check messages for correctness.
pub type Message<T> = GenericMessage<Header, T>;

mod primitives;
pub use primitives::{
    ClientId, InterfaceVersion, MessageId, MessageType, MessageTypeField, MethodId,
    ProtocolVersion, RequestId, ReturnCode, ServiceId, SessionId,
};

mod endpoint;
pub use endpoint::{Endpoint, EndpointError};

mod interface;
pub use interface::{Interface, InterfaceType, MethodType};

#[cfg(feature = "tp")]
pub mod tp;
