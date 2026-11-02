//! [`JoinHandle`], the handle of a spawned task, and [`JoinError`], how a task ends without the
//! output of its future.

use super::executor::{panic_message, Failure, Header, LocalCallback};
use super::shared::Shared;
use std::any::Any;
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::task::{Context, Poll, Waker};

/// Why a task ended without the output of its future.
pub enum JoinError {
    /// The task was aborted ([`JoinHandle::abort`]): its future was dropped.
    Canceled,
    /// The future of the task panicked: the payload of the panic (for a [`Shared`] task, its
    /// message as a `String`).
    Panicked(Box<dyn Any + Send + 'static>),
}

impl JoinError {
    /// Whether the task was aborted.
    pub fn is_canceled(&self) -> bool {
        matches!(self, Self::Canceled)
    }

    /// Whether the future of the task panicked.
    pub fn is_panic(&self) -> bool {
        matches!(self, Self::Panicked(_))
    }

    /// The payload of the panic.
    ///
    /// # Panics
    /// When the task was aborted; [`try_into_panic`](Self::try_into_panic) does not.
    pub fn into_panic(self) -> Box<dyn Any + Send + 'static> {
        match self.try_into_panic() {
            Ok(payload) => payload,
            Err(error) => panic!("`JoinError::into_panic` of a task that did not panic: {error}"),
        }
    }

    /// The payload of the panic, or the error itself when the task was aborted.
    pub fn try_into_panic(self) -> Result<Box<dyn Any + Send + 'static>, JoinError> {
        match self {
            Self::Panicked(payload) => Ok(payload),
            error => Err(error),
        }
    }

    /// Raises the panic of the task again on the calling thread, or panics with the error when
    /// the task was aborted.
    pub fn resume_unwind(self) -> ! {
        match self {
            Self::Panicked(payload) => std::panic::resume_unwind(payload),
            error => panic!("{error}"),
        }
    }

    /// The message of a panic, as a `String` payload.
    fn message(&self) -> Option<String> {
        match self {
            Self::Canceled => None,
            Self::Panicked(payload) => Some(panic_message(&**payload)),
        }
    }

    /// A copy for one more observer: the payload of a panic becomes its message.
    pub(crate) fn duplicate(&self) -> Self {
        match self.message() {
            None => Self::Canceled,
            Some(message) => Self::Panicked(Box::new(message)),
        }
    }
}

impl From<Failure> for JoinError {
    fn from(failure: Failure) -> Self {
        match failure {
            Failure::Canceled => Self::Canceled,
            Failure::Panicked(payload) => Self::Panicked(payload),
        }
    }
}

impl fmt::Debug for JoinError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.message() {
            None => f.write_str("Canceled"),
            Some(message) => f.debug_tuple("Panicked").field(&message).finish(),
        }
    }
}

impl fmt::Display for JoinError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.message() {
            None => f.write_str("the task was canceled"),
            Some(message) => write!(f, "the task panicked: {message}"),
        }
    }
}

impl std::error::Error for JoinError {}

/// The outcome of a task, written once by the task and taken once by its handle.
pub(crate) struct JoinCell<T> {
    finished: AtomicBool,
    slot: Mutex<JoinSlot<T>>,
}

struct JoinSlot<T> {
    outcome: Option<Result<T, JoinError>>,
    /// Who is told when the outcome is written: the waker of the handle's last poll, or the
    /// [`Shared`] made from the handle.
    waker: Option<Waker>,
    taken: bool,
}

impl<T> JoinCell<T> {
    pub(crate) fn new() -> Self {
        Self { finished: AtomicBool::new(false), slot: Mutex::new(JoinSlot { outcome: None, waker: None, taken: false }) }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, JoinSlot<T>> {
        self.slot.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Writes the outcome unless one was written, and wakes the handle; gives back an outcome that
    /// came too late.
    pub(crate) fn complete(&self, outcome: Result<T, JoinError>) -> Option<Result<T, JoinError>> {
        let waker = {
            let mut slot = self.lock();
            if self.finished.load(Ordering::Acquire) {
                return Some(outcome);
            }
            slot.outcome = Some(outcome);
            self.finished.store(true, Ordering::Release);
            slot.waker.take()
        };
        // Woken outside of the lock.
        if let Some(waker) = waker {
            waker.wake();
        }
        None
    }

    fn is_finished(&self) -> bool {
        self.finished.load(Ordering::Acquire)
    }

    /// Takes the outcome when it is written.
    pub(crate) fn take(&self) -> Option<Result<T, JoinError>> {
        let mut slot = self.lock();
        let outcome = slot.outcome.take();
        slot.taken |= outcome.is_some();
        outcome
    }
}

/// The handle of a spawned task ([`spawn_local`](super::spawn_local),
/// [`spawn_local_on`](super::spawn_local_on)): a future of its outcome, the output of its future
/// or a [`JoinError`].
///
/// Dropping the handle detaches the task, which keeps running. The handle is `Send` when the
/// output is: it may be awaited or aborted from another thread, while the task runs on its own.
/// It has one owner; [`shared`](Self::shared) makes a handle for several observers.
pub struct JoinHandle<T> {
    header: Arc<Header>,
    cell: Arc<JoinCell<T>>,
}

impl<T> JoinHandle<T> {
    pub(crate) fn new(header: Arc<Header>, cell: Arc<JoinCell<T>>) -> Self {
        Self { header, cell }
    }

    /// Whether the task has ended, in any way.
    pub fn is_finished(&self) -> bool {
        self.cell.is_finished()
    }

    /// Aborts the task: its future is dropped and the outcome is [`JoinError::Canceled`], unless
    /// it has ended. On the task's thread the future is dropped at once (when the task is being
    /// polled, right after that poll); from another thread, when the task's thread polls it next
    /// (its schedule, or [`run_pending`](super::run_pending) for the queue of the thread).
    pub fn abort(&self) {
        if !self.is_finished() {
            self.header.abort();
        }
    }

    /// Takes the outcome when the task has ended.
    pub(crate) fn try_take(&self) -> Option<Result<T, JoinError>> {
        self.cell.take()
    }

    /// The outcome of the task for several observers, on the current thread: see [`Shared`].
    ///
    /// # Panics
    /// When the handle has given the outcome already (it was polled to the end).
    pub fn shared(self) -> Shared<T>
    where
        T: Clone + 'static,
    {
        let shared = Shared::new(self.header.clone());
        let (cell, target) = (self.cell.clone(), shared.clone());
        let callback = LocalCallback::register(Box::new(move || {
            if let Some(outcome) = cell.take() {
                target.finish(outcome);
            }
        }));
        let notify = callback.handle();
        let finished = {
            let mut slot = self.cell.lock();
            if slot.outcome.is_some() {
                true
            } else {
                assert!(!slot.taken, "a `JoinHandle` was made shared after it gave the outcome of its task");
                slot.waker = Some(Waker::from(Arc::new(callback.handle())));
                false
            }
        };
        if finished {
            notify.run();
        }
        shared
    }
}

impl<T> Future for JoinHandle<T> {
    type Output = Result<T, JoinError>;

    /// # Panics
    /// When it is polled again after it gave the outcome.
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut slot = self.cell.lock();
        if let Some(outcome) = slot.outcome.take() {
            slot.taken = true;
            return Poll::Ready(outcome);
        }
        assert!(!slot.taken, "a `JoinHandle` was polled after it gave the outcome of its task");
        if !slot.waker.as_ref().is_some_and(|waker| waker.will_wake(cx.waker())) {
            slot.waker = Some(cx.waker().clone());
        }
        Poll::Pending
    }
}

impl<T> fmt::Debug for JoinHandle<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("JoinHandle").field("id", &self.header.id()).field("finished", &self.is_finished()).finish()
    }
}
