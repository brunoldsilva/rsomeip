use crate::{net::util::Buffer, socket};
use tokio::sync::broadcast::{self, error::RecvError};

/// An object for receiving data from an UDP socket.
#[derive(Debug)]
pub struct Receiver {
    inner: broadcast::Receiver<Buffer>,
}

impl Receiver {
    /// Creates a new [`Receiver`].
    pub fn new(inner: broadcast::Receiver<Buffer>) -> Self {
        Self { inner }
    }
}

impl socket::Receiver for Receiver {
    /// Receives a buffer from the connected socket.
    async fn recv(&mut self) -> Option<Buffer> {
        loop {
            match self.inner.recv().await {
                Ok(buffer) => return Some(buffer),
                Err(RecvError::Closed) => return None,
                Err(RecvError::Lagged(count)) => {
                    eprintln!("Receiver lagged too far behind. {count} messages were skipped.");
                    continue;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::socket::Receiver;

    #[tokio::test]
    async fn recv() {
        let (tx, rx) = broadcast::channel(1);
        let mut receiver = super::Receiver::new(rx);
        tx.send(Buffer::from([1u8])).expect("should send the value");
        let result = receiver.recv().await.expect("should receive the value");
        assert_eq!(&result[..], &[1u8]);
    }

    #[tokio::test]
    async fn recv_none() {
        let (_, rx) = broadcast::channel(1);
        let mut receiver = super::Receiver::new(rx);
        let result = receiver.recv().await;
        assert!(result.is_none());
    }
}
