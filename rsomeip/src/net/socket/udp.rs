use crate::net::{
    socket::{Message, Operation, Packet},
    util::BufferPool,
    Error, Result, SocketAddr,
};
use std::sync::Arc;
use tokio::{
    net::UdpSocket,
    sync::{mpsc, oneshot},
};

#[cfg(test)]
mod tests;

/// An abstraction over an UDP socket.
#[derive(Debug)]
pub struct Socket {
    inner: tokio::net::UdpSocket,
    packets: mpsc::Sender<Packet>,
    buffers: BufferPool,
}

impl Socket {
    /// Creates a new [`Socket`].
    ///
    /// # Errors
    ///
    /// Returns an error if the UDP socket cannot bind to the given `address`.
    pub async fn bind(address: SocketAddr) -> Result<(Self, mpsc::Receiver<Packet>)> {
        let inner = UdpSocket::bind(address).await?;
        let (packets, receiver) = mpsc::channel(32);
        Ok((
            Self {
                inner,
                packets,
                buffers: BufferPool::new(),
            },
            receiver,
        ))
    }

    /// Processes messages until the receiver is closed or the socket becomes unavailable.
    pub async fn process(&mut self, mut messages: mpsc::Receiver<Message>) {
        loop {
            tokio::select! {
                message = messages.recv() => {
                    match message {
                        Some(Message { operation, response }) => {
                            self.process_operation(operation, response).await;
                            continue;
                        },
                        None => break,
                    }
                },
                received = self.recv() => {
                    if received.is_err() {
                        break;
                    }
                },
            }
        }
    }

    /// Performs the provided `operation` and sends the result to the `response` channel.
    ///
    /// `Send`, `Connect` and `Disconnect` operations are supported. All other operations are
    /// ignored.
    async fn process_operation(&self, operation: Operation, response: oneshot::Sender<Result<()>>) {
        let result: Result<()> = match operation {
            Operation::Send((address, data)) => self.send(data, address).await,
            Operation::Connect(address) => self.connect(address),
            Operation::Disconnect(address) => self.disconnect(address),
            _ => Ok(()),
        };
        let _ = response.send(result);
    }

    /// Sends the `data` to the given `address`.
    ///
    /// # Errors
    ///
    /// Returns an error if the socket was unable to send.
    async fn send(&self, data: Arc<[u8]>, address: SocketAddr) -> Result<()> {
        match self.inner.send_to(data.as_ref(), address).await {
            Ok(_size) => Ok(()),
            Err(err) => Err(Error::from(err)),
        }
    }

    /// Establishes a connection to the target at the given `address`.
    ///
    /// Only multicast addresses are supported.
    ///
    /// # Errors
    ///
    /// Returns an error if the connection cannot be established.
    fn connect(&self, address: SocketAddr) -> Result<()> {
        if address.ip().is_multicast() {
            match address {
                SocketAddr::V4(address) => {
                    return self
                        .inner
                        .join_multicast_v4(*address.ip(), *address.ip())
                        .map_err(Error::from);
                }
                SocketAddr::V6(address) => {
                    return self
                        .inner
                        .join_multicast_v6(address.ip(), address.scope_id())
                        .map_err(Error::from);
                }
            }
        }
        Ok(())
    }

    /// Closes the connection to the target at the given `address`.
    ///
    /// Only multicast addresses are supported.
    ///
    /// # Errors
    ///
    /// Returns an error if the connection cannot be closed.
    fn disconnect(&self, address: SocketAddr) -> Result<()> {
        if address.ip().is_multicast() {
            match address {
                SocketAddr::V4(address) => {
                    return self
                        .inner
                        .leave_multicast_v4(*address.ip(), *address.ip())
                        .map_err(Error::from);
                }
                SocketAddr::V6(address) => {
                    return self
                        .inner
                        .leave_multicast_v6(address.ip(), address.scope_id())
                        .map_err(Error::from);
                }
            }
        }
        Ok(())
    }

    /// Receives a packet from the socket and sends it to the `packets` channel.
    ///
    /// Uses a buffer from the `buffers` pool to store the received data.
    ///
    /// # Errors
    ///
    /// Returns an error if the socket was unable to receive.
    async fn recv(&mut self) -> Result<()> {
        let mut buffer = self.buffers.pull_or_else(|| Arc::new([0u8; 1024]));
        let Some(mut_buffer) = Arc::get_mut(&mut buffer) else {
            // This should never happen. `pull_or_else()` will always return a unique buffer.
            return Err(Error::Failure("buffer is not unique"));
        };
        let result = match self.inner.recv_from(mut_buffer).await {
            Ok((_, address)) => self
                .packets
                .send((address, buffer.clone()))
                .await
                .map_err(|_| Error::Failure("sender is closed")),
            Err(err) => Err(Error::from(err)),
        };
        self.buffers.push(buffer);
        result
    }
}
