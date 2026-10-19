//! The registration of a callback with a cancellation token, as a disposable
//! (`CancellationTokenRegistration` disposed by a sink).

use ferroui_system::threading::{CancellationToken, LocalCancellationTokenRegistration};
use ferroui_system::IDisposable;
use std::rc::Rc;

/// A callback that runs on the thread of the subscription when the token is canceled, from any
/// thread ([`CancellationToken::register_local`]); disposing it removes the registration.
pub(crate) struct TokenRegistration(LocalCancellationTokenRegistration);

impl TokenRegistration {
    /// Registers `callback`; a token canceled already runs it at once.
    pub(crate) fn register(token: &CancellationToken, callback: impl FnOnce() + 'static) -> Rc<Self> {
        Rc::new(Self(token.register_local(callback)))
    }
}

impl IDisposable for TokenRegistration {
    fn dispose(&self) {
        self.0.dispose();
    }
}
