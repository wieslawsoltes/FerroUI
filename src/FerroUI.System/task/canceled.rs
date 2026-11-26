//! [`Canceled`]: the error of an outcome that will never come.

use std::fmt;

/// The error of an outcome that will never come: the sender of a [`oneshot`](super::oneshot)
/// channel was dropped without sending.
///
/// It is the one cancellation error of the workspace (docs/idiomatic-rust-async.md, section 3.7):
/// the error of a canceled dispatcher operation, of an awaited token, and of the futures of the
/// reactive libraries that their token stops.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Canceled;

impl fmt::Display for Canceled {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("the operation was canceled")
    }
}

impl std::error::Error for Canceled {}

impl Canceled {
    /// Whether `error` is a cancellation, a [`Canceled`] (`OperationCanceledException` and
    /// `TaskCanceledException` caught as such).
    pub fn is_cancellation(error: &crate::ObservableError) -> bool {
        error.is::<Canceled>()
    }
}
