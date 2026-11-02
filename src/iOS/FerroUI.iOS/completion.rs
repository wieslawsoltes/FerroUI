//! A value that arrives later, on the main thread: what stood for the
//! task completion sources of the reference, whose tasks the callers
//! await. Deprecated: the platform uses the one-shot channel of
//! `ferroui_system::task::oneshot`, which these types wrap with the
//! behaviour they had (a value that never comes leaves the future pending).

use ferroui_system::task::oneshot;
use std::cell::RefCell;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll};

/// The side that sets the value.
#[deprecated(
    since = "0.5.0",
    note = "use `ferroui_system::task::oneshot::channel`; a dropped sender cancels its receiver"
)]
pub struct Completion<T> {
    sender: Rc<RefCell<Option<oneshot::Sender<T>>>>,
}

/// The side that waits for the value.
#[deprecated(since = "0.5.0", note = "use `ferroui_system::task::oneshot::Receiver`")]
pub struct CompletionFuture<T> {
    receiver: oneshot::Receiver<T>,
}

#[allow(deprecated)]
impl<T> Clone for Completion<T> {
    fn clone(&self) -> Self {
        Self { sender: self.sender.clone() }
    }
}

#[allow(deprecated)]
impl<T> Completion<T> {
    /// Creates a completion and the future of its value.
    pub fn new() -> (Completion<T>, CompletionFuture<T>) {
        let (sender, receiver) = oneshot::channel();
        (Completion { sender: Rc::new(RefCell::new(Some(sender))) }, CompletionFuture { receiver })
    }

    /// Sets the value, unless one was set; tells whether it was set now.
    pub fn try_set_result(&self, result: T) -> bool {
        // Taken before it sends: sending may poll the task that waits right away.
        let sender = self.sender.borrow_mut().take();
        match sender {
            Some(sender) => {
                // A future that is gone does not want the value.
                let _ = sender.send(result);
                true
            }
            None => false,
        }
    }
}

#[allow(deprecated)]
impl<T> Future for CompletionFuture<T> {
    type Output = T;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<T> {
        match Pin::new(&mut self.receiver).poll(cx) {
            Poll::Ready(Ok(result)) => Poll::Ready(result),
            // Every completion was dropped without a value, or the value was taken: the future
            // stays pending, as the completion it wraps did.
            Poll::Ready(Err(_)) | Poll::Pending => Poll::Pending,
        }
    }
}

#[cfg(test)]
#[allow(deprecated)]
mod tests {
    // Not from the reference.
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::task::{Wake, Waker};

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
