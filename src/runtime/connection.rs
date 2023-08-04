use std::{io, net::SocketAddr, sync::Arc};
use tokio::sync::mpsc::{Receiver, Sender};

mod tests;
mod udp;

trait Connection: Send + Sync {}

#[non_exhaustive]
pub(super) enum Protocol {
    Udp,
    Tcp,
}

pub(super) struct Handle {
    pub(super) receiver: Receiver<[u8; 1024]>,
    pub(super) sender: Option<Sender<[u8; 1024]>>,
    connection: Arc<dyn Connection>,
}

pub(super) struct Factory;

impl Factory {
    pub(super) async fn connect(
        protocol: Protocol,
        src: SocketAddr,
        dst: Option<SocketAddr>,
    ) -> Result<Handle, io::Error> {
        match protocol {
            Protocol::Udp => udp::Factory::connect(src, dst).await,
            Protocol::Tcp => todo!(),
            #[allow(unreachable_patterns)] // More protocols might be added later.
            _ => unimplemented!("Protocol not implemented."),
        }
    }
}
