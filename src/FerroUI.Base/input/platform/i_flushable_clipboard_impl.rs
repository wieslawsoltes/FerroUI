use super::{ClipboardError, IClipboardImpl};
use crate::input::LocalBoxFuture;

/// Represents a clipboard implementation that can be flushed.
///
/// This is an implementation detail of the platform backends.
pub trait IFlushableClipboardImpl: IClipboardImpl {
    /// Permanently adds the data that is on the clipboard so that it is
    /// available after the data's original application closes.
    fn flush_async(&self) -> LocalBoxFuture<Result<(), ClipboardError>>;
}
