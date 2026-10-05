use super::{ClipboardError, IFlushableClipboardImpl, IOwnedClipboardImpl};
use crate::input::{IAsyncDataTransfer, LocalBoxFuture};
use std::rc::Rc;

/// Represents the platform implementation of the clipboard.
///
/// This is an implementation detail of the platform backends.
///
/// A failed operation resolves to a [`ClipboardError`] whose kind tells
/// what happened (where the reference throws from the awaited task).
pub trait IClipboardImpl {
    /// Tries to get the data on the clipboard, if any.
    fn try_get_data_async(&self) -> LocalBoxFuture<Result<Option<Rc<dyn IAsyncDataTransfer>>, ClipboardError>>;

    /// Sets the data on the clipboard.
    fn set_data_async(&self, data_transfer: Rc<dyn IAsyncDataTransfer>) -> LocalBoxFuture<Result<(), ClipboardError>>;

    /// Clears the clipboard.
    fn clear_async(&self) -> LocalBoxFuture<Result<(), ClipboardError>>;

    /// The implementation as a flushable clipboard, if it is one.
    fn as_flushable_clipboard_impl(&self) -> Option<&dyn IFlushableClipboardImpl> {
        None
    }

    /// The implementation as an owned clipboard, if it is one.
    fn as_owned_clipboard_impl(&self) -> Option<&dyn IOwnedClipboardImpl> {
        None
    }
}
