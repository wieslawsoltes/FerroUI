//! `Task<TResult>`, `TaskStatus`, `TaskCompletionSource<TResult>` and `TaskCanceledException`
//! (`System.Threading.Tasks`), single-threaded (docs/porting/reactive.md, section 5.8).

use crate::ObservableError;
use std::cell::{Cell, RefCell};
use std::error::Error;
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::task::{Context, Poll, Waker};

/// The state of a task (`TaskStatus`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TaskStatus {
    /// Made, and not started (`Created`).
    Created,
    /// Waiting for its completion source or its future (`WaitingForActivation`).
    WaitingForActivation,
    /// Its future is being polled (`Running`).
    Running,
    /// Ended with a result (`RanToCompletion`).
    RanToCompletion,
    /// Ended with an error (`Faulted`).
    Faulted,
    /// Ended by a cancellation (`Canceled`).
    Canceled,
}

/// The error of a canceled task (`TaskCanceledException`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TaskCanceledError {
    /// The id of the task, when the error names one (`TaskCanceledException.Task`).
    pub task_id: Option<u64>,
}

impl fmt::Display for TaskCanceledError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("A task was canceled.")
    }
}

impl Error for TaskCanceledError {}

impl TaskCanceledError {
    /// Whether `error` is a cancellation: a [`TaskCanceledError`].
    pub fn is_cancellation(error: &ObservableError) -> bool {
        error.downcast_ref::<TaskCanceledError>().is_some()
    }
}

fn next_task_id() -> u64 {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    NEXT.fetch_add(1, Ordering::Relaxed)
}

type Continuation = Box<dyn FnOnce()>;

/// A continuation registered on a task that has not ended ([`Task::continue_with`]), for
/// [`Task::remove_continuation`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ContinuationId(u64);

struct TaskInner<T> {
    id: u64,
    status: Cell<TaskStatus>,
    result: RefCell<Option<T>>,
    error: RefCell<Option<ObservableError>>,
    continuations: RefCell<Vec<(u64, Continuation)>>,
    next_continuation: Cell<u64>,
    wakers: RefCell<Vec<Waker>>,
}

/// The outcome of an asynchronous operation (`Task<TResult>`; `Task` is `Task<()>`): a shared
/// handle that ends once, with a result, an error or a cancellation.
///
/// Continuations ([`continue_with`](Self::continue_with)) run synchronously when the task ends
/// (`TaskContinuationOptions.ExecuteSynchronously`); a task is also a [`Future`] of its outcome.
/// Tasks belong to the thread that made them, like the observables
/// (docs/porting/reactive.md, section 5.8).
pub struct Task<T> {
    inner: Rc<TaskInner<T>>,
}

impl<T> Clone for Task<T> {
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone() }
    }
}

impl<T> fmt::Debug for Task<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Task").field("id", &self.inner.id).field("status", &self.inner.status.get()).finish()
    }
}

impl<T: 'static> Task<T> {
    pub(crate) fn with_status(status: TaskStatus) -> Self {
        Self {
            inner: Rc::new(TaskInner {
                id: next_task_id(),
                status: Cell::new(status),
                result: RefCell::new(None),
                error: RefCell::new(None),
                continuations: RefCell::new(Vec::new()),
                next_continuation: Cell::new(0),
                wakers: RefCell::new(Vec::new()),
            }),
        }
    }

    /// A task that has ended with `result` (`Task.FromResult`).
    pub fn from_result(result: T) -> Self {
        let task = Self::with_status(TaskStatus::WaitingForActivation);
        task.try_complete(Ok(result));
        task
    }

    /// A task that has ended with `error` (`Task.FromException`); a [`TaskCanceledError`] makes it
    /// canceled.
    pub fn from_exception(error: ObservableError) -> Self {
        let task = Self::with_status(TaskStatus::WaitingForActivation);
        task.try_complete(Err(error));
        task
    }

    /// A canceled task (`Task.FromCanceled`).
    pub fn from_canceled() -> Self {
        let task = Self::with_status(TaskStatus::WaitingForActivation);
        task.try_cancel();
        task
    }

    /// Runs `future` on the current thread and gives its task (the counterpart of an `async`
    /// method): see [`spawn_local`](super::spawn_local).
    pub fn spawn_local(future: impl Future<Output = Result<T, ObservableError>> + 'static) -> Self {
        super::local_executor::spawn_local(future)
    }

    /// The identity of the task (`Id`).
    pub fn id(&self) -> u64 {
        self.inner.id
    }

    /// The state of the task (`Status`).
    pub fn status(&self) -> TaskStatus {
        self.inner.status.get()
    }

    /// Whether the task has ended, in any way (`IsCompleted`).
    pub fn is_completed(&self) -> bool {
        matches!(self.status(), TaskStatus::RanToCompletion | TaskStatus::Faulted | TaskStatus::Canceled)
    }

    /// Whether the task has ended with a result (`IsCompletedSuccessfully`).
    pub fn is_completed_successfully(&self) -> bool {
        self.status() == TaskStatus::RanToCompletion
    }

    /// Whether the task has ended with an error (`IsFaulted`).
    pub fn is_faulted(&self) -> bool {
        self.status() == TaskStatus::Faulted
    }

    /// Whether the task has been canceled (`IsCanceled`).
    pub fn is_canceled(&self) -> bool {
        self.status() == TaskStatus::Canceled
    }

    /// The error of a faulted task (`Exception`, unwrapped from its `AggregateException`).
    pub fn exception(&self) -> Option<ObservableError> {
        self.inner.error.borrow().clone()
    }

    /// The outcome of an ended task: the result, the error of a faulted task, or a
    /// [`TaskCanceledError`] (`Result`, `GetAwaiter().GetResult()`).
    ///
    /// # Panics
    /// When the task has not ended: upstream blocks, which would deadlock the only thread.
    pub fn result(&self) -> Result<T, ObservableError>
    where
        T: Clone,
    {
        match self.status() {
            TaskStatus::RanToCompletion => Ok(self.inner.result.borrow().clone().expect("a completed task holds its result")),
            TaskStatus::Faulted => Err(self.exception().expect("a faulted task holds its error")),
            TaskStatus::Canceled => Err(Rc::new(TaskCanceledError { task_id: Some(self.id()) })),
            _ => panic!("the result of a task that has not ended was asked for; await it or use continue_with"),
        }
    }

    /// Runs `continuation` when the task ends, synchronously on the thread that ends it, or at once
    /// when it has ended (`ContinueWith` with `TaskContinuationOptions.ExecuteSynchronously`). The
    /// id removes a continuation that has not run ([`remove_continuation`](Self::remove_continuation),
    /// the cancellation token of `ContinueWith`); `None` when it ran at once.
    pub fn continue_with(&self, continuation: impl FnOnce(&Task<T>) + 'static) -> Option<ContinuationId> {
        let task = self.clone();
        if self.is_completed() {
            continuation(&task);
            return None;
        }
        let id = self.inner.next_continuation.get();
        self.inner.next_continuation.set(id + 1);
        self.inner.continuations.borrow_mut().push((id, Box::new(move || continuation(&task))));
        Some(ContinuationId(id))
    }

    /// Removes a continuation that has not run; false when it has run or was removed.
    pub fn remove_continuation(&self, id: ContinuationId) -> bool {
        let removed = {
            let mut continuations = self.inner.continuations.borrow_mut();
            let position = continuations.iter().position(|(entry, _)| *entry == id.0);
            position.map(|position| continuations.remove(position))
        };
        // Dropped outside of the borrow: what the continuation holds may reach the task.
        removed.is_some()
    }

    /// Ends the task with `outcome` unless it has ended; an error that is a [`TaskCanceledError`]
    /// cancels it. False when it had ended.
    pub(crate) fn try_complete(&self, outcome: Result<T, ObservableError>) -> bool {
        if self.is_completed() {
            return false;
        }
        match outcome {
            Ok(result) => {
                *self.inner.result.borrow_mut() = Some(result);
                self.inner.status.set(TaskStatus::RanToCompletion);
            }
            Err(error) if TaskCanceledError::is_cancellation(&error) => {
                self.inner.status.set(TaskStatus::Canceled);
            }
            Err(error) => {
                *self.inner.error.borrow_mut() = Some(error);
                self.inner.status.set(TaskStatus::Faulted);
            }
        }
        self.finish();
        true
    }

    pub(crate) fn try_cancel(&self) -> bool {
        if self.is_completed() {
            return false;
        }
        self.inner.status.set(TaskStatus::Canceled);
        self.finish();
        true
    }

    fn finish(&self) {
        let continuations = std::mem::take(&mut *self.inner.continuations.borrow_mut());
        for (_, continuation) in continuations {
            continuation();
        }
        let wakers = std::mem::take(&mut *self.inner.wakers.borrow_mut());
        for waker in wakers {
            waker.wake();
        }
    }
}

impl<T: Clone + 'static> Future for Task<T> {
    type Output = Result<T, ObservableError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.is_completed() {
            return Poll::Ready(self.result());
        }
        let mut wakers = self.inner.wakers.borrow_mut();
        if !wakers.iter().any(|waker| waker.will_wake(cx.waker())) {
            wakers.push(cx.waker().clone());
        }
        Poll::Pending
    }
}

/// The producer side of a task that code ends by hand (`TaskCompletionSource<TResult>`).
pub struct TaskCompletionSource<T> {
    task: Task<T>,
}

impl<T: 'static> Default for TaskCompletionSource<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Clone for TaskCompletionSource<T> {
    fn clone(&self) -> Self {
        Self { task: self.task.clone() }
    }
}

impl<T: 'static> TaskCompletionSource<T> {
    /// A source with a task that has not ended.
    pub fn new() -> Self {
        Self { task: Task::with_status(TaskStatus::WaitingForActivation) }
    }

    /// The task of the source (`Task`).
    pub fn task(&self) -> Task<T> {
        self.task.clone()
    }

    /// Ends the task with `result` (`SetResult`).
    ///
    /// # Panics
    /// When the task has ended (`InvalidOperationException`).
    pub fn set_result(&self, result: T) {
        assert!(self.try_set_result(result), "An attempt was made to transition a task to a final state when it had already completed.");
    }

    /// Ends the task with `result` unless it has ended (`TrySetResult`).
    pub fn try_set_result(&self, result: T) -> bool {
        self.task.try_complete(Ok(result))
    }

    /// Ends the task with `error` (`SetException`); a [`TaskCanceledError`] cancels it.
    ///
    /// # Panics
    /// When the task has ended (`InvalidOperationException`).
    pub fn set_exception(&self, error: ObservableError) {
        assert!(self.try_set_exception(error), "An attempt was made to transition a task to a final state when it had already completed.");
    }

    /// Ends the task with `error` unless it has ended (`TrySetException`).
    pub fn try_set_exception(&self, error: ObservableError) -> bool {
        self.task.try_complete(Err(error))
    }

    /// Cancels the task (`SetCanceled`).
    ///
    /// # Panics
    /// When the task has ended (`InvalidOperationException`).
    pub fn set_canceled(&self) {
        assert!(self.try_set_canceled(), "An attempt was made to transition a task to a final state when it had already completed.");
    }

    /// Cancels the task unless it has ended (`TrySetCanceled`).
    pub fn try_set_canceled(&self) -> bool {
        self.task.try_cancel()
    }
}
