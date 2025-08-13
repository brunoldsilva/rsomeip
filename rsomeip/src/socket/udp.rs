#![allow(clippy::module_name_repetitions)]
//! User Datagram Protocol.
//!
//! UDP is a connection-less, datagram-based communication protocol of the Internet Protocol suite
//! designed for speed and efficiency.
//!
//! This module provides concrete implementations of the [`socket`] traits for the UDP protocol:
//!
//! [`UdpSocket`] implements the [`Connector`] trait for establishing connections between UDP
//! sockets.
//!
//! [`UdpSender`] and [`UdpReceiver`] implement the [`Sender`] and [`Receiver`] traits,
//! respectively. Together, they represent a connection between two sockets capable of sending and
//! receiving data.
//!
//! [`UdpListener`] implements the [`Listener`] trait for accepting incoming UDP connections.
//!
//! [`socket`]: crate::socket

use crate::{
    socket::{Connector, IoResult, Listener, Receiver, RecvError, SendError, Sender, SocketAddr},
    support::collections::SharedMap,
};
use rsomeip_bytes::Bytes;
use std::{
    net::Ipv4Addr,
    sync::{Arc, Mutex},
};
use tokio::{
    net,
    sync::{broadcast, mpsc},
};
use tokio_util::sync::CancellationToken;

use super::ProtocolType;

/// A thread-safe optional value.
type SharedOption<T> = Arc<Mutex<Option<T>>>;
/// Sends connections to the associated receiver.
type ConnectionSender = mpsc::Sender<((UdpSender, UdpReceiver), SocketAddr)>;

/// Max size of an UDP datagram in bytes.
pub const MAX_DATAGRAM_SIZE: u32 = 1400;

/// A socket of the UDP protocol.
///
/// Implements the [`Connector`] trait for establishing connections to other UDP sockets.
///
/// # Examples
///
/// Establishing connections:
///
/// ```rust
/// # tokio_test::block_on(async move {
/// use rsomeip::socket::{SocketAddr, Connector, Sender, udp::UdpSocket};
/// use rsomeip_bytes::Bytes;
/// let local_address: SocketAddr = "127.0.0.1:30503".parse().unwrap();
/// let mut socket = UdpSocket::bind(local_address).await.unwrap();
///
/// let peer_address: SocketAddr = "127.0.0.2:30503".parse().unwrap();
/// let (mut sender, receiver) = socket.connect(&peer_address).await.unwrap();
/// sender.send(Bytes::copy_from_slice(&[1u8])).await.unwrap();
/// # });
/// ```
///
/// Accepting connections:
///
/// ```rust
/// # tokio_test::block_on(async move {
/// use rsomeip::socket::{SocketAddr, Connector, Receiver, Listener, udp::UdpSocket};
/// use rsomeip_bytes::Bytes;
/// let local_address: SocketAddr = "127.0.0.1:30504".parse().unwrap();
/// let mut socket = UdpSocket::bind(local_address).await.unwrap();
///
/// let mut listener = socket.listen(1).await.unwrap();
/// # let peer_address: SocketAddr = "127.0.0.2:30504".parse().unwrap();
/// # let peer = tokio::net::UdpSocket::bind(&peer_address).await.unwrap();
/// # peer.send_to(&[1u8], local_address).await.unwrap();
/// let ((sender, mut receiver), address) = listener.accept().await.unwrap();
/// # assert_eq!(address, peer_address);
/// let data = receiver.recv().await.unwrap();
/// # assert_eq!(&data[..], &[1u8][..]);
/// # });
/// ```
pub struct UdpSocket {
    // UDP socket for sending and receiving data.
    socket: Arc<net::UdpSocket>,
    // Receivers of data from remote sockets.
    receivers: SharedMap<SocketAddr, broadcast::Sender<Bytes>>,
    // Listener for incoming connections.
    listener: SharedOption<ConnectionSender>,
    // Cancellation token bound to all connections of the socket.
    token: CancellationToken,
}

impl UdpSocket {
    /// Binds a [`UdpSocket`] to the given address.
    ///
    /// If it's a multicast address, the socket will also join the multicast group.
    ///
    /// # Errors
    ///
    /// Returns an error if the address is already in use. In the case of multicast addresses, this
    /// also returns an error if the multicast group cannot be joined.
    pub async fn bind(address: SocketAddr) -> IoResult<Self> {
        let socket = Arc::new(net::UdpSocket::bind(address).await?);
        if address.ip().is_multicast() {
            match address {
                SocketAddr::V4(address) => {
                    socket.join_multicast_v4(*address.ip(), Ipv4Addr::UNSPECIFIED)?;
                }
                SocketAddr::V6(address) => socket.join_multicast_v6(address.ip(), 0)?,
            };
        }
        let receivers = SharedMap::default();
        let listener = SharedOption::<ConnectionSender>::default();
        let token = CancellationToken::new();
        ReceiverTask::spawn(
            socket.clone(),
            receivers.clone(),
            listener.clone(),
            token.child_token(),
        );
        Ok(Self {
            socket,
            receivers,
            listener,
            token,
        })
    }
}

impl Connector for UdpSocket {
    type Sender = UdpSender;
    type Receiver = UdpReceiver;
    type Listener = UdpListener;

    const PROTOCOL_TYPE: ProtocolType = ProtocolType::Datagram(MAX_DATAGRAM_SIZE);

    async fn connect(&mut self, address: &SocketAddr) -> IoResult<(Self::Sender, Self::Receiver)> {
        let (sender, receiver) = broadcast::channel(8);
        self.receivers.insert(*address, sender);
        Ok((
            UdpSender {
                socket: self.socket.clone(),
                peer_address: *address,
                token: self.token.child_token(),
            },
            UdpReceiver { inner: receiver },
        ))
    }

    async fn listen(&mut self, backlog: u32) -> IoResult<Self::Listener> {
        let (sender, receiver) = mpsc::channel(backlog as usize);
        {
            #[allow(clippy::expect_used)]
            let mut listener = self.listener.lock().expect("mutex should not be poisoned");
            let _ = listener.insert(sender);
        }
        Ok(UdpListener { inner: receiver })
    }
}

/// The sender half of an UDP connection.
///
/// Implements the [`Sender`] trait for sending data to the connected socket.
#[derive(Debug)]
pub struct UdpSender {
    // Socket for sending data.
    socket: Arc<net::UdpSocket>,
    // Address to which data is sent.
    peer_address: SocketAddr,
    // Cancellation token bound to the socket.
    token: CancellationToken,
}

impl Sender for UdpSender {
    async fn send(&mut self, buffer: Bytes) -> Result<(), SendError<Bytes>> {
        if buffer.len() > (MAX_DATAGRAM_SIZE as usize) || self.token.is_cancelled() {
            return Err(SendError(buffer));
        }
        self.socket
            .send_to(&buffer[..], self.peer_address)
            .await
            .map_or_else(|_| Err(SendError(buffer)), |_| Ok(()))
    }
}

/// The receiver half of an UDP connection.
///
/// Implements the [`Receiver`] trait for receiving data from the connected socket.
#[derive(Debug)]
pub struct UdpReceiver {
    inner: broadcast::Receiver<Bytes>,
}

impl Receiver for UdpReceiver {
    async fn recv(&mut self) -> Result<Bytes, RecvError> {
        self.inner.recv().await.map_err(|error| match error {
            broadcast::error::RecvError::Closed => RecvError::Closed,
            broadcast::error::RecvError::Lagged(count) => RecvError::Lagged(count),
        })
    }
}

/// Forwards incoming packets to each receiver.
#[derive(Debug)]
struct ReceiverTask {
    // UDP socket for sending and receiving data.
    socket: Arc<net::UdpSocket>,
    // Receivers of data from remote sockets.
    receivers: SharedMap<SocketAddr, broadcast::Sender<Bytes>>,
    // Listener for incoming connections.
    listener: SharedOption<ConnectionSender>,
    // Cancellation token bound to all connections of the socket.
    token: CancellationToken,
    // Internal data storage.
    buffer: Box<[u8]>,
}

impl ReceiverTask {
    /// Creates a new [`ReceiverTask`].
    fn new(
        socket: Arc<net::UdpSocket>,
        receivers: SharedMap<SocketAddr, broadcast::Sender<Bytes>>,
        listener: SharedOption<ConnectionSender>,
        token: CancellationToken,
    ) -> Self {
        Self {
            socket,
            receivers,
            listener,
            token,
            buffer: Box::new([0u8; 1400]),
        }
    }

    /// Spawn a new [`ReceiverTask`] and runs it asynchronously.
    ///
    /// The task will run until the socket is closed.
    fn spawn(
        socket: Arc<net::UdpSocket>,
        receivers: SharedMap<SocketAddr, broadcast::Sender<Bytes>>,
        listener: SharedOption<ConnectionSender>,
        token: CancellationToken,
    ) {
        let mut task = Self::new(socket, receivers, listener, token.clone());
        tokio::spawn(async move {
            tokio::select! {
                () = token.cancelled() => {},
                () = task.run() => {}
            };
        });
    }

    /// Reads messages from the socket and sends them to the correct receiver.
    async fn run(&mut self) {
        while let Ok((size, address)) = self.socket.recv_from(&mut self.buffer[..]).await {
            let packet = Bytes::copy_from_slice(&self.buffer[..size]);
            self.process_packet(packet, address);
        }
    }

    /// Forwards a packet to the correct receiver.
    ///
    /// Creates a new connection if no receiver is found and the listener is active.
    fn process_packet(&self, packet: Bytes, address: SocketAddr) {
        match self.receivers.get(&address) {
            Some(receiver) => match receiver.send(packet) {
                Ok(_) => (),
                Err(broadcast::error::SendError(packet)) => {
                    self.receivers.remove(&address);
                    self.process_new_connection(packet, address);
                }
            },
            None => self.process_new_connection(packet, address),
        }
    }

    /// Creates a new connection if the listener is active.
    ///
    /// New connections will include the given packet.
    fn process_new_connection(&self, packet: Bytes, address: SocketAddr) {
        #[allow(clippy::expect_used)]
        let mut listener = self.listener.lock().expect("mutex should not be poisoned");
        if let Some(ref connection) = *listener {
            let udp_sender = UdpSender {
                socket: self.socket.clone(),
                peer_address: address,
                token: self.token.clone(),
            };
            let (sender, receiver) = broadcast::channel(32);
            let udp_receiver = UdpReceiver { inner: receiver };
            match connection.try_send(((udp_sender, udp_receiver), address)) {
                Ok(()) => {
                    let _ = sender.send(packet);
                    self.receivers.insert(address, sender);
                }
                Err(mpsc::error::TrySendError::Full(_)) => {}
                Err(mpsc::error::TrySendError::Closed(_)) => {
                    *listener = None;
                }
            }
        }
    }
}

/// Listens for incoming connections.
///
/// Implements the [`Listener`] trait for accepting incoming connections to the socket.
#[derive(Debug)]
pub struct UdpListener {
    inner: mpsc::Receiver<((UdpSender, UdpReceiver), SocketAddr)>,
}

impl Listener for UdpListener {
    type Sender = UdpSender;
    type Receiver = UdpReceiver;

    async fn accept(&mut self) -> Option<((Self::Sender, Self::Receiver), SocketAddr)> {
        self.inner.recv().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::ipv4;
    use rsomeip_bytes::BytesMut;

    #[tokio::test]
    async fn socket_binds_to_unicast() {
        let address = ipv4!([127, 0, 0, 1]);
        let _ = UdpSocket::bind(address)
            .await
            .expect("should bind to unicast address");
    }

    #[tokio::test]
    async fn socket_binds_to_multicast() {
        let address = ipv4!([224, 0, 0, 1]);
        let _ = UdpSocket::bind(address)
            .await
            .expect("should bind to unicast address");
    }

    #[tokio::test]
    async fn connection_sends_data_to_unicast() {
        // Create an UdpSocket.
        let local_address = ipv4!();
        let mut socket = UdpSocket::bind(local_address)
            .await
            .expect("should bind to address");

        // Create a Udp receiver.
        let peer_address = ipv4!();
        let peer = net::UdpSocket::bind(peer_address)
            .await
            .expect("should bind to address");

        // Connect to the peer.
        let (mut sender, _) = socket
            .connect(&peer_address)
            .await
            .expect("should connect to the peer");

        // Send data to the peer.
        let raw_data = [1u8];
        let data = BytesMut::from(&raw_data[..]);
        sender
            .send(data.into())
            .await
            .expect("should send the data");

        // Check if the peer received the data.
        let mut buffer = [0u8];
        let (size, address) = peer
            .recv_from(&mut buffer)
            .await
            .expect("should receive the data");
        assert_eq!(size, 1);
        assert_eq!(&raw_data, &buffer);
        assert_eq!(address, local_address);
    }

    #[tokio::test]
    async fn connection_sends_data_to_multicast() {
        // Create an UdpSocket.
        let local_address = ipv4!([127, 0, 0, 1]);
        let mut socket = UdpSocket::bind(local_address)
            .await
            .expect("should bind to address");

        // Create a multicast Udp receiver.
        let peer_address = ipv4!([224, 0, 0, 1]);
        let peer = net::UdpSocket::bind(peer_address)
            .await
            .expect("should bind to address");
        peer.join_multicast_v4(Ipv4Addr::new(224, 0, 0, 1), Ipv4Addr::UNSPECIFIED)
            .expect("should join the multicast group");

        // Connect to the peer.
        let (mut sender, _) = socket
            .connect(&peer_address)
            .await
            .expect("should connect to the peer");

        // Send data to the peer.
        let raw_data = [1u8];
        let data = BytesMut::from(&raw_data[..]);
        sender
            .send(data.into())
            .await
            .expect("should send the data");

        // Check if the peer received the data.
        let mut buffer = [0u8];
        let (size, address) = peer
            .recv_from(&mut buffer)
            .await
            .expect("should receive the data");
        assert_eq!(size, 1);
        assert_eq!(&raw_data, &buffer);
        assert_eq!(address, local_address);
    }

    #[tokio::test]
    async fn connection_receives_data_from_unicast() {
        // Create an UdpSocket.
        let local_address = ipv4!([127, 0, 0, 1]);
        let mut socket = UdpSocket::bind(local_address)
            .await
            .expect("should bind to the address");

        // Create a Udp sender.
        let peer_address = ipv4!([127, 0, 0, 2]);
        let peer = net::UdpSocket::bind(peer_address)
            .await
            .expect("should bind to the address");

        // Connect to the peer.
        let (_, mut receiver) = socket
            .connect(&peer_address)
            .await
            .expect("should connect to the peer");

        // Send data to the socket.
        let data = [1u8];
        peer.send_to(&data, local_address)
            .await
            .expect("should send the data");

        // Check if the socket received the data.
        let buffer = receiver.recv().await.expect("should received the data");
        assert_eq!(&data[..], &buffer[..]);
    }

    #[tokio::test]
    async fn connection_receives_data_from_multicast() {
        // Create a multicast UdpSocket.
        let multicast_address = ipv4!([224, 0, 0, 1]);
        let mut socket = UdpSocket::bind(multicast_address)
            .await
            .expect("should bind to the address");

        // Create a Udp sender.
        let peer_address = ipv4!([127, 0, 0, 2]);
        let peer = net::UdpSocket::bind(peer_address)
            .await
            .expect("should bind to the address");

        // Connect to the peer.
        let (_, mut receiver) = socket
            .connect(&peer_address)
            .await
            .expect("should connect to the peer");

        // Send data to the socket.
        let data = [1u8];
        peer.send_to(&data, multicast_address)
            .await
            .expect("should send the data");

        // Check if the socket received the data.
        let buffer = receiver.recv().await.expect("should received the data");
        assert_eq!(&data[..], &buffer[..]);
    }

    #[tokio::test]
    async fn listener_accepts_connections() {
        // Create an UdpSocket.
        let local_address = ipv4!([127, 0, 0, 1]);
        let mut socket = UdpSocket::bind(local_address)
            .await
            .expect("should bind to the address");

        // Create a listener.
        let mut listener = socket.listen(1).await.expect("should create a listener");

        // Create a Udp sender.
        let peer_address = ipv4!([127, 0, 0, 2]);
        let peer = net::UdpSocket::bind(peer_address)
            .await
            .expect("should bind to the address");

        // Send data to the socket.
        let data = [1u8];
        peer.send_to(&data, local_address)
            .await
            .expect("should send the data");

        // Accept the connection from the peer.
        let ((_, mut receiver), address) = listener
            .accept()
            .await
            .expect("should accept the connection");
        assert_eq!(address, peer_address);

        // Check if the connection contains the first message.
        let buffer = receiver.recv().await.expect("should received the data");
        assert_eq!(&data[..], &buffer[..]);
    }
}
