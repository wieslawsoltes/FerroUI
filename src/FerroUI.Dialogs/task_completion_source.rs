//! The completion source of a result produced by a later callback (the
//! `TaskCompletionSource<T>` of the managed original): the dialogs wait on
//! it until a window, a popup or a flyout closes. It is the sender of a
//! one-shot channel (`ferroui_system::task::oneshot`) that the callbacks
//! share; the first result any of them sets completes the channel.

use ferroui_system::task::oneshot;
use std::cell::RefCell;
use std::rc::Rc;

/// The side of a one-shot channel that sets the result once, from any of
/// the callbacks of the UI thread that hold a clone.
pub(crate) struct TaskCompletionSource<T>(Rc<RefCell<Option<oneshot::Sender<T>>>>);

impl<T> Clone for TaskCompletionSource<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> TaskCompletionSource<T> {
    /// A source and the receiver of its result. The receiver ends with
    /// `Canceled` when every clone of the source is dropped without a
    /// result; a dialog keeps its own clone while it waits, so it never
    /// sees that.
    pub(crate) fn new() -> (Self, oneshot::Receiver<T>) {
        let (sender, receiver) = oneshot::channel();
        (Self(Rc::new(RefCell::new(Some(sender)))), receiver)
    }

    /// Completes the source with `value`, unless it is already completed.
    /// Returns whether it did.
    pub(crate) fn try_set_result(&self, value: T) -> bool {
        // Taken before it sends: sending may poll the task that waits right away.
        let sender = self.0.borrow_mut().take();
        match sender {
            Some(sender) => {
                // A receiver that is gone does not want the result.
                let _ = sender.send(value);
                true
            }
            None => false,
        }
    }
}

#[cfg(test)]
mod tests {
    // Not from upstream: a helper of the port.
    use super::*;
    use ferroui_system::task::oneshot::Canceled;
    use std::future::Future;
    use std::pin::Pin;
    use std::task::{Context, Poll, Waker};

    #[test]
    fn completes_once_with_the_first_result() {
        let (source, mut task) = TaskCompletionSource::new();
        let mut cx = Context::from_waker(Waker::noop());

        assert!(Pin::new(&mut task).poll(&mut cx).is_pending());
        assert!(source.try_set_result(1));
        assert!(!source.clone().try_set_result(2));
        assert_eq!(Poll::Ready(Ok(1)), Pin::new(&mut task).poll(&mut cx));
    }

    #[test]
    fn is_canceled_when_every_clone_is_dropped_without_a_result() {
        let (source, mut task) = TaskCompletionSource::<bool>::new();
        let mut cx = Context::from_waker(Waker::noop());
        let other = source.clone();
        drop(source);
        assert!(Pin::new(&mut task).poll(&mut cx).is_pending());
        drop(other);
        assert_eq!(Poll::Ready(Err(Canceled)), Pin::new(&mut task).poll(&mut cx));
    }
}
