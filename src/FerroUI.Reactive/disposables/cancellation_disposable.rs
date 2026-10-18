//! `CancellationDisposable`.

use super::ICancelable;
use ferroui_system::threading::{CancellationToken, CancellationTokenSource};
use ferroui_system::IDisposable;
use std::rc::Rc;

/// A disposable that cancels a cancellation token source when it is disposed
/// (`CancellationDisposable`).
pub struct CancellationDisposable {
    source: CancellationTokenSource,
}

impl CancellationDisposable {
    /// A disposable over a new cancellation token source.
    pub fn new() -> Rc<Self> {
        Self::with_source(CancellationTokenSource::new())
    }

    /// A disposable that cancels `source` when it is disposed.
    pub fn with_source(source: CancellationTokenSource) -> Rc<Self> {
        Rc::new(Self { source })
    }

    /// The token of the source.
    pub fn token(&self) -> CancellationToken {
        self.source.token()
    }
}

impl IDisposable for CancellationDisposable {
    fn dispose(&self) {
        self.source.cancel();
    }
}

impl ICancelable for CancellationDisposable {
    fn is_disposed(&self) -> bool {
        self.source.is_cancellation_requested()
    }
}
