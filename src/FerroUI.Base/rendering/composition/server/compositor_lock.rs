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
use std::mem::ManuallyDrop;
use std::rc::Rc;
use std::sync::{Arc, Condvar, Mutex};
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
    /// `None` once the compositor has released it. Only touched under
    /// `lock` (or by the one thread of the dispatcher-thread mode).
    server: std::cell::UnsafeCell<Option<Rc<ServerCompositor>>>,
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
        Self { lock: CompositorLock::new(), server: std::cell::UnsafeCell::new(Some(server)) }
    }

    /// Runs `f` with the server compositor, under the lock.
    ///
    /// # Panics
    ///
    /// When the compositor has released the server compositor.
    pub fn with<R>(&self, f: impl FnOnce(&Rc<ServerCompositor>) -> R) -> R {
        match self.try_with(f) {
            Some(result) => result,
            None => panic!("the server compositor has been released"),
        }
    }

    /// Runs `f` with the server compositor, under the lock; `None` once the
    /// compositor has released it.
    pub fn try_with<R>(&self, f: impl FnOnce(&Rc<ServerCompositor>) -> R) -> Option<R> {
        let _guard = self.lock.enter();
        // SAFETY: the cell is only read and written under the lock, which
        // this thread holds. The handle taken here keeps the server
        // compositor alive for the call if `f` releases it, and is dropped
        // before the guard, under the lock.
        let server = unsafe { (*self.server.get()).clone() }?;
        Some(f(&server))
    }

    /// Whether `server` is the server compositor behind this lock. Called by
    /// a thread that holds a reference to a server compositor, which it only
    /// has inside the lock (or as the one thread of the dispatcher-thread
    /// mode).
    pub(crate) fn is_server(&self, server: &ServerCompositor) -> bool {
        // SAFETY: see above; the cell is read by a thread that is inside the
        // lock or is the only one.
        unsafe { (*self.server.get()).as_ref() }.is_some_and(|own| std::ptr::eq(Rc::as_ptr(own), server))
    }

    /// Runs `f` under the lock, without the server compositor.
    pub(crate) fn under_lock<R>(&self, f: impl FnOnce() -> R) -> R {
        let _guard = self.lock.enter();
        f()
    }

    /// Releases the server compositor, under the lock, on the calling
    /// thread. The compositor does this when it is dropped: the graph holds
    /// handles that it shares with objects of the UI thread (the render
    /// interface, the platform graphics), so it is released there and not by
    /// whichever thread drops the last handle to this object.
    pub(crate) fn release(&self) {
        let _guard = self.lock.enter();
        // SAFETY: as in `try_with`.
        let server = unsafe { (*self.server.get()).take() };
        drop(server);
    }

    /// The server compositor without the lock: for the mode in which the
    /// thread of the compositor is the only one that ever enters.
    pub(crate) fn same_thread(&self) -> &Rc<ServerCompositor> {
        // SAFETY: in the dispatcher-thread mode one thread reads the cell and
        // releases it (when the compositor is dropped, after which nothing
        // calls this).
        match unsafe { (*self.server.get()).as_ref() } {
            Some(server) => server,
            None => panic!("the server compositor has been released"),
        }
    }
}

/// A value confined to the compositor lock: an object of the server side
/// that is not part of the graph of the server compositor.
///
/// Upstream an object of the render interface (an imported GPU image, the
/// feature that imports it, the context it belongs to) is created at the
/// request of the UI thread, then used and disposed by jobs of the render
/// thread; the garbage collector shares it. Here such an object is an `Rc`,
/// so it is bound to the lock under which every job runs: the value is
/// reached with the server compositor in hand, which a thread only has while
/// it is inside the lock (or, in the mode without a second thread, while it
/// is the one thread of the compositor), and it is dropped inside the lock.
/// Unlike a `ThreadBound` it does not depend on the thread: both the render
/// thread and the thread of the compositor reach it, one at a time.
pub struct LockBound<T> {
    value: ManuallyDrop<T>,
    server: Arc<LockedServerCompositor>,
}

// SAFETY: the value is only reached through `get`, which asks for the server
// compositor the value is bound to: a reference a thread has only inside
// `LockedServerCompositor::with` (a job, or `Compositor::with_server`), or
// from the same-thread accessor in the mode in which no second thread
// enters. The value is dropped inside the lock too. So no two threads are in
// the value at once, and every use is ordered by the lock.
//
// What the type cannot check is on the caller: every handle that shares
// state with the value without synchronisation (a clone of an `Rc` inside
// it, an `Rc` it was built from) must itself only be touched, cloned and
// dropped inside the lock, which in practice means it lives in a `LockBound`
// of the same compositor or in the graph of the server compositor. A caller
// must not keep such a clone outside, clone one out of `get`, or bind a
// value that still shares an `Rc` with an object of its own thread.
unsafe impl<T> Send for LockBound<T> {}
unsafe impl<T> Sync for LockBound<T> {}

impl<T> LockBound<T> {
    /// Binds `value` to the lock of `locked`. `server` is the proof that the
    /// caller is inside the lock, where the value has to be put together
    /// when it clones handles of the server side.
    ///
    /// # Panics
    ///
    /// When `server` is not the server compositor of `locked`.
    pub(crate) fn new(locked: &Arc<LockedServerCompositor>, server: &ServerCompositor, value: T) -> Self {
        let bound = Self { value: ManuallyDrop::new(value), server: locked.clone() };
        bound.verify(server);
        bound
    }

    fn verify(&self, server: &ServerCompositor) {
        assert!(
            self.server.is_server(server),
            "the value is bound to the lock of another compositor"
        );
    }

    /// The value, for a caller that is inside the compositor lock: `server`
    /// is what a job receives and what `with` hands out.
    ///
    /// # Panics
    ///
    /// When `server` is not the server compositor the value is bound to.
    pub fn get<'a>(&'a self, server: &'a ServerCompositor) -> &'a T {
        self.verify(server);
        &self.value
    }
}

impl<T> Drop for LockBound<T> {
    fn drop(&mut self) {
        // The last handle may be dropped by either thread, outside a job:
        // the value is released inside the lock.
        let server = self.server.clone();
        // SAFETY: the value is not used after this.
        server.under_lock(|| unsafe { ManuallyDrop::drop(&mut self.value) });
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
