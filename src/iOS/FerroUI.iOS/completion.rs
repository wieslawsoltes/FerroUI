//! A value that arrives later, on the main thread: what stands for the
//! task completion sources of the reference, whose tasks the callers
//! await. The completion blocks of UIKit set the value; the future that
//! waits for it is polled on the dispatcher of the main thread.

use std::cell::RefCell;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};

struct State<T> {
    result: Option<T>,
    waker: Option<Waker>,
}

/// The side that sets the value.
pub struct Completion<T> {
    state: Rc<RefCell<State<T>>>,
}

/// The side that waits for the value.
pub struct CompletionFuture<T> {
    state: Rc<RefCell<State<T>>>,
}

impl<T> Clone for Completion<T> {
    fn clone(&self) -> Self {
        Self { state: self.state.clone() }
    }
}

impl<T> Completion<T> {
    /// Creates a completion and the future of its value.
    pub fn new() -> (Completion<T>, CompletionFuture<T>) {
        let state = Rc::new(RefCell::new(State { result: None, waker: None }));
        (Completion { state: state.clone() }, CompletionFuture { state })
    }

    /// Sets the value, unless one was set; tells whether it was set now.
    pub fn try_set_result(&self, result: T) -> bool {
        let waker = {
            let mut state = self.state.borrow_mut();
            if state.result.is_some() {
                return false;
            }
            state.result = Some(result);
            state.waker.take()
        };
        if let Some(waker) = waker {
            waker.wake();
        }
        true
    }
}

impl<T> Future for CompletionFuture<T> {
    type Output = T;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<T> {
        let mut state = self.state.borrow_mut();
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
    // Not from the reference.
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::task::Wake;

    struct CountingWaker(AtomicUsize);

    impl Wake for CountingWaker {
        fn wake(self: Arc<Self>) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[test]
    fn the_future_is_ready_once_the_value_is_set() {
        let (completion, mut future) = Completion::<u32>::new();
        let counter = Arc::new(CountingWaker(AtomicUsize::new(0)));
        let waker = Waker::from(counter.clone());
        let mut cx = Context::from_waker(&waker);

        assert_eq!(Poll::Pending, Pin::new(&mut future).poll(&mut cx));
        assert_eq!(0, counter.0.load(Ordering::SeqCst));
        assert!(completion.try_set_result(7));
        assert_eq!(1, counter.0.load(Ordering::SeqCst));
        // The first value stays.
        assert!(!completion.clone().try_set_result(8));
        assert_eq!(Poll::Ready(7), Pin::new(&mut future).poll(&mut cx));
    }

    #[test]
    fn a_value_set_before_the_first_poll_is_returned_by_it() {
        let (completion, mut future) = Completion::<&str>::new();
        assert!(completion.try_set_result("done"));
        let waker = Waker::from(Arc::new(CountingWaker(AtomicUsize::new(0))));
        let mut cx = Context::from_waker(&waker);
        assert_eq!(Poll::Ready("done"), Pin::new(&mut future).poll(&mut cx));
    }
}
