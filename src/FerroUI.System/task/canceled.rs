//! [`Canceled`]: the error of an outcome that will never come.

use std::fmt;

/// The error of an outcome that will never come: the sender of a [`oneshot`](super::oneshot)
/// channel was dropped without sending.
///
/// It is the one cancellation error of the workspace (docs/idiomatic-rust-async.md, section 3.7),
/// the place of `OperationCanceledError` (`ferroui-base`) and
/// [`TaskCanceledError`](crate::threading::tasks::TaskCanceledError), which convert to and from it
/// (`From`) until their uses move to it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Canceled;

impl fmt::Display for Canceled {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("the operation was canceled")
    }
}

impl std::error::Error for Canceled {}
