//! SOME/IP endpoint, Protocol Version 1.
//!
//! This module provides types for creating SOME/IP endpoints which can be used to serve and consume
//! SOME/IP service instances.
//!
//! - [`Endpoint`] is a handle to an asynchronous SOME/IP endpoint.
//!
//! - [`WeakEndpoint`] is a non-owning version of the regular [`Endpoint`].
//!
//! - [`Stub`] is used to send and receive messages from multiple remote endpoints.
//!
//! - [`Proxy`] is used to send and receive messages from a single remote endpoint.

use crate::{
    endpoint::{self, InterfaceId},
    socket::{self, SocketAddr},
    someip, Result,
};
use rsomeip_bytes::Bytes;
use tokio::sync::{mpsc, oneshot};

mod connection;
use connection::{Connection, Connections};

mod interface;
use interface::{Interface, Interfaces};

mod listener;
use listener::{Listener, WeakListener};

/// Handle to an asynchronous SOME/IP endpoint.
///
/// Used to create [`Stub`] and [`Proxy`] handles to service interfaces on local and remote
/// endpoints, respectively.
///
/// # Examples
///
/// Create a [`Stub`] to offer a service on this endpoint:
///
/// ```rust
/// use rsomeip::{
///     endpoint::{InterfaceId, Server as _, v1::Endpoint},
///     socket::udp::UdpSocket,
/// };
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// # tokio::task::LocalSet::new().run_until(async {
///
/// // Create a UDP or TCP socket.
/// let address = "127.0.0.1:46834".parse()?;
/// let socket = UdpSocket::bind(address).await?;
///
/// // Create the endpoint.
/// let mut endpoint = Endpoint::spawn(socket);
///
/// // Create a service stub.
/// let id = InterfaceId::new(0x1234, 01);
/// let stub = endpoint.serve(id).await?;
///
/// # Ok(())
/// # }).await
/// # }
/// ```
///
/// Create a [`Proxy`] to consume a service on a remote endpoint:
///
/// ```rust
/// use rsomeip::{
///     endpoint::{InterfaceId, Server as _, v1::Endpoint},
///     socket::udp::UdpSocket,
/// };
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// # tokio::task::LocalSet::new().run_until(async {
///
/// // Create a UDP or TCP socket.
/// let address = "127.0.0.1:38603".parse()?;
/// let socket = UdpSocket::bind(address).await?;
///
/// // Create the endpoint.
/// let mut endpoint = Endpoint::spawn(socket);
///
/// // Create a service proxy.
/// let id = InterfaceId::new(0x1234, 01);
/// let remote = "127.0.0.2:38603".parse()?;
/// let proxy = endpoint.proxy(id, remote).await?;
///
/// # Ok(())
/// # }).await
/// # }
/// ```
#[must_use]
#[derive(Debug, Clone)]
pub struct Endpoint {
    /// Channel for sending commands to the [`EndpointTask`].
    commands: mpsc::Sender<Command>,
}

impl Endpoint {
    /// Creates a new [`Endpoint`].
    fn new(commands: mpsc::Sender<Command>) -> Self {
        Self { commands }
    }

    /// Spawns an asynchronous task to manage SOME/IP service interfaces.
    ///
    /// The given `connector` is used to establish the underlying connections to other endpoints.
    ///
    /// # Runtime
    ///
    /// This method must only be run inside of a [`tokio::task::LocalSet`].
    pub fn spawn<C>(connector: C) -> Self
    where
        C: socket::Connector + 'static,
    {
        // Create a handle to the task.
        let (tx_commands, rx_commands) = mpsc::channel(8);
        let endpoint = Self::new(tx_commands);

        // Spawn the asynchronous task to manage the endpoint.
        let mut task = EndpointTask::new(connector, rx_commands, endpoint.downgrade());
        tokio::task::spawn_local(async move { task.run().await });

        // Return the handle to the task.
        endpoint
    }

    /// Establishes a [`Connection`] to the given address.
    ///
    /// # Errors
    ///
    /// Returns an error if the connection cannot be established, or if the endpoint is closed.
    async fn connect(&self, address: SocketAddr) -> Result<Connection> {
        let (tx, rx) = oneshot::channel();
        self.commands
            .send(Command::Connect { address, tx })
            .await
            .map_err(|_| "endpoint closed")?;
        let connection = rx.await??;
        Ok(connection)
    }

    /// Downgrades this [`Endpoint`] into to a [`WeakEndpoint`] which does not count towards
    /// ownership semantics.
    pub fn downgrade(&self) -> WeakEndpoint {
        WeakEndpoint {
            commands: self.commands.downgrade(),
        }
    }
}

impl endpoint::Server for Endpoint {
    // Concrete type of the endpoint stub.
    type Stub = Stub;
    // Concrete type of the endpoint proxy.
    type Proxy = Proxy;

    async fn serve(&mut self, interface: InterfaceId) -> Result<Self::Stub> {
        let (tx, rx) = oneshot::channel();
        self.commands
            .send(Command::Serve { interface, tx })
            .await
            .map_err(|_| "endpoint closed")?;
        let stub = rx.await??;
        Ok(stub)
    }

    async fn proxy(&mut self, interface: InterfaceId, address: SocketAddr) -> Result<Self::Proxy> {
        let (tx, rx) = oneshot::channel();
        self.commands
            .send(Command::Proxy {
                interface,
                address,
                tx,
            })
            .await
            .map_err(|_| "endpoint closed")?;
        let proxy = rx.await??;
        Ok(proxy)
    }
}

/// Weak handle to an asynchronous SOME/IP endpoint.
///
/// This is a weak version of the normal [`Endpoint`] which does not count towards ownership
/// semantics of the endpoint. As such, it will not prevent the endpoint from being dropped when
/// the last [`Endpoint`] handle is also dropped.
///
/// Use [`WeakEndpoint::upgrade`] to upgrade this [`WeakEndpoint`] into a normal [`Endpoint`].
#[derive(Debug, Clone)]
pub struct WeakEndpoint {
    /// Weak channel for sending commands to the [`EndpointTask`].
    commands: mpsc::WeakSender<Command>,
}

impl WeakEndpoint {
    /// Upgrades this [`WeakEndpoint`] into a normal [`Endpoint`].
    ///
    /// Returns [`None`] if the endpoint has already been dropped.
    pub fn upgrade(&self) -> Option<Endpoint> {
        self.commands
            .upgrade()
            .map(|commands| Endpoint { commands })
    }
}

/// Stub of a local service interface.
///
/// This is used to asynchronously send and receive SOME/IP messages addressed to a specific
/// service interface.
///
/// Unlike a [`Proxy`] which can only communicate with one remote endpoint, a [`Stub`] is able
/// to communicate with several remote endpoints simultaneously.
///
/// [`Stub`] handles are created using the [`Endpoint::serve`] method.
///
/// [`Endpoint::serve`]: crate::endpoint::Server::serve
#[must_use]
pub struct Stub {
    /// Connections to remote endpoints.
    connections: Connections,
    /// Incoming SOME/IP message receiver.
    receiver: mpsc::Receiver<(someip::Message<Bytes>, SocketAddr)>,
    /// Non-owning handle to the endpoint.
    endpoint: WeakEndpoint,
    /// Handle to the connection listener.
    _listener: Listener,
}

impl Stub {
    /// Establishes a [`Connection`] to a remote endpoint.
    ///
    /// # Errors
    ///
    /// Returns an error if the connection cannot be established or if the endpoint has already
    /// been dropped.
    async fn connect(&self, address: SocketAddr) -> Result<()> {
        if let Some(endpoint) = self.endpoint.upgrade() {
            let connection = endpoint.connect(address).await?;
            self.connections.share_and_store(address, connection).await;
        } else {
            Err("not connected")?;
        };
        Ok(())
    }
}

impl endpoint::Stub for Stub {
    // Sends the message to the given address.
    async fn send_to(
        &mut self,
        address: SocketAddr,
        message: someip::Message<Bytes>,
    ) -> Result<()> {
        if !self.connections.check(address).await {
            self.connect(address).await?;
        }
        self.connections.send_to(address, message).await
    }

    // Receives a message from a remote address.
    async fn recv_from(&mut self) -> Result<(someip::Message<Bytes>, SocketAddr)> {
        match self.receiver.recv().await {
            Some(message) => Ok(message),
            None => Err("connection is closed")?,
        }
    }
}

/// Proxy of a remote service interface.
///
/// This is used to asynchronously send and receive SOME/IP messages addressed to a specific
/// service interface.
///
/// Unlike a [`Stub`] which can communicate with several remote endpoints simultaneously, a
/// [`Proxy`] is only able to communicate with one remote endpoint.
///
/// [`Proxy`] handles are created using the [`Endpoint::proxy`] method.
///
/// [`Endpoint::proxy`]: crate::endpoint::Server::proxy
#[must_use]
#[derive(Debug)]
pub struct Proxy {
    /// Connection to remote endpoint.
    connection: Connection,
    /// SOME/IP message receiver.
    receiver: mpsc::Receiver<(someip::Message<Bytes>, SocketAddr)>,
    /// Remote endpoint address.
    remote: SocketAddr,
}

impl endpoint::Proxy for Proxy {
    // Sends the message to the given address.
    async fn send(&mut self, message: someip::Message<Bytes>) -> Result<()> {
        self.connection.send(message).await
    }

    // Receives a message from the remote address.
    async fn recv(&mut self) -> Result<someip::Message<Bytes>> {
        'recv: loop {
            match self.receiver.recv().await {
                Some((message, source)) => {
                    if self.remote == source {
                        return Ok(message);
                    }
                    continue 'recv;
                }
                None => {
                    // Connection is closed.
                    Err("endpoint is closed")?;
                }
            }
        }
    }
}

/// Asynchronously SOME/IP endpoint.
///
/// This task is used to establish connections to remote SOME/IP endpoints and manage local service
/// interfaces.
///
/// Use [`Endpoint::spawn`] to spawn this task and return a handle to it.
struct EndpointTask<C>
where
    C: socket::Connector,
{
    /// Used to establish remote connections.
    connector: C,
    /// Receiver of [`Endpoint`] commands.
    commands: mpsc::Receiver<Command>,
    /// Weak handle to the asynchronous listener.
    listener: WeakListener,
    /// Connections associated with this endpoint.
    connections: Connections,
    /// Service interfaces associated with this endpoint.
    interfaces: Interfaces,
    /// Non-owning self-reference.
    endpoint: WeakEndpoint,
}

impl<C> EndpointTask<C>
where
    C: socket::Connector,
{
    /// Creates a new [`EndpointTask<C>`].
    fn new(connector: C, commands: mpsc::Receiver<Command>, endpoint: WeakEndpoint) -> Self {
        Self {
            connector,
            commands,
            listener: WeakListener::new(),
            connections: Connections::new(),
            interfaces: Interfaces::new(),
            endpoint,
        }
    }

    /// Runs the task asynchronously.
    ///
    /// This method returns once all [`Endpoint`] handles are dropped.
    async fn run(&mut self) {
        self.command().await;
    }

    /// Process commands sent by the [`Endpoint`] handle.
    ///
    /// This method returns once all [`Endpoint`] handles are dropped.
    async fn command(&mut self) {
        while let Some(command) = self.commands.recv().await {
            match command {
                Command::Connect { address, tx } => {
                    _ = tx.send(self.connect(address).await);
                }
                Command::Serve { interface, tx } => _ = tx.send(self.stub(interface).await),
                Command::Proxy {
                    interface,
                    address,
                    tx,
                } => _ = tx.send(self.proxy(interface, address).await),
            }
        }
    }

    /// Establishes a [`Connection`] to the given address.
    ///
    /// This is meant to be called only by [`Stub`] trying to establish connections to remote
    /// endpoints.
    ///
    /// [`Proxy`] connections should be established using the [`EndpointTask::proxy`] or
    /// [`EndpointTask::connection_or_connect`].
    ///
    /// # Errors
    ///
    /// Returns an error if the connection cannot be established.
    ///
    /// # Protocol Type
    ///
    /// Depending on the [`ProtocolType`] of the underlying communication protocol, a [`Stub`]
    /// may or may not be able to establish connections to remote endpoints.
    ///
    /// `Datagram` type protocols allow this behavior, but `Stream` type protocols require that the
    /// client be the one to establish the connection.
    async fn connect(&mut self, address: SocketAddr) -> Result<Connection> {
        if matches!(C::PROTOCOL_TYPE, socket::ProtocolType::Datagram(_)) {
            let connection = self.connector.connect(&address).await?;
            Ok(Connection::spawn(
                connection,
                self.interfaces.clone(),
                address,
            ))
        } else {
            Err("not allowed")?
        }
    }

    /// Creates a [`Stub`] to the given service interface.
    ///
    /// This will create a [`Listener`] to accept incoming connections, if one does not already
    /// exist.
    ///
    /// # Errors
    ///
    /// Returns an error if the interface already exists or if the listener cannot be created.
    async fn stub(&mut self, id: InterfaceId) -> Result<Stub> {
        // Check if the interface already exists.
        if self.interfaces.check(id).await {
            return Err("already exists")?;
        }

        // Get or create a listener for incoming connections.
        let listener = self.listener_or_listen().await?;

        // Add the new interface to the listener.
        let (interface, receiver) = self.create_interface(id).await;
        listener.take(id, interface).await?;

        // Return a service stub.
        Ok(Stub {
            connections: self.connections.clone(),
            _listener: listener,
            receiver,
            endpoint: self.endpoint.clone(),
        })
    }

    /// Returns this endpoints [`Listener`] or creates a new one.
    ///
    /// # Errors
    ///
    /// Returns an error if the listener cannot be created.
    async fn listener_or_listen(&mut self) -> Result<Listener> {
        let listener = match self.listener.upgrade() {
            Some(listener) => listener,
            None => self.listen().await?,
        };
        self.listener = listener.downgrade();
        Ok(listener)
    }

    /// Creates a new [`Listener`].
    ///
    /// # Errors
    ///
    /// Returns an error if the listener cannot be created.
    async fn listen(&mut self) -> Result<Listener> {
        let listener = self.connector.listen(8).await?;
        Ok(Listener::spawn(
            listener,
            self.connections.clone(),
            self.interfaces.clone(),
        ))
    }

    /// Creates a [`Proxy`] to the given service interface.
    ///
    /// This will establish a [`Connection`] to the remote endpoint, if one does not already exist.
    ///
    /// # Errors
    ///
    /// Returns an error if the interface already exists or if the connection cannot be established.
    async fn proxy(&mut self, id: InterfaceId, address: SocketAddr) -> Result<Proxy> {
        // Check if the interface already exists.
        if self.interfaces.check(id).await {
            return Err("already exists")?;
        }

        // Get or create a connection to the remote address.
        let connection = self.connection_or_connect(address).await?;

        // Create the interface and bind it to the connection.
        let (interface, receiver) = self.create_interface(id).await;
        connection.take(id, interface).await?;

        // Return a service proxy.
        Ok(Proxy {
            connection,
            receiver,
            remote: address,
        })
    }

    /// Returns the [`Connection`] to the given address or creates a new one.
    ///
    /// # Errors
    ///
    /// Returns an error if the connection cannot be established.
    async fn connection_or_connect(&mut self, address: SocketAddr) -> Result<Connection> {
        if let Some(connection) = self.connections.get(address).await {
            return Ok(connection);
        }
        Ok(Connection::spawn(
            self.connector.connect(&address).await?,
            self.interfaces.clone(),
            address,
        ))
    }

    /// Creates an [`Interface`] handle and adds it to the endpoint's shared storage.
    async fn create_interface(
        &self,
        id: InterfaceId,
    ) -> (
        Interface,
        mpsc::Receiver<(someip::Message<Bytes>, SocketAddr)>,
    ) {
        let (interface, receiver) = Interface::channel(16);
        self.interfaces.share(id, interface.downgrade()).await;
        (interface, receiver)
    }
}

/// Endpoint commands.
///
/// These are sent from a [`Endpoint`] to be processed by the corresponding [`EndpointTask`].
enum Command {
    /// Establish a connection to the given address.
    Connect {
        /// Address to connect to.
        address: SocketAddr,
        /// Response channel.
        tx: oneshot::Sender<Result<Connection>>,
    },
    /// Serve the given interface on this endpoint.
    Serve {
        /// Interface to serve.
        interface: InterfaceId,
        /// Response channel.
        tx: oneshot::Sender<Result<Stub>>,
    },
    /// Proxy the given interface on the remote address.
    Proxy {
        /// Interface to proxy.
        interface: InterfaceId,
        /// Address to connect to.
        address: SocketAddr,
        /// Response channel.
        tx: oneshot::Sender<Result<Proxy>>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        endpoint::{Proxy as _, Server as _, Stub as _},
        socket::udp::UdpSocket,
        someip,
        testing::ipv4,
    };

    #[tokio::test]
    async fn service_handles_requests() {
        tokio::task::LocalSet::new()
            .run_until(async {
                // Create a local endpoint.
                let local_address = ipv4!();
                let socket = UdpSocket::bind(local_address)
                    .await
                    .expect("should bind to the address");
                let mut local = super::Endpoint::spawn(socket);

                // Serve an interface on the endpoint.
                let interface = InterfaceId::new(0x1234, 1);
                let mut stub = local
                    .serve(interface)
                    .await
                    .expect("should serve the service");

                // Create a remote endpoint.
                let peer_address = ipv4!();
                let socket = UdpSocket::bind(peer_address)
                    .await
                    .expect("should bind to the address");
                let mut peer = super::Endpoint::spawn(socket);

                // Create a proxy to the local endpoint.
                let mut proxy = peer
                    .proxy(interface, local_address)
                    .await
                    .expect("should create a proxy");

                // Send a message to the endpoint.
                let message = someip::Message::new(Bytes::copy_from_slice(&[1u8]))
                    .with_service(interface.service)
                    .with_interface(interface.version)
                    .with_type(someip::MessageType::Request);
                proxy.send(message).await.expect("should send the message");

                // Process the message.
                let (message, address) = stub.recv_from().await.expect("should receive a message");
                assert_eq!(address, peer_address);
                assert_eq!(message.service, interface.service);
                assert_eq!(message.interface, interface.version);
                assert_eq!(message.message_type, someip::MessageType::Request);
                stub.send_to(
                    peer_address,
                    message.with_type(someip::MessageType::Response),
                )
                .await
                .expect("should send the message");

                // Wait for the response.
                let message = proxy.recv().await.expect("should deserialize the message");
                assert_eq!(message.service, interface.service);
                assert_eq!(message.interface, interface.version);
                assert_eq!(message.message_type, someip::MessageType::Response);
                assert_eq!(&message.payload[..], &[1u8]);
            })
            .await;
    }

    #[tokio::test]
    async fn service_handles_notifications() {
        tokio::task::LocalSet::new()
            .run_until(async {
                // Create a local endpoint.
                let local_address = ipv4!();
                let socket = UdpSocket::bind(local_address)
                    .await
                    .expect("should bind to the address");
                let mut local = super::Endpoint::spawn(socket);

                // Serve an interface on the endpoint.
                let interface = InterfaceId::new(0x1234, 1);
                let mut stub = local
                    .serve(interface)
                    .await
                    .expect("should serve the service");

                // Create a remote endpoint.
                let peer_address = ipv4!();
                let socket = UdpSocket::bind(peer_address)
                    .await
                    .expect("should bind to the address");
                let mut peer = super::Endpoint::spawn(socket);

                // Create a proxy to the local endpoint.
                let mut proxy = peer
                    .proxy(interface, local_address)
                    .await
                    .expect("should create a proxy");

                // Send a notification to the remote endpoint.
                let message = someip::Message::new(Bytes::copy_from_slice(&[1u8]))
                    .with_service(interface.service)
                    .with_interface(interface.version)
                    .with_type(someip::MessageType::Notification);
                stub.send_to(peer_address, message)
                    .await
                    .expect("should send the message");

                // Process the message.
                let message = proxy.recv().await.expect("should receive a message");
                assert_eq!(message.service, interface.service);
                assert_eq!(message.interface, interface.version);
                assert_eq!(message.message_type, someip::MessageType::Notification);
                assert_eq!(&message.payload[..], &[1u8]);
            })
            .await;
    }

    #[tokio::test]
    async fn duplicate_interfaces_not_allowed() {
        tokio::task::LocalSet::new()
            .run_until(async {
                // Create a local endpoint.
                let local_address = ipv4!();
                let socket = UdpSocket::bind(local_address)
                    .await
                    .expect("should bind to the address");
                let mut endpoint = super::Endpoint::spawn(socket);

                // Serve an interface on the endpoint.
                let interface = InterfaceId::new(0x1234, 1);
                let _stub = endpoint
                    .serve(interface)
                    .await
                    .expect("should serve the service");

                // Try to proxy the same interface,
                let peer_address = ipv4!();
                let _proxy = endpoint
                    .proxy(interface, peer_address)
                    .await
                    .expect_err("should not allow duplicate interfaces");
            })
            .await;
    }
}
