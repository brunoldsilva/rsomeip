use super::{Buffer, Connection, Listener, Socket};
use std::{
    collections::HashMap,
    io,
    net::SocketAddr,
    sync::{Arc, Mutex, Weak},
};
use tokio::{
    net::{TcpListener, TcpSocket, TcpStream},
    sync::mpsc::{self, Receiver, Sender},
};

mod tests;

type AddressedBuffer = (SocketAddr, Buffer);

impl Socket for TcpStream {}
impl Socket for TcpListener {}

pub(super) struct Factory;

impl Factory {
    pub(super) async fn connect(src: SocketAddr, dst: SocketAddr) -> Result<Connection, io::Error> {
        let socket = TcpSocket::new_v4().and_then(|s| s.bind(src).and(Ok(s)))?;
        let stream = socket.connect(dst).await.map(Arc::new)?;
        let (sender, receiver) = Self::connect_channel(Arc::downgrade(&stream));
        Ok(Connection {
            receiver,
            sender: Some(sender),
            socket: stream,
        })
    }

    fn connect_channel(stream: Weak<TcpStream>) -> (Sender<Buffer>, Receiver<Buffer>) {
        (
            Self::connect_send(stream.clone()),
            Self::connect_recv(stream),
        )
    }

    fn connect_send(stream: Weak<TcpStream>) -> Sender<Buffer> {
        let (sender, mut receiver) = mpsc::channel::<Buffer>(8);
        tokio::spawn(async move {
            loop {
                let Some(stream) = stream.upgrade() else {
                    return;
                };
                let Some(buf) = receiver.recv().await else {
                    return;
                };
                if stream.writable().await.is_err() {
                    return;
                }
                if stream.try_write(&buf[..]).is_err() {
                    return;
                }
            }
        });
        sender
    }

    fn connect_recv(stream: Weak<TcpStream>) -> Receiver<Buffer> {
        let (sender, receiver) = mpsc::channel::<Buffer>(8);
        tokio::spawn(async move {
            loop {
                let Some(stream) = stream.upgrade() else {
                    return;
                };
                if stream.readable().await.is_err() {
                    return;
                }
                let mut buf = [0u8; 1024];
                if stream.try_read(&mut buf[..]).is_err() {
                    return;
                }
                if sender.send(buf).await.is_err() {
                    return;
                }
            }
        });
        receiver
    }

    pub(super) async fn listen(src: SocketAddr) -> Result<Listener, io::Error> {
        let listener = TcpListener::bind(src).await.map(Arc::new)?;
        let (sender, receiver) = Self::listen_channels(Arc::downgrade(&listener));
        Ok(Listener {
            receiver,
            sender,
            socket: listener,
        })
    }

    fn listen_channels(
        listener: Weak<TcpListener>,
    ) -> (Sender<AddressedBuffer>, Receiver<AddressedBuffer>) {
        let (tx_sender, rx_sender) = mpsc::channel::<AddressedBuffer>(8);
        let (tx_receiver, rx_receiver) = mpsc::channel::<AddressedBuffer>(8);
        tokio::spawn(async move {
            let connections = Arc::from(Mutex::from(HashMap::<SocketAddr, Handle>::new()));
            Self::listen_send(rx_sender, Arc::downgrade(&connections));
            loop {
                let Some(listener) = listener.upgrade() else {
                    return;
                };
                let Ok((socket, addr)) = listener.accept().await.map(|(s, a)| (Arc::new(s), a))
                else {
                    return;
                };
                let (sender, receiver) = Self::connect_channel(Arc::downgrade(&socket));
                {
                    connections
                        .lock()
                        .ok()
                        .and_then(|mut c| c.insert(addr, Handle { socket, sender }));
                }
                Self::listen_recv(receiver, tx_receiver.clone(), addr);
            }
        });
        (tx_sender, rx_receiver)
    }

    fn listen_send(
        mut receiver: Receiver<AddressedBuffer>,
        connections: Weak<Mutex<HashMap<SocketAddr, Handle>>>,
    ) {
        tokio::spawn(async move {
            loop {
                let Some((addr, buf)) = receiver.recv().await else {
                    return;
                };
                let Some(connections) = connections.upgrade() else {
                    return;
                };
                let Some(sender) = connections
                    .lock()
                    .ok()
                    .and_then(|c| c.get(&addr).map(|h| h.sender.clone()))
                else {
                    return;
                };
                if sender.send(buf).await.is_err() {
                    return;
                }
            }
        });
    }

    fn listen_recv(
        mut receiver: Receiver<Buffer>,
        sender: Sender<AddressedBuffer>,
        addr: SocketAddr,
    ) {
        tokio::spawn(async move {
            loop {
                let Some(buf) = receiver.recv().await else {
                    return;
                };
                if sender.send((addr, buf)).await.is_err() {
                    return;
                }
            }
        });
    }
}

struct Handle {
    socket: Arc<dyn Socket>,
    sender: Sender<Buffer>,
}
