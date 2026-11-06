//! `Task<TResult>`, `TaskStatus`, `TaskCompletionSource<TResult>` and `TaskCanceledException`
//! (`System.Threading.Tasks`), single-threaded (docs/porting/reactive.md, section 5.8).
//!
//! Deprecated since 0.5.0 (step A7 of phase C4 of #423, docs/idiomatic-rust-async.md, section 4):
//! a `Task<T>` is a [`Shared`] of a `Result<T, ObservableError>` with a status and an id, and a
//! `TaskCompletionSource<T>` ends that `Shared`. They keep their behaviour: continuations run
//! synchronously before the awaiting futures are woken, a canceled task gives a
//! [`TaskCanceledError`] that names it, and a dropped source leaves its task pending (G20). They
//! convert to the new types: a task to its [`Shared`], a source to a [`oneshot::Sender`] and a
//! [`oneshot::Receiver`] to a task.
#![allow(deprecated)]

use crate::task::{oneshot, Canceled, OnFinishedId, Shared};
use crate::ObservableError;
use std::cell::Cell;
use std::error::Error;
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::task::{Context, Poll};

/// The state of a task (`TaskStatus`).
#[deprecated(since = "0.5.0", note = "a `ferroui_system::task::Shared` is finished or not (`is_finished`), and its outcome says how")]
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

/// The error of a canceled task that names it (`TaskCanceledException` with its `Task`).
///
/// A cancellation that names no task is [`Canceled`], the one cancellation error of the workspace
/// (docs/idiomatic-rust-async.md, section 3.7), which the new types give.
#[deprecated(since = "0.5.0", note = "use `ferroui_system::task::Canceled`, and `Canceled::is_cancellation` to recognize a cancellation")]
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
    /// Whether `error` is a cancellation: a [`Canceled`] or a [`TaskCanceledError`]
    /// ([`Canceled::is_cancellation`]).
    pub fn is_cancellation(error: &ObservableError) -> bool {
        Canceled::is_cancellation(error)
    }
}

/// The one cancellation error of the workspace (docs/idiomatic-rust-async.md, section 3.7): a
/// cancellation that names no task.
impl From<Canceled> for TaskCanceledError {
    fn from(_: Canceled) -> Self {
        Self::default()
    }
}

/// The one cancellation error of the workspace (docs/idiomatic-rust-async.md, section 3.7); the
/// id of the task is not kept.
impl From<TaskCanceledError> for Canceled {
    fn from(_: TaskCanceledError) -> Self {
        Canceled
    }
}

fn next_task_id() -> u64 {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    NEXT.fetch_add(1, Ordering::Relaxed)
}

/// A continuation registered on a task that has not ended ([`Task::continue_with`]), for
/// [`Task::remove_continuation`].
#[deprecated(since = "0.5.0", note = "use `ferroui_system::task::OnFinishedId` of `Shared::on_finished`")]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ContinuationId(OnFinishedId);

struct TaskInner<T> {
    id: u64,
    status: Cell<TaskStatus>,
    /// The outcome: the result, the error of a faulted task, or the [`TaskCanceledError`] of a
    /// canceled one.
    shared: Shared<Result<T, ObservableError>>,
}

/// The outcome of an asynchronous operation (`Task<TResult>`; `Task` is `Task<()>`): a shared
/// handle that ends once, with a result, an error or a cancellation.
///
/// Continuations ([`continue_with`](Self::continue_with)) run synchronously when the task ends
/// (`TaskContinuationOptions.ExecuteSynchronously`); a task is also a [`Future`] of its outcome.
/// Tasks belong to the thread that made them, like the observables
/// (docs/porting/reactive.md, section 5.8).
#[deprecated(
    since = "0.5.0",
    note = "use `ferroui_system::task::Shared<Result<T, ObservableError>>` (`Shared::channel`, `JoinHandle::shared`, `Shared::on_finished`); `Shared::from(task)` converts"
)]
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
        Self { inner: Rc::new(TaskInner { id: next_task_id(), status: Cell::new(status), shared: Shared::detached() }) }
    }

    /// A task that has ended with `result` (`Task.FromResult`).
    pub fn from_result(result: T) -> Self {
        let task = Self::with_status(TaskStatus::WaitingForActivation);
        task.try_complete(Ok(result));
        task
    }

    /// A task that has ended with `error` (`Task.FromException`); a cancellation ([`Canceled`],
    /// [`TaskCanceledError`]) makes it canceled.
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
        if !self.is_faulted() {
            return None;
        }
        self.inner.shared.with_outcome(|outcome| match outcome {
            Some(Ok(Err(error))) => Some(error.clone()),
            _ => None,
        })
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
        match self.inner.shared.result() {
            Some(outcome) if self.is_completed() => outcome,
            _ => panic!("the result of a task that has not ended was asked for; await it or use continue_with"),
        }
    }

    /// Runs `continuation` when the task ends, synchronously on the thread that ends it, or at once
    /// when it has ended (`ContinueWith` with `TaskContinuationOptions.ExecuteSynchronously`). The
    /// id removes a continuation that has not run ([`remove_continuation`](Self::remove_continuation),
    /// the cancellation token of `ContinueWith`); `None` when it ran at once.
    pub fn continue_with(&self, continuation: impl FnOnce(&Task<T>) + 'static) -> Option<ContinuationId> {
        let task = self.clone();
        self.inner.shared.on_finished(move |_| continuation(&task)).map(ContinuationId)
    }

    /// Removes a continuation that has not run; false when it has run or was removed.
    pub fn remove_continuation(&self, id: ContinuationId) -> bool {
        self.inner.shared.remove_on_finished(id.0)
    }

    /// Ends the task with `outcome` unless it has ended; an error that is a cancellation
    /// ([`Canceled`], [`TaskCanceledError`]) cancels it. False when it had ended.
    pub(crate) fn try_complete(&self, outcome: Result<T, ObservableError>) -> bool {
        if self.is_completed() {
            return false;
        }
        let outcome = match outcome {
            Ok(result) => {
                self.inner.status.set(TaskStatus::RanToCompletion);
                Ok(result)
            }
            Err(error) if Canceled::is_cancellation(&error) => {
                self.inner.status.set(TaskStatus::Canceled);
                Err(self.canceled_error())
            }
            Err(error) => {
                self.inner.status.set(TaskStatus::Faulted);
                Err(error)
            }
        };
        self.inner.shared.finish(Ok(outcome));
        true
    }

    pub(crate) fn try_cancel(&self) -> bool {
        if self.is_completed() {
            return false;
        }
        self.inner.status.set(TaskStatus::Canceled);
        self.inner.shared.finish(Ok(Err(self.canceled_error())));
        true
    }

    fn canceled_error(&self) -> ObservableError {
        Rc::new(TaskCanceledError { task_id: Some(self.id()) })
    }
}

impl<T: Clone + 'static> Future for Task<T> {
    type Output = Result<T, ObservableError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut shared = self.inner.shared.clone().flatten();
        Pin::new(&mut shared).poll(cx)
    }
}

/// A task as the [`Shared`] of its outcome: the result, the error of a faulted task, or the
/// [`TaskCanceledError`] of a canceled one, with the same continuations and awaiters.
impl<T: 'static> From<Task<T>> for Shared<Result<T, ObservableError>> {
    fn from(task: Task<T>) -> Self {
        task.inner.shared.clone()
    }
}

/// A future of the outcome of a [`oneshot`] channel of a `Result` as a task, which runs on the
/// current thread ([`spawn_local`](super::spawn_local)): the result, the error, or a cancellation
/// when the sender is dropped without sending.
impl<T: 'static> From<oneshot::Receiver<Result<T, ObservableError>>> for Task<T> {
    fn from(receiver: oneshot::Receiver<Result<T, ObservableError>>) -> Self {
        super::local_executor::spawn_local(async move { receiver.await.unwrap_or_else(|canceled| Err(Rc::new(canceled) as ObservableError)) })
    }
}

/// A completion source as the sender of a [`oneshot`] channel of a `Result`: what is sent ends the
/// task of the source when the current thread runs the task that receives it (inside the call that
/// sends, on this thread outside of a dispatcher job), and dropping the sender cancels the task.
impl<T: 'static> From<TaskCompletionSource<T>> for oneshot::Sender<Result<T, ObservableError>> {
    fn from(source: TaskCompletionSource<T>) -> Self {
        let (sender, receiver) = oneshot::channel();
        drop(crate::task::spawn_local(async move {
            match receiver.await {
                Ok(Ok(result)) => {
                    source.try_set_result(result);
                }
                Ok(Err(error)) => {
                    source.try_set_exception(error);
                }
                Err(Canceled) => {
                    source.try_set_canceled();
                }
            }
        }));
        sender
    }
}

/// The producer side of a task that code ends by hand (`TaskCompletionSource<TResult>`).
#[deprecated(
    since = "0.5.0",
    note = "use `ferroui_system::task::oneshot::channel` (a sender of a `Result`; dropping it cancels), or `Shared::channel` for synchronous continuations"
)]
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

    /// Ends the task with `error` (`SetException`); a cancellation ([`Canceled`],
    /// [`TaskCanceledError`]) cancels it.
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
