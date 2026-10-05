//! Cancellation callbacks for UI-thread state.
//!
//! A [`CancellationToken`] can be cancelled from any thread and runs its
//! callbacks on the cancelling thread, so they must be `Send`. What an
//! animation has to do on cancellation touches thread-affine objects. The
//! action therefore stays in a per-thread table and the token only carries
//! its key: cancelling on the owning thread runs the action right away,
//! cancelling on another thread posts it to the owning thread's dispatcher.

use crate::threading::{CancellationToken, CancellationTokenRegistration, Dispatcher, DispatcherPriority};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;

thread_local! {
    static ACTIONS: RefCell<HashMap<u64, Box<dyn FnOnce()>>> = RefCell::new(HashMap::new());
    static NEXT_ID: Cell<u64> = const { Cell::new(0) };
}

fn run_local(id: u64) {
    let action = ACTIONS.with(|actions| actions.borrow_mut().remove(&id));
    if let Some(action) = action {
        action();
    }
}

/// An action registered with [`register_local`].
pub(crate) struct LocalCancellationRegistration {
    id: u64,
    registration: CancellationTokenRegistration,
}

impl LocalCancellationRegistration {
    /// Removes the action so that it no longer runs on cancellation. Must
    /// be called on the thread that registered it.
    pub(crate) fn dispose(&self) {
        self.registration.dispose();
        let action = ACTIONS.with(|actions| actions.borrow_mut().remove(&self.id));
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
    LocalCancellationRegistration { id, registration }
}
