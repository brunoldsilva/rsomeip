use super::{Receiver, Sender};
use crate::{
    net::{util::Buffer, SocketAddr},
    socket,
};
use tokio::sync::{broadcast, mpsc};

/// An object for sending and receiving data from an UDP socket.
#[derive(Debug)]
pub(super) struct Connection {
    sender: Sender,
    receiver: Receiver,
}

impl Connection {
    /// Creates a new [`Connection`].
    pub fn new(
        sender: mpsc::Sender<(Buffer, SocketAddr)>,
        receiver: broadcast::Receiver<Buffer>,
        peer: SocketAddr,
    ) -> Self {
        Self {
            sender: Sender::new(sender, peer),
            receiver: Receiver::new(receiver),
        }
    }
}

impl socket::Connection for Connection {
    /// Splits the connection into a [`Sender`] and [`Receiver`] pair.
    fn split(self) -> (impl socket::Sender, impl socket::Receiver) {
        (self.sender, self.receiver)
    }
}

impl socket::Sender for Connection {
    /// Sends the given buffer to the connected socket.
    async fn send(&self, value: Buffer) -> socket::SendResult<Buffer> {
        self.sender.send(value).await
    }
}

impl socket::Receiver for Connection {
    /// Receives a buffer from the connected socket.
    async fn recv(&mut self) -> Option<Buffer> {
        self.receiver.recv().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        socket::{Connection, Receiver, Sender},
        testing::ipv4,
    };

    #[test]
    fn split() {
        let (sender, _) = mpsc::channel(1);
        let (_, receiver) = broadcast::channel(1);
        let address = ipv4!([127, 0, 0, 1]);
        let connection = super::Connection::new(sender, receiver, address);
        let (_, _) = connection.split();
    }

    #[tokio::test]
    async fn send() {
        let (tx_outgoing, mut rx_outgoing) = mpsc::channel(1);
        let (_, rx_incoming) = broadcast::channel(1);
        let address = ipv4!([127, 0, 0, 1]);
        let connection = super::Connection::new(tx_outgoing, rx_incoming, address);
        connection
            .send(Buffer::from([1u8]))
            .await
            .expect("should send the value");
        let (value, peer) = rx_outgoing.recv().await.expect("should receive the value");
        assert_eq!(&value[..], &[1u8]);
        assert_eq!(peer, address);
    }

    #[tokio::test]
    async fn send_err() {
        let (tx_outgoing, _) = mpsc::channel(1);
        let (_, rx_incoming) = broadcast::channel(1);
        let address = ipv4!([127, 0, 0, 1]);
        let connection = super::Connection::new(tx_outgoing, rx_incoming, address);
        let result = connection
            .send(Buffer::from([1u8]))
            .await
            .expect_err("should send the value");
        assert_eq!(&result.value[..], &[1u8]);
    }

    #[tokio::test]
    async fn recv() {
        let (tx_outgoing, _) = mpsc::channel(1);
        let (tx_incoming, rx_incoming) = broadcast::channel(1);
        let address = ipv4!([127, 0, 0, 1]);
        let mut connection = super::Connection::new(tx_outgoing, rx_incoming, address);
        tx_incoming
            .send(Buffer::from([1u8]))
            .expect("should send the value");
        let result = connection.recv().await.expect("should receive the value");
        assert_eq!(&result[..], &[1u8]);
    }

    #[tokio::test]
    async fn recv_none() {
        let (tx_outgoing, _) = mpsc::channel(1);
        let (_, rx_incoming) = broadcast::channel(1);
        let address = ipv4!([127, 0, 0, 1]);
        let mut connection = super::Connection::new(tx_outgoing, rx_incoming, address);
        let result = connection.recv().await;
        assert!(result.is_none());
    }
}
