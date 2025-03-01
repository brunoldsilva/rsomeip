#![allow(clippy::module_name_repetitions)]
//! Transmission Control Protocol.
//!
//! TCP is a connection-oriented, reliable, ordered, and error-checked communication protocol of
//! the Internet Protocol suite.
//!
//! This module provides concrete implementations of the [`socket`] traits for the TCP protocol:
//!
//! [`TcpSocket`] implements the [`Connector`] trait for establishing connections between TCP
//! sockets.
//!
//! [`TcpSender`] and [`TcpReceiver`] implement the [`Sender`] and [`Receiver`] traits,
//! respectively. Together, they represent a connection between two sockets capable of sending and
//! receiving data.
//!
//! [`TcpListener`] implements the [`Listener`] trait for accepting incoming TCP connections.
//!
//! [`socket`]: crate::socket

use crate::socket::{
    Connector, IoResult, Listener, Receiver, RecvError, SendError, Sender, SocketAddr,
};
use bytes::Bytes;
use tokio::net;

use super::Type;

/// A socket of the TCP protocol.
///
/// Implements the [`Connector`] trait for establishing connections to other TCP sockets.
///
/// # Examples
///
/// Establishing connections:
///
/// ```rust
/// # tokio_test::block_on(async move {
/// use rsomeip::socket::{SocketAddr, Connector, Sender, tcp::TcpSocket};
/// use bytes::Bytes;
/// let local_address: SocketAddr = "127.0.0.1:30501".parse().unwrap();
/// let mut socket = TcpSocket::new(local_address);
///
/// let peer_address: SocketAddr = "127.0.0.2:30501".parse().unwrap();
/// # let peer = tokio::net::TcpSocket::new_v4().unwrap();
/// # peer.set_reuseaddr(true).unwrap();
/// # peer.bind(peer_address).unwrap();
/// # let listener = peer.listen(1).unwrap();
/// # tokio::spawn(async move { listener.accept().await.unwrap(); });
/// let (mut sender, receiver) = socket.connect(&peer_address).await.unwrap();
/// sender.send(Bytes::copy_from_slice(&[1u8])).await.unwrap();
/// # });
/// ```
///
/// Accepting connections:
///
/// ```rust
/// # tokio_test::block_on(async move {
/// use rsomeip::socket::{SocketAddr, Connector, Receiver, Listener, tcp::TcpSocket};
/// use bytes::Bytes;
/// let local_address: SocketAddr = "127.0.0.1:30502".parse().unwrap();
/// let mut socket = TcpSocket::new(local_address);
///
/// let mut listener = socket.listen(1).await.unwrap();
/// # let peer_address: SocketAddr = "127.0.0.2:30502".parse().unwrap();
/// # let peer = tokio::net::TcpSocket::new_v4().unwrap();
/// # peer.set_reuseaddr(true).unwrap();
/// # peer.bind(peer_address).unwrap();
/// # tokio::spawn(async move {
/// #   let stream = peer.connect(local_address).await.unwrap();
/// #   stream.writable().await.unwrap();
/// #   stream.try_write(&[1u8]).unwrap();
/// # });
/// let ((sender, mut receiver), address) = listener.accept().await.unwrap();
/// # assert_eq!(address, peer_address);
/// let data = receiver.recv().await.unwrap();
/// # assert_eq!(&data[..], &[1u8][..]);
/// # });
/// ```
#[derive(Debug)]
pub struct TcpSocket {
    /// Source address for establishing connections and binding listeners.
    address: SocketAddr,
}

impl TcpSocket {
    /// Creates a new [`TcpSocket`].
    pub fn new(address: SocketAddr) -> Self {
        Self { address }
    }
}

impl Connector for TcpSocket {
    type Sender = TcpSender;
    type Receiver = TcpReceiver;
    type Listener = TcpListener;

    const PROTOCOL_TYPE: Type = Type::Stream;

    async fn connect(&mut self, address: &SocketAddr) -> IoResult<(Self::Sender, Self::Receiver)> {
        let socket = match self.address {
            SocketAddr::V4(_) => net::TcpSocket::new_v4(),
            SocketAddr::V6(_) => net::TcpSocket::new_v6(),
        }?;
        socket.set_reuseaddr(true)?;
        socket.bind(self.address)?;
        let (receiver, sender) = socket.connect(*address).await?.into_split();
        Ok((TcpSender { inner: sender }, TcpReceiver::new(receiver)))
    }

    async fn listen(&mut self, backlog: u32) -> IoResult<Self::Listener> {
        let socket = match self.address {
            SocketAddr::V4(_) => net::TcpSocket::new_v4(),
            SocketAddr::V6(_) => net::TcpSocket::new_v6(),
        }?;
        socket.set_reuseaddr(true)?;
        socket.bind(self.address)?;
        let listener = socket.listen(backlog)?;
        Ok(TcpListener { inner: listener })
    }
}

/// The sender half of a TCP connection.
///
/// Implements the [`Sender`] trait for sending data to the connected socket.
#[derive(Debug)]
pub struct TcpSender {
    inner: net::tcp::OwnedWriteHalf,
}

impl Sender for TcpSender {
    async fn send(&mut self, buffer: Bytes) -> Result<(), SendError<Bytes>> {
        let mut sent = 0;
        loop {
            match self.inner.writable().await {
                Ok(()) => {
                    match self.inner.try_write(&buffer[sent..]) {
                        Ok(size) => sent += size,
                        Err(_) => return Err(SendError(buffer)),
                    }
                    if sent < buffer.len() {
                        continue;
                    }
                    return Ok(());
                }
                Err(ref error) if error.kind() == std::io::ErrorKind::WouldBlock => continue,
                Err(_) => return Err(SendError(buffer)),
            }
        }
    }
}

/// The receiver half of a TCP connection.
///
/// Implements the [`Receiver`] trait for receiving data from the connected socket.
#[derive(Debug)]
pub struct TcpReceiver {
    inner: net::tcp::OwnedReadHalf,
    buffer: Box<[u8]>,
}

impl TcpReceiver {
    /// Creates a new [`TcpReceiver`].
    pub fn new(receiver: net::tcp::OwnedReadHalf) -> Self {
        Self {
            inner: receiver,
            buffer: Box::new([0u8; 1400]),
        }
    }
}

impl Receiver for TcpReceiver {
    async fn recv(&mut self) -> Result<Bytes, RecvError> {
        loop {
            let result = match self.inner.readable().await {
                Ok(()) => self.inner.try_read(&mut self.buffer[..]).map_or_else(
                    |_| Err(RecvError::Closed),
                    |size| Ok(Bytes::copy_from_slice(&self.buffer[..size])),
                ),
                Err(ref error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    // False positive. Socket is not readable.
                    continue;
                }
                Err(_) => Err(RecvError::Closed),
            };
            return result;
        }
    }
}

/// Listens for incoming connections.
///
/// Implements the [`Listener`] trait for accepting incoming connections to the socket.
#[derive(Debug)]
pub struct TcpListener {
    inner: net::TcpListener,
}

impl Listener for TcpListener {
    type Sender = TcpSender;
    type Receiver = TcpReceiver;

    async fn accept(&mut self) -> Option<((Self::Sender, Self::Receiver), SocketAddr)> {
        let (stream, address) = self.inner.accept().await.ok()?;
        let (receiver, sender) = stream.into_split();
        Some((
            (TcpSender { inner: sender }, TcpReceiver::new(receiver)),
            address,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::ipv4;
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn socket_connects_to_socket() {
        // Create a TCP socket.
        let local_address = ipv4!([127, 0, 0, 1]);
        let mut socket = TcpSocket::new(local_address);

        // Create a remote receiver.
        let peer_address = ipv4!([127, 0, 0, 2]);
        let mut receiver = listen(peer_address);

        // Connect to the remote receiver.
        let _ = socket
            .connect(&peer_address)
            .await
            .expect("should connect to the peer");
        receiver
            .recv()
            .await
            .expect("should receive a connection")
            .expect("should have connected successfully");
    }

    #[tokio::test]
    async fn socket_connects_to_multiple_sockets() {
        // Create a TCP socket.
        let local_address = ipv4!([127, 0, 0, 1]);
        let mut socket = TcpSocket::new(local_address);

        // Create multiple remote receivers.
        let peer_addresses = [ipv4!([127, 0, 0, 2]), ipv4!([127, 0, 0, 3])];
        let mut receivers: [_; 2] = std::array::from_fn(|i| listen(peer_addresses[i]));

        // Connect to each remote socket.
        for (i, address) in peer_addresses.iter().enumerate() {
            let _ = socket
                .connect(address)
                .await
                .expect("should connect to the peer");
            receivers[i]
                .recv()
                .await
                .expect("should receive a connection")
                .expect("should have connected successfully");
        }
    }

    #[tokio::test]
    async fn connection_sends_data() {
        // Establish a TCP connection.
        let (local_address, peer_address) = (ipv4!([127, 0, 0, 1]), ipv4!([127, 0, 0, 2]));
        let mut socket = TcpSocket::new(local_address);
        let ((mut sender, _), stream) = connect(&mut socket, peer_address).await;

        // Send data through the connection.
        let data = Bytes::copy_from_slice(&[1u8]);
        sender
            .send(data.clone())
            .await
            .expect("should send the data");

        // Check if the data was received.
        stream.readable().await.expect("should become readable");
        let mut buffer = [0u8];
        let size = stream
            .try_read(&mut buffer)
            .expect("should be able to read the data");
        assert_eq!(size, 1);
        assert_eq!(&buffer[..], &data[..]);
    }

    #[tokio::test]
    async fn connection_receives_data() {
        // Establish a TCP connection.
        let (local_address, peer_address) = (ipv4!([127, 0, 0, 1]), ipv4!([127, 0, 0, 2]));
        let mut socket = TcpSocket::new(local_address);
        let ((_, mut receiver), stream) = connect(&mut socket, peer_address).await;

        // Send data through the stream.
        stream.writable().await.expect("should become writable");
        let data = [1u8];
        stream.try_write(&data).expect("should write the data");

        // Check if the data was received.
        let buffer = receiver.recv().await.expect("should receive the data");
        assert_eq!(&buffer[..], &data[..]);
    }

    #[tokio::test]
    async fn socket_creates_listener() {
        // Create a TCP socket.
        let local_address = ipv4!([127, 0, 0, 1]);
        let mut socket = TcpSocket::new(local_address);

        // Create a listener.
        let _ = socket.listen(1).await.expect("should create a listener");
    }

    #[tokio::test]
    async fn listener_accepts_connections() {
        // Create a TCP socket.
        let local_address = ipv4!([127, 0, 0, 1]);
        let mut socket = TcpSocket::new(local_address);

        // Create a listener.
        let mut listener = socket.listen(1).await.expect("should create a listener");

        // Connect to the listener.
        let peer_address = ipv4!([127, 0, 0, 2]);
        let connection = {
            let socket = net::TcpSocket::new_v4().expect("should create a socket");
            socket
                .bind(peer_address)
                .expect("should bind to the address");
            tokio::spawn(async move {
                socket
                    .connect(local_address)
                    .await
                    .expect("should connect to the socket")
            })
        };

        // Check if the connection was established.
        let (_, address) = listener.accept().await.expect("should accept a connection");
        assert_eq!(address, peer_address);
        connection.await.expect("should complete successfully");
    }

    /// Establishes a connection between the socket and a remote address.
    ///
    /// Returns the connection and the remote stream.
    async fn connect(
        socket: &mut TcpSocket,
        peer_address: SocketAddr,
    ) -> ((TcpSender, TcpReceiver), net::TcpStream) {
        // Listen for a connection at the peer address.
        let mut receiver = listen(peer_address);

        // Connect to the peer address.
        let connection = socket
            .connect(&peer_address)
            .await
            .expect("should connect to the peer");

        // Get the peer's stream.
        let (stream, _) = receiver
            .recv()
            .await
            .expect("should receive a connection")
            .expect("should connect successfully");

        (connection, stream)
    }

    /// Listens for and accepts TCP streams to the given address.
    fn listen(address: SocketAddr) -> mpsc::Receiver<IoResult<(net::TcpStream, SocketAddr)>> {
        // Create a TCP socket.
        let socket = net::TcpSocket::new_v4().expect("should create a socket");
        socket.bind(address).expect("should bind to peer address");

        // Create a listener.
        let listener = socket.listen(1).expect("should create a listener");

        // Listen for incoming connections.
        let (sender, receiver) = tokio::sync::mpsc::channel(1);
        tokio::spawn(async move {
            while matches!(sender.send(listener.accept().await).await, Ok(())) {}
        });
        receiver
    }
}
