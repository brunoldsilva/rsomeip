use crate::{
    net::{util::Buffer, SocketAddr},
    socket,
};
use std::io;
use tokio::sync::mpsc;

/// Sends values to the associated receiver.
#[derive(Debug)]
pub struct Sender {
    inner: mpsc::Sender<(Buffer, SocketAddr)>,
    peer: SocketAddr,
}

impl Sender {
    /// Creates a new [`Sender`].
    pub fn new(inner: mpsc::Sender<(Buffer, SocketAddr)>, peer: SocketAddr) -> Self {
        Self { inner, peer }
    }
}

impl socket::Sender for Sender {
    async fn send(&self, value: Buffer) -> crate::socket::SendResult<Buffer> {
        self.inner
            .send((value, self.peer))
            .await
            .map_err(|value| socket::SendError::new(value.0 .0, io::ErrorKind::Other.into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{socket::Sender, testing::ipv4};

    #[tokio::test]
    async fn send() {
        let (tx, mut rx) = mpsc::channel(1);
        let address = ipv4!([127, 0, 0, 1]);
        let sender = super::Sender::new(tx, address);

        sender
            .send(Buffer::from([1u8]))
            .await
            .expect("should send the value");

        let (value, peer) = rx.recv().await.expect("should receive the value");

        assert_eq!(&value[..], &[1u8]);
        assert_eq!(peer, address);
    }

    #[tokio::test]
    async fn send_err() {
        let (tx, _) = mpsc::channel(1);
        let address = ipv4!([127, 0, 0, 1]);
        let sender = super::Sender::new(tx, address);

        let result = sender
            .send(Buffer::from([1u8]))
            .await
            .expect_err("should not send the value");

        assert_eq!(result.value.as_ref(), &[1u8]);
    }
}
