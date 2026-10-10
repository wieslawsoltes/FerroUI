//! Reading the values of the clipboard (the port of
//! `ClipboardDataReader.cs`).

use crate::selections::clipboard::clipboard_data_transfer_item::ClipboardDataTransferItem;
use crate::selections::clipboard::clipboard_read_session_factory;
use crate::selections::selection_data_reader::{
    try_get_async_core, ISelectionDataReader, RawValueFuture, SelectionDataReader,
};
use crate::selections::selection_read_session::SelectionReadSession;
use crate::x11_platform::FerroX11Platform;
use crate::xlib::{self, Atom, XDisplay, XID};
use ferroui_base::input::platform::PlatformDataTransferItem;
use ferroui_base::input::{DataFormat, IAsyncDataTransferItem};
use std::cell::Cell;
use std::rc::{Rc, Weak};

/// An object used to read values, converted to the correct format, from the X11 clipboard.
pub struct ClipboardDataReader {
    base: SelectionDataReader,
    platform: Weak<FerroX11Platform>,
    display: XDisplay,
    selection: Atom,
    owner: Cell<XID>,
}

impl ClipboardDataReader {
    pub fn new(
        platform: &Rc<FerroX11Platform>,
        selection: Atom,
        text_format_atoms: Vec<Atom>,
        data_formats: Vec<DataFormat>,
        owner: XID,
    ) -> Rc<Self> {
        Rc::new(Self {
            base: SelectionDataReader::new(platform.info().clone(), text_format_atoms, data_formats),
            platform: Rc::downgrade(platform),
            display: platform.display(),
            selection,
            owner: Cell::new(owner),
        })
    }

    fn is_owner_still_valid(&self) -> bool {
        self.owner.get() != 0 && xlib::x_get_selection_owner(self.display, self.selection) == self.owner.get()
    }
}

impl ISelectionDataReader for ClipboardDataReader {
    type Item = dyn IAsyncDataTransferItem;

    fn reader(&self) -> &SelectionDataReader {
        &self.base
    }

    fn try_get_async(self: Rc<Self>, format: &DataFormat) -> RawValueFuture {
        if !self.is_owner_still_valid() {
            return Box::pin(std::future::ready(Ok(None)));
        }

        try_get_async_core(self, format)
    }

    fn create_single_item(self: Rc<Self>, non_file_formats: Vec<DataFormat>) -> Rc<dyn IAsyncDataTransferItem> {
        ClipboardDataTransferItem::new(self, non_file_formats)
    }

    fn create_file_item(item: Rc<PlatformDataTransferItem>) -> Rc<dyn IAsyncDataTransferItem> {
        item
    }

    fn create_read_session(&self) -> Option<SelectionReadSession> {
        let platform = self.platform.upgrade()?;
        Some(clipboard_read_session_factory::create_session(&platform, self.selection))
    }

    fn dispose(&self) {
        self.owner.set(0);
    }
}
