use super::Message;
use std::{
    io,
    net::SocketAddr,
    sync::{Arc, Weak},
};
use tokio::{net::UdpSocket, sync::mpsc};

mod tests;

pub(super) struct UdpEndpoint {
    socket: Arc<UdpSocket>,
    sender: mpsc::Sender<Message>,
}

impl UdpEndpoint {
    pub(super) async fn bind(
        addr: SocketAddr,
    ) -> Result<(Self, mpsc::Receiver<Message>), io::Error> {
        let socket = UdpSocket::bind(addr).await.map(Arc::new)?;

        let weak = Arc::downgrade(&socket);
        let (tx_sender, rx_sender) = mpsc::channel::<Message>(8);
        tokio::spawn(async move { Self::send(weak, rx_sender).await });

        let weak = Arc::downgrade(&socket);
        let (tx_receiver, rx_receiver) = mpsc::channel::<Message>(8);
        tokio::spawn(async move { Self::recv(weak, tx_receiver).await });

        Ok((
            Self {
                socket,
                sender: tx_sender,
            },
            rx_receiver,
        ))
    }

    async fn send(socket: Weak<UdpSocket>, mut receiver: mpsc::Receiver<Message>) {
        loop {
            let Some((addr, msg)) = receiver.recv().await else {
                return;
            };
            let Some(socket) = socket.upgrade() else {
                return;
            };
            if socket.send_to(&msg[..], addr).await.is_err() {
                return;
            }
        }
    }

    async fn recv(socket: Weak<UdpSocket>, sender: mpsc::Sender<Message>) {
        loop {
            let Some(socket) = socket.upgrade() else {
                return;
            };
            let mut buf = [0u8; 128];
            let Ok((_size, addr)) = socket.recv_from(&mut buf[..]).await else {
                return;
            };
            if sender.send((addr, buf)).await.is_err() {
                return;
            }
        }
    }

    fn sender(&self) -> mpsc::Sender<Message> {
        self.sender.clone()
    }
}
