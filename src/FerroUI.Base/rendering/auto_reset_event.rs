// Blocking waits do not exist on bare WebAssembly.
#![cfg(not(all(target_family = "wasm", target_os = "unknown")))]

use std::sync::{Condvar, Mutex, PoisonError};
use std::time::Duration;

/// An event that releases one waiter each time it is set and resets itself
/// when a waiter is released. Used by the render timers that own a thread.
pub(crate) struct AutoResetEvent {
    signaled: Mutex<bool>,
    condition: Condvar,
}

impl AutoResetEvent {
    pub(crate) fn new(initial_state: bool) -> Self {
        Self { signaled: Mutex::new(initial_state), condition: Condvar::new() }
    }

    /// Sets the event.
    pub(crate) fn set(&self) {
        *self.signaled.lock().unwrap_or_else(PoisonError::into_inner) = true;
        self.condition.notify_one();
    }

    /// Blocks until the event is set, then resets it.
    pub(crate) fn wait_one(&self) {
        let mut signaled = self.signaled.lock().unwrap_or_else(PoisonError::into_inner);
        while !*signaled {
            signaled = self.condition.wait(signaled).unwrap_or_else(PoisonError::into_inner);
        }
        *signaled = false;
    }

    /// Blocks until the event is set or `timeout` elapses. Returns whether
    /// the event was set (and resets it in that case).
    pub(crate) fn wait_one_timeout(&self, timeout: Duration) -> bool {
        let signaled = self.signaled.lock().unwrap_or_else(PoisonError::into_inner);
        let (mut signaled, _) = self
            .condition
            .wait_timeout_while(signaled, timeout, |signaled| !*signaled)
            .unwrap_or_else(PoisonError::into_inner);
        std::mem::take(&mut *signaled)
    }
}
