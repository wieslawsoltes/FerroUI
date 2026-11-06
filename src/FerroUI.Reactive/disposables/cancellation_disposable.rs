//! `CancellationDisposable`.

use super::ICancelable;
use ferroui_system::threading::CancellationToken;
use ferroui_system::IDisposable;
use std::rc::Rc;

/// A disposable that cancels a cancellation token when it is disposed
/// (`CancellationDisposable`).
pub struct CancellationDisposable {
    token: CancellationToken,
}

impl CancellationDisposable {
    /// A disposable over a new cancellation token.
    pub fn new() -> Rc<Self> {
        Self::with_token(CancellationToken::new())
    }

    /// A disposable that cancels `token` when it is disposed (`CancellationDisposable(cts)`).
    pub fn with_token(token: CancellationToken) -> Rc<Self> {
        Rc::new(Self { token })
    }

    /// A disposable that cancels `source` when it is disposed.
    #[deprecated(since = "0.5.0", note = "use `with_token`, with a `CancellationToken::new()` in the place of the source")]
    #[allow(deprecated)]
    pub fn with_source(source: ferroui_system::threading::CancellationTokenSource) -> Rc<Self> {
        Self::with_token(source.into())
    }

    /// The token that disposing cancels (a clone of it).
    pub fn token(&self) -> CancellationToken {
        self.token.clone()
    }
}

impl IDisposable for CancellationDisposable {
    fn dispose(&self) {
        self.token.cancel();
    }
}

impl ICancelable for CancellationDisposable {
    fn is_disposed(&self) -> bool {
        self.token.is_cancelled()
    }
}
