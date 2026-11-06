//! [`Canceled`]: the error of an outcome that will never come.

use std::fmt;

/// The error of an outcome that will never come: the sender of a [`oneshot`](super::oneshot)
/// channel was dropped without sending.
///
/// It is the one cancellation error of the workspace (docs/idiomatic-rust-async.md, section 3.7):
/// the error of a canceled dispatcher operation, of an awaited token, and of the futures of the
/// reactive libraries that their token stops. `OperationCanceledError` (`ferroui-base`) and
/// `TaskCanceledError` (`threading::tasks`), the error of a canceled `Task` that names it, both
/// deprecated, convert to and from it (`From`); [`Canceled::is_cancellation`] recognizes both.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Canceled;

impl fmt::Display for Canceled {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("the operation was canceled")
    }
}

impl std::error::Error for Canceled {}

impl Canceled {
    /// Whether `error` is a cancellation: a [`Canceled`], or the deprecated `TaskCanceledError` of
    /// a canceled `Task` (`OperationCanceledException` and `TaskCanceledException` caught as such).
    pub fn is_cancellation(error: &crate::ObservableError) -> bool {
        #[allow(deprecated)]
        let named = error.is::<crate::threading::tasks::TaskCanceledError>();
        error.is::<Canceled>() || named
    }
}
