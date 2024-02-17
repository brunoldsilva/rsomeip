use crate::net::{
    socket::{Message, Operation, Packet},
    util::{BufferPool, ResponseSender},
    IoResult, SocketAddr,
};
use std::{io, sync::Arc};
use tokio::{net::UdpSocket, sync::mpsc};

#[cfg(test)]
mod tests;

/// An abstraction over an UDP socket.
#[derive(Debug)]
pub struct Socket {
    inner: Arc<tokio::net::UdpSocket>,
    packets: mpsc::Sender<Packet>,
}

impl Socket {
    /// Creates a new [`Socket`].
    ///
    /// # Errors
    ///
    /// Returns an error if the UDP socket cannot bind to the given `address`.
    pub async fn bind(address: SocketAddr) -> IoResult<(Self, mpsc::Receiver<Packet>)> {
        let inner = UdpSocket::bind(address).await.map(Arc::new)?;
        let (packets, receiver) = mpsc::channel(32);
        Ok((Self { inner, packets }, receiver))
    }

    /// Processes messages until the receiver is closed or the socket becomes unavailable.
    pub async fn process(&mut self, mut messages: mpsc::Receiver<Message>) {
        let (socket, packets) = (self.inner.clone(), self.packets.clone());
        let receiver_handle = tokio::spawn(async move {
            recv(socket, packets).await;
        });
        while let Some(Message {
            operation,
            response,
        }) = messages.recv().await
        {
            self.process_operation(operation, response).await;
        }
        receiver_handle.abort();
    }

    /// Performs the provided `operation` and sends the result to the `response` channel.
    ///
    /// `Send`, `Connect` and `Disconnect` operations are supported. All other operations are
    /// ignored.
    async fn process_operation(
        &self,
        operation: Operation,
        response: ResponseSender<(), io::Error>,
    ) {
        let result: IoResult<()> = match operation {
            Operation::Send((address, data)) => self.send(data, address).await,
            Operation::Connect(address) => self.connect(address),
            Operation::Disconnect(address) => self.disconnect(address),
            _ => Ok(()),
        };
        response.send(result);
    }

    /// Sends the `data` to the given `address`.
    ///
    /// # Errors
    ///
    /// Returns an error if the socket was unable to send.
    async fn send(&self, data: Arc<[u8]>, address: SocketAddr) -> IoResult<()> {
        self.inner.send_to(data.as_ref(), address).await.map(|_| ())
    }

    /// Establishes a connection to the target at the given `address`.
    ///
    /// Only multicast addresses are supported.
    ///
    /// # Errors
    ///
    /// Returns an error if the connection cannot be established.
    fn connect(&self, address: SocketAddr) -> IoResult<()> {
        if address.ip().is_multicast() {
            return match address {
                SocketAddr::V4(address) => {
                    self.inner.join_multicast_v4(*address.ip(), *address.ip())
                }
                SocketAddr::V6(address) => self
                    .inner
                    .join_multicast_v6(address.ip(), address.scope_id()),
            };
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
    fn disconnect(&self, address: SocketAddr) -> IoResult<()> {
        if address.ip().is_multicast() {
            return match address {
                SocketAddr::V4(address) => {
                    self.inner.leave_multicast_v4(*address.ip(), *address.ip())
                }
                SocketAddr::V6(address) => self
                    .inner
                    .leave_multicast_v6(address.ip(), address.scope_id()),
            };
        }
        Ok(())
    }
}

/// Receives packets from the socket and sends them to the `packets` channel.
///
/// It will keep receiving packets until the socket becomes unavailable, or the `packets` channel
/// is closed.
async fn recv(socket: Arc<UdpSocket>, packets: mpsc::Sender<Packet>) {
    let mut buffers = BufferPool::new();
    loop {
        let mut buffer = buffers.pull_or_else(|| Arc::new([0u8; 1024]));
        let Some(mut_buffer) = Arc::get_mut(&mut buffer) else {
            // This should never happen. `pull_or_else()` will always return a unique buffer.
            eprintln!("buffer is not unique");
            break;
        };
        let Ok((_size, address)) = socket.recv_from(mut_buffer).await else {
            println!("socket is unavailable");
            break;
        };
        let Ok(()) = packets.send((address, buffer.clone())).await else {
            println!("packets channel is closed");
            break;
        };
        buffers.push(buffer);
    }
}
