//! [`run`] (`Task.Run`), whose work runs on another thread and whose outcome ends on the thread that
//! started it, its error [`TaskWorkError`], and [`canceled`] (docs/porting/reactive.md, section
//! 5.8). The futures are tasks of the executor of the crate ([`crate::task`]).
//!
//! The wrappers over [`crate::task`] that this module had (`spawn_local`, `block_on`,
//! `run_pending_wakes`, `set_thread_poster` and `ThreadPoster`; step A7 of phase C4 of #423) were
//! deprecated in 0.5.0 and removed in 0.7.0 (#625): `crate::task::spawn_local`, `block_on`,
//! `run_pending`, `set_thread_schedule` and `Schedule` take their place.

use crate::task::Canceled;
use crate::task::Shared;
use crate::ObservableError;
use std::rc::Rc;
use std::sync::{Arc, Mutex, PoisonError};
use std::task::{Poll, Waker};

/// The error of work run by [`run`] that ended with an error or panicked: its message, since an
/// error crossing threads must be `Send`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskWorkError {
    pub message: String,
}

impl std::fmt::Display for TaskWorkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for TaskWorkError {}

/// Runs `work` on another thread and gives the outcome, which ends on the current thread
/// (`Task.Run`): the outcome is the [`Shared`] of a task of the current thread
/// ([`crate::task::spawn_local`]), woken from the worker through the schedule of the thread, or the
/// queue of [`crate::task::run_pending`]. An `Err` of the work, or a panic, is a
/// [`TaskWorkError`] with its message; [`Shared::flatten`] is a future of the result.
pub fn run<T: Clone + Send + 'static>(
    work: impl FnOnce() -> Result<T, Box<dyn std::error::Error + Send + Sync>> + Send + 'static,
) -> Shared<Result<T, ObservableError>> {
    struct Slot<T> {
        outcome: Option<Result<T, String>>,
        waker: Option<Waker>,
    }
    let slot = Arc::new(Mutex::new(Slot { outcome: None, waker: None }));
    let worker_slot = slot.clone();
    std::thread::spawn(move || {
        // Boundary (#567): the top of the worker thread; the work is application code, and its
        // panic is the error of the task (`TaskWorkError`), as the exception of `Task.Run` faults
        // its task in the reference.
        let outcome = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(work)) {
            Ok(Ok(value)) => Ok(value),
            Ok(Err(error)) => Err(error.to_string()),
            Err(panic) => Err(panic
                .downcast_ref::<&str>()
                .map(|text| (*text).to_string())
                .or_else(|| panic.downcast_ref::<String>().cloned())
                .unwrap_or_else(|| "the work panicked".to_string())),
        };
        let waker = {
            let mut slot = worker_slot.lock().unwrap_or_else(PoisonError::into_inner);
            slot.outcome = Some(outcome);
            slot.waker.take()
        };
        if let Some(waker) = waker {
            waker.wake();
        }
    });
    crate::task::spawn_local(std::future::poll_fn(move |context| {
        let mut slot = slot.lock().unwrap_or_else(PoisonError::into_inner);
        match slot.outcome.take() {
            Some(Ok(value)) => Poll::Ready(Ok(value)),
            Some(Err(message)) => Poll::Ready(Err(Rc::new(TaskWorkError { message }) as ObservableError)),
            None => {
                slot.waker = Some(context.waker().clone());
                Poll::Pending
            }
        }
    }))
    .shared()
}

/// A future that ends with a cancellation (`Task.FromCanceled` awaited): for code that stops
/// because its token was cancelled.
pub fn canceled<T>() -> Result<T, ObservableError> {
    Err(Rc::new(Canceled))
}
