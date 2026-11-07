//! Keeps a panic from unwinding out of a window procedure.
//!
//! The system calls a window procedure through its own frames, which a
//! panic must not cross. A window procedure of this backend therefore runs
//! its body under [`guard`]: a panic is caught and kept, the procedure
//! returns a default result, and the message loop raises the panic again
//! once the system has returned to it ([`resume_pending`]). The reference
//! has no counterpart: the managed runtime carries an exception through the
//! frames of the system.
//!
//! The WinUI integration (`ferroui-winui`) runs the handlers the system
//! calls under the same guard. Its message loop is the one of the XAML
//! application, which does not ask for the kept panic: the integration sets
//! a hook ([`set_pending_hook`]) that ends that loop when a panic is kept,
//! and raises the panic once the loop has returned.

use std::any::Any;
use std::cell::RefCell;
use std::panic::{catch_unwind, resume_unwind, AssertUnwindSafe};
use std::rc::Rc;

ferroui_base::ferro_thread_local! {
    static PENDING: RefCell<Option<Box<dyn Any + Send>>> = const { RefCell::new(None) };
    static PENDING_HOOK: RefCell<Option<Rc<dyn Fn()>>> = const { RefCell::new(None) };
}

/// Runs the body of a window procedure. A panic of the body is kept for
/// [`resume_pending`] and `default` is returned in its place; while a panic
/// is kept, bodies are not run at all, so that the first panic is the one
/// that is reported.
pub fn guard<R>(default: R, body: impl FnOnce() -> R) -> R {
    if is_pending() {
        return default;
    }

    // Boundary (#567): the body of a window procedure the system calls, which a panic must not
    // unwind through. The panic is kept and raised again by `resume_pending` in the loop.
    match catch_unwind(AssertUnwindSafe(body)) {
        Ok(result) => result,
        Err(payload) => {
            let first = PENDING.with(|pending| {
                let mut pending = pending.borrow_mut();
                let first = pending.is_none();
                if first {
                    *pending = Some(payload);
                }
                first
            });
            if first {
                let hook = PENDING_HOOK.with(|hook| hook.borrow().clone());
                if let Some(hook) = hook {
                    hook();
                }
            }
            default
        }
    }
}

/// Whether a panic is kept on this thread.
pub fn is_pending() -> bool {
    PENDING.with(|pending| pending.borrow().is_some())
}

/// Sets what is called on this thread when a panic is first kept: a message
/// loop that does not call [`resume_pending`] after every message is told
/// to end.
pub fn set_pending_hook(hook: Option<Rc<dyn Fn()>>) {
    PENDING_HOOK.with(|slot| *slot.borrow_mut() = hook);
}

/// Raises the panic a window procedure was kept from raising, if there is
/// one. Called by code of this backend that the system has returned to.
pub fn resume_pending() {
    let payload = PENDING.with(|pending| pending.borrow_mut().take());
    if let Some(payload) = payload {
        resume_unwind(payload);
    }
}
