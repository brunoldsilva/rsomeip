use std::net::SocketAddr;

mod udp;

type Payload = [u8; 128];
type Message = (SocketAddr, Payload);

trait Socket: Sync + Send {}

pub(super) enum Protocol {
    Udp,
    Tcp,
}
