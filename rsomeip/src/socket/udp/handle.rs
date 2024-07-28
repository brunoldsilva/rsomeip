use crate::{
    net::util::{response_channels, ResponseSender},
    socket::{IoResult, SocketAddr},
};
use std::io;
use tokio::sync::mpsc;

/// A handle for communicating with a [`Socket`].
#[derive(Debug)]
pub(super) struct Handle {
    inner: mpsc::Sender<Message>,
}

impl Handle {
    /// Creates a new [`Handle`].
    pub fn new(inner: mpsc::Sender<Message>) -> Self {
        Self { inner }
    }
}

impl crate::socket::Socket for Handle {
    /// Establishes a [`Connection`] from the socket to the given address.
    async fn connect(&self, address: SocketAddr) -> IoResult<impl crate::socket::Connection> {
        let (tx_response, rx_response) = response_channels();
        let message = Message::connect(address, tx_response);
        self.inner
            .send(message)
            .await
            .map_err(|_| io::Error::from(io::ErrorKind::Other))?;
        rx_response
            .get()
            .await
            .unwrap_or(Err(io::ErrorKind::Other.into()))
    }

    /// Creates a [`Listener`] for incoming remote connections to the socket.
    async fn listen(&self) -> IoResult<impl crate::socket::Listener> {
        let (tx_response, rx_response) = response_channels();
        let message = Message::listen(tx_response);
        self.inner
            .send(message)
            .await
            .map_err(|_| io::Error::from(io::ErrorKind::Other))?;
        rx_response
            .get()
            .await
            .unwrap_or(Err(io::ErrorKind::Other.into()))
    }
}

pub(super) enum Message {
    /// Establish a connection to the address.
    Connect {
        address: SocketAddr,
        response: ResponseSender<super::Connection, io::Error>,
    },
    /// Listen for incoming remote connections.
    Listen {
        response: ResponseSender<super::Listener, io::Error>,
    },
}

impl Message {
    /// Creates a new [`Message::Connect`].
    fn connect(
        address: SocketAddr,
        response: ResponseSender<super::Connection, io::Error>,
    ) -> Self {
        Self::Connect { address, response }
    }

    /// Creates a new [`Message::Listen`].
    fn listen(response: ResponseSender<super::Listener, io::Error>) -> Self {
        Self::Listen { response }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        socket::{
            udp::{connection::Connection, listener::Listener},
            Socket,
        },
        testing::ipv4,
    };
    use tokio::sync::broadcast;

    #[tokio::test]
    async fn connect() {
        let (tx, mut rx) = mpsc::channel(1);
        let peer_address = ipv4!([127, 0, 0, 1]);
        let response = async move {
            let Message::Connect { address, response } =
                rx.recv().await.expect("should receive a message")
            else {
                panic!("should receive a `Connect` message");
            };
            assert_eq!(address, peer_address);
            let (sender, _) = mpsc::channel(1);
            let (_, receiver) = broadcast::channel(1);
            response.ok(Connection::new(sender, receiver, peer_address));
        };
        let handle = Handle::new(tx);
        let ((), connection) = tokio::join!(response, handle.connect(peer_address));
        assert!(connection.is_ok());
    }

    #[tokio::test]
    async fn listen() {
        let (tx, mut rx) = mpsc::channel(1);
        let response = async move {
            let Message::Listen { response } = rx.recv().await.expect("should receive a message")
            else {
                panic!("should receive a `Listen` message");
            };
            let (_, receiver) = mpsc::channel(1);
            response.ok(Listener::new(receiver));
        };
        let handle = Handle::new(tx);
        let ((), listener) = tokio::join!(response, handle.listen());
        assert!(listener.is_ok());
    }
}
