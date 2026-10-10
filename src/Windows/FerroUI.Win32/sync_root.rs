//! The lock the composition modes share between the thread that renders,
//! the thread of the composition timer and the UI thread.
//!
//! Not a file of the reference: there the lock is the monitor of an object
//! (`SyncRoot`), entered by `BeginTransaction` and left when the transaction
//! is disposed, and entered again by the thread that already holds it (the
//! timer of the Windows Runtime composition ticks, and so renders, inside
//! it). A guard of a Rust mutex cannot live in an object behind a contract
//! and a Rust mutex cannot be entered twice, so the monitor is written here:
//! an owner, a count and a condition variable.

use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::thread::{self, ThreadId};

#[derive(Default)]
struct State {
    owner: Option<ThreadId>,
    count: u32,
}

/// A monitor: a lock a thread may enter more than once.
#[derive(Default)]
pub(crate) struct SyncRoot {
    state: Mutex<State>,
    released: Condvar,
}

impl SyncRoot {
    pub fn new() -> Arc<SyncRoot> {
        Arc::new(SyncRoot::default())
    }

    fn state(&self) -> MutexGuard<'_, State> {
        // The state is two numbers that every path leaves consistent.
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// `Monitor.Enter`: waits until no other thread holds the lock.
    pub fn enter(&self) {
        let current = thread::current().id();
        let mut state = self.state();
        while state.owner.is_some_and(|owner| owner != current) {
            state = self.released.wait(state).unwrap_or_else(PoisonError::into_inner);
        }
        state.owner = Some(current);
        state.count += 1;
    }

    /// `Monitor.Exit`.
    ///
    /// # Panics
    /// Panics when the calling thread does not hold the lock (the
    /// `SynchronizationLockException` of the reference).
    pub fn exit(&self) {
        let mut state = self.state();
        if state.owner != Some(thread::current().id()) {
            panic!("Object synchronization method was called from an unsynchronized block of code.");
        }
        state.count -= 1;
        if state.count == 0 {
            state.owner = None;
            drop(state);
            self.released.notify_one();
        }
    }

    /// `Monitor.IsEntered`: whether the calling thread holds the lock.
    #[allow(dead_code)] // Asked by the assertions of the Windows Runtime composition mode.
    pub fn is_entered(&self) -> bool {
        self.state().owner == Some(thread::current().id())
    }

    /// The `lock` statement: the lock is held until the guard is dropped.
    pub fn lock(&self) -> SyncRootGuard<'_> {
        self.enter();
        SyncRootGuard { root: self }
    }
}

/// Holds a [`SyncRoot`] for a scope.
pub(crate) struct SyncRootGuard<'a> {
    root: &'a SyncRoot,
}

impl Drop for SyncRootGuard<'_> {
    fn drop(&mut self) {
        self.root.exit();
    }
}

/// A COM pointer that is shared between threads.
///
/// The pointers of the COM runtime of the port are of one thread, which is
/// right for the objects of an apartment. The composition modes hold
/// objects of libraries whose objects are free-threaded, and use them, as
/// the reference does, from the thread that renders, from the thread of
/// the composition timer and (to release them) from the UI thread, with the
/// calls serialized by the [`SyncRoot`] of the mode.
pub(crate) struct SharedCom<T: ferroui_microcom::Interface>(ferroui_microcom::ComPtr<T>);

impl<T: ferroui_microcom::Interface> SharedCom<T> {
    /// # Safety
    /// The object has to be one that may be called and released from any
    /// thread: an object of DirectComposition, of the Windows Runtime
    /// composition (agile objects) or of DXGI and Direct3D 11 (whose
    /// reference counting and whose device are free-threaded).
    pub unsafe fn new(pointer: ferroui_microcom::ComPtr<T>) -> SharedCom<T> {
        SharedCom(pointer)
    }
}

impl<T: ferroui_microcom::Interface> std::ops::Deref for SharedCom<T> {
    type Target = ferroui_microcom::ComPtr<T>;

    fn deref(&self) -> &ferroui_microcom::ComPtr<T> {
        &self.0
    }
}

// SAFETY: the contract of `SharedCom::new`: the object behind the pointer
// may be called and released from any thread.
unsafe impl<T: ferroui_microcom::Interface> Send for SharedCom<T> {}
// SAFETY: as above.
unsafe impl<T: ferroui_microcom::Interface> Sync for SharedCom<T> {}

#[cfg(test)]
mod tests {
    // Not from upstream.
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::Duration;

    #[test]
    fn a_thread_enters_the_lock_it_holds_again() {
        let root = SyncRoot::new();
        assert!(!root.is_entered());

        root.enter();
        {
            let _again = root.lock();
            assert!(root.is_entered());
        }
        // Left once of twice: still held.
        assert!(root.is_entered());
        root.exit();
        assert!(!root.is_entered());
    }

    #[test]
    fn another_thread_waits_until_the_lock_is_left() {
        let root = SyncRoot::new();
        let entered = Arc::new(AtomicBool::new(false));
        root.enter();

        let thread = {
            let (root, entered) = (root.clone(), entered.clone());
            thread::spawn(move || {
                // Held by the other thread, not by this one.
                assert!(!root.is_entered());
                let _guard = root.lock();
                entered.store(true, Ordering::SeqCst);
            })
        };

        thread::sleep(Duration::from_millis(50));
        assert!(!entered.load(Ordering::SeqCst));
        root.exit();
        thread.join().unwrap();
        assert!(entered.load(Ordering::SeqCst));
        assert!(!root.is_entered());
    }

    #[test]
    #[should_panic(expected = "unsynchronized block of code")]
    fn leaving_a_lock_that_is_not_held_is_an_error() {
        SyncRoot::new().exit();
    }
}
