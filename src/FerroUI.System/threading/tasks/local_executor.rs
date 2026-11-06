//! The running of futures on the thread that started them (docs/porting/reactive.md, section 5.8):
//! [`spawn_local`], the thread poster through which another thread wakes them, and the queue of
//! such wakes on a thread that has no poster; [`run`] (`Task.Run`) and [`canceled`].
//!
//! The futures are tasks of the executor of the crate ([`crate::task`]), in the queue of their
//! thread. A future is polled when it is spawned, and again whenever it is woken: at once on its
//! own thread, or right after the poll that runs when the wake comes from inside it. A wake from
//! another thread goes into the queue of the task's thread and through that thread's poster, which
//! runs [`run_pending_wakes`] there; the dispatcher of the user interface is the poster (the
//! schedule) of its thread. A future spawned in a job of a dispatcher takes the dispatcher at the
//! priority of the job instead ([`crate::task::spawn_local`]): a wake in a job of the same
//! priority polls it at once, any other wake queues a job at that priority. A panic of a future
//! faults its task with a [`TaskWorkError`] that holds the message of the panic.
//!
//! Deprecated since 0.5.0 (step A7 of phase C4 of #423): [`spawn_local`], [`block_on`],
//! [`run_pending_wakes`], [`set_thread_poster`] and [`ThreadPoster`] are wrappers over
//! [`crate::task`] (`spawn_local`, `block_on`, `run_pending`, `set_thread_schedule`, `Schedule`).
#![allow(deprecated)]

use super::task::Task;
use crate::task::Shared;
use crate::task::Canceled;
use crate::task::executor::{panic_message, spawn_task, Failure};
use crate::task::{FirstPoll, Schedule};
use crate::ObservableError;
use std::future::Future;
use std::rc::Rc;
use std::sync::{Arc, Mutex, PoisonError};
use std::task::{Poll, Waker};

/// Runs work on the thread it belongs to, from any thread (`SynchronizationContext.Post`).
#[deprecated(since = "0.5.0", note = "implement `ferroui_system::task::Schedule`, the schedule of a thread")]
pub type ThreadPoster = Arc<dyn Fn(Box<dyn FnOnce() + Send>) + Send + Sync>;

/// A thread poster as the schedule of its thread.
struct PosterSchedule(ThreadPoster);

impl Schedule for PosterSchedule {
    fn schedule(&self, poll: Box<dyn FnOnce() + Send>) {
        (self.0)(poll);
    }
}

/// Runs `future` on the current thread and gives its task: the output of the future ends it, an
/// error that is a cancellation ([`Canceled`], [`TaskCanceledError`](super::TaskCanceledError))
/// cancels it, and a panic faults it with a [`TaskWorkError`]. The future is polled at once, and
/// again whenever it is woken (the module's documentation).
#[deprecated(
    since = "0.5.0",
    note = "use `ferroui_system::task::spawn_local`, which gives a `JoinHandle` (`.shared()` for several observers)"
)]
pub fn spawn_local<T: 'static>(future: impl Future<Output = Result<T, ObservableError>> + 'static) -> Task<T> {
    let task = Task::with_status(super::TaskStatus::Running);
    let (target, failed) = (task.clone(), task.clone());
    spawn_task(
        Box::pin(async move {
            let outcome = future.await;
            target.try_complete(outcome);
        }),
        None,
        FirstPoll::Now,
        Box::new(move |failure: Failure| match failure {
            Failure::Panicked(payload) => {
                // A panic after the task ended (in a continuation that its end ran) cannot fault
                // it: it goes on from the call that polled the future, as before.
                if !failed.try_complete(Err(Rc::new(TaskWorkError { message: panic_message(&*payload) }))) {
                    std::panic::resume_unwind(payload);
                }
            }
            Failure::Canceled => {
                failed.try_cancel();
            }
        }),
    );
    task
}

/// Makes `poster` the way other threads reach the current thread (the `SynchronizationContext` of
/// the thread): a wake of one of its futures from another thread is posted through it, and the
/// work it is given must run on this thread. `None` removes it. It is the schedule of the thread
/// ([`crate::task::set_thread_schedule`]).
#[deprecated(since = "0.5.0", note = "use `ferroui_system::task::set_thread_schedule` with a `Schedule`")]
pub fn set_thread_poster(poster: Option<ThreadPoster>) {
    crate::task::set_thread_schedule(poster.map(|poster| Arc::new(PosterSchedule(poster)) as Arc<dyn Schedule>));
}

/// Polls the futures of the current thread that other threads woke, and runs the callbacks they
/// asked for (a cancellation, [`CancellationToken::register_local`](crate::threading::CancellationToken::register_local)),
/// until none is left; true when there was any ([`crate::task::run_pending`]).
#[deprecated(since = "0.5.0", note = "use `ferroui_system::task::run_pending`")]
pub fn run_pending_wakes() -> bool {
    crate::task::run_pending()
}

/// Runs `future` on the current thread until it ends, waiting for wakes from other threads in
/// between (a thread with nothing else to run: a console program, a test)
/// ([`crate::task::block_on`]).
///
/// # Panics
/// With the panic of the future. Never returns when nothing wakes the future again: a future that
/// waits for this thread itself (a scheduler of the thread, a task that only this thread ends) has
/// to be ended by something that runs before it waits.
#[deprecated(since = "0.5.0", note = "use `ferroui_system::task::block_on`, which takes any future")]
pub fn block_on<T: Clone + 'static>(future: impl Future<Output = Result<T, ObservableError>> + 'static) -> Result<T, ObservableError> {
    crate::task::block_on(future)
}

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
