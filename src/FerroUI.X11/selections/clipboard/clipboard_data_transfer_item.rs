//! The item of the clipboard that has every format but the files (the
//! port of `ClipboardDataTransferItem.cs`).

use crate::selections::clipboard::clipboard_data_reader::ClipboardDataReader;
use crate::selections::selection_data_reader::ISelectionDataReader;
use ferroui_base::input::platform::{ClipboardError, PlatformAsyncDataTransferItem, PlatformAsyncDataTransferItemImpl};
use ferroui_base::input::{DataFormat, LocalBoxFuture};
use std::any::Any;
use std::rc::Rc;

/// Implementation of the asynchronous data transfer item for the X11
/// clipboard: the reader is the object used to read values.
pub struct ClipboardDataTransferItem {
    reader: Rc<ClipboardDataReader>,
    formats: Vec<DataFormat>,
}

impl ClipboardDataTransferItem {
    pub fn new(reader: Rc<ClipboardDataReader>, formats: Vec<DataFormat>) -> Rc<PlatformAsyncDataTransferItem> {
        PlatformAsyncDataTransferItem::new(ClipboardDataTransferItem { reader, formats })
    }
}

impl PlatformAsyncDataTransferItemImpl for ClipboardDataTransferItem {
    fn provide_formats(&self) -> Result<Vec<DataFormat>, ClipboardError> {
        Ok(self.formats.clone())
    }

    fn try_get_raw_core_async(&self, format: &DataFormat) -> LocalBoxFuture<Result<Option<Rc<dyn Any>>, ClipboardError>> {
        self.reader.clone().try_get_async(format)
    }
}
