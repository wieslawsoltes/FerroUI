//! The data of the clipboard of another client (the port of
//! `ClipboardDataTransfer.cs`).

use crate::selections::clipboard::clipboard_data_reader::ClipboardDataReader;
use crate::selections::selection_data_reader::ISelectionDataReader;
use ferroui_base::input::platform::{ClipboardError, PlatformAsyncDataTransfer, PlatformAsyncDataTransferImpl};
use ferroui_base::input::{DataFormat, IAsyncDataTransferItem};
use std::rc::Rc;

/// Implementation of the asynchronous data transfer for the X11 clipboard:
/// the reader is the object used to read values.
///
/// Formats and items are pre-populated because we don't want to do some sync-over-async calls.
/// Note that this does not pre-populate values, which are still retrieved asynchronously on demand.
pub struct ClipboardDataTransfer {
    reader: Rc<ClipboardDataReader>,
    formats: Vec<DataFormat>,
    items: Vec<Rc<dyn IAsyncDataTransferItem>>,
}

impl ClipboardDataTransfer {
    pub fn new(
        reader: Rc<ClipboardDataReader>,
        formats: Vec<DataFormat>,
        items: Vec<Rc<dyn IAsyncDataTransferItem>>,
    ) -> Rc<PlatformAsyncDataTransfer> {
        PlatformAsyncDataTransfer::new(ClipboardDataTransfer { reader, formats, items })
    }
}

impl PlatformAsyncDataTransferImpl for ClipboardDataTransfer {
    fn provide_formats(&self) -> Result<Vec<DataFormat>, ClipboardError> {
        Ok(self.formats.clone())
    }

    fn provide_items(&self) -> Result<Vec<Rc<dyn IAsyncDataTransferItem>>, ClipboardError> {
        Ok(self.items.clone())
    }

    fn dispose(&self, _owner: &PlatformAsyncDataTransfer) {
        self.reader.dispose();
    }
}
