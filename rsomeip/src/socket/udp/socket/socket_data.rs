use crate::{net::util::Buffer, socket::SocketAddr};
use std::{collections::HashMap, sync::Mutex};
use tokio::sync::{broadcast, mpsc};

pub(super) struct SocketData {
    sender: mpsc::Sender<(Buffer, SocketAddr)>,
    receivers: Mutex<HashMap<SocketAddr, broadcast::Sender<Buffer>>>,
    listener: Mutex<Option<mpsc::Sender<(super::Connection, SocketAddr)>>>,
}

impl SocketData {
    /// Creates a new [`SocketData`].
    pub(super) fn new(sender: mpsc::Sender<(Buffer, SocketAddr)>) -> Self {
        Self {
            sender,
            receivers: Mutex::default(),
            listener: Mutex::default(),
        }
    }

    /// Returns a sender to the socket.
    pub(super) fn sender(&self) -> mpsc::Sender<(Buffer, SocketAddr)> {
        self.sender.clone()
    }

    /// Returns the listener of this socket.
    #[allow(clippy::expect_used)]
    pub(super) fn listener(&self) -> Option<mpsc::Sender<(super::Connection, SocketAddr)>> {
        self.listener
            .lock()
            .expect("lock should not be poisoned")
            .clone()
    }

    /// Sets the listener of this [`SocketData`].
    #[allow(clippy::expect_used)]
    pub(super) fn set_listener(&self, listener: mpsc::Sender<(super::Connection, SocketAddr)>) {
        let _ = self
            .listener
            .lock()
            .expect("lock should not be poisoned")
            .insert(listener);
    }

    /// Returns a sender to the connection.
    #[allow(clippy::expect_used)]
    pub(super) fn get(&self, address: &SocketAddr) -> Option<broadcast::Sender<Buffer>> {
        self.receivers
            .lock()
            .expect("lock should not be poisoned")
            .get(address)
            .cloned()
    }

    /// Inserts a sender to the connection.
    #[allow(clippy::expect_used)]
    pub(super) fn insert(
        &self,
        address: SocketAddr,
        sender: broadcast::Sender<Buffer>,
    ) -> Option<broadcast::Sender<Buffer>> {
        self.receivers
            .lock()
            .expect("lock should not be poisoned")
            .insert(address, sender)
    }
}

#[cfg(test)]
mod tests {
    use crate::testing::ipv4;

    use super::*;
    #[test]
    fn sender() {
        let (tx, _) = mpsc::channel(1);
        let socket_data = SocketData::new(tx);
        let _ = socket_data.sender();
    }

    #[test]
    fn listener() {
        let (tx, _) = mpsc::channel(1);
        let socket_data = SocketData::new(tx);
        assert!(socket_data.listener().is_none());
    }

    #[test]
    fn listener_set() {
        let (tx, _) = mpsc::channel(1);
        let socket_data = SocketData::new(tx);
        let (tx, _) = mpsc::channel(1);
        socket_data.set_listener(tx);
        assert!(socket_data.listener().is_some());
    }

    #[test]
    fn get() {
        let (tx, _) = mpsc::channel(1);
        let socket_data = SocketData::new(tx);

        let address = ipv4!([127, 0, 0, 1]);
        let (tx, _) = broadcast::channel(1);

        assert!(socket_data.get(&address).is_none());
        assert!(socket_data.insert(address, tx).is_none());
        assert!(socket_data.get(&address).is_some());
    }

    #[test]
    fn insert() {
        let (tx, _) = mpsc::channel(1);
        let socket_data = SocketData::new(tx);
        let address = ipv4!([127, 0, 0, 1]);
        let (tx, _) = broadcast::channel(1);
        assert!(socket_data.insert(address, tx.clone()).is_none());
        assert!(socket_data.insert(address, tx).is_some());
    }
}
