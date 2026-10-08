use crate::reactive::IDisposable;
use std::cell::Cell;
use std::rc::Rc;
use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, ThreadId};

/// A lock whose holder gets a disposable that releases it.
///
/// The thread that holds the lock may take it again (the lock of the
/// reference is a monitor); each disposable releases one entry, on the
/// thread that took it.
#[derive(Clone, Default)]
pub struct DisposableLock {
    monitor: Arc<Monitor>,
}

#[derive(Default)]
struct Monitor {
    state: Mutex<State>,
    exited: Condvar,
}

#[derive(Default)]
struct State {
    owner: Option<ThreadId>,
    entries: usize,
}

impl Monitor {
    fn enter(&self, wait: bool) -> bool {
        let current = thread::current().id();
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        while state.owner.is_some_and(|owner| owner != current) {
            if !wait {
                return false;
            }
            state = self.exited.wait(state).unwrap_or_else(|e| e.into_inner());
        }
        state.owner = Some(current);
        state.entries += 1;
        true
    }

    fn exit(&self) {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        state.entries -= 1;
        if state.entries == 0 {
            state.owner = None;
            drop(state);
            self.exited.notify_one();
        }
    }
}

impl DisposableLock {
    pub fn new() -> Self {
        Self::default()
    }

    /// Tries to take the lock. Returns the disposable that releases it if
    /// the lock was obtained.
    pub fn try_lock(&self) -> Option<Rc<dyn IDisposable>> {
        self.monitor.enter(false).then(|| self.unlock_disposable())
    }

    /// Enters a waiting lock.
    pub fn lock(&self) -> Rc<dyn IDisposable> {
        self.monitor.enter(true);
        self.unlock_disposable()
    }

    fn unlock_disposable(&self) -> Rc<dyn IDisposable> {
        Rc::new(UnlockDisposable { monitor: Cell::new(Some(self.monitor.clone())) })
    }
}

/// Releases the entry once, however often it is disposed, and when it is
/// dropped without having been disposed.
struct UnlockDisposable {
    monitor: Cell<Option<Arc<Monitor>>>,
}

impl IDisposable for UnlockDisposable {
    fn dispose(&self) {
        if let Some(monitor) = self.monitor.take() {
            monitor.exit();
        }
    }
}

impl Drop for UnlockDisposable {
    fn drop(&mut self) {
        self.dispose();
    }
}

#[cfg(test)]
mod tests {
    // Not from upstream, which has no tests of the lock.
    use super::*;

    #[test]
    fn the_holder_locks_again_and_another_thread_does_not() {
        let lock = DisposableLock::new();
        let first = lock.lock();
        let second = lock.try_lock().expect("the holder may enter again");

        let other = lock.clone();
        assert!(thread::spawn(move || other.try_lock().is_none()).join().unwrap());

        second.dispose();
        second.dispose();
        let other = lock.clone();
        assert!(thread::spawn(move || other.try_lock().is_none()).join().unwrap());

        first.dispose();
        let other = lock.clone();
        assert!(thread::spawn(move || other.try_lock().is_some()).join().unwrap());
    }

    #[test]
    fn a_waiting_lock_is_entered_when_the_holder_releases() {
        let lock = DisposableLock::new();
        let held = lock.lock();
        let other = lock.clone();
        let waiter = thread::spawn(move || {
            let entered = other.lock();
            entered.dispose();
        });
        thread::sleep(std::time::Duration::from_millis(20));
        assert!(!waiter.is_finished());
        drop(held);
        waiter.join().unwrap();
    }
}
