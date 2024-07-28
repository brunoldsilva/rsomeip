use super::{Connection, Handle, Listener, Message};
use crate::{
    net::util::{Buffer, BufferPool},
    socket::{IoResult, SocketAddr},
};
use std::{io, sync::Arc};
use tokio::{
    net::UdpSocket,
    sync::{broadcast, mpsc},
};

/// A wrapper around a [`UdpSocket`].
#[derive(Debug)]
pub struct Socket {
    inner: UdpSocket,
}

impl Socket {
    /// Creates a new [`Socket`].
    fn new(inner: UdpSocket) -> Self {
        Self { inner }
    }

    /// Binds a [`Socket`] to the given address.
    ///
    /// # Errors
    ///
    /// Returns an [`io::Error`] if the socket cannot be bound to the address.
    pub async fn bind(address: SocketAddr) -> IoResult<Self> {
        UdpSocket::bind(address).await.map(Self::new)
    }

    /// Binds a [`Socket`] to the given address, and returns a [`Handle`] to it.
    ///
    /// # Errors
    ///
    /// Returns an [`io::Error`] if the socket cannot be bound to the address.
    pub async fn spawn(address: SocketAddr) -> IoResult<impl crate::socket::Socket> {
        let mut socket = Self::bind(address).await?;
        let (tx, rx) = mpsc::channel(1);
        tokio::spawn(async move {
            socket.process(rx).await;
        });
        let handle = Handle::new(tx);
        Ok(handle)
    }

    /// Processes messages from the given receiver.
    async fn process(&mut self, messages: mpsc::Receiver<Message>) {
        let (tx_outgoing, rx_outgoing) = mpsc::channel(1);
        let socket_data = SocketData::new(tx_outgoing);
        tokio::select! {
            () = self.handle(messages, &socket_data) => {},
            () = self.send(rx_outgoing) => {},
            () = self.recv(&socket_data) => {},
        }
    }

    /// Handle messages from the given receiver.
    async fn handle(&self, mut messages: mpsc::Receiver<Message>, data: &SocketData) {
        while let Some(message) = messages.recv().await {
            match message {
                Message::Connect { address, response } => {
                    let connection = Self::connect(address, data);
                    response.ok(connection);
                }
                Message::Listen { response } => {
                    let listener = Self::listen(data);
                    response.ok(listener);
                }
            }
        }
    }

    /// Sends data to the given address.
    async fn send(&self, mut outgoing: mpsc::Receiver<(Buffer, SocketAddr)>) {
        while let Some((buffer, address)) = outgoing.recv().await {
            match self.inner.send_to(&buffer[..], address).await {
                Ok(_) => continue,
                Err(error) => match error.kind() {
                    io::ErrorKind::InvalidInput => continue,
                    _ => break,
                },
            }
        }
    }

    /// Receives data from remote sockets.
    async fn recv(&self, data: &SocketData) {
        let mut buffers = BufferPool::new();
        loop {
            let mut buffer = buffers.pull_or_else(|| Arc::new([0u8; 1024]));
            let Some(mut_buffer) = Arc::get_mut(&mut buffer) else {
                // This should never happen. `pull_or_else()` will always return a unique buffer.
                eprintln!("buffer is not unique");
                continue;
            };
            let Ok((size, address)) = self.inner.recv_from(mut_buffer).await else {
                println!("socket is unavailable");
                break;
            };
            buffers.push(buffer.clone());
            let Some(sender) = Self::accept(address, data).await else {
                continue;
            };
            let Ok(_) = sender.send(Buffer::new(buffer, 0..size)) else {
                println!("packets channel is closed");
                continue;
            };
        }
    }

    /// Adds a connection to the socket data.
    fn connect(address: SocketAddr, data: &SocketData) -> Connection {
        let tx_outgoing = data.sender();
        let (tx_incoming, rx_incoming) = broadcast::channel(1);
        let _ = data.insert(address, tx_incoming);
        Connection::new(tx_outgoing, rx_incoming, address)
    }

    /// Sets the listener on the socket data.
    fn listen(data: &SocketData) -> Listener {
        let (tx_listener, rx_listener) = mpsc::channel(1);
        data.set_listener(tx_listener);
        Listener::new(rx_listener)
    }

    /// Accepts a connection.
    async fn accept(address: SocketAddr, data: &SocketData) -> Option<broadcast::Sender<Buffer>> {
        if let Some(sender) = data.get(&address) {
            return Some(sender);
        }
        if let Some(listener) = data.listener() {
            let connection = Self::connect(address, data);
            let _ = listener.send((connection, address)).await;
            return data.get(&address);
        }
        None
    }
}

mod socket_data;
use socket_data::SocketData;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        socket::{Listener, Receiver, Sender, Socket},
        testing::ipv4,
    };
    use tokio::time::{timeout, Duration};

    #[tokio::test]
    async fn bind() {
        let address = ipv4!([127, 0, 0, 1]);
        let socket = super::Socket::bind(address).await;
        assert!(socket.is_ok());
    }

    #[tokio::test]
    async fn spawn() {
        let address = ipv4!([127, 0, 0, 1]);
        let handle = super::Socket::spawn(address).await;
        assert!(handle.is_ok());
    }

    #[tokio::test]
    async fn connect() {
        let local_address = ipv4!([127, 0, 0, 1]);
        let handle = super::Socket::spawn(local_address)
            .await
            .expect("should spawn a socket");

        let peer_address = ipv4!([127, 0, 0, 2]);
        let _ = handle
            .connect(peer_address)
            .await
            .expect("should connect to the peer");
    }

    #[tokio::test]
    async fn send() {
        let local_address = ipv4!([127, 0, 0, 1]);
        let handle = super::Socket::spawn(local_address)
            .await
            .expect("should spawn a socket");

        let peer_address = ipv4!([127, 0, 0, 2]);
        let peer = UdpSocket::bind(peer_address)
            .await
            .expect("should bind to the address");

        let connection = handle
            .connect(peer_address)
            .await
            .expect("should connect to the peer");

        let value = Buffer::from([1u8]);
        connection.send(value).await.expect("should send the value");

        let mut buffer = [0u8];
        timeout(Duration::from_millis(10), peer.recv(&mut buffer))
            .await
            .expect("should not timeout")
            .expect("should receive the value");

        assert_eq!(&buffer[..], &[1u8]);
    }

    #[tokio::test]
    async fn recv() {
        let local_address = ipv4!([127, 0, 0, 1]);
        let handle = super::Socket::spawn(local_address)
            .await
            .expect("should spawn a socket");

        let peer_address = ipv4!([127, 0, 0, 2]);
        let peer = UdpSocket::bind(peer_address)
            .await
            .expect("should bind to the address");

        let mut connection = handle
            .connect(peer_address)
            .await
            .expect("should connect to the peer");

        peer.send_to(&[1u8], local_address)
            .await
            .expect("should send the value");

        let value = timeout(Duration::from_millis(10), connection.recv())
            .await
            .expect("should not timeout")
            .expect("should receive the value");

        assert_eq!(&value[..], &[1u8]);
    }

    #[tokio::test]
    async fn listen() {
        let local_address = ipv4!([127, 0, 0, 1]);
        let handle = super::Socket::spawn(local_address)
            .await
            .expect("should spawn a socket");

        let peer_address = ipv4!([127, 0, 0, 2]);
        let peer = UdpSocket::bind(peer_address)
            .await
            .expect("should bind to the address");

        let mut listener = handle.listen().await.expect("should create a listener");

        peer.send_to(&[1u8], local_address)
            .await
            .expect("should send the value");

        let (mut connection, _) = timeout(Duration::from_millis(10), listener.accept())
            .await
            .expect("should not timeout")
            .expect("should accept a connection from the peer");

        let value = timeout(Duration::from_millis(10), connection.recv())
            .await
            .expect("should not timeout")
            .expect("should receive the value");

        assert_eq!(&value[..], &[1u8]);
    }
}
