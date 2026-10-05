use crate::browser_data_format_helper::to_browser_format;
use crate::browser_data_transfer_helper::{self, IReadableDataItem};
use ferroui_base::input::platform::{ClipboardError, PlatformDataTransferItem, PlatformDataTransferItemImpl};
use ferroui_base::input::DataFormat;
use std::any::Any;
use std::rc::Rc;

/// Wraps a readable data item (a type of the script module) into a data
/// transfer item. Synchronous only: used to read a drag-and-drop item.
pub(crate) struct BrowserDragDataTransferItem {
    readable_data_item: Rc<dyn IReadableDataItem>,
}

impl BrowserDragDataTransferItem {
    pub(crate) fn new(readable_data_item: Rc<dyn IReadableDataItem>) -> Rc<PlatformDataTransferItem> {
        PlatformDataTransferItem::new(Self { readable_data_item })
    }
}

impl PlatformDataTransferItemImpl for BrowserDragDataTransferItem {
    fn provide_formats(&self) -> Result<Vec<DataFormat>, ClipboardError> {
        Ok(browser_data_transfer_helper::get_readable_item_formats(&*self.readable_data_item))
    }

    fn try_get_raw_core(&self, format: &DataFormat) -> Result<Option<Rc<dyn Any>>, ClipboardError> {
        let format_string = to_browser_format(format);
        let value = self.readable_data_item.try_get_value(&format_string);
        browser_data_transfer_helper::try_get_value(value, format)
    }
}
