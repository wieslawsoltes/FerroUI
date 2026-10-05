use super::ClipboardError;
use crate::input::{IAsyncDataTransfer, LocalBoxFuture};
use std::rc::Rc;

/// Represents the system clipboard.
///
/// This interface is not meant to be implemented by applications.
///
/// Every operation resolves to a [`ClipboardError`] when the platform
/// clipboard fails (where the reference throws from the awaited task).
pub trait IClipboard {
    /// Clears any data from the system clipboard.
    fn clear_async(&self) -> LocalBoxFuture<Result<(), ClipboardError>>;

    /// Places a data object on the clipboard. The data may be queried
    /// lazily by the system: the data transfer must stay valid until the
    /// clipboard is cleared or replaced. `None` clears the clipboard.
    ///
    /// By calling this method, the clipboard will often be instantly
    /// cleared, and data will be lazily requested later when it is pasted.
    ///
    /// The data transfer must NOT be disposed by the caller after this
    /// call: the clipboard will dispose of it automatically when it becomes
    /// unused.
    fn set_data_async(
        &self,
        data_transfer: Option<Rc<dyn IAsyncDataTransfer>>,
    ) -> LocalBoxFuture<Result<(), ClipboardError>>;

    /// Permanently adds the data that is on the clipboard so that it is
    /// available after the data's original application closes.
    ///
    /// This method is only supported on the Windows platform; it does
    /// nothing on other platforms.
    fn flush_async(&self) -> LocalBoxFuture<Result<(), ClipboardError>>;

    /// Retrieves data previously stored on the clipboard.
    ///
    /// The returned data transfer MUST be disposed by the caller.
    fn try_get_data_async(&self) -> LocalBoxFuture<Result<Option<Rc<dyn IAsyncDataTransfer>>, ClipboardError>>;

    /// Retrieves the exact instance of a data transfer previously placed on
    /// the clipboard by [`set_data_async`](Self::set_data_async), if any.
    ///
    /// This method cannot be used to retrieve a data transfer set by
    /// another process or by the system.
    fn try_get_in_process_data_async(
        &self,
    ) -> LocalBoxFuture<Result<Option<Rc<dyn IAsyncDataTransfer>>, ClipboardError>>;
}
