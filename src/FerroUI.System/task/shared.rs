//! [`Shared`]: the outcome of a task for several observers.

use super::executor::Header;
use super::join::JoinError;
use std::cell::RefCell;
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::Arc;
use std::task::{Context, Poll, Waker};

type OnFinished<T> = Box<dyn FnOnce(&Result<T, JoinError>)>;

struct SharedInner<T> {
    header: Arc<Header>,
    outcome: RefCell<Option<Result<T, JoinError>>>,
    callbacks: RefCell<Vec<OnFinished<T>>>,
    wakers: RefCell<Vec<Waker>>,
}

/// The outcome of a task for several observers ([`JoinHandle::shared`](super::JoinHandle::shared)):
/// each clone is a future of a copy of it, and [`on_finished`](Self::on_finished) adds
/// synchronous continuations.
///
/// When the task ends, the continuations run first, in the order they were added, and then the
/// futures awaiting the outcome are woken, in the order they awaited it: on the thread of the
/// `Shared`, inside the call that ends the task when it ends there, otherwise when that thread
/// runs [`run_pending`](super::run_pending). The payload of a panic reaches each observer as its
/// message, a `String`. A `Shared` belongs to the thread that made it.
pub struct Shared<T> {
    inner: Rc<SharedInner<T>>,
}

impl<T> Clone for Shared<T> {
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone() }
    }
}

impl<T: Clone + 'static> Shared<T> {
    pub(crate) fn new(header: Arc<Header>) -> Self {
        Self { inner: Rc::new(SharedInner { header, outcome: RefCell::new(None), callbacks: RefCell::new(Vec::new()), wakers: RefCell::new(Vec::new()) }) }
    }

    /// Keeps the outcome of the task, runs the continuations, then wakes the awaiting futures.
    pub(crate) fn finish(&self, outcome: Result<T, JoinError>) {
        let outcome = outcome.map_err(|error| error.duplicate());
        *self.inner.outcome.borrow_mut() = Some(outcome);
        let callbacks = std::mem::take(&mut *self.inner.callbacks.borrow_mut());
        for callback in callbacks {
            let outcome = self.inner.outcome.borrow();
            callback(outcome.as_ref().expect("a finished task keeps its outcome"));
        }
        let wakers = std::mem::take(&mut *self.inner.wakers.borrow_mut());
        for waker in wakers {
            waker.wake();
        }
    }

    /// Whether the task has ended, and its observers have been told.
    pub fn is_finished(&self) -> bool {
        self.inner.outcome.borrow().is_some()
    }

    /// Runs `callback` with the outcome when the task ends, before the awaiting futures are woken,
    /// or at once when it has ended.
    pub fn on_finished(&self, callback: impl FnOnce(&Result<T, JoinError>) + 'static) {
        if let Some(outcome) = self.inner.outcome.borrow().as_ref() {
            callback(outcome);
            return;
        }
        self.inner.callbacks.borrow_mut().push(Box::new(callback));
    }

    /// A copy of the outcome when the task has ended.
    pub fn outcome(&self) -> Option<Result<T, JoinError>> {
        self.inner.outcome.borrow().as_ref().map(|outcome| match outcome {
            Ok(value) => Ok(value.clone()),
            Err(error) => Err(error.duplicate()),
        })
    }

    /// Aborts the task (see [`JoinHandle::abort`](super::JoinHandle::abort)).
    pub fn abort(&self) {
        if !self.is_finished() {
            self.inner.header.abort();
        }
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
        f.debug_struct("Shared").field("id", &self.inner.header.id()).field("finished", &self.inner.outcome.borrow().is_some()).finish()
    }
}
