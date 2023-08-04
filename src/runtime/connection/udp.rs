use super::{Connection, Handle};
use std::{
    io,
    net::SocketAddr,
    sync::{Arc, Weak},
};
use tokio::{
    net::UdpSocket,
    sync::mpsc::{self, Receiver, Sender},
};

mod tests;

impl Connection for UdpSocket {}

pub(super) struct Factory;

impl Factory {
    pub(super) async fn connect(
        src: SocketAddr,
        dst: Option<SocketAddr>,
    ) -> Result<Handle, io::Error> {
        let socket = UdpSocket::bind(src).await.map(Arc::new)?;
        let receiver = Self::recv(Arc::downgrade(&socket));
        let sender = match dst {
            Some(dst) => {
                socket.connect(dst).await?;
                Some(Self::send(Arc::downgrade(&socket)))
            }
            None => None,
        };
        Ok(Handle {
            connection: socket,
            receiver,
            sender,
        })
    }

    fn recv(socket: Weak<UdpSocket>) -> Receiver<[u8; 1024]> {
        let (sender, receiver) = mpsc::channel::<[u8; 1024]>(8);
        tokio::spawn(async move {
            loop {
                let Some(socket) = socket.upgrade() else {
                    return;
                };
                let mut buf = [0u8; 1024];
                let Ok(_) = socket.recv_from(&mut buf[..]).await else {
                    return;
                };
                if sender.send(buf).await.is_err() {
                    return;
                }
            }
        });
        receiver
    }

    fn send(socket: Weak<UdpSocket>) -> Sender<[u8; 1024]> {
        let (sender, mut receiver) = mpsc::channel::<[u8; 1024]>(8);
        tokio::spawn(async move {
            loop {
                let Some(buf) = receiver.recv().await else {
                    return;
                };
                let Some(socket) = socket.upgrade() else {
                    return;
                };
                let Ok(_) = socket.send(&buf[..]).await else {
                    return;
                };
            }
        });
        sender
    }
}
