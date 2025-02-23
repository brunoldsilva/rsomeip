//! Runtime state.

use tokio::sync::watch;

/// State of the `rsomeip` runtime.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum State {
    /// The runtime is waiting to start. This is the initial state.
    #[default]
    Waiting,
    /// The runtime is running.
    Running,
    /// The runtime is stopped.
    Stopped,
}

/// A watcher for runtime [`State`] changes.
#[derive(Debug)]
pub struct StateWatcher {
    inner: watch::Sender<State>,
}

impl StateWatcher {
    /// Creates a new [`StateWatcher`].
    pub fn new() -> Self {
        let (sender, _) = watch::channel(State::default());
        Self { inner: sender }
    }

    /// Returns the current [`State`].
    #[expect(dead_code, reason = "Nice-to-have, but not currently used.")]
    pub fn state(&self) -> State {
        *self.inner.borrow()
    }

    /// Sets the current [`State`].
    ///
    /// Returns the previous value.
    pub fn set_state(&self, value: State) -> State {
        self.inner.send_replace(value)
    }

    /// Waits for the state to match the given value.
    pub async fn wait_for(&self, value: State) {
        _ = self
            .inner
            .subscribe()
            .wait_for(|&state| state == value)
            .await;
    }

    /// Sets the current state to [`State::Waiting`].
    #[expect(dead_code, reason = "Nice-to-have, but not currently used.")]
    pub fn wait(&self) {
        self.set_state(State::Waiting);
    }

    /// Waits for the state to become [`State::Running`].
    #[expect(dead_code, reason = "Nice-to-have, but not currently used.")]
    pub async fn waiting(&self) {
        self.wait_for(State::Running).await;
    }

    /// Sets the current state to [`State::Running`].
    pub fn run(&self) {
        self.set_state(State::Running);
    }

    /// Waits for the state to become [`State::Running`].
    pub async fn running(&self) {
        self.wait_for(State::Running).await;
    }

    /// Sets the current state to [`State::Stopped`].
    pub fn stop(&self) {
        self.set_state(State::Stopped);
    }

    /// Waits for the state to become [`State::Stopped`].
    pub async fn stopped(&self) {
        self.wait_for(State::Stopped).await;
    }
}
