//! Async runtime.

use std::{sync::Arc, thread};

mod state;
use state::StateWatcher;

mod shared;
use shared::Shared;

mod task;
pub use task::Scope;
use task::Task;

mod registry {
    use crate::{endpoint::InterfaceId, someip::InstanceId};
    use std::{net::SocketAddr, num::NonZero};

    pub struct Registry {
        interfaces: scc::HashMap<InterfaceId, InterfaceInfo>,
    }

    pub struct InterfaceInfo {
        instances: scc::HashMap<InstanceId, InstanceInfo>,
    }

    pub struct InstanceInfo {
        endpoints: scc::HashSet<(SocketAddr, Protocol)>,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Protocol(pub u8);

    impl Protocol {
        /// Transmission Control Protocol.
        ///
        /// TCP is a connection-oriented, reliable, ordered, and error-checked communication
        /// protocol of the Internet Protocol suite.
        pub const TCP: Self = Self(0x06);

        /// User Datagram Protocol.
        ///
        /// UDP is a connection-less, datagram-based communication protocol of the Internet
        /// Protocol suite designed for speed and efficiency.
        pub const UDP: Self = Self(0x11);
    }

    impl std::fmt::Display for Protocol {
        /// Formats the [`Protocol`] into a string.
        ///
        /// # Examples
        ///
        /// ```rust
        /// use rsomeip::runtime::Protocol;
        ///
        /// assert_eq!(format!("{}", Protocol::UDP), "UDP");
        /// assert_eq!(format!("{}", Protocol::TCP), "TCP");
        /// assert_eq!(format!("{}", Protocol(0x01)), "Other(01)");
        /// ```
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match *self {
                Self::TCP => write!(f, "TCP"),
                Self::UDP => write!(f, "UDP"),
                Self(other) => write!(f, "Other({other:02x?})"),
            }
        }
    }
}
pub use registry::Protocol;

/// SOME/IP asynchronous runtime.
///
/// Used to create service interfaces and communicate with other SOME/IP applications
/// asynchronously.
#[derive(Debug)]
pub struct Runtime {
    // Data shared between the runtime and its threads.
    shared: Arc<Shared>,
    // Parallel threads of execution.
    threads: Vec<thread::JoinHandle<()>>,
}

impl Runtime {
    /// Creates a new [`Runtime`].
    pub fn new() -> Self {
        Self {
            shared: Arc::new(Shared::new()),
            threads: Vec::new(),
        }
    }

    /// Spawns the given future in a separate thread.
    ///
    /// The future will run until it completes or the runtime is stopped.
    ///
    /// # Panics
    ///
    /// Panics if an async runtime cannot be created.
    #[expect(clippy::expect_used)]
    pub fn spawn<F>(&mut self, future: F)
    where
        F: AsyncFnOnce(Scope) + Send + 'static,
    {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("could not create async runtime");
        self.spawn_with(runtime, future);
    }

    /// Spawns the given future in a separate thread.
    ///
    /// Uses the given runtime to drive the future.
    ///
    /// The future will run until it completes or the runtime is stopped.
    pub fn spawn_with<F>(&mut self, runtime: tokio::runtime::Runtime, future: F)
    where
        F: AsyncFnOnce(Scope) + Send + 'static,
    {
        let shared = self.shared.clone();
        let handle = std::thread::spawn(move || Task::new(runtime, shared).run(future));
        self.threads.push(handle);
    }

    /// Starts the runtime.
    ///
    /// Blocks the current thread until stopped, or until all spawn threads complete their
    /// execution.
    pub fn run(&mut self) {
        // Notify all threads to begin executing their futures.
        self.shared.state().run();

        // Wait for all threads to finish execution.
        self.join_threads();

        // Ensure that the state is updated. This is necessary in case `stop` was not called.
        self.shared.state().stop();
    }

    /// Runs the given future until it completes, blocking the current thread.
    ///
    /// # Panics
    ///
    /// Panics if the async runtime cannot be created.
    #[expect(clippy::expect_used)]
    pub fn run_until<F>(&mut self, future: F)
    where
        F: AsyncFnOnce(Scope),
    {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("could not create async runtime");
        self.run_with(runtime, future);
    }

    /// Runs the given future until it completes, blocking the current thread.
    ///
    /// Uses the provided `tokio` runtime to drive the future.
    pub fn run_with<F>(&mut self, runtime: tokio::runtime::Runtime, future: F)
    where
        F: AsyncFnOnce(Scope),
    {
        // Notify all threads to being executing their futures.
        self.shared.state().run();

        // Run the future in the current thread and wait for it to complete.
        Task::new(runtime, self.shared.clone()).run(future);

        // Notify all threads to stop execution.
        self.shared.state().stop();

        // Wait for all threads to finish execution.
        self.join_threads();
    }

    /// Joins all spawned threads with the current thread.
    fn join_threads(&mut self) {
        for thread in self.threads.drain(0..) {
            let id = thread.thread().id();
            match thread.join() {
                Ok(()) => {
                    eprintln!("Thread {id:?} joined.");
                }
                Err(_) => {
                    eprintln!("Thread {id:?} not joined.");
                }
            };
        }
    }
}

impl Default for Runtime {
    // Creates a new [`Runtime`].
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    };

    #[test]
    fn run() {
        // Create a runtime.
        let mut runtime = Runtime::new();

        // Create a flag to share between threads.
        let block_executed = Arc::new(AtomicBool::new(false));
        let executed_ref = block_executed.clone();

        // Spawn a new task to change the flag.
        runtime.spawn(async move |_: Scope| {
            executed_ref.store(true, Ordering::SeqCst);
        });

        // The spawned task should not run before `Runtime::run`.
        assert!(!block_executed.load(Ordering::SeqCst));

        runtime.run();

        // `Runtime::run` should return once the task is finished.
        assert!(block_executed.load(Ordering::SeqCst));
    }

    #[test]
    fn run_until() {
        // Create a runtime.
        let mut runtime = Runtime::new();

        // Create a flag to track execution.
        let mut block_executed = false;

        runtime.run_until(async |_: Scope| {
            block_executed = true;
        });

        // `Runtime::run` should return once the future is finished.
        assert!(block_executed);
    }
}
