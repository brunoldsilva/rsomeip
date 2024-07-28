use crate::{net::util::Buffer, socket};
use tokio::sync::mpsc;

/// Creates a connected channel of [`Sender`] and [`Receiver`] pairs.
pub fn channel(buffer: usize) -> (Sender, Receiver) {
    let (tx, rx) = mpsc::channel(buffer);
    (Sender::new(tx), Receiver::new(rx))
}

/// Receives data from the paired [`Sender`].
pub struct Receiver {
    inner: mpsc::Receiver<Buffer>,
}

impl Receiver {
    /// Creates a new [`Receiver`].
    pub fn new(inner: mpsc::Receiver<Buffer>) -> Self {
        Self { inner }
    }
}

impl socket::Receiver for Receiver {
    async fn recv(&mut self) -> Option<Buffer> {
        self.inner.recv().await
    }
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

    /// Sends the value to the paired [`Receiver`].
    pub async fn send(&self, value: Buffer) -> Result<(), mpsc::error::SendError<Buffer>> {
        self.inner.send(value).await
    }
}

#[cfg(test)]
mod tests {
    use super::{mpsc, socket::Receiver, Buffer};

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
        let Err(mpsc::error::SendError(value)) = sender.send(Buffer::from(VALUE)).await else {
            panic!("should receive a send error")
        };
        assert_eq!(&value[..], &VALUE);
    }

    /// Creates a connected pair of [`mpsc::Sender`] and [`super::Receiver`].
    fn receiving_channel(buffer: usize) -> (mpsc::Sender<Buffer>, super::Receiver) {
        let (tx, rx) = mpsc::channel(buffer);
        (tx, super::Receiver::new(rx))
    }

    /// Creates a connected pair of [`super::Sender`] and [`mpsc::Receiver`].
    fn sending_channel(buffer: usize) -> (super::Sender, mpsc::Receiver<Buffer>) {
        let (tx, rx) = mpsc::channel(buffer);
        (super::Sender::new(tx), rx)
    }

    /// The value that is received from the connected socket.
    const VALUE: [u8; 1] = [1u8];
}
