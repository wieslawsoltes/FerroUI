//! The threads of the module: the id of the calling thread, and a call made
//! on another one.
//!
//! Not from upstream, where the runtime carries a call from one thread to
//! another. A module built with threads runs each thread in a web worker,
//! and a worker only does what arrives at its event loop: a call for another
//! thread is put into a queue of the module and the thread is notified. The
//! two uses are the wake-up of the dispatcher from the render thread and the
//! frame out of turn the thread of the user interface asks the render thread
//! for (`docs/porting/browser-render-worker.md`, section 3 and "B2.3").
//!
//! The work only carries what is `Send`: no object of the page crosses.
//!
//! In a build without threads there is one thread, its id is 0 and no call
//! is ever queued.

/// What is run on another thread.
pub(crate) type ThreadWork = Box<dyn FnOnce() + Send>;

/// The signature of [`run_on_thread`]; a test puts a recorder in its place.
pub(crate) type RunOnThread = fn(usize, ThreadWork) -> bool;

/// The id of the calling thread: the key of its worker in the table of
/// threads of the module. 0 in a build without threads.
pub fn current_thread() -> usize {
    native::current_thread()
}

/// Queues `work` for `thread` (what [`current_thread`] returned there) and
/// returns at once. The work runs when the thread is at its event loop:
/// never inside a call the thread is making, and not while the thread waits
/// for a lock or a condition.
///
/// The thread has to be one that does not end: the callers of the crate pass
/// the thread of the dispatcher, which is the thread of the page, and a
/// render thread, which is kept alive for as long as the page. The id of a
/// thread that has ended names memory the runtime has released.
///
/// Returns whether the work was queued; `false` in a build without threads,
/// and the work is dropped then.
pub(crate) fn run_on_thread(thread: usize, work: ThreadWork) -> bool {
    native::run_on_thread(thread, work)
}

#[cfg(all(target_os = "emscripten", target_feature = "atomics"))]
mod native {
    use super::ThreadWork;
    use std::ffi::c_void;
    use std::sync::OnceLock;

    /// A queue of calls between threads (`em_proxying_queue`).
    #[repr(C)]
    struct ProxyingQueue {
        _opaque: [u8; 0],
    }

    extern "C" {
        fn pthread_self() -> usize;
        fn em_proxying_queue_create() -> *mut ProxyingQueue;
        fn emscripten_proxy_async(
            queue: *mut ProxyingQueue,
            target_thread: usize,
            func: extern "C" fn(*mut c_void),
            arg: *mut c_void,
        ) -> bool;
    }

    /// The queue of the platform, as an address; 0 when it could not be
    /// created. It is never destroyed.
    ///
    /// A queue of its own, not the system queue of the runtime: the main
    /// thread runs the work of the system queue wherever it waits for a lock,
    /// in the middle of whatever it was doing, and runs the work of any other
    /// queue only from its event loop.
    static QUEUE: OnceLock<usize> = OnceLock::new();

    pub(super) fn current_thread() -> usize {
        // SAFETY: the function takes no argument and returns the address of
        // the descriptor of the calling thread.
        unsafe { pthread_self() }
    }

    /// Runs on the target thread.
    extern "C" fn run(arg: *mut c_void) {
        // SAFETY: `arg` is the pointer `run_on_thread` made with
        // `Box::into_raw` for this call, and the queue calls the function at
        // most once for it.
        let work = unsafe { Box::from_raw(arg.cast::<ThreadWork>()) };
        work();
    }

    pub(super) fn run_on_thread(thread: usize, work: ThreadWork) -> bool {
        // SAFETY: the function takes no argument and returns a new queue or
        // null.
        let queue = *QUEUE.get_or_init(|| unsafe { em_proxying_queue_create() } as usize) as *mut ProxyingQueue;
        if queue.is_null() || thread == 0 {
            return false;
        }
        // A box of the box: the closure is a wide pointer, the queue carries
        // a thin one.
        let arg = Box::into_raw(Box::new(work));
        // SAFETY: `queue` is a live queue, `run` has the signature the queue
        // calls and `arg` stays valid until `run` takes it back. `thread`
        // is the descriptor of a thread that has not ended, as the callers
        // of the crate guarantee (see `super::run_on_thread`).
        let queued = unsafe { emscripten_proxy_async(queue, thread, run, arg.cast()) };
        if !queued {
            // SAFETY: the queue refused the work, so `run` will not be called
            // and the pointer is still the only one to the box.
            drop(unsafe { Box::from_raw(arg) });
        }
        queued
    }
}

#[cfg(not(all(target_os = "emscripten", target_feature = "atomics")))]
mod native {
    use super::ThreadWork;

    pub(super) fn current_thread() -> usize {
        0
    }

    pub(super) fn run_on_thread(_thread: usize, _work: ThreadWork) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn without_threads_there_is_one_thread_and_nothing_is_queued() {
        assert_eq!(0, current_thread());
        assert!(!run_on_thread(1, Box::new(|| panic!("the work must not run"))));
    }
}
