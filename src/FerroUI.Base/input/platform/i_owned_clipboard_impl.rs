use super::{ClipboardError, IClipboardImpl};
use crate::input::LocalBoxFuture;

/// Represents a clipboard implementation that knows whether the
/// application is the owner of the data on the clipboard.
///
/// This is an implementation detail of the platform backends.
pub trait IOwnedClipboardImpl: IClipboardImpl {
    /// Gets whether the current instance still owns the system clipboard.
    fn is_current_owner_async(&self) -> LocalBoxFuture<Result<bool, ClipboardError>>;
}
