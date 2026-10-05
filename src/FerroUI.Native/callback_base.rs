//! What every Rust object called by native code has in common: a panic must
//! not unwind into native frames.
//!
//! The reference implementation gives its callback classes a base class that
//! hands an exception raised by a callback to the dispatcher implementation,
//! which stops the innermost run loop and rethrows the exception from it.
//! Here every callback body runs inside [`guard`], which does the same with
//! a panic.
//!
//! The lifetime half of that base class (an object kept alive by native
//! references and notified when the last one is released) is covered by
//! `ferroui_microcom::ComObject`: the Rust value is dropped with the last
//! reference, so `Drop` is the "destroyed" notification.

use crate::dispatcher_impl::DispatcherImpl;
use std::any::Any;
use std::panic::{catch_unwind, AssertUnwindSafe};

/// Runs a native callback body. A panic is handed to the dispatcher
/// implementation of the current thread and `default` is returned to native
/// code.
pub(crate) fn guard<R>(default: R, body: impl FnOnce() -> R) -> R {
    match catch_unwind(AssertUnwindSafe(body)) {
        Ok(result) => result,
        Err(payload) => {
            raise_exception(payload);
            default
        }
    }
}

/// Hands a panic caught in a native callback to the dispatcher
/// implementation of the current thread, if the thread has one.
pub(crate) fn raise_exception(payload: Box<dyn Any + Send>) {
    if let Some(dispatcher_impl) = DispatcherImpl::current() {
        dispatcher_impl.propagate_callback_exception(payload);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guard_returns_the_result_of_the_body() {
        assert_eq!(guard(0, || 7), 7);
    }

    #[test]
    fn guard_returns_the_default_when_the_body_panics() {
        // No dispatcher implementation on this thread: the panic is dropped,
        // as in the reference implementation.
        let hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));
        let result = guard(-1, || -> i32 { panic!("callback failed") });
        std::panic::set_hook(hook);
        assert_eq!(result, -1);
    }
}
