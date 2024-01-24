use crate::net::{
    socket::{Message, Operation, Packet},
    util::BufferPool,
};
use std::{
    collections::HashMap,
    io,
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    sync::{Arc, Mutex},
};
use tokio::{
    net::{TcpListener, TcpSocket, TcpStream},
    select,
    sync::{mpsc, oneshot},
};
use tokio_util::sync::CancellationToken;

/// Binds a TCP [`super::Socket`] to the given address.
///
/// It will start receiving [`Message`]s and sending [`Packet`]s to the given channels.
pub fn bind(address: SocketAddr, channels: SocketChannels) {
    let mut socket = Socket::new(address);
    tokio::spawn(async move {
        socket.process(channels).await;
    });
}

/// A TCP [`Socket`] implementation.
#[derive(Debug)]
struct Socket {
    address: SocketAddr,
    connections: SharedConnectionMap,
    listener: Option<Listener>,
}

impl Socket {
    /// Creates a new [`Socket`].
    #[must_use]
    fn new(address: SocketAddr) -> Self {
        Self {
            address,
            connections: SharedConnectionMap::new(),
            listener: None,
        }
    }

    /// Starts processing [`Message`]s from the given channel.
    async fn process(&mut self, channels: SocketChannels) {
        let (packets, mut messages) = channels;
        while let Some(Message {
            operation,
            response,
        }) = messages.recv().await
        {
            match operation {
                Operation::Connect(address) => {
                    self.connect(address, packets.clone(), response);
                }
                Operation::Disconnect(address) => self.disconnect(address, response),
                Operation::Open => self.open(packets.clone(), response),
                Operation::Close => self.close(response),
                Operation::Send(packet) => self.send(packet, response).await,
            };
        }
    }

    /// Establish a TCP connection from the [`Socket`] address to the given address.
    ///
    /// # Errors
    ///
    /// Returns an error if there is an issue with the TCP connection.
    fn connect(&mut self, address: SocketAddr, packets: mpsc::Sender<Packet>, response: Response) {
        if self.check(&address).is_some() {
            let _ = response.send(Ok(()));
            return;
        }
        let local_address = self.address;
        let (sender, receiver) = mpsc::channel(32);
        tokio::spawn(async move {
            let mut connection = match Connection::connect(local_address, address).await {
                Ok(connection) => {
                    let _ = response.send(Ok(()));
                    connection
                }
                Err(error) => {
                    let _ = response.send(Err(error.into()));
                    return;
                }
            };
            let _ = connection.process(packets, receiver).await;
        });
        self.connections.insert(address, sender);
    }

    /// Closes the TCP connection to the given address.
    ///
    /// # Errors
    ///
    /// Does not actually return an error.
    fn disconnect(&mut self, address: SocketAddr, response: Response) {
        let _ = self.connections.remove(&address);
        let _ = response.send(Ok(()));
    }

    /// Opens the [`Socket`] to incoming TCP connections.
    fn open(&mut self, packets: mpsc::Sender<Packet>, response: Response) {
        let address = self.address;
        let listener = Listener::new(self.connections.clone());
        self.listener = Some(listener.clone());
        tokio::spawn(async move {
            let _ = listener.open(address, packets, response).await;
        });
    }

    /// Closes the [`Socket`] to incoming TCP connections.
    fn close(&mut self, response: Response) {
        if let Some(ref listener) = self.listener {
            listener.close();
        }
        let _ = response.send(Ok(()));
    }

    /// Sends the packet to its intended target.
    ///
    /// # Errors
    ///
    /// Returns an error if the connection to the target is not established.
    async fn send(&mut self, packet: Packet, response: Response) {
        match self.check(&packet.0) {
            Some(connection) => {
                let _ = connection.send((packet, response)).await;
            }
            None => {
                let _ = response.send(Err(io::Error::from(io::ErrorKind::NotConnected).into()));
            }
        }
    }

    /// Returns the [`Connection`] channel for the given address.
    ///
    /// If the channel is already closed, it removes it and returns [`None`].
    fn check(&mut self, address: &SocketAddr) -> Option<mpsc::Sender<(Packet, Response)>> {
        match self.connections.get(address) {
            Some(connection) => {
                if connection.is_closed() {
                    let _ = self.connections.remove(address);
                    None
                } else {
                    Some(connection)
                }
            }
            None => None,
        }
    }
}

/// A TCP connection between two addresses.
#[derive(Debug)]
struct Connection {
    stream: TcpStream,
}

impl Connection {
    /// Creates a new [`Connection`].
    fn new(stream: TcpStream) -> Self {
        Self { stream }
    }

    /// Establish a TCP connection from the `src` address to the `dst`.
    ///
    /// # Errors
    ///
    /// Returns an error if there is an issue with the TCP connection.
    async fn connect(src: SocketAddr, dst: SocketAddr) -> IoResult<Self> {
        let socket = match src {
            SocketAddr::V4(_) => TcpSocket::new_v4(),
            SocketAddr::V6(_) => TcpSocket::new_v6(),
        }
        .and_then(|socket| {
            socket.bind(src)?;
            Ok(socket)
        })?;
        socket.connect(dst).await.map(|stream| Self { stream })
    }

    /// Send and receive packets to and from the [`Connection`] peer.
    ///
    /// # Errors
    ///
    /// Returns an error if there is an issue with the [`Connection`].
    async fn process(
        &mut self,
        sender: mpsc::Sender<Packet>,
        receiver: mpsc::Receiver<(Packet, Response)>,
    ) -> IoResult<()> {
        select! {
            result = self.send(receiver) => result,
            result = self.recv(sender) => result,
        }
    }

    /// Sends packets to the [`Connection`] peer.
    ///
    /// # Errors
    ///
    /// Returns an error if there is an issue with the [`Connection`].
    async fn send(&self, mut packets: mpsc::Receiver<(Packet, Response)>) -> IoResult<()> {
        while let Some(((dst, buffer), response)) = packets.recv().await {
            self.stream.writable().await?;
            match self.stream.try_write(buffer.as_ref()) {
                Ok(0) => {
                    println!("Reached end-of-file.");
                    let _ = response.send(Ok(()));
                    break;
                }
                Ok(len) => {
                    println!("Sent {len} bytes to {dst}.");
                    let _ = response.send(Ok(()));
                    continue;
                }
                Err(error) => {
                    eprintln!("Failed to send data to {dst}: {error}");
                    let _ = response.send(Ok(()));
                    return Err(error);
                }
            }
        }
        println!("Done receiving packets.");
        Ok(())
    }

    /// Receives packets from the [`Connection`] peer.
    ///
    /// # Errors
    ///
    /// Returns an error if there is an issue with the [`Connection`].
    async fn recv(&self, packets: mpsc::Sender<Packet>) -> IoResult<()> {
        let mut buffers = BufferPool::new();
        let mut buffer = buffers.pull_or_else(|| Arc::new([0u8; 1024]));
        let dst = self.peer_address();
        loop {
            self.stream.readable().await?;
            let Some(buffer_mut) = Arc::get_mut(&mut buffer) else {
                eprintln!("Failed to create a buffer.");
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    "buffer creation failed",
                ));
            };
            match self.stream.try_read(buffer_mut.as_mut()) {
                Ok(0) => {
                    println!("Reached end-of-file.");
                    break;
                }
                Ok(len) => {
                    println!("Received {len} bytes from {dst}.");
                }
                Err(ref error) if error.kind() == io::ErrorKind::WouldBlock => {
                    continue;
                }
                Err(error) => {
                    eprintln!("Failed to receive data from {dst}: {error}");
                    return Err(error);
                }
            }
            if packets.send((dst, buffer.clone())).await.is_err() {
                break;
            }
            buffers.push(buffer.clone());
            buffer = buffers.pull_or_else(|| Arc::new([0u8; 1024]));
        }
        println!("Done sending packets.");
        Ok(())
    }

    /// Returns the peer address of this [`Connection`].
    fn peer_address(&self) -> SocketAddr {
        self.stream
            .peer_addr()
            .unwrap_or_else(|_| SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0).into())
    }
}

/// A thread-safe [`ConnectionMap`].
#[derive(Debug, Default, Clone)]
struct SharedConnectionMap {
    inner: Arc<Mutex<ConnectionMap>>,
}

#[allow(clippy::expect_used)] // We expect to panic when we find a poisoned mutex.
impl SharedConnectionMap {
    /// Creates a new [`SharedConnectionMap`].
    fn new() -> Self {
        Self::default()
    }

    /// Inserts a connection into the map.
    fn insert(&self, address: SocketAddr, connection: mpsc::Sender<(Packet, Response)>) {
        let _ = self
            .inner
            .lock()
            .expect("mutex should not be poisoned")
            .insert(address, connection);
    }

    /// Returns the connection corresponding to the address, by cloning.
    fn get(&self, address: &SocketAddr) -> Option<mpsc::Sender<(Packet, Response)>> {
        self.inner
            .lock()
            .expect("mutex should not be poisoned")
            .get(address)
            .cloned()
    }

    /// Removes a connection from the map, returning that connection if it existed.
    fn remove(&self, address: &SocketAddr) -> Option<mpsc::Sender<(Packet, Response)>> {
        self.inner
            .lock()
            .expect("mutex should not be poisoned")
            .remove(address)
    }
}

/// A listener for incoming TCP connections.
#[derive(Debug, Clone)]
struct Listener {
    connections: SharedConnectionMap,
    token: CancellationToken,
}

impl Listener {
    /// Creates a new [`Listener`].
    fn new(connections: SharedConnectionMap) -> Self {
        Self {
            connections,
            token: CancellationToken::new(),
        }
    }

    /// Starts processing new TCP connections until the [`Listener`] is cancelled.
    ///
    /// This function will create a [`TcpListener`] at the given address, sending back the result
    /// through the [`Response`], and will start listening for incoming TCP connections. These
    /// connections will be added to the [`Listener`]s [`SharedConnectionMap`].
    ///
    /// Calling [`Listener::close`] on any clone of this [`Listener`] will cause it to stop
    /// listening for new connections, and for this function to return.
    ///
    /// # Errors
    ///
    /// Returns an error if there is an issue with the [`TcpListener`].
    async fn open(
        &self,
        address: SocketAddr,
        packets: mpsc::Sender<Packet>,
        response: Response,
    ) -> IoResult<()> {
        let listener = match TcpListener::bind(address).await {
            Ok(listener) => {
                let _ = response.send(Ok(()));
                listener
            }
            Err(error) => {
                let error_kind = error.kind();
                let _ = response.send(Err(error.into()));
                return Err(error_kind.into());
            }
        };
        select! {
            result = self.listen(listener, packets) => result,
            () = self.closed() => Ok(())
        }
    }

    /// Listens for incoming TCP connections.
    ///
    /// This function will wait on the given [`TcpListener`] to receive remote TCP connections,
    /// which will then be added to the [`SharedConnectionMap`] used by this [`Listener`].
    ///
    /// # Errors
    ///
    /// Returns an error if there is an issue with the [`TcpListener`].
    async fn listen(&self, listener: TcpListener, packets: mpsc::Sender<Packet>) -> IoResult<()> {
        while let Ok((stream, address)) = listener.accept().await {
            let mut connection = Connection::new(stream);
            let (sender, receiver) = mpsc::channel(32);
            let packets = packets.clone();
            tokio::spawn(async move {
                let _ = connection.process(packets, receiver).await;
            });
            self.connections.insert(address, sender);
        }
        Ok(())
    }

    /// Waits for the [`Listener`] to be closed by a call to [`Listener::closed`].
    async fn closed(&self) {
        self.token.cancelled().await;
    }

    /// Closes the [`Listener`], waking up all tasks that are waiting on [`Listener::closed`].
    fn close(&self) {
        self.token.cancel();
    }
}

/// A key-value collection that maps addresses to connections.
type ConnectionMap = HashMap<SocketAddr, mpsc::Sender<(Packet, Response)>>;

/// A pair of channels for receiving [`Message`]s and sending [`Packet`]s.
type SocketChannels = (mpsc::Sender<Packet>, mpsc::Receiver<Message>);

/// A specialized [Result] type for I/O operations.
type IoResult<T> = std::io::Result<T>;

/// A channel for sending the [`Result`] of [`Socket`] operations.
type Response = oneshot::Sender<super::Result<()>>;

#[cfg(test)]
mod tests;
