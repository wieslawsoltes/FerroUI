//! Keeps a panic from unwinding out of a window procedure.
//!
//! The system calls a window procedure through its own frames, which a
//! panic must not cross. A window procedure of this backend therefore runs
//! its body under [`guard`]: a panic is caught and kept, the procedure
//! returns a default result, and the message loop raises the panic again
//! once the system has returned to it ([`resume_pending`]). The reference
//! has no counterpart: the managed runtime carries an exception through the
//! frames of the system.

use std::any::Any;
use std::cell::RefCell;
use std::panic::{catch_unwind, resume_unwind, AssertUnwindSafe};

thread_local! {
    static PENDING: RefCell<Option<Box<dyn Any + Send>>> = const { RefCell::new(None) };
}

/// Runs the body of a window procedure. A panic of the body is kept for
/// [`resume_pending`] and `default` is returned in its place; while a panic
/// is kept, bodies are not run at all, so that the first panic is the one
/// that is reported.
pub(crate) fn guard<R>(default: R, body: impl FnOnce() -> R) -> R {
    if PENDING.with(|pending| pending.borrow().is_some()) {
        return default;
    }

    match catch_unwind(AssertUnwindSafe(body)) {
        Ok(result) => result,
        Err(payload) => {
            PENDING.with(|pending| {
                let mut pending = pending.borrow_mut();
                if pending.is_none() {
                    *pending = Some(payload);
                }
            });
            default
        }
    }
}

/// Raises the panic a window procedure was kept from raising, if there is
/// one. Called by code of this backend that the system has returned to.
pub(crate) fn resume_pending() {
    let payload = PENDING.with(|pending| pending.borrow_mut().take());
    if let Some(payload) = payload {
        resume_unwind(payload);
    }
}
