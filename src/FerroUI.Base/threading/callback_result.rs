use std::error::Error;

use super::Dispatcher;

/// What an application callback returns when the framework that runs it has no
/// caller to give an error to (an event handler, a posted job): `()`, or
/// `Result<(), E>` whose error goes to
/// [`Dispatcher::raise_unhandled_exception`] of the current thread (#567).
///
/// A callback that fails returns its error instead of panicking: the
/// unhandled-exception events of the dispatcher receive it, also when the build
/// aborts on panic, and a panic stays what it is, a bug. As with
/// [`std::process::Termination`] for `main`, a closure with a `?` names its
/// result type: `move |_, _| -> Result<(), MyError> { .. }`.
pub trait CallbackResult {
    /// Hands the error, if any, to the dispatcher of the current thread.
    fn report(self);
}

impl CallbackResult for () {
    #[inline(always)]
    fn report(self) {}
}

impl<E: Into<Box<dyn Error + Send + Sync>>> CallbackResult for Result<(), E> {
    #[inline]
    fn report(self) {
        if let Err(error) = self {
            Dispatcher::current_dispatcher().raise_unhandled_exception(error);
        }
    }
}
