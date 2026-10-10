//! Reading the values of a drag (the port of `DragDropDataReader.cs`).

use crate::selections::drag_drop::drag_drop_data_transfer_item::DragDropDataTransferItem;
use crate::selections::drag_drop::synchronous_x_event_waiter::SynchronousXEventWaiter;
use crate::selections::selection_data_provider::get_result;
use crate::selections::selection_data_reader::{create_items_async, ISelectionDataReader, SelectionDataReader};
use crate::selections::selection_helper::XlibSelectionConnection;
use crate::selections::selection_read_session::SelectionReadSession;
use crate::x11_info::X11Info;
use crate::xlib::{Atom, XDisplay, XID};
use ferroui_base::input::platform::{ClipboardError, PlatformDataTransferItem};
use ferroui_base::input::DataFormat;
use std::any::Any;
use std::rc::Rc;

/// The items of a drag, read when they are first asked for (what
/// [`DragDropDataTransfer`](super::drag_drop_data_transfer::DragDropDataTransfer)
/// needs of its reader).
pub trait IDragDropItemsSource {
    fn create_items(self: Rc<Self>) -> Result<Vec<Rc<PlatformDataTransferItem>>, ClipboardError>;

    fn dispose(&self);
}

/// An object used to read values, converted to the correct format, from a Xdnd selection.
pub struct DragDropDataReader {
    base: SelectionDataReader,
    info: Rc<X11Info>,
    display: XDisplay,
    target_window: XID,
}

impl DragDropDataReader {
    pub fn new(
        info: Rc<X11Info>,
        text_format_atoms: Vec<Atom>,
        data_formats: Vec<DataFormat>,
        display: XDisplay,
        target_window: XID,
    ) -> Rc<Self> {
        Rc::new(Self {
            base: SelectionDataReader::new(info.clone(), text_format_atoms, data_formats),
            info,
            display,
            target_window,
        })
    }

    /// Reads the value of a format.
    pub fn try_get(self: Rc<Self>, format: &DataFormat) -> Result<Option<Rc<dyn Any>>, ClipboardError> {
        // Note: this doesn't cause any deadlock, TryGetAsync() will always complete synchronously
        // thanks to the SynchronousXEventWaiter used in the SelectionReadSession.
        get_result(self.try_get_async(format))
    }
}

impl IDragDropItemsSource for DragDropDataReader {
    fn create_items(self: Rc<Self>) -> Result<Vec<Rc<PlatformDataTransferItem>>, ClipboardError> {
        // Note: this doesn't cause any deadlock, CreateItemsAsync() will always complete synchronously
        // thanks to the SynchronousXEventWaiter used in the SelectionReadSession.
        get_result(create_items_async(&self))
    }

    fn dispose(&self) {}
}

impl ISelectionDataReader for DragDropDataReader {
    type Item = PlatformDataTransferItem;

    fn reader(&self) -> &SelectionDataReader {
        &self.base
    }

    fn create_single_item(self: Rc<Self>, non_file_formats: Vec<DataFormat>) -> Rc<PlatformDataTransferItem> {
        DragDropDataTransferItem::new(self, non_file_formats)
    }

    fn create_file_item(item: Rc<PlatformDataTransferItem>) -> Rc<PlatformDataTransferItem> {
        item
    }

    fn create_read_session(&self) -> Option<SelectionReadSession> {
        let event_waiter = Rc::new(SynchronousXEventWaiter::new(self.display));
        Some(SelectionReadSession::new(
            XlibSelectionConnection::new(self.display),
            self.target_window,
            self.info.atoms().XdndSelection,
            event_waiter,
            self.info.atoms(),
        ))
    }

    fn dispose(&self) {}
}
