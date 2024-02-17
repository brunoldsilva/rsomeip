use crate::net::{
    socket::{Message, Operation, Packet},
    util::{BufferPool, ResponseSender, SharedMap},
    IoResult,
};
use std::{
    io,
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    sync::Arc,
};
use tokio::{
    net::{TcpListener, TcpSocket, TcpStream},
    select,
    sync::mpsc,
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

/// A TCP [`super::Socket`] implementation.
#[derive(Debug)]
struct Socket {
    address: SocketAddr,
    connections: ConnectionMap,
    listener: Option<Listener>,
}

impl Socket {
    /// Creates a new [`Socket`].
    #[must_use]
    fn new(address: SocketAddr) -> Self {
        Self {
            address,
            connections: ConnectionMap::new(),
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
    fn connect(
        &mut self,
        address: SocketAddr,
        packets: mpsc::Sender<Packet>,
        response: ResponseSender<(), io::Error>,
    ) {
        if self.check(&address).is_some() {
            response.ok(());
            return;
        }
        let local_address = self.address;
        let (sender, receiver) = mpsc::channel(32);
        tokio::spawn(async move {
            let mut connection = match Connection::connect(local_address, address).await {
                Ok(connection) => {
                    response.ok(());
                    connection
                }
                Err(error) => {
                    response.err(error);
                    return;
                }
            };
            let _ = connection.process(packets, receiver).await;
        });
        let _ = self.connections.insert(address, sender);
    }

    /// Closes the TCP connection to the given address.
    ///
    /// # Errors
    ///
    /// Does not actually return an error.
    fn disconnect(&mut self, address: SocketAddr, response: ResponseSender<(), io::Error>) {
        let _ = self.connections.remove(&address);
        response.ok(());
    }

    /// Opens the [`Socket`] to incoming TCP connections.
    fn open(&mut self, packets: mpsc::Sender<Packet>, response: ResponseSender<(), io::Error>) {
        let address = self.address;
        let listener = Listener::new(self.connections.clone());
        self.listener = Some(listener.clone());
        tokio::spawn(async move {
            let _ = listener.open(address, packets, response).await;
        });
    }

    /// Closes the [`Socket`] to incoming TCP connections.
    fn close(&mut self, response: ResponseSender<(), io::Error>) {
        if let Some(ref listener) = self.listener {
            listener.close();
        }
        response.ok(());
    }

    /// Sends the packet to its intended target.
    ///
    /// # Errors
    ///
    /// Returns an error if the connection to the target is not established.
    async fn send(&mut self, packet: Packet, response: ResponseSender<(), io::Error>) {
        match self.check(&packet.0) {
            Some(connection) => {
                let _ = connection.send((packet, response)).await;
            }
            None => {
                response.err(io::ErrorKind::NotConnected.into());
            }
        }
    }

    /// Returns the [`Connection`] channel for the given address.
    ///
    /// If the channel is already closed, it removes it and returns [`None`].
    fn check(
        &mut self,
        address: &SocketAddr,
    ) -> Option<mpsc::Sender<(Packet, ResponseSender<(), io::Error>)>> {
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
        receiver: mpsc::Receiver<(Packet, ResponseSender<(), io::Error>)>,
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
    async fn send(
        &self,
        mut packets: mpsc::Receiver<(Packet, ResponseSender<(), io::Error>)>,
    ) -> IoResult<()> {
        while let Some(((dst, buffer), response)) = packets.recv().await {
            self.stream.writable().await?;
            match self.stream.try_write(buffer.as_ref()) {
                Ok(0) => {
                    println!("Reached end-of-file.");
                    response.ok(());
                    break;
                }
                Ok(len) => {
                    println!("Sent {len} bytes to {dst}.");
                    response.ok(());
                    continue;
                }
                Err(error) => {
                    eprintln!("Failed to send data to {dst}: {error}");
                    response.ok(());
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

/// A listener for incoming TCP connections.
#[derive(Debug, Clone)]
struct Listener {
    connections: ConnectionMap,
    token: CancellationToken,
}

impl Listener {
    /// Creates a new [`Listener`].
    fn new(connections: ConnectionMap) -> Self {
        Self {
            connections,
            token: CancellationToken::new(),
        }
    }

    /// Starts processing new TCP connections until the [`Listener`] is cancelled.
    ///
    /// This function will create a [`TcpListener`] at the given address, sending back the result
    /// through the [`ResponseSender<(), io::Error>`], and will start listening for incoming TCP connections. These
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
        response: ResponseSender<(), io::Error>,
    ) -> IoResult<()> {
        let listener = match TcpListener::bind(address).await {
            Ok(listener) => {
                response.ok(());
                listener
            }
            Err(error) => {
                let error_kind = error.kind();
                response.err(error);
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

/// A thread-safe map of addresses to connections.
type ConnectionMap = SharedMap<SocketAddr, mpsc::Sender<(Packet, ResponseSender<(), io::Error>)>>;

/// A pair of channels for receiving [`Message`]s and sending [`Packet`]s.
type SocketChannels = (mpsc::Sender<Packet>, mpsc::Receiver<Message>);

#[cfg(test)]
mod tests;
