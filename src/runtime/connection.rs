use std::{io, net::SocketAddr, sync::Arc};
use tokio::sync::mpsc::{Receiver, Sender};

mod tcp;
mod tests;
mod udp;

type Buffer = [u8; 1024];

trait Socket: Send + Sync {}

#[non_exhaustive]
pub(super) enum Protocol {
    Udp,
    Tcp,
}

pub(super) struct Connection {
    pub(super) receiver: Receiver<Buffer>,
    pub(super) sender: Option<Sender<Buffer>>,
    socket: Arc<dyn Socket>,
}

pub(super) struct Listener {
    pub(super) receiver: Receiver<(SocketAddr, Buffer)>,
    pub(super) sender: Sender<(SocketAddr, Buffer)>,
    socket: Arc<dyn Socket>,
}

pub(super) struct Factory;

impl Factory {
    pub(super) async fn connect(
        protocol: Protocol,
        src: SocketAddr,
        dst: Option<SocketAddr>,
    ) -> Result<Connection, io::Error> {
        match protocol {
            Protocol::Udp => udp::Factory::connect(src, dst).await,
            Protocol::Tcp => tcp::Factory::connect(src, dst.unwrap()).await,
            #[allow(unreachable_patterns)] // More protocols might be added later.
            _ => unimplemented!("Protocol not implemented."),
        }
    }

    pub(super) async fn listen(protocol: Protocol, src: SocketAddr) -> Result<Listener, io::Error> {
        match protocol {
            Protocol::Udp => todo!(),
            Protocol::Tcp => tcp::Factory::listen(src).await,
            #[allow(unreachable_patterns)] // More protocols might be added later.
            _ => unimplemented!("Protocol not implemented."),
        }
    }
}
