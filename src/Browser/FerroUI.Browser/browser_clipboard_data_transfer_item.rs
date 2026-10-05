use crate::browser_data_format_helper::to_browser_format;
use crate::browser_data_transfer_helper::{self, IReadableDataItem};
use ferroui_base::input::platform::{ClipboardError, PlatformAsyncDataTransferItem, PlatformAsyncDataTransferItemImpl};
use ferroui_base::input::{DataFormat, LocalBoxFuture};
use std::any::Any;
use std::rc::Rc;

/// Wraps a readable data item (a type of the script module) into an
/// asynchronous data transfer item. Asynchronous only: used to read a
/// clipboard item.
pub(crate) struct BrowserClipboardDataTransferItem {
    readable_data_item: Rc<dyn IReadableDataItem>,
}

impl BrowserClipboardDataTransferItem {
    pub(crate) fn new(readable_data_item: Rc<dyn IReadableDataItem>) -> Rc<PlatformAsyncDataTransferItem> {
        PlatformAsyncDataTransferItem::new(Self { readable_data_item })
    }
}

impl PlatformAsyncDataTransferItemImpl for BrowserClipboardDataTransferItem {
    fn provide_formats(&self) -> Result<Vec<DataFormat>, ClipboardError> {
        Ok(browser_data_transfer_helper::get_readable_item_formats(&*self.readable_data_item))
    }

    fn try_get_raw_core_async(&self, format: &DataFormat) -> LocalBoxFuture<Result<Option<Rc<dyn Any>>, ClipboardError>> {
        let format_string = to_browser_format(format);
        let value = self.readable_data_item.try_get_value_async(&format_string);
        let format = format.clone();
        Box::pin(async move {
            let value = value.await.map_err(|error| ClipboardError::other(error.to_string()))?;
            browser_data_transfer_helper::try_get_value(value, &format)
        })
    }
}
