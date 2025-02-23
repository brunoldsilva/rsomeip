//! Resources shared between `rsomeip` tasks.

use super::StateWatcher;

/// Runtime shared data.
#[derive(Debug)]
pub struct Shared {
    /// Current [`super::State`] of the runtime.
    state: StateWatcher,
}

impl Shared {
    /// Creates a new [`Shared`].
    pub fn new() -> Self {
        Self {
            state: StateWatcher::new(),
        }
    }

    /// Returns a shared with the given state.
    #[expect(dead_code, reason = "Nice-to-have, but not currently used.")]
    pub fn with_state(mut self, state: StateWatcher) -> Self {
        self.state = state;
        self
    }

    /// Returns a reference to the current state of the runtime.
    pub fn state(&self) -> &StateWatcher {
        &self.state
    }
}
