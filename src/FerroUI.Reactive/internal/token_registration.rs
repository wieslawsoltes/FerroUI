//! The registration of a callback with a cancellation token, as a disposable
//! (`CancellationTokenRegistration` disposed by a sink).

use ferroui_system::threading::{CancellationGuard, CancellationToken};
use ferroui_system::IDisposable;
use std::cell::RefCell;
use std::rc::Rc;

/// A callback that runs on the thread of the subscription when the token is canceled, from any
/// thread ([`CancellationToken::register_local`]); disposing it removes the registration, and
/// dropping it keeps the registration (the contract of `IDisposable`, which the sinks rely on).
pub(crate) struct TokenRegistration(RefCell<Option<CancellationGuard>>);

impl TokenRegistration {
    /// Registers `callback`; a token canceled already runs it at once.
    pub(crate) fn register(token: &CancellationToken, callback: impl FnOnce() + 'static) -> Rc<Self> {
        Rc::new(Self(RefCell::new(Some(token.register_local(callback)))))
    }
}

impl IDisposable for TokenRegistration {
    fn dispose(&self) {
        // Dropped outside of the borrow.
        let guard = self.0.borrow_mut().take();
        drop(guard);
    }
}

impl Drop for TokenRegistration {
    fn drop(&mut self) {
        if let Some(guard) = self.0.get_mut().take() {
            guard.detach();
        }
    }
}
