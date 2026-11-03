//! [`TaskCompletion`]: the end of a [`Shared`] of a result that an operator ends by hand (the
//! `TaskCompletionSource<TResult>` of upstream's operators, step A7 of phase C4 of #423).

use ferroui_system::task::{Canceled, Shared, SharedSender};
use ferroui_system::ObservableError;
use std::cell::RefCell;
use std::rc::Rc;

struct Inner<T> {
    sender: RefCell<Option<SharedSender<Result<T, ObservableError>>>>,
    shared: Shared<Result<T, ObservableError>>,
}

/// The end of a [`Shared`] of a result that an operator ends by hand, with the `TrySet` forms of a
/// `TaskCompletionSource<TResult>`: the first one ends the `Shared` (its continuations, then its
/// awaiters, inside the call), the others do nothing. Clones end the same `Shared`; dropping all of
/// them leaves it pending, as upstream's task stays.
pub(crate) struct TaskCompletion<T> {
    inner: Rc<Inner<T>>,
}

impl<T> Clone for TaskCompletion<T> {
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone() }
    }
}

impl<T: 'static> TaskCompletion<T> {
    pub(crate) fn new() -> Self {
        let (sender, shared) = Shared::channel();
        Self { inner: Rc::new(Inner { sender: RefCell::new(Some(sender)), shared }) }
    }

    /// The `Shared` that this ends (`Task`).
    pub(crate) fn shared(&self) -> Shared<Result<T, ObservableError>> {
        self.inner.shared.clone()
    }

    /// Ends the `Shared` with `result` unless it has ended; false when it had.
    pub(crate) fn try_send(&self, result: Result<T, ObservableError>) -> bool {
        // Taken out of the cell before it sends: the continuations may reach this completion.
        let sender = self.inner.sender.borrow_mut().take();
        match sender {
            Some(sender) => {
                sender.send(result);
                true
            }
            None => false,
        }
    }

    /// `TrySetResult`.
    pub(crate) fn try_set_result(&self, value: T) -> bool {
        self.try_send(Ok(value))
    }

    /// `TrySetException`.
    pub(crate) fn try_set_exception(&self, error: ObservableError) -> bool {
        self.try_send(Err(error))
    }

    /// `TrySetCanceled`: the error [`Canceled`].
    pub(crate) fn try_set_canceled(&self) -> bool {
        self.try_send(Err(Rc::new(Canceled) as ObservableError))
    }
}
