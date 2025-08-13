//! Socket abstraction layer.
//!
//! This module provides trait definitions for abstracting the underlying communication protocols
//! used by SOME/IP, and provides reference implementations for the UDP and TCP protocols.
//!
//! These traits include [`Connector`] and [`Listener`], which allow establishing connections
//! between sockets, and [`Sender`] and [`Receiver`], which allow sending and receiving data
//! between the sockets.
//!
//! # Transmission Control Protocol.
//!
//! TCP is a connection-oriented, reliable, ordered, and error-checked communication protocol of
//! the Internet Protocol suite.
//!
//! A basic socket implementation of this protocol is provided by the [`tcp`] module.
//!
//! # User Datagram Protocol (UDP)
//!
//! UDP is a connection-less, datagram-based communication protocol of the Internet Protocol suite
//! designed for speed and efficiency.
//!
//! A basic socket implementation of this protocol is provided by the [`udp`] module.

use rsomeip_bytes::Bytes;

/// Re-export for convenience.
pub use std::net::SocketAddr;

pub mod tcp;

pub mod udp;

/// A specialized [Result] type for I/O operations.
type IoResult<T> = std::io::Result<T>;

/// A trait for establishing connections between sockets.
///
/// Using the [`connect`] method, a connection can be established between the socket's address and
/// the target address, and using the [`listen`] method, incoming connections can be accepted by
/// the socket.
///
/// [`connect`]: Connector::connect
/// [`listen`]: Connector::listen
pub trait Connector {
    /// Type of the connection's sender half.
    type Sender: Sender + 'static;
    /// Type of the connection's receiver half.
    type Receiver: Receiver + 'static;
    /// Type of the socket's connection listener.
    type Listener: Listener + 'static;

    /// Type of the underlying protocol.
    const PROTOCOL_TYPE: ProtocolType;

    /// Establishes a connection to the target address.
    ///
    /// The connection is split into two halves: one for sending data, and another for receiving
    /// data.
    ///
    /// # Errors
    ///
    /// Returns an error if the connection cannot be established, either because the address is
    /// invalid, or because there is an issue with the socket itself.
    ///
    /// # Behavior
    ///
    /// The behavior of this method varies depending on the underlying protocol type.
    ///
    /// ## Stream
    ///
    /// A call to `connect` will attempt to create a stream between this connector and the
    /// listener at the given address. If the stream is accepted by the listener, the method will
    /// return a [`Sender`] and [`Receiver`] bound to the other socket.
    ///
    /// ## Datagram
    ///
    /// A call to `connect` will return a [`Sender`] and [`Receiver`] bound to the given address,
    /// without establishing any underlying stream between the two sockets. This means that the
    /// other socket will only become aware of this socket once some data is sent to it.
    #[allow(async_fn_in_trait)]
    async fn connect(&mut self, address: &SocketAddr) -> IoResult<(Self::Sender, Self::Receiver)>;

    /// Creates a listener for accepting incoming connections.
    ///
    /// `backlog` defines the maximum number of pending connections which can be queued at any
    /// given time. Connection are removed from the queue with [`Listener::accept`]. When the queue
    /// is full, new connections will be rejected.
    ///
    /// # Errors
    ///
    /// Returns an error if the listener cannot be created, possibly because there is an issue
    /// with the socket.
    #[allow(async_fn_in_trait)]
    async fn listen(&mut self, backlog: u32) -> IoResult<Self::Listener>;
}

/// A trait for sending to connected sockets.
pub trait Sender {
    /// Sends the contents of the buffer to the connected socket.
    ///
    /// # Errors
    ///
    /// Returns an error if the data could not be sent, either because there is a problem with the
    /// connection, or with the data itself.
    ///
    /// # Buffer Size
    ///
    /// Datagram based protocols may impose a maximum payload size. If the buffer is larger than
    /// the maximum size, the payload may be split or the operation may fail, depending on the
    /// implementation.
    #[allow(async_fn_in_trait)]
    async fn send(&mut self, buffer: Bytes) -> Result<(), SendError<Bytes>>;
}

/// Represents an error when sending data through a [`Sender`].
///
/// Contains the data that could not be sent.
#[derive(Debug)]
pub struct SendError<T>(T);

/// A trait for receiving data from connected sockets.
pub trait Receiver {
    /// Receives data from the connected socket.
    ///
    /// # Errors
    ///
    /// Returns an error if the data could not be received, possibly due of a problem with the
    /// connection.
    #[allow(async_fn_in_trait)]
    async fn recv(&mut self) -> Result<Bytes, RecvError>;
}

/// Represents an error when receiving data from a [`Receiver`].
#[derive(Debug)]
pub enum RecvError {
    /// The receiver is closed and will not receive any more data.
    Closed,
    /// The receiver lagged too far behind. Attempting to receive again will return the oldest
    /// message still retained by the channel.
    Lagged(u64),
}

/// A trait for accepting incoming connections to the socket.
///
/// Implementations of this trait allow accepting incoming connections from remote sockets,
/// represented by a [`Sender`]-[`Receiver`] pair and a source address.
///
/// # Behavior
///
/// The way an incoming connection is detected varies depending on the underlying protocol type.
///
/// # Stream
///
/// Stream based listeners detect incoming streams created by remote sockets which target the
/// address of the listener.
///
/// # Datagram
///
/// Datagram based listeners detect connections based on the source address of incoming packets.
/// When accepting a connection, the received datagram will already be present in the [`Receiver`].
pub trait Listener {
    /// Type of the connection's sender half.
    type Sender: Sender + 'static;
    /// Type of the connection's receiver half.
    type Receiver: Receiver + 'static;

    /// Accepts an incoming connection, and returns its source address.
    ///
    /// The connection is split into sending and receiving halves.
    ///
    /// Returns [`None`] if the listener is closed.
    #[allow(async_fn_in_trait)]
    async fn accept(&mut self) -> Option<((Self::Sender, Self::Receiver), SocketAddr)>;
}

/// Underlying protocol type.
///
/// This is used to account for differences in individual protocol behavior, mainly to do with
/// connection management, which might need to be accounted for by the user of the socket.
pub enum ProtocolType {
    /// Stream oriented communication.
    ///
    /// These types of sockets provide sequenced, reliable, two-way, connection-based byte streams.
    Stream,
    /// Datagram oriented communication.
    ///
    /// These types of sockets support datagrams (connectionless, unreliable messages of a fixed
    /// maximum length).
    Datagram(u32),
}
