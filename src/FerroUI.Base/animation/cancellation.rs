//! Cancellation callbacks for UI-thread state.
//!
//! A [`CancellationToken`] can be cancelled from any thread and runs its
//! callbacks on the cancelling thread, so they must be `Send`. What an
//! animation has to do on cancellation touches thread-affine objects. The
//! action therefore stays in a per-thread table and the token only carries
//! its key: cancelling on the owning thread runs the action right away,
//! cancelling on another thread posts it to the owning thread's dispatcher.
//!
//! [`CancellationToken::register_local`] brings a callback back to its thread
//! through the thread's schedule, which the dispatcher is not yet (step A5 of
//! phase C4 of #423, docs/idiomatic-rust-async.md, section 3.1): a callback
//! cancelled from another thread would wait for `run_pending_wakes`. This
//! module keeps posting to the dispatcher until then, with the guard of the
//! token.

use crate::threading::{CancellationGuard, CancellationToken, Dispatcher, DispatcherPriority};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;

crate::ferro_thread_local! {
    static ACTIONS: RefCell<HashMap<u64, Box<dyn FnOnce()>>> = RefCell::new(HashMap::new());
    static NEXT_ID: Cell<u64> = const { Cell::new(0) };
}

fn run_local(id: u64) {
    let action = ACTIONS.with(|actions| actions.borrow_mut().remove(&id));
    if let Some(action) = action {
        action();
    }
}

/// An action registered with [`register_local`]; dropping it removes the
/// action, on the thread that registered it.
pub(crate) struct LocalCancellationRegistration {
    id: u64,
    _registration: CancellationGuard,
    /// The action belongs to the thread that registered it.
    _thread_bound: std::marker::PhantomData<std::rc::Rc<()>>,
}

impl LocalCancellationRegistration {
    /// Removes the action so that it no longer runs on cancellation, the same
    /// as dropping the registration.
    pub(crate) fn dispose(self) {
        drop(self);
    }
}

impl Drop for LocalCancellationRegistration {
    fn drop(&mut self) {
        // Dropped outside of the borrow: the action may own something whose
        // drop reaches the table.
        let action = ACTIONS.try_with(|actions| actions.borrow_mut().remove(&self.id)).ok().flatten();
        drop(action);
    }
}

/// Registers `action` to run on the current thread when `token` is
/// cancelled. When the token is already cancelled it runs immediately.
pub(crate) fn register_local(token: &CancellationToken, action: impl FnOnce() + 'static) -> LocalCancellationRegistration {
    let id = NEXT_ID.with(|next| {
        let id = next.get();
        next.set(id + 1);
        id
    });
    ACTIONS.with(|actions| actions.borrow_mut().insert(id, Box::new(action)));

    let dispatcher = Dispatcher::current_dispatcher();
    let registration = token.register(move || {
        if dispatcher.check_access() {
            run_local(id);
        } else {
            dispatcher.post(move || run_local(id), DispatcherPriority::SEND);
        }
    });
    LocalCancellationRegistration { id, _registration: registration, _thread_bound: std::marker::PhantomData }
}
