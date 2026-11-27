use crate::browser_data_format_helper::{from_web_clipboard_format, to_browser_format};
use crate::browser_data_transfer_helper::{self, IReadableDataItem};
use crate::interop::input_helper::ReadableDataValueContent;
use ferroui_base::input::platform::{ClipboardError, PlatformAsyncDataTransferItem, PlatformAsyncDataTransferItemImpl};
use ferroui_base::input::{DataFormat, DataFormatKind, LocalBoxFuture};
use std::any::{Any, TypeId};
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
        Ok(browser_data_transfer_helper::get_readable_item_formats(&*self.readable_data_item)
            .into_iter()
            .map(|format| from_web_clipboard_format(format.identifier()).unwrap_or(format))
            .collect())
    }

    fn try_get_raw_core_async(&self, format: &DataFormat) -> LocalBoxFuture<Result<Option<Rc<dyn Any>>, ClipboardError>> {
        let web_format = if format.kind() == DataFormatKind::Application {
            self.readable_data_item.formats().into_iter().find_map(|name| {
                let decoded = from_web_clipboard_format(&name)?;
                (decoded == *format).then(|| (name, decoded.data_type() == Some(TypeId::of::<String>())))
            })
        } else {
            None
        };
        let (format_string, web_text) = web_format.unwrap_or_else(|| (to_browser_format(format), false));
        let value = self.readable_data_item.try_get_value_async(&format_string);
        let format = format.clone();
        Box::pin(async move {
            let value = value.await.map_err(|error| ClipboardError::other(error.to_string()))?;
            if web_text {
                let text = match value {
                    Some(ReadableDataValueContent::String(text)) => Some(text),
                    Some(ReadableDataValueContent::Bytes(bytes)) => String::from_utf8(bytes).ok(),
                    _ => None,
                };
                return Ok(text.map(|text| Rc::new(text) as Rc<dyn Any>));
            }
            browser_data_transfer_helper::try_get_value(value, &format)
        })
    }
}
