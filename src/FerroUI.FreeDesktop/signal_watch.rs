//! Subscriptions to signals, and cancellation (no upstream file: these
//! stand for facilities of the D-Bus library and of the runtime of the
//! reference, `Watch...Async` returning an `IDisposable` and
//! `CancellationTokenSource`).
//!
//! A signal of the D-Bus library is a stream. A subscription is a task of
//! the UI dispatcher that takes the items of the stream and calls a
//! handler with each, on the UI thread, until it is disposed.

use ferroui_base::reactive::IDisposable;
use ferroui_base::threading::{CancellationToken, Dispatcher, WaitForCancellation};
use futures_util::future::{select, Either};
use futures_util::{Stream, StreamExt};
use std::future::Future;
use std::pin::{pin, Pin};
use std::rc::Rc;
use std::task::{Context, Poll};

/// Runs `future` until it completes or `token` is canceled; `None` when the token was canceled
/// first, at once when it already is. The future is polled before the cancellation, so a future
/// that completes in the poll that sees the cancellation gives its value.
pub async fn run_until_cancelled<T>(token: &CancellationToken, future: impl Future<Output = T>) -> Option<T> {
    if token.is_cancelled() {
        return None;
    }
    match select(pin!(future), token.cancelled()).await {
        Either::Left((value, _)) => Some(value),
        Either::Right(_) => None,
    }
}

/// A flag that is set once and that any number of tasks of the UI thread
/// can wait for.
#[deprecated(
    since = "0.5.0",
    note = "use `ferroui_base::threading::CancellationToken` (`cancelled()`) and `run_until_cancelled`"
)]
#[derive(Clone)]
pub struct CancellationFlag {
    token: CancellationToken,
}

#[allow(deprecated)]
impl Default for CancellationFlag {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(deprecated)]
impl CancellationFlag {
    pub fn new() -> Self {
        Self { token: CancellationToken::new() }
    }

    pub fn cancel(&self) {
        self.token.cancel();
    }

    pub fn is_cancellation_requested(&self) -> bool {
        self.token.is_cancelled()
    }

    /// Resolves when the flag is set.
    pub fn cancelled(&self) -> Cancelled {
        Cancelled(self.token.cancelled())
    }

    /// Runs `future` until it completes or the flag is set; `None` when
    /// the flag was set first.
    pub async fn run<T>(&self, future: impl Future<Output = T>) -> Option<T> {
        run_until_cancelled(&self.token, future).await
    }
}

/// The token of the flag.
#[allow(deprecated)]
impl From<CancellationFlag> for CancellationToken {
    fn from(flag: CancellationFlag) -> Self {
        flag.token
    }
}

/// The future of [`CancellationFlag::cancelled`].
#[deprecated(since = "0.5.0", note = "use `CancellationToken::cancelled`")]
pub struct Cancelled(WaitForCancellation);

#[allow(deprecated)]
impl Future for Cancelled {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        Pin::new(&mut self.0).poll(cx)
    }
}

struct Subscription {
    token: CancellationToken,
}

impl IDisposable for Subscription {
    fn dispose(&self) {
        self.token.cancel();
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
    let token = CancellationToken::new();
    let task_token = token.clone();
    drop(Dispatcher::ui_thread().invoke_async_task_local(move || async move {
        let mut stream = pin!(stream);
        while let Some(Some(item)) = run_until_cancelled(&task_token, stream.next()).await {
            if task_token.is_cancelled() {
                break;
            }
            handler(item);
        }
    }));

    Rc::new(Subscription { token })
}

#[cfg(test)]
mod tests {
    // Not from the reference: these types stand for facilities of its
    // runtime and of its D-Bus library.
    use super::*;
    use ferroui_system::task::oneshot;
    use std::cell::RefCell;

    #[test]
    fn a_future_runs_until_the_token_is_canceled() {
        let _scope = crate::test_support::scope();
        let token = CancellationToken::new();
        let (_never, pending) = oneshot::channel::<()>();
        let outcome = Rc::new(RefCell::new(Vec::new()));

        {
            let (token, outcome) = (token.clone(), outcome.clone());
            drop(Dispatcher::ui_thread().invoke_async_task_local(move || async move {
                let first = run_until_cancelled(&token, async { 7 }).await;
                outcome.borrow_mut().push(first);
                let second = run_until_cancelled(&token, async {
                    let _ = pending.await;
                    8
                })
                .await;
                outcome.borrow_mut().push(second);
                // Once canceled, nothing is started.
                let third = run_until_cancelled(&token, async { 9 }).await;
                outcome.borrow_mut().push(third);
            }));
        }
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(*outcome.borrow(), [Some(7)]);

        token.cancel();
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
