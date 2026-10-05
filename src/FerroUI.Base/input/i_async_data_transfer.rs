use super::platform::ClipboardError;
use super::{DataFormat, IAsyncDataTransferItem, IDataTransfer};
use crate::reactive::IDisposable;
use std::rc::Rc;

/// Represents an object providing a list of [`IAsyncDataTransferItem`]
/// usable by the clipboard.
///
/// When an implementation of this interface is put into the clipboard using
/// [`IClipboard::set_data_async`](super::platform::IClipboard::set_data_async),
/// it must NOT be disposed by the caller. The system will dispose of it
/// automatically when it becomes unused.
///
/// When an implementation of this interface is returned from the clipboard
/// via [`IClipboard::try_get_data_async`](super::platform::IClipboard::try_get_data_async),
/// it MUST be disposed by the caller.
///
/// See [`DataTransfer`](super::DataTransfer) for the mutable
/// implementation.
pub trait IAsyncDataTransfer: IDisposable {
    /// Gets the formats supported by the data transfer.
    fn formats(&self) -> Rc<[DataFormat]>;

    /// Gets a list of [`IAsyncDataTransferItem`] contained in this object.
    ///
    /// Some platforms (such as Windows and X11) may only support a single
    /// data item for all formats except files.
    ///
    /// Items returned by this property must stay valid until the data
    /// transfer is disposed.
    fn items(&self) -> Rc<[Rc<dyn IAsyncDataTransferItem>]>;

    /// Gets the formats supported by the data transfer, or the failure of
    /// the platform that provides them.
    ///
    /// [`formats`](Self::formats) has no way to report a failure (where the
    /// reference throws from the property, it panics); the asynchronous
    /// clipboard operations use this member instead, so that the failure
    /// reaches whoever awaits them. Data transfers whose formats cannot
    /// fail keep the default.
    fn try_formats(&self) -> Result<Rc<[DataFormat]>, ClipboardError> {
        Ok(self.formats())
    }

    /// Gets the items of the data transfer, or the failure of the platform
    /// that provides them. See [`try_formats`](Self::try_formats).
    fn try_items(&self) -> Result<Rc<[Rc<dyn IAsyncDataTransferItem>]>, ClipboardError> {
        Ok(self.items())
    }

    /// The object as a synchronous data transfer, if it also is one.
    fn as_data_transfer(self: Rc<Self>) -> Option<Rc<dyn IDataTransfer>> {
        None
    }
}
