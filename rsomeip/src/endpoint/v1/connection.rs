//! Connections between SOME/IP endpoints.
//!
//! This module provides types for creating and managing asynchronous connections between SOME/IP
//! endpoints.
//!
//! - [`Connection`] is a handle to an asynchronous task for processing SOME/IP messages.
//!
//! - [`WeakConnection`] is a non-owning version of a regular [`Connection`].
//!
//! - [`Connections`] is a shared collection of [`Connection`] and [`WeakConnection`] handles.
//!
//! - [`ConnectionTask`] is an asynchronous task for processing SOME/IP messages.

use super::{Interface, InterfaceId, Interfaces};
use crate::{
    bytes::{Bytes, BytesMut, Deserialize, Serialize},
    socket::{self, RecvError, SocketAddr},
    someip::{self, Message},
    Result,
};
use std::rc::Rc;
use tokio::sync::mpsc;

/// Handle to an asynchronous SOME/IP connection.
///
/// Used to send and receive messages from the connected endpoint.
///
/// # Ownership
///
/// A [`Connection`] counts towards the ownership semantics of the underlying connection. This
/// means that the connection will only be dropped when the last [`Connection`] handle is also
/// dropped.
///
/// See [`WeakConnection`] for a non-owning version of this [`Connection`].
#[must_use]
#[derive(Debug, Clone)]
pub struct Connection {
    /// Channel for sending commands to the [`ConnectionTask`].
    commands: mpsc::Sender<Command>,
}

impl Connection {
    /// Spawns an asynchronous task to process SOME/IP messages from the given address.
    ///
    /// Outgoing messages can be sent through the connection using the [`Connection::send`] method.
    ///
    /// Incoming messages are forwarded to the corresponding service interface in the given
    /// [`Interfaces`] collection.
    pub fn spawn<S, R>(
        (sender, receiver): (S, R),
        interfaces: Interfaces,
        peer_address: SocketAddr,
    ) -> Self
    where
        S: socket::Sender + 'static,
        R: socket::Receiver + 'static,
    {
        // Create a task for sending messages to the connected endpoint.
        let (outgoing, messages) = mpsc::channel(1);
        let sender = SenderTask {
            sender,
            commands: messages,
            interfaces: interfaces.clone(),
        };

        // Create a task for receiving messages from the connected endpoint.
        let receiver = ReceiverTask {
            receiver,
            interfaces,
            address: peer_address,
        };

        // Spawn an asynchronous task to handle the connection.
        tokio::task::spawn_local(async move {
            ConnectionTask::run(sender, receiver).await;
        });

        // Return a handle to the connection.
        Self { commands: outgoing }
    }

    /// Sends a SOME/IP message through the connection.
    ///
    /// # Errors
    ///
    /// Returns an error if the connection is closed.
    pub async fn send(&self, message: someip::Message<Bytes>) -> Result<()> {
        self.commands
            .send(Command::Message(message))
            .await
            .map_err(|_| "not connected")?;
        Ok(())
    }

    /// Takes ownership of the given [`Interface`].
    ///
    /// This means that the given interface will not be dropped until this connection is also
    /// dropped.
    ///
    /// # Errors
    ///
    /// Returns an error if the connection is closed.
    pub async fn take(&self, id: InterfaceId, interface: Interface) -> Result<()> {
        self.commands
            .send(Command::Take(id, interface))
            .await
            .map_err(|_| "not connected")?;
        Ok(())
    }

    /// Downgrades this [`Connection`] into to a [`WeakConnection`] which does not count towards
    /// ownership semantics.
    pub fn downgrade(&self) -> WeakConnection {
        WeakConnection {
            messages: self.commands.downgrade(),
        }
    }
}

/// Weak handle to an asynchronous [`ConnectionTask`].
///
/// This is a weak version of the normal [`Connection`] which does not count towards ownership
/// semantics of the underlying connection. As such, it will not prevent the connection from being
/// dropped when the last [`Connection`] handle is also dropped.
///
/// Use [`WeakConnection::upgrade`] to upgrade this [`WeakConnection`] into a normal [`Connection`].
#[must_use]
#[derive(Debug, Clone)]
pub struct WeakConnection {
    /// Weak channel for sending commands to the [`ConnectionTask`].
    messages: mpsc::WeakSender<Command>,
}

impl WeakConnection {
    /// Upgrades this [`WeakConnection`] into a normal [`Connection`].
    ///
    /// Returns [`None`] if the connection has already been closed.
    pub fn upgrade(&self) -> Option<Connection> {
        self.messages
            .upgrade()
            .map(|messages| Connection { commands: messages })
    }
}

/// A collection of async connection handles.
///
/// This contains both a local storage of owning [`Connection`] handles as well as a shared
/// storage of non-owning [`WeakConnection`] handles.
///
/// The local storage is used to associate the lifetime of the connections to this [`Connections`]
/// instance, while the shared storage is used to route SOME/IP messages to the corresponding
/// endpoints.
#[derive(Debug)]
pub struct Connections {
    /// Connection handles shared between [`Connections`].
    shared: Rc<scc::HashMap<SocketAddr, WeakConnection>>,
    /// Connection handles managed by this instance.
    local: scc::HashMap<SocketAddr, Connection>,
}

impl Connections {
    /// Creates a new [`Interfaces`].
    pub fn new() -> Self {
        Self {
            shared: Rc::new(scc::HashMap::default()),
            local: scc::HashMap::default(),
        }
    }

    /// Sends the message to the given address.
    ///
    /// This will search for connections whose remote address matches the given address and try to
    /// send the message through it.
    ///
    /// Performs some cleanup on the shared [`WeakConnection`] storage if it detects a closed
    /// connection.
    ///
    /// # Errors
    ///
    /// Returns an error if the connection does not exist or is closed.
    pub async fn send_to(
        &self,
        address: SocketAddr,
        message: someip::Message<Bytes>,
    ) -> Result<()> {
        use scc::hash_map::Entry;

        match self.shared.entry_async(address).await {
            Entry::Occupied(entry) => {
                if let Some(connection) = entry.upgrade() {
                    if connection.send(message).await.is_err() {
                        _ = entry.remove_entry();
                        Err("not connected")?;
                    }
                } else {
                    _ = entry.remove_entry();
                    Err("not connected")?;
                }
            }
            Entry::Vacant(_) => Err("not connected")?,
        }

        Ok(())
    }

    /// Adds the [`WeakConnection`] to the shared storage.
    ///
    /// This storage can be accessed by all clones of the instance.
    pub async fn share(&self, address: SocketAddr, connection: WeakConnection) {
        self.shared
            .entry_async(address)
            .await
            .insert_entry(connection);
    }

    /// Adds the [`Connection`] to the local storage.
    ///
    /// This storage can only be accessed by this instance.
    pub async fn store(&self, address: SocketAddr, connection: Connection) {
        self.local
            .entry_async(address)
            .await
            .insert_entry(connection);
    }

    /// Adds a non-owning version of the given connection to the shared storage, and adds the
    /// connection itself to the local storage.
    ///
    /// This is the same as calling [`Connections::share`] followed by [`Connections::store`].
    pub async fn share_and_store(&self, address: SocketAddr, connection: Connection) {
        self.share(address, connection.downgrade()).await;
        self.store(address, connection).await;
    }

    /// Gets a [`Connection`] to the given address from the shared storage.
    ///
    /// Returns an error if the connection does not exist or is closed.
    pub async fn get(&self, address: SocketAddr) -> Option<Connection> {
        self.shared
            .read_async(&address, |_, connection| connection.upgrade())
            .await
            .unwrap_or_default()
    }

    /// Checks if this instance contains an open connection handle to the given address.
    ///
    /// Performs some cleanup on the shared storage if it detects a closed connection.
    pub async fn check(&self, address: SocketAddr) -> bool {
        use scc::hash_map::Entry;

        match self.shared.entry_async(address).await {
            Entry::Occupied(entry) => entry.upgrade().is_some(),
            Entry::Vacant(_) => false,
        }
    }
}

impl Clone for Connections {
    /// Returns a copy of this [`Connections`] without the local handles.
    fn clone(&self) -> Self {
        Self {
            shared: self.shared.clone(),
            local: scc::HashMap::default(),
        }
    }
}

/// Asynchronous SOME/IP connection.
///
/// This task is used to asynchronously process incoming and outgoing SOME/IP messages from the
/// connected endpoint.
///
/// Use [`Connection::spawn`] to spawn this task and return a handle to it. Outgoing messages can
/// be sent through the connection using the [`Connection::send`] method.
///
/// Incoming messages are forwarded to the corresponding service interface in a shared
/// [`Interfaces`] collection.
#[derive(Debug)]
struct ConnectionTask;

impl ConnectionTask {
    /// Runs the given sender and receiver tasks asynchronously.
    ///
    /// This method will return once either of the task stops running.
    async fn run<S: socket::Sender, R: socket::Receiver>(
        sender: SenderTask<S>,
        receiver: ReceiverTask<R>,
    ) {
        tokio::select! {
            () = sender.run() => {}
            () = receiver.run() => {}
        }
    }
}

/// Asynchronous SOME/IP message receiver task.
///
/// This task is used to asynchronously poll a [`socket::Receiver`] for incoming SOME/IP messages
/// and forward these to the corresponding service interface.
pub struct ReceiverTask<R>
where
    R: socket::Receiver + 'static,
{
    /// Receiver half of the connection.
    receiver: R,
    /// Services using this endpoint.
    interfaces: Interfaces,
    /// Address of the remote endpoint.
    address: SocketAddr,
}

impl<R> ReceiverTask<R>
where
    R: socket::Receiver,
{
    /// Runs the task asynchronously.
    ///
    /// This method returns once the [`socket::Receiver`] is closed.
    async fn run(mut self) {
        self.recv().await;
    }

    /// Receives SOME/IP messages from the [`socket::Receiver`].
    ///
    /// This method will continuously poll and deserialize messages from the socket and forward
    /// these to the corresponding service interface.
    ///
    /// Messages that are malformed or addressed to unknown services will be dropped.
    ///
    /// This method runs until the receiver is closed.
    async fn recv(&mut self) {
        'recv: loop {
            match self.receiver.recv().await {
                Ok(mut buffer) => {
                    // Extract messages from the buffer.
                    match Vec::<Message<Bytes>>::deserialize(&mut buffer) {
                        Ok(messages) => {
                            // Route each message to the correct service.
                            let messages = messages.into_iter().map(|msg| (msg, self.address));
                            self.interfaces.send_all(messages).await;
                        }
                        Err(error) => {
                            eprintln!("deserialization failed: {error:?}");
                            continue 'recv;
                        }
                    };
                }
                Err(RecvError::Lagged(count)) => {
                    // Took too long to process packets and some were lost.
                    eprintln!("lagged behind {count} packets");
                    continue 'recv;
                }
                Err(RecvError::Closed) => {
                    // Socket is closed and will not receive any more data.
                    eprintln!("receiver is closed");
                    break 'recv;
                }
            }
        }
    }
}

/// Asynchronous SOME/IP message sender task.
///
/// This task is used to asynchronously send SOME/IP messages to the connected endpoint.
pub struct SenderTask<S>
where
    S: socket::Sender,
{
    /// Sender half of the connection.
    sender: S,
    /// Receiver of connection commands.
    commands: mpsc::Receiver<Command>,
    /// Service interfaces bound to this connection.
    interfaces: Interfaces,
}

impl<S> SenderTask<S>
where
    S: socket::Sender,
{
    /// Runs the task asynchronously.
    ///
    /// This method returns once all [`Connection`] handles are dropped or the [`socket::Sender`]
    /// is closed.
    async fn run(mut self) {
        _ = self.commands().await;
    }

    /// Processes commands sent by the [`Connection`] handle.
    ///
    /// SOME/IP messages are forwarded to the connected endpoint and [`Interface`] handles are
    /// added to the local storage.
    ///
    /// This method returns once all [`Connection`] handles are dropped or the [`socket::Sender`]
    /// is closed.
    async fn commands(&mut self) -> Result<()> {
        while let Some(command) = self.commands.recv().await {
            match command {
                Command::Message(message) => self.send(message).await?,
                Command::Take(id, interface) => self.store(id, interface).await,
            }
        }
        Ok(())
    }

    /// Sends the message through the connected endpoint.
    ///
    /// Malformed messages are silently dropped.
    ///
    /// # Errors
    ///
    /// Returns an error if the [`socket::Sender`] is closed.
    async fn send(&mut self, message: someip::Message<Bytes>) -> Result<()> {
        let mut buffer = BytesMut::with_capacity(someip::HEADER_SIZE + message.payload.len());
        if let Err(error) = message.serialize(&mut buffer) {
            eprintln!("serialization failed {error:?}");
        };
        if self.sender.send(buffer.freeze()).await.is_err() {
            eprintln!("socket is closed");
            Err("socket is closed")?;
        }
        Ok(())
    }

    /// Adds the [`Interface`] to the local storage.
    ///
    /// This means that the given interface will only be dropped once this task is also dropped.
    async fn store(&self, id: InterfaceId, interface: Interface) {
        self.interfaces.store(id, interface).await;
    }
}

/// Connection commands.
///
/// These are sent from a [`Connection`] to be processed by the corresponding [`ConnectionTask`].
enum Command {
    /// Send the SOME/IP message to the connected endpoint.
    Message(someip::Message<Bytes>),
    /// Take ownership of the given [`Interface`].
    Take(InterfaceId, Interface),
}
