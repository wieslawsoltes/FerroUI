use super::ObservableError;

/// The handlers an [`AnonymousObserver`](super::AnonymousObserver) uses for
/// the notifications it was given no closure for.
pub(crate) struct AnonymousObserverNonGenericHelper;

impl AnonymousObserverNonGenericHelper {
    /// Rethrows the error: panics with its message.
    pub fn throws_on_error(error: ObservableError) {
        panic!("{error}")
    }

    /// Does nothing.
    pub fn no_op_completed() {}
}
