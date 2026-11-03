//! [`Shared`]: the outcome of a task, or of code that ends it by hand ([`SharedSender`]), for
//! several observers.

use super::executor::Header;
use super::join::JoinError;
use crate::threading::tasks::TaskWorkError;
use crate::ObservableError;
use std::cell::{Cell, RefCell};
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::Arc;
use std::task::{Context, Poll, Waker};

type OnFinished<T> = Box<dyn FnOnce(&Result<T, JoinError>)>;

struct SharedInner<T> {
    /// The task whose outcome this is; `None` for a `Shared` that code ends ([`Shared::channel`]).
    header: Option<Arc<Header>>,
    outcome: RefCell<Option<Result<T, JoinError>>>,
    callbacks: RefCell<Vec<(u64, OnFinished<T>)>>,
    next_callback: Cell<u64>,
    wakers: RefCell<Vec<Waker>>,
}

/// A continuation added with [`Shared::on_finished`] that has not run, for
/// [`Shared::remove_on_finished`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct OnFinishedId(u64);

/// The outcome of a task for several observers ([`JoinHandle::shared`](super::JoinHandle::shared)),
/// or of code that ends it by hand ([`Shared::channel`], the role of `Task<T>` and
/// `TaskCompletionSource<T>` of .NET in the reactive libraries): each clone is a future of a copy of
/// it, and [`on_finished`](Self::on_finished) adds synchronous continuations
/// (`ContinueWith` with `TaskContinuationOptions.ExecuteSynchronously`).
///
/// When it ends, the continuations run first, in the order they were added, and then the futures
/// awaiting the outcome are woken, in the order they awaited it: on the thread of the `Shared`,
/// inside the call that ends it when it ends there (the task's end, [`SharedSender::send`]),
/// otherwise when that thread runs [`run_pending`](super::run_pending). The payload of a panic
/// reaches each observer as its message, a `String`. A `Shared` belongs to the thread that made it.
///
/// The outcome of a future that can fail is part of `T`: the asynchronous forms of the reactive
/// libraries give a `Shared<Result<T, ObservableError>>`, whose [`flatten`](Self::flatten) is a
/// future of `Result<T, ObservableError>`.
pub struct Shared<T> {
    inner: Rc<SharedInner<T>>,
}

impl<T> Clone for Shared<T> {
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone() }
    }
}

impl<T: 'static> Shared<T> {
    fn with_header(header: Option<Arc<Header>>) -> Self {
        Self {
            inner: Rc::new(SharedInner {
                header,
                outcome: RefCell::new(None),
                callbacks: RefCell::new(Vec::new()),
                next_callback: Cell::new(0),
                wakers: RefCell::new(Vec::new()),
            }),
        }
    }

    pub(crate) fn new(header: Arc<Header>) -> Self {
        Self::with_header(Some(header))
    }

    /// A `Shared` that no task ends, which the crate ends with [`finish`](Self::finish).
    pub(crate) fn detached() -> Self {
        Self::with_header(None)
    }

    /// A `Shared` that no task ends, and the [`SharedSender`] that ends it: the port of
    /// `TaskCompletionSource<T>` and its `Task` for code of one thread whose observers continue
    /// synchronously (the reactive libraries).
    pub fn channel() -> (SharedSender<T>, Self) {
        let shared = Self::detached();
        (SharedSender { shared: shared.clone() }, shared)
    }

    /// A `Shared` that has ended with `value` (`Task.FromResult`).
    pub fn ready(value: T) -> Self {
        Self::from_outcome(Ok(value))
    }

    /// A `Shared` that has ended with `outcome`.
    pub fn from_outcome(outcome: Result<T, JoinError>) -> Self {
        let shared = Self::detached();
        shared.finish(outcome);
        shared
    }

    /// Keeps the outcome, runs the continuations, then wakes the awaiting futures; nothing when it
    /// has ended already.
    pub(crate) fn finish(&self, outcome: Result<T, JoinError>) {
        if self.is_finished() {
            return;
        }
        let outcome = outcome.map_err(|error| error.duplicate());
        *self.inner.outcome.borrow_mut() = Some(outcome);
        let callbacks = std::mem::take(&mut *self.inner.callbacks.borrow_mut());
        for (_, callback) in callbacks {
            let outcome = self.inner.outcome.borrow();
            callback(outcome.as_ref().expect("a finished task keeps its outcome"));
        }
        let wakers = std::mem::take(&mut *self.inner.wakers.borrow_mut());
        for waker in wakers {
            waker.wake();
        }
    }

    /// Runs `f` with the outcome, `None` while it has not ended.
    pub(crate) fn with_outcome<R>(&self, f: impl FnOnce(Option<&Result<T, JoinError>>) -> R) -> R {
        f(self.inner.outcome.borrow().as_ref())
    }

    /// Whether it has ended, and its observers have been told.
    pub fn is_finished(&self) -> bool {
        self.inner.outcome.borrow().is_some()
    }

    /// Runs `callback` with the outcome when it ends, before the awaiting futures are woken, or at
    /// once when it has ended. The id removes a continuation that has not run
    /// ([`remove_on_finished`](Self::remove_on_finished), the cancellation token of `ContinueWith`);
    /// `None` when it ran at once.
    pub fn on_finished(&self, callback: impl FnOnce(&Result<T, JoinError>) + 'static) -> Option<OnFinishedId> {
        if let Some(outcome) = self.inner.outcome.borrow().as_ref() {
            callback(outcome);
            return None;
        }
        let id = self.inner.next_callback.get();
        self.inner.next_callback.set(id + 1);
        self.inner.callbacks.borrow_mut().push((id, Box::new(callback)));
        Some(OnFinishedId(id))
    }

    /// Removes a continuation that has not run; false when it has run or was removed.
    pub fn remove_on_finished(&self, id: OnFinishedId) -> bool {
        let removed = {
            let mut callbacks = self.inner.callbacks.borrow_mut();
            let position = callbacks.iter().position(|(entry, _)| *entry == id.0);
            position.map(|position| callbacks.remove(position))
        };
        // Dropped outside of the borrow: what the continuation holds may reach the `Shared`.
        removed.is_some()
    }

    /// Aborts the task (see [`JoinHandle::abort`](super::JoinHandle::abort)); nothing for a
    /// `Shared` that no task ends.
    pub fn abort(&self) {
        if !self.is_finished() {
            if let Some(header) = &self.inner.header {
                header.abort();
            }
        }
    }

    /// Whether two handles are of the same outcome.
    pub fn ptr_eq(this: &Self, other: &Self) -> bool {
        Rc::ptr_eq(&this.inner, &other.inner)
    }
}

impl<T: Clone + 'static> Shared<T> {
    /// A copy of the outcome when it has ended.
    pub fn outcome(&self) -> Option<Result<T, JoinError>> {
        self.inner.outcome.borrow().as_ref().map(|outcome| match outcome {
            Ok(value) => Ok(value.clone()),
            Err(error) => Err(error.duplicate()),
        })
    }
}

/// The error of a [`JoinError`] in the error path of the reactive libraries: an abort is
/// [`Canceled`](super::Canceled), a panic a [`TaskWorkError`] with its message.
pub(crate) fn join_error_to_observable(error: JoinError) -> ObservableError {
    match error {
        JoinError::Canceled => Rc::new(super::Canceled) as ObservableError,
        JoinError::Panicked(payload) => Rc::new(TaskWorkError { message: super::executor::panic_message(&*payload) }) as ObservableError,
    }
}

impl<T: Clone + 'static> Shared<Result<T, ObservableError>> {
    /// A future of the outcome as one result ([`Flatten`]).
    pub fn flatten(self) -> Flatten<T> {
        Flatten { shared: self }
    }

    /// The outcome as one result when it has ended: an abort is the error
    /// [`Canceled`](super::Canceled), a panic a [`TaskWorkError`] with its message.
    pub fn result(&self) -> Option<Result<T, ObservableError>> {
        self.outcome().map(|outcome| outcome.unwrap_or_else(|error| Err(join_error_to_observable(error))))
    }
}

impl<T: Clone + 'static> Future for Shared<T> {
    type Output = Result<T, JoinError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if let Some(outcome) = self.outcome() {
            return Poll::Ready(outcome);
        }
        let mut wakers = self.inner.wakers.borrow_mut();
        if !wakers.iter().any(|waker| waker.will_wake(cx.waker())) {
            wakers.push(cx.waker().clone());
        }
        Poll::Pending
    }
}

impl<T> fmt::Debug for Shared<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Shared")
            .field("id", &self.inner.header.as_ref().map(|header| header.id()))
            .field("finished", &self.inner.outcome.borrow().is_some())
            .finish()
    }
}

/// A [`Shared`] of a `Result<T, ObservableError>` as a future of that result
/// ([`Shared::flatten`]): the futures that the asynchronous forms of the reactive libraries take
/// are futures of `Result<T, ObservableError>`, and an abort or a panic of the task is an error of
/// that result ([`Shared::result`]). Clones await the same outcome.
pub struct Flatten<T> {
    shared: Shared<Result<T, ObservableError>>,
}

impl<T> Clone for Flatten<T> {
    fn clone(&self) -> Self {
        Self { shared: self.shared.clone() }
    }
}

impl<T> Flatten<T> {
    /// The [`Shared`] it awaits.
    pub fn into_shared(self) -> Shared<Result<T, ObservableError>> {
        self.shared
    }

    /// The [`Shared`] it awaits.
    pub fn shared(&self) -> &Shared<Result<T, ObservableError>> {
        &self.shared
    }
}

impl<T: Clone + 'static> Future for Flatten<T> {
    type Output = Result<T, ObservableError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match Pin::new(&mut self.shared).poll(cx) {
            Poll::Ready(Ok(result)) => Poll::Ready(result),
            Poll::Ready(Err(error)) => Poll::Ready(Err(join_error_to_observable(error))),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl<T> fmt::Debug for Flatten<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Flatten").field(&self.shared).finish()
    }
}

/// The end of a [`Shared`] that code ends by hand ([`Shared::channel`]): once, on the thread of
/// the `Shared` (the role of `TaskCompletionSource<T>`: `SetResult` is [`send`](Self::send),
/// `SetCanceled` is [`cancel`](Self::cancel), and `SetException` is `send(Err(error))` on a
/// `Shared` of a `Result`).
///
/// Dropping it without sending leaves the `Shared` pending, as a `TaskCompletionSource` that is
/// dropped leaves its task, and as the reactive libraries need (`ToTask` of a sequence that lets
/// go of its observer without ending, `Never`, gives a task that never ends). This differs from
/// [`oneshot`](super::oneshot), whose dropped sender cancels the receiver.
pub struct SharedSender<T> {
    shared: Shared<T>,
}

impl<T: 'static> SharedSender<T> {
    /// Ends the `Shared` with `value`, running its continuations and then waking its awaiting
    /// futures inside this call.
    pub fn send(self, value: T) {
        self.shared.finish(Ok(value));
    }

    /// Ends the `Shared` with [`JoinError::Canceled`], running its continuations and then waking
    /// its awaiting futures inside this call.
    pub fn cancel(self) {
        self.shared.finish(Err(JoinError::Canceled));
    }

    /// The `Shared` that this ends.
    pub fn shared(&self) -> Shared<T> {
        self.shared.clone()
    }
}

impl<T> fmt::Debug for SharedSender<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SharedSender").field("finished", &self.shared.inner.outcome.borrow().is_some()).finish()
    }
}
