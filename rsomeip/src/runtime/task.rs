//! Asynchronous tasks.

use super::Shared;
use std::sync::Arc;

#[derive(Debug)]
pub struct Task {
    /// Used to run futures in the task.
    runtime: tokio::runtime::Runtime,
    /// Endpoint shared data.
    shared: Arc<Shared>,
}

impl Task {
    /// Creates a new [`Task`].
    pub fn new(runtime: tokio::runtime::Runtime, shared: Arc<Shared>) -> Self {
        Self { runtime, shared }
    }

    /// Runs the given future, blocking the current thread.
    ///
    /// This method returns when the future completes or the runtime is stopped.
    pub fn run<F>(&self, future: F)
    where
        F: AsyncFnOnce(Scope),
    {
        // Use a LocalSet to run all futures in the current thread.
        tokio::task::LocalSet::new().block_on(&self.runtime, async {
            // Wait for the runtime to start.
            self.running().await;

            // Create a local scope for managing the task.
            let scope = Scope::new(self.shared.clone());

            // Run the future until it completes or the runtime stops.
            tokio::select! {
                () = self.stopped() => {
                    eprintln!("Runtime stopped.");
                }
                () = future(scope) => {
                    eprintln!("Task complete.");
                }
            }
        });
    }

    /// Waits for the runtime to start.
    async fn running(&self) {
        self.shared.state().running().await;
    }

    /// Waits for the runtime to stop.
    async fn stopped(&self) {
        self.shared.state().stopped().await;
    }
}

/// A runtime thread scope.
#[derive(Debug)]
pub struct Scope {
    /// Endpoint shared data.
    shared: Arc<Shared>,
}

impl Scope {
    /// Creates a new [`Scope`].
    pub(crate) fn new(shared: Arc<Shared>) -> Self {
        Self { shared }
    }

    /// Stops the runtime.
    ///
    /// Notifies all threads to stop execution.
    pub fn stop_all(&self) {
        self.shared.state().stop();
    }
}
