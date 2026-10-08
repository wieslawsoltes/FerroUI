//! The compositor lock: what the server compositor and its objects are
//! confined to.
//!
//! Upstream the server compositor is not bound to a thread. The render
//! thread renders its frames, and the UI thread renders it too at the
//! synchronous points of a platform that asks for that (a resize, the first
//! show, the disposal of a target); `ServerCompositor.Render` takes
//! `lock (_lock)` and server objects check that they are only touched under
//! it. The graph of server objects here is made of `Rc` and cells, so the
//! rule is carried by a type: [`LockedServerCompositor`] holds the graph and
//! only hands it out inside the lock.

use super::ServerCompositor;
use std::rc::Rc;
use std::sync::{Condvar, Mutex};
use std::thread::{self, ThreadId};

/// A lock that the thread holding it may enter again (`lock` in C#).
#[derive(Default)]
pub struct CompositorLock {
    state: Mutex<LockState>,
    released: Condvar,
}

#[derive(Default)]
struct LockState {
    owner: Option<ThreadId>,
    depth: usize,
}

impl CompositorLock {
    pub fn new() -> Self {
        Self::default()
    }

    /// Enters the lock, waiting while another thread holds it.
    pub fn enter(&self) -> CompositorLockGuard<'_> {
        let current = thread::current().id();
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        while state.owner.is_some_and(|owner| owner != current) {
            state = self.released.wait(state).unwrap_or_else(|e| e.into_inner());
        }
        state.owner = Some(current);
        state.depth += 1;
        CompositorLockGuard { lock: self }
    }

    /// Whether the calling thread holds the lock.
    pub fn is_held_by_current_thread(&self) -> bool {
        self.state.lock().unwrap_or_else(|e| e.into_inner()).owner == Some(thread::current().id())
    }
}

/// Leaves the lock when dropped.
pub struct CompositorLockGuard<'a> {
    lock: &'a CompositorLock,
}

impl Drop for CompositorLockGuard<'_> {
    fn drop(&mut self) {
        let mut state = self.lock.state.lock().unwrap_or_else(|e| e.into_inner());
        state.depth -= 1;
        if state.depth == 0 {
            state.owner = None;
            drop(state);
            self.lock.released.notify_one();
        }
    }
}

/// The server compositor behind its lock.
pub struct LockedServerCompositor {
    lock: CompositorLock,
    server: Rc<ServerCompositor>,
}

// SAFETY: the server compositor and every object reachable from it are only
// touched by the thread that holds `lock`: `with` is the only way in, and
// what it hands out cannot outlive the call. Nothing that crosses to or from
// the server side is an `Rc` into this graph: batches, resources and jobs
// are `Send` (asserted where they are defined), the readback is atomic, and
// a job that runs inside the graph is called under the lock. A caller of
// `with` that lets a handle of the graph escape the closure (by cloning an
// `Rc` out of it) breaks this; the same-thread accessor of the compositor
// exists for the mode in which no second thread enters.
unsafe impl Send for LockedServerCompositor {}
unsafe impl Sync for LockedServerCompositor {}

impl LockedServerCompositor {
    pub(crate) fn new(server: Rc<ServerCompositor>) -> Self {
        Self { lock: CompositorLock::new(), server }
    }

    /// Runs `f` with the server compositor, under the lock.
    pub fn with<R>(&self, f: impl FnOnce(&Rc<ServerCompositor>) -> R) -> R {
        let _guard = self.lock.enter();
        f(&self.server)
    }

    /// The server compositor without the lock: for the mode in which the
    /// thread of the compositor is the only one that ever enters.
    pub(crate) fn same_thread(&self) -> &Rc<ServerCompositor> {
        &self.server
    }
}

#[cfg(test)]
mod tests {
    // Not from upstream.
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    #[test]
    fn the_holder_enters_again_and_others_wait() {
        let lock = Arc::new(CompositorLock::new());
        let inside = Arc::new(AtomicUsize::new(0));

        let outer = lock.enter();
        let inner = lock.enter();
        assert!(lock.is_held_by_current_thread());

        let (thread_lock, thread_inside) = (lock.clone(), inside.clone());
        let other = thread::spawn(move || {
            assert!(!thread_lock.is_held_by_current_thread());
            let _guard = thread_lock.enter();
            thread_inside.fetch_add(1, Ordering::SeqCst);
        });

        thread::sleep(std::time::Duration::from_millis(20));
        assert_eq!(0, inside.load(Ordering::SeqCst));
        drop(inner);
        thread::sleep(std::time::Duration::from_millis(20));
        assert_eq!(0, inside.load(Ordering::SeqCst));
        drop(outer);
        other.join().unwrap();
        assert_eq!(1, inside.load(Ordering::SeqCst));
        assert!(!lock.is_held_by_current_thread());
    }
}
