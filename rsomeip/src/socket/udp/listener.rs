use super::Connection;
use crate::{net::SocketAddr, socket};
use tokio::sync::mpsc;

/// A listener for incoming remote connections.
#[derive(Debug)]
pub(super) struct Listener {
    inner: mpsc::Receiver<(Connection, SocketAddr)>,
}

impl Listener {
    /// Creates a new [`UdpListener`].
    pub fn new(inner: mpsc::Receiver<(Connection, SocketAddr)>) -> Self {
        Self { inner }
    }
}

impl socket::Listener for Listener {
    /// Accepts a connection from a remote socket.
    async fn accept(&mut self) -> Option<(impl crate::socket::Connection, SocketAddr)> {
        self.inner.recv().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{socket::Listener, testing::ipv4};
    use tokio::sync::broadcast;

    #[tokio::test]
    async fn accept() {
        let (tx_listener, rx_listener) = mpsc::channel(1);
        let mut listener = super::Listener::new(rx_listener);

        let (sender, _) = mpsc::channel(1);
        let (_, receiver) = broadcast::channel(1);
        let address = ipv4!([127, 0, 0, 1]);
        let connection = Connection::new(sender, receiver, address);

        let address = ipv4!([127, 0, 0, 1]);
        tx_listener
            .send((connection, address))
            .await
            .expect("should send the connection");

        let (_, peer) = listener
            .accept()
            .await
            .expect("should receive the connection");
        assert_eq!(peer, address);
    }

    #[tokio::test]
    async fn accept_none() {
        let (_, rx_listener) = mpsc::channel(1);
        let mut listener = super::Listener::new(rx_listener);

        let result = listener.accept().await;
        assert!(result.is_none());
    }
}
