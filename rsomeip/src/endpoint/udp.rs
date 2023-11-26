use crate::{
    bytes::{Deserialize, Deserializer, Serialize, Serializer},
    endpoint::{Command, Controller, Request, Result},
    someip::Message,
};
use std::net::SocketAddr;
use tokio::sync::mpsc;

pub(super) async fn bind(addr: SocketAddr) -> Result<Controller> {
    let endpoint = Endpoint::new(addr).await?;
    let (tx, rx) = mpsc::channel(1);
    tokio::spawn(main(endpoint, rx));
    Ok(Controller::new(tx))
}

async fn main(mut endpoint: Endpoint, mut commands: mpsc::Receiver<Command>) {
    loop {
        tokio::select! {
            _ = endpoint.recv() => {},
            Some(command) = commands.recv() => {
                endpoint.process(command).await;
            }
        }
    }
    todo!()
}

struct Endpoint {
    socket: std::sync::Arc<tokio::net::UdpSocket>,
    relays: std::collections::HashMap<u32, mpsc::Sender<Result<Message>>>,
}

impl Endpoint {
    async fn new(addr: SocketAddr) -> Result<Self> {
        let socket = tokio::net::UdpSocket::bind(addr)
            .await
            .map(std::sync::Arc::new)?;
        Ok(Self {
            socket,
            relays: std::collections::HashMap::default(),
        })
    }

    async fn process(&mut self, command: Command) {
        let (request, tx) = command.into_parts();
        let response = match request {
            Request::Connect(_) | Request::Disconnect(_) => Ok(()),
            Request::Send { msg, dst } => self.send(msg, dst).await,
            Request::Relay { id, tx } => self.add_relay(id, tx),
            Request::Shutdown => todo!(),
        };
        let _ = tx.send(response).await;
    }

    async fn send(&self, msg: Message, dst: SocketAddr) -> Result<()> {
        let mut buf = [0u8; 1024];
        let mut ser = Serializer::new(&mut buf);
        msg.serialize(&mut ser);
        match self.socket.send_to(&buf, dst).await {
            Ok(_) => Ok(()),
            Err(err) => Err(err.into()),
        }
    }

    async fn recv(&self) -> Result<()> {
        let mut buf = [0u8; 1024];
        let length = self.socket.recv(&mut buf).await?;
        let mut de = Deserializer::new(&buf[..length]);
        if let Ok(msg) = Message::deserialize(&mut de) {
            if let Some(tx) = self.relays.get(&msg.message_id()) {
                let _ = tx.send(Ok(msg)).await;
            }
        }
        Ok(())
    }

    fn add_relay(&mut self, id: u32, tx: mpsc::Sender<Result<Message>>) -> Result<()> {
        let _ = self.relays.insert(id, tx);
        Ok(())
    }

    async fn shutdown(&self) {
        todo!()
    }
}
