//! The completion source of a result produced by a later callback (the
//! `TaskCompletionSource<T>` of the managed original): the dialogs wait on
//! it until a window, a popup or a flyout closes.

use std::cell::RefCell;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};

struct State<T> {
    result: Option<T>,
    completed: bool,
    waker: Option<Waker>,
}

/// A result that is set once and awaited by one task of the UI thread.
/// Clones are handles to the same source.
pub(crate) struct TaskCompletionSource<T>(Rc<RefCell<State<T>>>);

impl<T> Clone for TaskCompletionSource<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> TaskCompletionSource<T> {
    pub(crate) fn new() -> Self {
        Self(Rc::new(RefCell::new(State { result: None, completed: false, waker: None })))
    }

    /// Completes the source with `value`, unless it is already completed.
    /// Returns whether it did.
    pub(crate) fn try_set_result(&self, value: T) -> bool {
        let waker = {
            let mut state = self.0.borrow_mut();
            if state.completed {
                return false;
            }
            state.completed = true;
            state.result = Some(value);
            state.waker.take()
        };
        // Woken outside of the borrow: waking may poll the task right away.
        if let Some(waker) = waker {
            waker.wake();
        }
        true
    }

    /// The result, once the source is completed.
    pub(crate) fn task(&self) -> CompletionTask<T> {
        CompletionTask(self.0.clone())
    }
}

/// The future of a [`TaskCompletionSource`].
pub(crate) struct CompletionTask<T>(Rc<RefCell<State<T>>>);

impl<T> Future for CompletionTask<T> {
    type Output = T;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<T> {
        let mut state = self.0.borrow_mut();
        match state.result.take() {
            Some(result) => Poll::Ready(result),
            None => {
                state.waker = Some(cx.waker().clone());
                Poll::Pending
            }
        }
    }
}

#[cfg(test)]
mod tests {
    // Not from upstream: a helper of the port.
    use super::*;
    use std::task::Wake;
    use std::sync::Arc;

    struct NoopWaker;

    impl Wake for NoopWaker {
        fn wake(self: Arc<Self>) {}
    }

    #[test]
    fn completes_once_with_the_first_result() {
        let source = TaskCompletionSource::new();
        let mut task = source.task();
        let waker = Waker::from(Arc::new(NoopWaker));
        let mut cx = Context::from_waker(&waker);

        assert!(Pin::new(&mut task).poll(&mut cx).is_pending());
        assert!(source.try_set_result(1));
        assert!(!source.clone().try_set_result(2));
        assert_eq!(Poll::Ready(1), Pin::new(&mut task).poll(&mut cx));
    }
}
