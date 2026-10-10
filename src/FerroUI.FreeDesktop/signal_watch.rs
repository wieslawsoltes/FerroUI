//! Subscriptions to signals, and cancellation (no upstream file: these
//! stand for facilities of the D-Bus library and of the runtime of the
//! reference, `Watch...Async` returning an `IDisposable` and
//! `CancellationTokenSource`).
//!
//! A signal of the D-Bus library is a stream. A subscription is a task of
//! the UI dispatcher that takes the items of the stream and calls a
//! handler with each, on the UI thread, until it is disposed.

use ferroui_base::reactive::IDisposable;
use ferroui_base::threading::Dispatcher;
use futures_util::future::{select, Either};
use futures_util::{Stream, StreamExt};
use std::cell::{Cell, RefCell};
use std::future::Future;
use std::pin::{pin, Pin};
use std::rc::Rc;
use std::task::{Context, Poll, Waker};

struct CancellationState {
    cancelled: Cell<bool>,
    wakers: RefCell<Vec<Waker>>,
}

/// A flag that is set once and that any number of tasks of the UI thread
/// can wait for.
#[derive(Clone)]
pub struct CancellationFlag {
    state: Rc<CancellationState>,
}

impl Default for CancellationFlag {
    fn default() -> Self {
        Self::new()
    }
}

impl CancellationFlag {
    pub fn new() -> Self {
        Self { state: Rc::new(CancellationState { cancelled: Cell::new(false), wakers: RefCell::new(Vec::new()) }) }
    }

    pub fn cancel(&self) {
        self.state.cancelled.set(true);
        let wakers = std::mem::take(&mut *self.state.wakers.borrow_mut());
        for waker in wakers {
            waker.wake();
        }
    }

    pub fn is_cancellation_requested(&self) -> bool {
        self.state.cancelled.get()
    }

    /// Resolves when the flag is set.
    pub fn cancelled(&self) -> Cancelled {
        Cancelled { state: self.state.clone() }
    }

    /// Runs `future` until it completes or the flag is set; `None` when
    /// the flag was set first.
    pub async fn run<T>(&self, future: impl Future<Output = T>) -> Option<T> {
        if self.is_cancellation_requested() {
            return None;
        }
        match select(pin!(future), self.cancelled()).await {
            Either::Left((value, _)) => Some(value),
            Either::Right(_) => None,
        }
    }
}

/// The future of [`CancellationFlag::cancelled`].
pub struct Cancelled {
    state: Rc<CancellationState>,
}

impl Future for Cancelled {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if self.state.cancelled.get() {
            return Poll::Ready(());
        }
        let mut wakers = self.state.wakers.borrow_mut();
        if !wakers.iter().any(|waker| waker.will_wake(cx.waker())) {
            wakers.push(cx.waker().clone());
        }
        Poll::Pending
    }
}

struct Subscription {
    flag: CancellationFlag,
}

impl IDisposable for Subscription {
    fn dispose(&self) {
        self.flag.cancel();
    }
}

/// Calls `handler` on the UI thread with every item of `stream` until the
/// returned handle is disposed or the stream ends. After the handle was
/// disposed the handler is not called again, also not for an item that
/// had already arrived.
///
/// # Panics
/// Panics when called from a thread other than the UI thread.
pub fn watch_stream<S>(stream: S, mut handler: impl FnMut(S::Item) + 'static) -> Rc<dyn IDisposable>
where
    S: Stream + 'static,
{
    let flag = CancellationFlag::new();
    let task_flag = flag.clone();
    drop(Dispatcher::ui_thread().invoke_async_task_local(move || async move {
        let mut stream = pin!(stream);
        while let Some(Some(item)) = task_flag.run(stream.next()).await {
            if task_flag.is_cancellation_requested() {
                break;
            }
            handler(item);
        }
    }));

    Rc::new(Subscription { flag })
}

#[cfg(test)]
mod tests {
    // Not from the reference: these types stand for facilities of its
    // runtime and of its D-Bus library.
    use super::*;
    use crate::dbus_call_queue::completion;

    #[test]
    fn a_future_runs_until_the_flag_is_set() {
        let _scope = crate::test_support::scope();
        let flag = CancellationFlag::new();
        let (_never, pending) = completion::<()>();
        let outcome = Rc::new(RefCell::new(Vec::new()));

        {
            let (flag, outcome) = (flag.clone(), outcome.clone());
            drop(Dispatcher::ui_thread().invoke_async_task_local(move || async move {
                let first = flag.run(async { 7 }).await;
                outcome.borrow_mut().push(first);
                let second = flag
                    .run(async {
                        pending.await;
                        8
                    })
                    .await;
                outcome.borrow_mut().push(second);
                // Once set, nothing is started.
                let third = flag.run(async { 9 }).await;
                outcome.borrow_mut().push(third);
            }));
        }
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(*outcome.borrow(), [Some(7)]);

        flag.cancel();
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(*outcome.borrow(), [Some(7), None, None]);
    }

    #[test]
    fn a_subscription_delivers_until_it_is_disposed() {
        let _scope = crate::test_support::scope();
        let seen = Rc::new(RefCell::new(Vec::new()));
        let subscription = {
            let seen = seen.clone();
            watch_stream(futures_util::stream::iter([1, 2, 3]), move |item| seen.borrow_mut().push(item))
        };
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(*seen.borrow(), [1, 2, 3]);
        subscription.dispose();

        // Disposed before the task ran: nothing is delivered.
        let seen = Rc::new(RefCell::new(Vec::new()));
        let subscription = {
            let seen = seen.clone();
            watch_stream(futures_util::stream::iter([1, 2, 3]), move |item: i32| seen.borrow_mut().push(item))
        };
        subscription.dispose();
        Dispatcher::ui_thread().run_jobs(None);
        assert!(seen.borrow().is_empty());
    }
}
