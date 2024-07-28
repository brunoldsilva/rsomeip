use super::{incoming, outgoing};
use crate::{net::util::Buffer, socket};

/// Sends and receives data from the connected socket.
pub struct Connection {
    sender: outgoing::Sender,
    receiver: incoming::Receiver,
}

impl Connection {
    /// Creates a new [`Connection`].
    pub fn new(sender: outgoing::Sender, receiver: incoming::Receiver) -> Self {
        Self { sender, receiver }
    }
}

impl socket::Connection for Connection {
    fn split(self) -> (impl socket::Sender, impl socket::Receiver) {
        (self.sender, self.receiver)
    }
}

impl socket::Sender for Connection {
    async fn send(&self, value: Buffer) -> socket::SendResult<Buffer> {
        socket::Sender::send(&self.sender, value).await
    }
}

impl socket::Receiver for Connection {
    async fn recv(&mut self) -> Option<Buffer> {
        socket::Receiver::recv(&mut self.receiver).await
    }
}

#[cfg(test)]
mod tests {
    use super::{
        incoming, outgoing,
        socket::{Connection, Receiver, SendError, Sender},
        Buffer,
    };

    #[tokio::test]
    async fn split() {
        // Create a new connection.
        let (tx_outgoing, mut rx_outgoing) = outgoing::channel(1);
        let (tx_incoming, rx_incoming) = incoming::channel(1);
        let connection = super::Connection::new(tx_outgoing, rx_incoming);

        // Split the connection into sender and receiver halves.
        let (sender, mut receiver) = connection.split();

        // Check if the sender is still connected.
        sender
            .send(Buffer::from(VALUE))
            .await
            .expect("should send the value");
        assert!(rx_outgoing.recv().await.is_some());

        // Check if the receiver is still connected.
        tx_incoming
            .send(Buffer::from(VALUE))
            .await
            .expect("should send the value");
        assert!(receiver.recv().await.is_some());
    }

    #[tokio::test]
    async fn send() {
        // Create a new connection.
        let (tx_outgoing, mut rx_outgoing) = outgoing::channel(1);
        let (_, rx_incoming) = incoming::channel(1);
        let connection = super::Connection::new(tx_outgoing, rx_incoming);

        // Send a value through the connection.
        connection
            .send(Buffer::from(VALUE))
            .await
            .expect("should send the value");

        // Check if the correct value is received.
        let value = rx_outgoing.recv().await.expect("should received the value");
        assert_eq!(&value[..], &VALUE);
    }

    #[tokio::test]
    async fn send_err() {
        // Create a new connection.
        let (tx_outgoing, _) = outgoing::channel(1);
        let (_, rx_incoming) = incoming::channel(1);
        let connection = super::Connection::new(tx_outgoing, rx_incoming);

        // Check if `send` returns an error if the other end has been dropped.
        let SendError { value, .. } = connection
            .send(Buffer::from(VALUE))
            .await
            .expect_err("should not send the value");
        assert_eq!(&value[..], &VALUE);
    }

    #[tokio::test]
    async fn recv() {
        // Create a new connection.
        let (tx_outgoing, _) = outgoing::channel(1);
        let (tx_incoming, rx_incoming) = incoming::channel(1);
        let mut connection = super::Connection::new(tx_outgoing, rx_incoming);

        // Send a value through the connection.
        tx_incoming
            .send(Buffer::from(VALUE))
            .await
            .expect("should send the value");

        // Check if the correct value is received.
        let value = connection.recv().await.expect("should receive the value");
        assert_eq!(&value[..], &VALUE);
    }

    #[tokio::test]
    async fn recv_none() {
        // Create a new connection.
        let (tx_outgoing, _) = outgoing::channel(1);
        let (_, rx_incoming) = incoming::channel(1);
        let mut connection = super::Connection::new(tx_outgoing, rx_incoming);

        // Check if `recv` returns `None` if the other end has been dropped.
        let result = connection.recv().await;
        assert!(result.is_none());
    }

    /// Value that is sent to the connected socket.
    const VALUE: [u8; 1] = [1u8];
}
