//! Listener for SOME/IP connections.
//!
//! This module provides types for creating and managing an asynchronous connection listener.
//!
//! - [`Listener`] is a handle to an asynchronous task for processing incoming SOME/IP connections.
//!
//! - [`WeakListener`] is a non-owning version of a regular [`Listener`].
//!
//! - [`ListenerTask`] is an asynchronous task for processing incoming SOME/IP connections.

use super::{Connection, Connections, Interface, InterfaceId, Interfaces};
use crate::{socket, Result};
use tokio::sync::mpsc;

/// Handle to an asynchronous listener for incoming SOME/IP connections.
///
/// This is used to manage the lifetime of the listener.
///
/// # Ownership
///
/// A [`Listener`] counts towards the ownership semantics of the underlying listener. This
/// means that the listener will only be dropped when the last [`Listener`] handle is also
/// dropped.
///
/// See [`WeakListener`] for a non-owning version of this [`Listener`].
///
/// Accepted connections will initially be owned by the asynchronous listener, which means that
/// they will be dropped once the listener is also dropped.
#[must_use]
#[derive(Debug, Clone)]
pub struct Listener {
    /// Channel for sending commands to the [`ListenerTask`].
    commands: mpsc::Sender<Command>,
}

impl Listener {
    /// Spawns an asynchronous task to process incoming SOME/IP connections.
    ///
    /// Accepted connections will be given copies of the shared `connections` and `interfaces`
    /// collections.
    pub fn spawn<L>(listener: L, connections: Connections, interfaces: Interfaces) -> Self
    where
        L: socket::Listener + 'static,
    {
        // Create a task for accepting the connections.
        let listener = AcceptTask {
            listener,
            connections,
            interfaces: interfaces.clone(),
        };

        // Create a task for processing the commands.
        let (tx_commands, commands) = mpsc::channel(8);
        let command = CommandTask {
            commands,
            interfaces,
        };

        // Spawn an asynchronous task.
        tokio::task::spawn_local(async move {
            ListenerTask::run(listener, command).await;
        });

        // Return a handle to the listener.
        Self {
            commands: tx_commands,
        }
    }

    /// Takes ownership of the given [`Interface`].
    ///
    /// This means that the given interface will not be dropped until this listener is also
    /// dropped.
    ///
    /// # Errors
    ///
    /// Returns an error if the listener is closed.
    pub async fn take(&self, id: InterfaceId, interface: Interface) -> Result<()> {
        self.commands.send(Command::Take(id, interface)).await?;
        Ok(())
    }

    /// Downgrades this [`Listener`] into to a [`WeakListener`] which does not count towards
    /// ownership semantics.
    pub fn downgrade(&self) -> WeakListener {
        WeakListener {
            commands: self.commands.downgrade(),
        }
    }
}

/// Weak handle to an asynchronous [`ListenerTask`].
///
/// This is a weak version of the normal [`Listener`] which does not count towards ownership
/// semantics of the underlying listener. As such, it will not prevent the listener from being
/// dropped when the last [`Listener`] is also dropped.
///
/// Use [`WeakListener::upgrade`] to upgrade this [`WeakListener`] into a normal [`Connection`].
#[must_use]
#[derive(Debug, Clone)]
pub struct WeakListener {
    /// Weak channel for sending commands to the [`ListenerTask`].
    commands: mpsc::WeakSender<Command>,
}

impl WeakListener {
    /// Creates a new [`WeakListener`].
    ///
    /// This listener is not associated with any [`Listener`] or [`ListenerTask`]. As such,
    /// [`WeakListener::upgrade`] will always return [`None`].
    ///
    /// Use [`Listener::downgrade`] if you which to create a valid [`WeakListener`] instead.
    pub fn new() -> Self {
        // This is a hacky was of creating a new WeakSender that's not actually connected to
        // anything.
        let (sender, _) = mpsc::channel(1);
        Self {
            commands: sender.downgrade(),
        }
    }

    /// Upgrades this [`WeakListener`] into a normal [`Listener`].
    ///
    /// Returns [`None`] if the listener has already been closed.
    pub fn upgrade(&self) -> Option<Listener> {
        self.commands
            .upgrade()
            .map(|commands| Listener { commands })
    }
}

/// Asynchronous listener for incoming SOME/IP connections.
///
/// Use [`Listener::spawn`] to spawn this task and return a handle to it.
struct ListenerTask;

impl ListenerTask {
    /// Runs the given accept and command tasks asynchronously.
    ///
    /// This method will return once either of the tasks stops running.
    async fn run<L>(mut listener: AcceptTask<L>, mut command: CommandTask)
    where
        L: socket::Listener + 'static,
    {
        tokio::select! {
            () = listener.run() => {}
            () = command.run() => {}
        }
    }
}

/// Asynchronous listener for incoming SOME/IP connections.
///
/// This task is used to asynchronously poll a [`socket::Listener`] for incoming SOME/IP connections
/// and turn them into [`Connection`]s.
struct AcceptTask<L>
where
    L: socket::Listener + 'static,
{
    /// Listener for incoming connections.
    listener: L,
    /// Connections associated with the endpoint.
    connections: Connections,
    /// Service interface associated with the endpoint.
    interfaces: Interfaces,
}

impl<L> AcceptTask<L>
where
    L: socket::Listener + 'static,
{
    /// Runs the task asynchronously.
    ///
    /// This method returns once the [`socket::Listener`] is closed.
    async fn run(&mut self) {
        self.accept().await;
    }

    /// Accepts incoming SOME/IP connections for the [`socket::Listener`].
    ///
    /// This method will continuously accept connections and create corresponding [`Connection`]s.
    ///
    /// This method runs until the listener is closed.
    async fn accept(&mut self) {
        'listen: loop {
            if let Some((connection, address)) = self.listener.accept().await {
                let connection = Connection::spawn(connection, self.interfaces.clone(), address);
                self.connections.share_and_store(address, connection).await;
            } else {
                eprintln!("listener closed");
                break 'listen;
            }
        }
    }
}

/// Asynchronous processor of [`Listener`] commands.
///
/// This task is mainly used to manage the lifetime of the [`ListenerTask`] and of service
/// [`Interface`] handles.
struct CommandTask {
    /// Receiver of listener commands.
    commands: mpsc::Receiver<Command>,
    /// Locally managed interfaces.
    interfaces: Interfaces,
}

impl CommandTask {
    /// Runs the task asynchronously.
    ///
    /// This method returns once all [`Listener`] handles are dropped.
    async fn run(&mut self) {
        self.command().await;
    }

    /// Processes commands sent by the [`Listener`] handle.
    ///
    /// This method returns once all [`Listener`] handles are dropped.
    async fn command(&mut self) {
        while let Some(command) = self.commands.recv().await {
            match command {
                Command::Take(id, interface) => self.interfaces.store(id, interface).await,
            }
        }
    }
}

/// Listener commands.
///
/// These are sent from a [`Listener`] to be processed by the corresponding [`ListenerTask`].
enum Command {
    /// Take ownership of the given [`Interface`].
    Take(InterfaceId, Interface),
}
