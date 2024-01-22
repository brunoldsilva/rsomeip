#![allow(dead_code)]
use crate::net::{
    socket::{Message, Operation, Packet},
    util::BufferPool,
    Error, Result, SocketAddr,
};
use std::{collections::HashMap, sync::Arc};
use tokio::{
    net::{TcpSocket, TcpStream},
    sync::{mpsc, oneshot},
};

#[cfg(test)]
mod tests;

type Response = oneshot::Sender<Result<()>>;

/// A [`super::Socket`] abstraction over multiple TCP connections using the same local address.
///
///
#[derive(Debug)]
pub struct Socket {
    address: SocketAddr,
    connections: ConnectionMap,
}

impl Socket {
    /// Creates a new [`Socket`].
    ///
    /// Use `address` to specify the local address to which new listeners and streams will bind.
    /// However, note that any binding will only take place when calling [`Socket::process`].
    pub fn new(address: SocketAddr) -> Self {
        Self {
            address,
            connections: ConnectionMap::new(),
        }
    }

    /// Processes messages from the given receiver, performing the requested operations and sending
    /// the results to the corresponding response channels.
    pub async fn process(
        &mut self,
        mut messages: mpsc::Receiver<Message>,
        packets: mpsc::Sender<Packet>,
    ) {
        while let Some(Message {
            operation,
            response,
        }) = messages.recv().await
        {
            match operation {
                Operation::Send(packet) => self.send(packet, response).await,
                Operation::Connect(address) => self.connect(address, packets.clone(), response),
                Operation::Disconnect(address) => self.disconnect(address, response),
                Operation::Open => todo!(),
                Operation::Close => todo!(),
            }
        }
    }

    /// Sends the given `packet` to the target.
    async fn send(&mut self, packet: (SocketAddr, Arc<[u8]>), response: Response) {
        let (address, data) = packet;
        match self.connections.check(&address) {
            Some(connection) => {
                let _ = connection.send((data, response)).await;
            }
            None => {
                let _ = response.send(Err(Error::Failure("connection not established")));
            }
        }
    }

    /// Establishes a TCP connection.
    ///
    /// This will allow data to be sent to and received from the target at the given address.
    ///
    /// A new channel is added to the connection map to allow sending data to the target.
    ///
    /// Inbound data is sent to the `packets` channel.
    ///
    /// # Errors
    ///
    /// Returns an error response if the socket cannot bind to the local address or if the
    /// connection cannot be established to the remote address.
    fn connect(
        &mut self,
        peer_address: SocketAddr,
        packets: mpsc::Sender<Packet>,
        response: Response,
    ) {
        if self.connections.check(&peer_address).is_some() {
            let _ = response.send(Ok(()));
            return;
        }
        let (tx_packets, rx_packets) = mpsc::channel(32);
        let local_address = self.address;
        tokio::spawn(async move {
            let result = async move {
                let socket = match peer_address {
                    SocketAddr::V4(_) => TcpSocket::new_v4()?,
                    SocketAddr::V6(_) => TcpSocket::new_v6()?,
                };
                socket.bind(local_address)?;
                socket.connect(peer_address).await
            }
            .await
            .map(|stream| process_stream(stream, packets, rx_packets))
            .map_err(Error::from);
            let _ = response.send(result);
        });
        self.connections.insert(peer_address, tx_packets);
    }

    /// Closes the connection to the target at the given `address`.
    fn disconnect(&mut self, address: SocketAddr, response: Response) {
        let _ = match self.connections.remove(&address) {
            Some(_) => response.send(Ok(())),
            None => response.send(Err(Error::Failure("connection not established"))),
        };
    }
}

/// Processes the given `stream`, sending and receiving packets asynchronously.
fn process_stream(
    stream: TcpStream,
    tx_packets: mpsc::Sender<(SocketAddr, Arc<[u8]>)>,
    rx_packets: mpsc::Receiver<(Arc<[u8]>, Response)>,
) {
    let (reader, writer) = stream.into_split();
    tokio::spawn(receive(reader, tx_packets));
    tokio::spawn(transmit(writer, rx_packets));
}

async fn receive(reader: tokio::net::tcp::OwnedReadHalf, packets: mpsc::Sender<Packet>) {
    let mut buffers = BufferPool::new();
    let address = match reader.local_addr() {
        Ok(address) => address,
        Err(error) => {
            eprintln!("failed to get local address: {error}");
            return;
        }
    };
    loop {
        if let Err(error) = reader.readable().await {
            eprintln!("failed to read from stream: {error}");
            break;
        }
        let mut buffer = buffers.pull_or_else(|| Arc::new([0; 1024]));
        let Some(mut_buffer) = Arc::get_mut(&mut buffer) else {
            eprintln!("failed to get mutable reference to buffer");
            break;
        };
        if let Err(error) = reader.try_read(mut_buffer) {
            eprintln!("failed to read from stream: {error}");
            break;
        }
        if let Err(error) = packets.send((address, buffer.clone())).await {
            eprintln!("failed to send packet: {error}");
            break;
        }
        buffers.push(buffer);
    }
}

async fn transmit(
    writer: tokio::net::tcp::OwnedWriteHalf,
    mut rx_packets: mpsc::Receiver<(Arc<[u8]>, Response)>,
) {
    loop {
        let Some((buf, response)) = rx_packets.recv().await else {
            eprintln!("failed to receive packet");
            break;
        };
        if let Err(error) = writer.writable().await {
            eprintln!("failed to write to stream: {error}");
            break;
        }
        let result = writer
            .try_write(buf.as_ref())
            .map(|_| ())
            .map_err(Error::from);
        let _ = response.send(result);
    }
}

#[derive(Debug, Clone)]
struct Connection {
    sender: mpsc::Sender<(Arc<[u8]>, Response)>,
    cancellation_token: tokio_util::sync::CancellationToken,
}

impl Connection {
    /// Creates a new [`Connection`] with the given `sender`.
    #[must_use]
    fn new(
        sender: mpsc::Sender<(Arc<[u8]>, Response)>,
        cancellation_token: tokio_util::sync::CancellationToken,
    ) -> Self {
        Self {
            sender,
            cancellation_token,
        }
    }

    /// Sends the `value` through the connection.
    ///
    /// # Errors
    ///
    /// Returns an error if the connection is closed, or if `value` could not be sent.
    pub async fn send(&self, value: (Arc<[u8]>, Response)) -> Result<()> {
        self.sender
            .send(value)
            .await
            .map_err(|_| Error::Failure("connection is closed"))
    }

    /// Cancels the connection, stopping it from receiving and sending data.
    pub fn cancel(&self) {
        self.cancellation_token.cancel();
    }
}

/// A collection of connection channels mapped to [`SocketAddr`]s.
#[derive(Debug, Default)]
struct ConnectionMap {
    inner: HashMap<SocketAddr, mpsc::Sender<(Arc<[u8]>, Response)>>,
}

impl ConnectionMap {
    /// Creates a new [`ConnectionMap`].
    fn new() -> Self {
        Self {
            inner: HashMap::new(),
        }
    }

    /// Adds a new `connection` with the given `address`.
    fn insert(&mut self, address: SocketAddr, connection: mpsc::Sender<(Arc<[u8]>, Response)>) {
        self.inner.insert(address, connection);
    }

    /// Removes the `connection` with the given `address` from the map, if it exists.
    fn remove(&mut self, address: &SocketAddr) -> Option<mpsc::Sender<(Arc<[u8]>, Response)>> {
        self.inner.remove(address)
    }

    /// Gets the `connection` with the given `address`, if it exists.
    fn get(&self, address: &SocketAddr) -> Option<mpsc::Sender<(Arc<[u8]>, Response)>> {
        self.inner.get(address).cloned()
    }

    /// Gets the `connection` with the given `address`, if it exists and is open.
    ///
    /// If it exists but is closed, then it removes it from the map and returns [`None`].
    fn check(&mut self, address: &SocketAddr) -> Option<mpsc::Sender<(Arc<[u8]>, Response)>> {
        match self.inner.get(address) {
            Some(connection) => {
                if connection.is_closed() {
                    let _ = self.inner.remove(address);
                    return None;
                }
                Some(connection.clone())
            }
            None => None,
        }
    }
}
