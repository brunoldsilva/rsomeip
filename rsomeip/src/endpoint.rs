//! SOME/IP endpoints.
//!
//! This module defines an interface for implementing SOME/IP endpoints, and provides a concrete
//! implementation for Version 1 of the protocol.
//!
//! - [`Server`] is a trait for creating [`Stub`] and [`Proxy`] handles to SOME/IP service
//!   interfaces.
//!
//! - [`Stub`] is a trait for sending and receiving SOME/IP messages from multiple remote endpoints.
//!
//! - [`Proxy`] is a trait for sending and receiving SOME/IP messages from a single remote endpoint.
//!
//! - [`InterfaceId`] is a unique identifier used to route SOME/IP messages to the correct
//!   interfaces.
//!
//! - [`v1`] is a concrete implementation of Version 1 of the SOME/IP protocol.

use crate::{bytes::Bytes, socket::SocketAddr, someip, Result};

pub mod v1;

/// A trait for implementing a SOME/IP endpoint.
///
/// Services can be served to remote endpoints using the [`serve`] method.
///
/// [`serve`]: Server::serve
pub trait Server {
    /// Type of stubs to services of this endpoint.
    type Stub: Stub + 'static;
    /// Type of stubs to services of this endpoint.
    type Proxy: Proxy + 'static;

    /// Serves the given service on this endpoint.
    ///
    /// SOME/IP messages with matching service id and interface version will be forwarded to the
    /// [`Stub`].
    ///
    /// # Errors
    ///
    /// Returns an error if the endpoint cannot create a listener for incoming connections.
    #[allow(async_fn_in_trait)]
    async fn serve(&mut self, interface: InterfaceId) -> Result<Self::Stub>;

    /// Proxies the service at the given address.
    ///
    /// SOME/IP messages with matching service id and interface version will be forwarded to the
    /// [`Proxy`].
    ///
    /// # Errors
    ///
    /// Returns an error if a connection cannot be established to the remote address.
    #[allow(async_fn_in_trait)]
    async fn proxy(&mut self, interface: InterfaceId, address: SocketAddr) -> Result<Self::Proxy>;
}

/// A trait for communicating with multiple endpoints.
pub trait Stub: Sender + Receiver {
    /// Type of the stub's sender half.
    type Sender: Sender + 'static;
    /// Type of the stub's receiver half.
    type Receiver: Receiver + 'static;

    /// Splits the [`Stub`] into [`Sender`] and [`Receiver`] halves.
    fn into_split(self) -> (Self::Sender, Self::Receiver);
}

/// A trait for sending messages to remote endpoints.
#[allow(async_fn_in_trait)]
pub trait Sender {
    /// Sends a message to the given address.
    ///
    /// This will attempt to establish a connection to the remote address, if one does not already
    /// exist, but this may fail depending on the sockets [`ProtocolType`].
    ///
    /// # Errors
    ///
    /// Returns an error if the connection to the address cannot be established, if the endpoint
    /// has already been dropped, or if the message is invalid.
    ///
    /// # Protocol Type
    ///
    /// Depending on the [`ProtocolType`] of the underlying communication protocol, a [`Stub`]
    /// may or may not be able to establish connections to remote endpoints.
    ///
    /// `Datagram` type protocols allow this behavior, but `Stream` type protocols require that the
    /// client be the one to establish the connection.
    ///
    /// [`ProtocolType`]: crate::socket::ProtocolType
    async fn send_to(&mut self, address: SocketAddr, message: someip::Message<Bytes>)
        -> Result<()>;
}

/// A trait for receiving messages from remote endpoints.
#[allow(async_fn_in_trait)]
pub trait Receiver {
    /// Receives a message from a remote address.
    ///
    /// # Errors
    ///
    /// Returns an error if the endpoint has already been dropped.
    async fn recv_from(&mut self) -> Result<(someip::Message<Bytes>, SocketAddr)>;
}

/// A trait for communicating with a single endpoint.
pub trait Proxy: ConnectedSender + ConnectedReceiver {
    /// Type of the proxy's sender half.
    type Sender: ConnectedSender + 'static;
    /// Type of the proxy's receiver half.
    type Receiver: ConnectedReceiver + 'static;

    /// Splits the [`Proxy`] into [`ConnectedSender`] and [`ConnectedReceiver`] halves.
    fn into_split(self) -> (Self::Sender, Self::Receiver);
}

/// A trait for sending messages to a connected endpoints.
#[allow(async_fn_in_trait)]
pub trait ConnectedSender {
    /// Sends a message to the connected endpoint.
    ///
    /// # Errors
    ///
    /// Returns an error if the connection to the address is closed. if the endpoint has already
    /// been dropped, or if the message is invalid.
    async fn send(&mut self, message: someip::Message<Bytes>) -> Result<()>;
}

/// A trait for receiving messages from a connected endpoint.
#[allow(async_fn_in_trait)]
pub trait ConnectedReceiver {
    /// Receives a message from the connected endpoint.
    ///
    /// # Errors
    ///
    /// Returns an error if the endpoint has already been dropped.
    async fn recv(&mut self) -> Result<someip::Message<Bytes>>;
}

/// Unique identifier of a SOME/IP service interface.
///
/// This is used when routing incoming messages to the correct handler.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct InterfaceId {
    /// Unique ID of the service.
    pub service: someip::ServiceId,
    /// Major version of the interface.
    pub version: someip::InterfaceVersion,
}

impl InterfaceId {
    /// Creates a new [`InterfaceId`].
    pub fn new(service: someip::ServiceId, version: someip::InterfaceVersion) -> Self {
        Self { service, version }
    }
}

impl<T> From<&someip::Message<T>> for InterfaceId {
    /// Converts a [`Message<T>`] reference into an [`InterfaceId`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rsomeip::{endpoint::InterfaceId, someip::Message};
    ///
    /// let message = Message::new(0u8)
    ///     .with_service(0x1234)
    ///     .with_interface(0x01);
    /// let id = InterfaceId::from(&message);
    /// assert_eq!(message.service, id.service);
    /// assert_eq!(message.interface, id.version);
    /// ```
    ///
    /// [`Message<T>`]: crate::someip::Message
    fn from(value: &someip::Message<T>) -> Self {
        Self {
            service: value.service,
            version: value.interface,
        }
    }
}

impl std::fmt::Display for InterfaceId {
    /// Formats the [`InterfaceId`] into a string.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rsomeip::endpoint::InterfaceId;
    ///
    /// let id = InterfaceId::new(0x1234, 0x01);
    /// assert_eq!(format!("{id}"), "I.1234.01");
    /// ```
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "I.{:04x?}.{:02x?}", self.service, self.version)
    }
}
