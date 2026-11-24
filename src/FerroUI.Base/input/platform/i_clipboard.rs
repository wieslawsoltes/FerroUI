use super::{ClipboardContentOptions, ClipboardError, ClipboardHistoryItemsResult, ClipboardHistoryItemsResultStatus};
use crate::input::{IAsyncDataTransfer, LocalBoxFuture};
use crate::reactive::Subscription;
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

    /// Occurs when the content of the clipboard changes (WinUI's `Clipboard.ContentChanged`),
    /// until the subscription is dropped. The handler reads nothing: the application asks for the
    /// data when it wants it.
    ///
    /// A change made by another application is reported where
    /// [`is_content_changed_supported`](Self::is_content_changed_supported) is true; a change
    /// this clipboard makes is reported on every platform.
    fn content_changed(&self, _handler: Rc<dyn Fn()>) -> Subscription {
        Subscription::new(crate::reactive::Disposable::empty())
    }

    /// Whether [`content_changed`](Self::content_changed) reports the changes of other
    /// applications on this platform.
    fn is_content_changed_supported(&self) -> bool {
        false
    }

    /// Whether the user has the clipboard history turned on (WinUI's `Clipboard.IsHistoryEnabled`);
    /// false on a platform without history.
    fn is_history_enabled(&self) -> bool {
        false
    }

    /// Whether the user has the synchronization of the clipboard to their other devices turned on
    /// (WinUI's `Clipboard.IsRoamingEnabled`); false on a platform without it.
    fn is_roaming_enabled(&self) -> bool {
        false
    }

    /// Places a data object on the clipboard with `options` (WinUI's
    /// `Clipboard.SetContentWithOptions`). A platform without history or roaming ignores them.
    fn set_data_with_options_async(
        &self,
        data_transfer: Option<Rc<dyn IAsyncDataTransfer>>,
        _options: ClipboardContentOptions,
    ) -> LocalBoxFuture<Result<(), ClipboardError>> {
        self.set_data_async(data_transfer)
    }

    /// Reads the clipboard history of the system (WinUI's `Clipboard.GetHistoryItemsAsync`). The
    /// history is the user's data: it is read only when the application calls this, and a
    /// platform without history answers `NotSupported`.
    fn get_history_items_async(&self) -> LocalBoxFuture<Result<ClipboardHistoryItemsResult, ClipboardError>> {
        Box::pin(std::future::ready(Ok(ClipboardHistoryItemsResult::empty(
            ClipboardHistoryItemsResultStatus::NotSupported,
        ))))
    }
}
