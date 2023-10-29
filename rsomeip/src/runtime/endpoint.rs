//! TCP/UDP bindings for rsomeip.

use std::net::SocketAddr;

mod udp;

type Payload = [u8; 128];
type Message = (SocketAddr, Payload);

/// A trait for abstracting over different socket types.
trait Endpoint: Sync + Send {}

/// The underlying protocol used for communication.
pub(super) enum Protocol {
    Udp,
    Tcp,
}
