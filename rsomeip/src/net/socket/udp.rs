use crate::net::{
    socket::{Message, Operation, Packet},
    Error, Result, SocketAddr,
};
use tokio::{
    net::UdpSocket,
    sync::{mpsc, oneshot},
};

#[cfg(test)]
mod tests;

#[derive(Debug)]
pub struct Socket {
    inner: tokio::net::UdpSocket,
    packets: mpsc::Sender<Packet>,
}

impl Socket {
    pub async fn bind(address: SocketAddr) -> Result<(Self, mpsc::Receiver<Packet>)> {
        let inner = UdpSocket::bind(address).await?;
        let (packets, receiver) = mpsc::channel(32);
        Ok((Self { inner, packets }, receiver))
    }

    pub async fn process(&mut self, mut messages: mpsc::Receiver<Message>) {
        loop {
            tokio::select! {
                message = messages.recv() => {
                    match message {
                        Some(Message { operation, response }) => {
                            if let Operation::Send((address, data)) = operation {
                                self.send_to(address, data, response).await;
                            } else {
                                let _ = response.send(Ok(()));
                            }
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

    async fn send_to(
        &self,
        address: SocketAddr,
        data: Box<[u8]>,
        response: oneshot::Sender<Result<()>>,
    ) {
        let res = self
            .inner
            .send_to(data.as_ref(), address)
            .await
            .map(|_| ())
            .map_err(Error::from);
        let _ = response.send(res);
    }

    async fn recv(&self) -> Result<()> {
        let mut buffer = Box::new([0u8; 1024]);
        match self
            .inner
            .recv_from(buffer.as_mut())
            .await
            .map_err(Error::from)
        {
            Ok((_, address)) => self
                .packets
                .send((address, buffer))
                .await
                .map_err(|_| Error::Failure("sender is closed")),
            Err(err) => Err(err),
        }
    }
}
