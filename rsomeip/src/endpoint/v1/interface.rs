//! Service interface handles.
//!
//! This module provides types for managing and communicating asynchronously with SOME/IP service
//! interfaces, such as [`Proxy`] and [`Stub`] handles.
//!
//! - [`Interface`] is a handle to a service interface for sending SOME/IP messages.
//!
//! - [`WeakInterface`] is a non-owning version of a regular [`Interface`].
//!
//! - [`Interfaces`] is a shared collection of [`Interface`] and [`WeakInterface`] handles.
//!
//! [`Stub`]: crate::endpoint::Stub
//! [`Proxy`]: crate::endpoint::Proxy

use crate::{endpoint::InterfaceId, socket::SocketAddr, someip, Result};
use rsomeip_bytes::Bytes;
use std::rc::Rc;
use tokio::sync::mpsc;

/// Handle to a [`Stub`] or [`Proxy`].
///
/// Used to asynchronously send SOME/IP messages to the connected interface.
///
/// # Ownership
///
/// An [`Interface`] counts towards the ownership semantics of the connected [`Stub`] or [`Proxy`].
/// This means that these service interfaces will only be dropped when the last [`Interface`] is
/// also dropped.
///
/// See [`WeakInterface`] for a non-owning version of this [`Interface`].
///
/// [`Stub`]: crate::endpoint::Stub
/// [`Proxy`]: crate::endpoint::Proxy
#[must_use]
#[derive(Debug, Clone)]
pub struct Interface {
    /// Channel for sending SOME/IP messages to the interface.
    messages: mpsc::Sender<(someip::Message<Bytes>, SocketAddr)>,
}

impl Interface {
    /// Creates a new [`Interface`].
    ///
    /// Incoming messages will be forwarded through the given channel.
    pub fn new(messages: mpsc::Sender<(someip::Message<Bytes>, SocketAddr)>) -> Self {
        Self { messages }
    }

    /// Creates a new [`Interface`]-[`mpsc::Receiver`] pair.
    ///
    /// `buffer` defines the capacity of the channel.
    pub fn channel(buffer: usize) -> (Self, mpsc::Receiver<(someip::Message<Bytes>, SocketAddr)>) {
        let (sender, receiver) = mpsc::channel(buffer);
        (Self::new(sender), receiver)
    }

    /// Sends a SOME/IP message to the connected service interface.
    ///
    /// # Errors
    ///
    /// Returns an error if the interface has already been dropped.
    pub async fn send(&self, value: (someip::Message<Bytes>, SocketAddr)) -> Result<()> {
        self.messages.send(value).await.map_err(|_| "closed")?;
        Ok(())
    }

    /// Downgrades this [`Interface`] into a [`WeakInterface`] which does not count towards
    /// ownership semantics.
    pub fn downgrade(&self) -> WeakInterface {
        WeakInterface {
            messages: self.messages.downgrade(),
        }
    }
}

/// Weak handle to a [`Stub`] or [`Proxy`].
///
/// This is a weak version of the normal [`Interface`] which does not count towards ownership
/// semantics of the connected service interfaces. As such, it will not prevent the service
/// interface from being dropped when the last [`Interface`] handle is also dropped.
///
/// Use [`WeakInterface::upgrade`] to upgrade this [`WeakInterface`] into a normal [`Interface`].
///
/// [`Stub`]: crate::endpoint::Stub
/// [`Proxy`]: crate::endpoint::Proxy
#[must_use]
#[derive(Debug, Clone)]
pub struct WeakInterface {
    /// Weak sender of SOME/IP messages.
    messages: mpsc::WeakSender<(someip::Message<Bytes>, SocketAddr)>,
}

impl WeakInterface {
    /// Upgrades this [`WeakInterface`] into a normal [`Interface`].
    ///
    /// Returns [`None`] if the interface has already been dropped.
    pub fn upgrade(&self) -> Option<Interface> {
        self.messages
            .upgrade()
            .map(|inner| Interface { messages: inner })
    }
}

/// A collection of service interface handles.
///
/// This contains both a local storage of normal [`Interface`] handles as well as a shared
/// storage of [`WeakInterface`] handles.
///
/// The local storage is used to associate the lifetime of the handles to this [`Interfaces`]
/// instance, while the shared storage is used to route SOME/IP messages to the corresponding
/// interfaces.
#[derive(Debug)]
pub struct Interfaces {
    /// Service interfaces shared between [`Interfaces`].
    shared: Rc<scc::HashMap<InterfaceId, WeakInterface>>,
    /// Service interfaces managed by this instance.
    local: scc::HashMap<InterfaceId, Interface>,
}

impl Interfaces {
    /// Creates a new [`Interfaces`].
    pub fn new() -> Self {
        Self {
            shared: Rc::new(scc::HashMap::default()),
            local: scc::HashMap::default(),
        }
    }

    /// Sends the message to the matching interface.
    ///
    /// This will search for an interface whose Ids matches the ones in the SOME/IP message, and
    /// try to send the message to it.
    ///
    /// Performs some cleanup on the shared [`WeakInterface`] storage if it detects a closed
    /// interface.
    ///
    /// # Errors
    ///
    /// Returns an error if the interface does not exist or has already been closed.
    pub async fn send(&self, value: (someip::Message<Bytes>, SocketAddr)) -> Result<()> {
        use scc::hash_map::Entry;
        let id = InterfaceId::from(&value.0);

        match self.shared.entry_async(id).await {
            Entry::Occupied(entry) => {
                if let Some(interface) = entry.upgrade() {
                    if interface.send(value).await.is_err() {
                        _ = entry.remove_entry();
                        Err("not available")?;
                    }
                } else {
                    _ = entry.remove_entry();
                    Err("not available")?;
                };
            }
            Entry::Vacant(_) => Err("not available")?,
        }

        Ok(())
    }

    /// Sends multiple messages to their respective interfaces.
    ///
    /// Messages addressed to unknown or closed interfaces will be dropped.
    pub async fn send_all<I>(&self, values: I)
    where
        I: IntoIterator<Item = (someip::Message<Bytes>, SocketAddr)>,
    {
        for value in values {
            _ = self.send(value).await;
        }
    }

    /// Adds the [`WeakInterface`] to the shared storage.
    ///
    /// This storage can be accessed by all clones of the instance.
    pub async fn share(&self, id: InterfaceId, interface: WeakInterface) {
        self.shared.entry_async(id).await.insert_entry(interface);
    }

    /// Adds the [`Interface`] to the local storage.
    ///
    /// This storage can only be accessed by this instance.
    pub async fn store(&self, id: InterfaceId, interface: Interface) {
        self.local.entry_async(id).await.insert_entry(interface);
    }

    /// Checks if this instance contains an open interface handle with the given Id.
    ///
    /// Performs some cleanup on the shared storage if it detects a closed interface.
    pub async fn check(&self, id: InterfaceId) -> bool {
        use scc::hash_map::Entry;

        match self.shared.entry_async(id).await {
            Entry::Occupied(entry) => entry.upgrade().is_some(),
            Entry::Vacant(_) => false,
        }
    }
}

impl Clone for Interfaces {
    /// Returns a copy of this [`Interfaces`] without the local handles.
    fn clone(&self) -> Self {
        Self {
            shared: self.shared.clone(),
            local: scc::HashMap::default(),
        }
    }
}
