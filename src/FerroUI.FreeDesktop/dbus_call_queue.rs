//! Calls made one at a time, in the order they were queued (the port of
//! `DBusCallQueue.cs`).
//!
//! The calls of an input method context have to arrive in order: a key
//! must not overtake the focus change that precedes it. D-Bus keeps the
//! order of messages, but a call that is made from the continuation of
//! another one is sent whenever that continuation runs; the queue makes
//! the order the one of `enqueue`.

use ferroui_base::input::LocalBoxFuture;
use ferroui_base::threading::Dispatcher;
use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};

/// Why a queued call did not give a result.
#[derive(Debug)]
pub enum DBusCallError {
    /// The queue was emptied before the call was made (`FailAll`: the
    /// `OperationCanceledException` of the reference).
    Canceled,
    /// The call failed.
    DBus(zbus::Error),
}

impl fmt::Display for DBusCallError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DBusCallError::Canceled => f.write_str("The operation was canceled."),
            DBusCallError::DBus(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for DBusCallError {}

impl From<zbus::Error> for DBusCallError {
    fn from(error: zbus::Error) -> Self {
        DBusCallError::DBus(error)
    }
}

impl From<zbus::fdo::Error> for DBusCallError {
    fn from(error: zbus::fdo::Error) -> Self {
        DBusCallError::DBus(error.into())
    }
}

/// The result of a call over D-Bus.
pub type DBusResult<T = ()> = Result<T, DBusCallError>;

type Callback = Box<dyn FnOnce() -> LocalBoxFuture<DBusResult>>;
type OnFinish = Box<dyn FnOnce(Option<DBusCallError>)>;
type ErrorHandler = Box<dyn Fn(DBusCallError) -> LocalBoxFuture<()>>;

struct Item {
    callback: Callback,
    on_finish: Option<OnFinish>,
}

struct Inner {
    error_handler: ErrorHandler,
    q: RefCell<VecDeque<Item>>,
    processing: Cell<bool>,
}

/// A queue of asynchronous calls of which one is in flight at a time.
#[derive(Clone)]
pub struct DBusCallQueue {
    inner: Rc<Inner>,
}

impl DBusCallQueue {
    /// `error_handler` is awaited with the failure of a call nobody waits
    /// for (one queued with [`enqueue`](Self::enqueue)).
    pub fn new(error_handler: impl Fn(DBusCallError) -> LocalBoxFuture<()> + 'static) -> Self {
        Self {
            inner: Rc::new(Inner {
                error_handler: Box::new(error_handler),
                q: RefCell::new(VecDeque::new()),
                processing: Cell::new(false),
            }),
        }
    }

    pub fn enqueue<F>(&self, cb: impl FnOnce() -> F + 'static)
    where
        F: Future<Output = DBusResult> + 'static,
    {
        self.inner.q.borrow_mut().push_back(Item { callback: Box::new(move || Box::pin(cb())), on_finish: None });
        self.process();
    }

    /// Queues a call and gives the future of its completion
    /// (`EnqueueAsync`).
    pub fn enqueue_async<F>(&self, cb: impl FnOnce() -> F + 'static) -> impl Future<Output = DBusResult> + 'static
    where
        F: Future<Output = DBusResult> + 'static,
    {
        let (sender, receiver) = completion::<DBusResult>();
        self.inner.q.borrow_mut().push_back(Item {
            callback: Box::new(move || Box::pin(cb())),
            on_finish: Some(Box::new(move |e| match e {
                None => sender.try_set(Ok(())),
                Some(e) => sender.try_set(Err(e)),
            })),
        });
        self.process();
        receiver
    }

    /// Queues a call with a result and gives the future of the result
    /// (`EnqueueAsync<T>`).
    pub fn enqueue_async_with_result<T, F>(
        &self,
        cb: impl FnOnce() -> F + 'static,
    ) -> impl Future<Output = DBusResult<T>> + 'static
    where
        T: 'static,
        F: Future<Output = DBusResult<T>> + 'static,
    {
        let (sender, receiver) = completion::<DBusResult<T>>();
        let result_sender = sender.clone();
        self.inner.q.borrow_mut().push_back(Item {
            callback: Box::new(move || {
                Box::pin(async move {
                    let res = cb().await?;
                    result_sender.try_set(Ok(res));
                    Ok(())
                })
            }),
            on_finish: Some(Box::new(move |e| {
                if let Some(e) = e {
                    sender.try_set(Err(e));
                }
            })),
        });
        self.process();
        receiver
    }

    /// Makes the queued calls, one after the other (`Process`, an
    /// `async void` of the reference: a task of the UI dispatcher here,
    /// which starts with the next job of the dispatcher and not inside the
    /// call that queued).
    fn process(&self) {
        if self.inner.processing.get() {
            return;
        }
        self.inner.processing.set(true);

        let inner = self.inner.clone();
        drop(Dispatcher::ui_thread().invoke_async_task_local(move || async move {
            // The flag is reset when the loop is left in any way, as the
            // `finally` of the reference.
            struct ResetProcessing(Rc<Inner>);
            impl Drop for ResetProcessing {
                fn drop(&mut self) {
                    self.0.processing.set(false);
                }
            }
            let _reset = ResetProcessing(inner.clone());

            loop {
                let Some(item) = inner.q.borrow_mut().pop_front() else {
                    break;
                };
                match (item.callback)().await {
                    Ok(()) => {
                        if let Some(on_finish) = item.on_finish {
                            on_finish(None);
                        }
                    }
                    Err(e) => {
                        if let Some(on_finish) = item.on_finish {
                            on_finish(Some(e));
                        } else {
                            (inner.error_handler)(e).await;
                        }
                    }
                }
            }
        }));
    }

    /// Empties the queue; the calls somebody waits for fail as canceled.
    pub fn fail_all(&self) {
        loop {
            let Some(item) = self.inner.q.borrow_mut().pop_front() else {
                break;
            };
            if let Some(on_finish) = item.on_finish {
                on_finish(Some(DBusCallError::Canceled));
            }
        }
    }
}

/// The two ends of a result that is set once (a `TaskCompletionSource`).
/// A receiver whose sender is gone without a result resolves as canceled
/// where the task of the reference would never complete: nothing waits
/// for ever on a call that was dropped.
pub(crate) fn completion<T: CanceledValue>() -> (CompletionSender<T>, CompletionReceiver<T>) {
    let state = Rc::new(CompletionState { value: RefCell::new(None), waker: RefCell::new(None), senders: Cell::new(1) });
    (CompletionSender { state: state.clone() }, CompletionReceiver { state })
}

/// A result type that can say "canceled".
pub(crate) trait CanceledValue {
    fn canceled() -> Self;
}

impl<T> CanceledValue for DBusResult<T> {
    fn canceled() -> Self {
        Err(DBusCallError::Canceled)
    }
}

impl CanceledValue for () {
    fn canceled() -> Self {}
}

struct CompletionState<T> {
    value: RefCell<Option<T>>,
    waker: RefCell<Option<Waker>>,
    senders: Cell<usize>,
}

impl<T> CompletionState<T> {
    fn wake(&self) {
        if let Some(waker) = self.waker.borrow_mut().take() {
            waker.wake();
        }
    }
}

pub(crate) struct CompletionSender<T> {
    state: Rc<CompletionState<T>>,
}

impl<T> CompletionSender<T> {
    /// Sets the result unless one was set (`TrySetResult`).
    pub(crate) fn try_set(&self, value: T) {
        {
            let mut slot = self.state.value.borrow_mut();
            if slot.is_some() {
                return;
            }
            *slot = Some(value);
        }
        self.state.wake();
    }
}

impl<T> Clone for CompletionSender<T> {
    fn clone(&self) -> Self {
        self.state.senders.set(self.state.senders.get() + 1);
        Self { state: self.state.clone() }
    }
}

impl<T> Drop for CompletionSender<T> {
    fn drop(&mut self) {
        self.state.senders.set(self.state.senders.get() - 1);
        if self.state.senders.get() == 0 {
            self.state.wake();
        }
    }
}

pub(crate) struct CompletionReceiver<T> {
    state: Rc<CompletionState<T>>,
}

impl<T: CanceledValue> Future for CompletionReceiver<T> {
    type Output = T;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<T> {
        if let Some(value) = self.state.value.borrow_mut().take() {
            return Poll::Ready(value);
        }
        if self.state.senders.get() == 0 {
            return Poll::Ready(T::canceled());
        }
        *self.state.waker.borrow_mut() = Some(cx.waker().clone());
        Poll::Pending
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests of this class.
    use super::*;
    use crate::test_support::pump_until;
    use std::cell::RefCell;

    fn queue_with_log(log: &Rc<RefCell<Vec<String>>>) -> DBusCallQueue {
        let log = log.clone();
        DBusCallQueue::new(move |e| {
            let log = log.clone();
            Box::pin(async move { log.borrow_mut().push(format!("error handler: {e}")) })
        })
    }

    fn failure(text: &str) -> DBusCallError {
        DBusCallError::DBus(zbus::Error::Failure(text.to_string()))
    }

    #[test]
    fn calls_run_one_at_a_time_in_the_order_they_were_queued() {
        let _scope = crate::test_support::scope();
        let log = Rc::new(RefCell::new(Vec::new()));
        let queue = queue_with_log(&log);
        let (release_first, first_released) = completion::<()>();

        {
            let log = log.clone();
            queue.enqueue(move || async move {
                log.borrow_mut().push("first starts".to_string());
                first_released.await;
                log.borrow_mut().push("first ends".to_string());
                Ok(())
            });
        }
        {
            let log = log.clone();
            queue.enqueue(move || async move {
                log.borrow_mut().push("second".to_string());
                Ok(())
            });
        }

        Dispatcher::ui_thread().run_jobs(None);
        // The second call waits for the first.
        assert_eq!(*log.borrow(), ["first starts"]);

        release_first.try_set(());
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(*log.borrow(), ["first starts", "first ends", "second"]);
    }

    #[test]
    fn a_failure_nobody_waits_for_goes_to_the_error_handler_and_the_queue_goes_on() {
        let _scope = crate::test_support::scope();
        let log = Rc::new(RefCell::new(Vec::new()));
        let queue = queue_with_log(&log);

        queue.enqueue(|| async { Err(failure("lost")) });
        {
            let log = log.clone();
            queue.enqueue(move || async move {
                log.borrow_mut().push("next".to_string());
                Ok(())
            });
        }

        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(log.borrow().len(), 2);
        assert!(log.borrow()[0].starts_with("error handler: "), "{:?}", log.borrow());
        assert!(log.borrow()[0].contains("lost"));
        assert_eq!(log.borrow()[1], "next");
    }

    #[test]
    fn an_awaited_call_gets_its_result_or_its_failure_and_not_the_error_handler() {
        let _scope = crate::test_support::scope();
        let log = Rc::new(RefCell::new(Vec::new()));
        let queue = queue_with_log(&log);

        let done = queue.enqueue_async(|| async { Ok(()) });
        let value = queue.enqueue_async_with_result(|| async { Ok(42) });
        let failed = queue.enqueue_async_with_result(|| async { Err::<i32, _>(failure("broken")) });

        let results = Rc::new(RefCell::new(None));
        {
            let results = results.clone();
            drop(Dispatcher::ui_thread().invoke_async_task_local(move || async move {
                let outcome = (done.await, value.await, failed.await);
                *results.borrow_mut() = Some(outcome);
            }));
        }
        pump_until(|| results.borrow().is_some());

        let (done, value, failed) = results.borrow_mut().take().unwrap();
        assert!(done.is_ok());
        assert_eq!(value.unwrap(), 42);
        assert!(matches!(failed, Err(DBusCallError::DBus(_))));
        assert!(log.borrow().is_empty());
    }

    #[test]
    fn fail_all_cancels_what_is_queued_behind_the_call_in_flight() {
        let _scope = crate::test_support::scope();
        let log = Rc::new(RefCell::new(Vec::new()));
        let queue = queue_with_log(&log);
        let (release_first, first_released) = completion::<()>();

        queue.enqueue(move || async move {
            first_released.await;
            Ok(())
        });
        let waiting = queue.enqueue_async_with_result(|| async { Ok(1) });
        {
            let log = log.clone();
            queue.enqueue(move || async move {
                log.borrow_mut().push("never".to_string());
                Ok(())
            });
        }
        Dispatcher::ui_thread().run_jobs(None);

        queue.fail_all();
        release_first.try_set(());

        let result = Rc::new(RefCell::new(None));
        {
            let result = result.clone();
            drop(Dispatcher::ui_thread().invoke_async_task_local(move || async move {
                *result.borrow_mut() = Some(waiting.await);
            }));
        }
        pump_until(|| result.borrow().is_some());

        assert!(matches!(result.borrow_mut().take().unwrap(), Err(DBusCallError::Canceled)));
        assert!(log.borrow().is_empty());
    }
}
