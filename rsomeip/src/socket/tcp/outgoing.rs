use crate::{net::util::Buffer, socket};
use std::io;
use tokio::sync::mpsc;

/// Creates a connected channel of [`Sender`] and [`Receiver`] pairs.
pub fn channel(buffer: usize) -> (Sender, Receiver) {
    let (tx, rx) = mpsc::channel(buffer);
    (Sender::new(tx), Receiver::new(rx))
}

/// Sends data to the paired [`Receiver`].
pub struct Sender {
    inner: mpsc::Sender<Buffer>,
}

impl Sender {
    /// Creates a new [`Sender`].
    fn new(inner: mpsc::Sender<Buffer>) -> Self {
        Self { inner }
    }
}

impl socket::Sender for Sender {
    async fn send(&self, value: Buffer) -> socket::SendResult<Buffer> {
        self.inner
            .send(value)
            .await
            .map_err(|error| socket::SendError::new(error.0, io::ErrorKind::Other.into()))
    }
}

/// Receives data from the paired [`Sender`].
pub struct Receiver {
    inner: mpsc::Receiver<Buffer>,
}

impl Receiver {
    /// Creates a new [`Receiver`].
    fn new(inner: mpsc::Receiver<Buffer>) -> Self {
        Self { inner }
    }

    /// Receives a value from the paired [`Sender`].
    pub async fn recv(&mut self) -> Option<Buffer> {
        self.inner.recv().await
    }
}

#[cfg(test)]
mod tests {
    use super::{
        io, mpsc,
        socket::{SendError, Sender},
        Buffer,
    };

    #[tokio::test]
    async fn channel() {
        let (sender, mut receiver) = super::channel(1);
        sender
            .send(Buffer::from(VALUE))
            .await
            .expect("should send the value");
        let value = receiver.recv().await.expect("should receive the value");
        assert_eq!(&value[..], &VALUE);
    }

    #[tokio::test]
    async fn send() {
        let (sender, mut rx) = sending_channel(1);
        sender
            .send(Buffer::from(VALUE))
            .await
            .expect("should send the value");
        let value = rx.recv().await.expect("should receive the value");
        assert_eq!(&value[..], &VALUE);
    }

    #[tokio::test]
    async fn send_err() {
        let (sender, _) = sending_channel(1);
        let Err(SendError { value, error }) = sender.send(Buffer::from(VALUE)).await else {
            panic!("should receive a send error")
        };
        assert_eq!(&value[..], &VALUE);
        assert_eq!(error.kind(), io::ErrorKind::Other);
    }

    #[tokio::test]
    async fn recv() {
        let (tx, mut receiver) = receiving_channel(1);
        tx.send(Buffer::from(VALUE))
            .await
            .expect("should send the value");
        let value = receiver.recv().await.expect("should receive the value");
        assert_eq!(&value[..], &VALUE);
    }

    #[tokio::test]
    async fn recv_none() {
        let (_, mut receiver) = receiving_channel(1);
        let result = receiver.recv().await;
        assert!(result.is_none());
    }

    /// Creates a connected pair of [`super::Sender`] and [`mpsc::Receiver`].
    fn sending_channel(buffer: usize) -> (super::Sender, mpsc::Receiver<Buffer>) {
        let (tx, rx) = mpsc::channel(buffer);
        (super::Sender::new(tx), rx)
    }

    /// Creates a connected pair of [`super::Sender`] and [`mpsc::Receiver`].
    fn receiving_channel(buffer: usize) -> (mpsc::Sender<Buffer>, super::Receiver) {
        let (tx, rx) = mpsc::channel(buffer);
        (tx, super::Receiver::new(rx))
    }

    /// Value that is sent to the connected socket.
    const VALUE: [u8; 1] = [1u8];
}
