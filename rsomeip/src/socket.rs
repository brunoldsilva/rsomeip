//! Socket abstraction layer.

use crate::net::{util::Buffer, IoResult, SocketAddr};

/// Establishes connections to remote sockets, and creates listeners for incoming
/// connections.
#[allow(async_fn_in_trait)]
pub trait Socket: Send {
    /// Establishes a [`Connection`] from the socket to the given address.
    async fn connect(&self, address: SocketAddr) -> IoResult<impl Connection>;

    /// Creates a [`Listener`] for incoming remote connections to the socket.
    async fn listen(&self) -> IoResult<impl Listener>;
}

/// Sends and receives data from the connected socket.
#[allow(async_fn_in_trait)]
pub trait Connection: Sender + Receiver {
    /// Splits the connection into a [`Sender`] and [`Receiver`] pair.
    fn split(self) -> (impl Sender, impl Receiver);
}

/// Sends data to the remote socket.
#[allow(async_fn_in_trait)]
pub trait Sender: Send {
    /// Sends data to the remote socket.
    async fn send(&self, value: Buffer) -> SendResult<Buffer>;
}

/// Receives data from the remote socket.
#[allow(async_fn_in_trait)]
pub trait Receiver: Send {
    /// Receives data from the remote socket.
    async fn recv(&mut self) -> Option<Buffer>;
}

/// Listens for connections from remote sockets.
#[allow(async_fn_in_trait)]
pub trait Listener: Send {
    /// Accepts a connection from a remote socket.
    async fn accept(&mut self) -> Option<(impl Connection, SocketAddr)>;
}

/// Represents either a success ([`Ok`]) or failure ([`Err`]) when sending data.
pub type SendResult<T> = Result<(), SendError<T>>;

/// Represents an error that occurred while sending data.
#[derive(Debug)]
pub struct SendError<T> {
    /// The value that failed to be sent.
    pub value: T,
    /// The error that occurred.
    pub error: std::io::Error,
}

impl<T> SendError<T> {
    /// Creates a new [`SendError<T>`].
    pub fn new(value: T, error: std::io::Error) -> Self {
        Self { value, error }
    }
}

pub mod tcp;

pub mod udp;
