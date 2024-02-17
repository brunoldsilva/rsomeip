//! Abstractions over TCP and UDP communication primitives.

use crate::net::{
    util::{response_channels, ResponseSender},
    IoResult, SocketAddr,
};
use std::{io, sync::Arc};
use tokio::sync::mpsc;

#[cfg(test)]
mod tests;

mod tcp;
mod udp;

/// Data addressed to a given target.
pub type Packet = (SocketAddr, Arc<[u8]>);

/// An abstraction over the underlying communication protocol.
///
/// Uses message passing to provide control over an asynchronous resource, such as a TCP or UDP
/// socket, ensuring thread-safety while decoupling from concrete protocol implementations.
///
/// A [`Socket`] is essentially just a wrapper around a channel of [`Message`]s with some
/// convenient methods for sending requests and getting the results. The actual communication
/// behavior is handled by the actor at the other end of the channel, which receives the messages
/// and performs some operation based on the kind of message it received.
#[derive(Debug)]
pub struct Socket {
    messages: mpsc::Sender<Message>,
}

impl Socket {
    /// Creates a new [`Socket`].
    pub fn new(messages: mpsc::Sender<Message>) -> Self {
        Self { messages }
    }

    /// Creates a new UDP [`Socket`], and a [`Packet`] receiver.
    ///
    /// # Errors
    ///
    /// Returns an error if the UDP socket cannot bind to the given `address`.
    pub async fn udp(address: SocketAddr) -> IoResult<(Self, mpsc::Receiver<Packet>)> {
        let (tx_messages, rx_messages) = mpsc::channel(32);
        let (mut socket, packets) = udp::Socket::bind(address).await?;
        tokio::spawn(async move { socket.process(rx_messages).await });
        Ok((Self::new(tx_messages), packets))
    }

    /// Binds a TCP [`Socket`] to the given address.
    pub fn tcp(address: SocketAddr) -> (Self, mpsc::Receiver<Packet>) {
        let (tx_messages, rx_messages) = mpsc::channel(32);
        let (tx_packets, rx_packets) = mpsc::channel(32);
        tcp::bind(address, (tx_packets, rx_messages));
        (Self::new(tx_messages), rx_packets)
    }

    /// Sends the `data` to the given `address`.
    ///
    /// Depending on the communication protocol that is being used, a connection might need to be
    /// established before sending the data.
    ///
    /// # Errors
    ///
    /// Returns an error if the socket is unavailable, or if the socket was unable to send
    /// the data.
    pub async fn send(&self, address: SocketAddr, data: Arc<[u8]>) -> IoResult<()> {
        self.send_message(Operation::Send((address, data))).await
    }

    /// Establishes a connection to the target at the given `address`.
    ///
    /// Some communication protocols do not have the concept of *connections*. In that case,
    /// `connect()` will have no effect on the socket, and will not result in an error.
    ///
    /// # Errors
    ///
    /// Returns an error if the socket is unavailable, or if a connection cannot be established.
    pub async fn connect(&self, address: SocketAddr) -> IoResult<()> {
        self.send_message(Operation::Connect(address)).await
    }

    /// Closes the connection to the target at the given `address`.
    ///
    /// Trying to close a connection which does not exist will not result in an error.
    ///
    /// # Errors
    ///
    /// Returns an error if the socket is unavailable, or if the connection cannot be closed.
    // TODO(brunoldsilva): Need to check if the second part of the last statement makes sense.
    pub async fn disconnect(&self, address: SocketAddr) -> IoResult<()> {
        self.send_message(Operation::Disconnect(address)).await
    }

    /// Opens the socket to new incoming connections.
    ///
    /// This allows remote clients to establish connections to the socket, instead of the socket
    /// having to establish the connection itself.
    ///
    /// # Errors
    ///
    /// Returns an error if the socket is unavailable, or if the socket cannot be opened.
    pub async fn open(&self) -> IoResult<()> {
        self.send_message(Operation::Open).await
    }

    /// Closes the socket to new incoming connections.
    ///
    /// This prevents remote clients from establishing connections to the socket, but does not
    /// terminate already establish connections.
    ///
    /// # Errors
    ///
    /// Returns an error if the socket is unavailable, or if the socket cannot be closed.
    pub async fn close(&self) -> IoResult<()> {
        self.send_message(Operation::Close).await
    }

    /// Sends a message through the message channel, and waits for the response.
    ///
    /// # Errors
    ///
    /// Returns an error if the channel is closed, or if the response is itself an error.
    async fn send_message(&self, kind: Operation) -> IoResult<()> {
        let (sender, receiver) = response_channels();
        let message = Message::new(kind, sender);
        self.messages
            .send(message)
            .await
            .map_err(|_| io::Error::from(io::ErrorKind::NotConnected))?;
        receiver
            .get()
            .await
            .unwrap_or_else(|| Err(io::Error::from(io::ErrorKind::NotConnected)))
    }
}

/// Encapsulates a request for an operation to be performed with the means to send back the result.
///
/// Sending these messages through a channel provides a way to safely request some operation to be
/// performed, and the result of said operation to be sent back to the caller asynchronously.
#[derive(Debug)]
pub struct Message {
    operation: Operation,
    response: ResponseSender<(), io::Error>,
}

impl Message {
    /// Creates a new [`Message`], and provides a receiver for the response.
    pub fn new(operation: Operation, response: ResponseSender<(), io::Error>) -> Self {
        Self {
            operation,
            response,
        }
    }

    /// Consumes the [`Message`], returning the operation and the response channel.
    pub fn into_parts(self) -> (Operation, ResponseSender<(), io::Error>) {
        (self.operation, self.response)
    }
}

/// Operations that can be performed by a socket.
///
/// These are purposefully generic and designed to accommodate a fair number o communication
/// protocol behaviors.
///
/// The `Send` operation is the only one that is supported by all protocols. All other operations
/// may or may not be implemented, depending on the capabilities of the communication protocols
/// that are being used. UDP, for example, does not have the concept of connections, so the
/// `Connect` and `Disconnect` operations do not make sense.
///
/// In the case that some operation cannot be implemented, the default behavior of the socket is to
/// ignore the request and send back an `Ok` result, as long as this does not affect the socket.
/// Otherwise, an appropriate error result should be used.
#[derive(Debug, PartialEq, Eq)]
pub enum Operation {
    /// Send the data to the provided address.
    Send(Packet),
    /// Establish a connection to the provided address.
    Connect(SocketAddr),
    /// Close the connection the provided address.
    Disconnect(SocketAddr),
    /// Accept connections from remote addresses.
    Open,
    /// Refuse connections from remote addresses.
    Close,
}
