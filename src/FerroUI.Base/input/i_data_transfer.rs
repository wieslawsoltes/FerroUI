use super::{DataFormat, IAsyncDataTransfer, IDataTransferItem};
use crate::reactive::IDisposable;
use std::rc::Rc;

/// Represents an object providing a list of [`IDataTransferItem`] usable
/// during a drag and drop operation.
///
/// When an implementation of this interface is used as a drag source using
/// [`DragDrop::do_drag_drop_async`](super::DragDrop::do_drag_drop_async),
/// it must NOT be disposed by the caller. The system will dispose of it
/// automatically when the drag operation completes.
///
/// See [`DataTransfer`](super::DataTransfer) for the mutable
/// implementation.
pub trait IDataTransfer: IDisposable {
    /// Gets the formats supported by the data transfer.
    fn formats(&self) -> Rc<[DataFormat]>;

    /// Gets a list of [`IDataTransferItem`] contained in this object.
    ///
    /// Some platforms (such as Windows and X11) may only support a single
    /// data item for all formats except files.
    ///
    /// Items returned by this property must stay valid until the data
    /// transfer is disposed.
    fn items(&self) -> Rc<[Rc<dyn IDataTransferItem>]>;

    /// The object as an asynchronous data transfer, if it also is one.
    fn as_async_data_transfer(self: Rc<Self>) -> Option<Rc<dyn IAsyncDataTransfer>> {
        None
    }
}
