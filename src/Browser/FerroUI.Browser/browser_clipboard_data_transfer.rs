use crate::browser_clipboard_data_transfer_item::BrowserClipboardDataTransferItem;
use crate::browser_data_transfer_helper::IReadableDataItems;
use ferroui_base::input::platform::{ClipboardError, PlatformAsyncDataTransfer, PlatformAsyncDataTransferImpl};
use ferroui_base::input::{DataFormat, IAsyncDataTransferItem};
use std::cell::RefCell;
use std::rc::Rc;

/// Wraps an array of readable data items (a type of the script module) into
/// an asynchronous data transfer. Asynchronous only: used to read the
/// clipboard.
pub(crate) struct BrowserClipboardDataTransfer {
    js_items: RefCell<Option<Rc<dyn IReadableDataItems>>>,
    // The items, created once: the formats of the data transfer are those of its items.
    items: RefCell<Option<Vec<Rc<dyn IAsyncDataTransferItem>>>>,
}

impl BrowserClipboardDataTransfer {
    pub(crate) fn new(js_items: Rc<dyn IReadableDataItems>) -> Rc<PlatformAsyncDataTransfer> {
        PlatformAsyncDataTransfer::new(Self { js_items: RefCell::new(Some(js_items)), items: RefCell::new(None) })
    }

    fn items(&self) -> Vec<Rc<dyn IAsyncDataTransferItem>> {
        if let Some(items) = self.items.borrow().as_ref() {
            return items.clone();
        }

        let items: Vec<Rc<dyn IAsyncDataTransferItem>> = match self.js_items.borrow().as_ref() {
            Some(js_items) => (0..js_items.len())
                .map(|i| BrowserClipboardDataTransferItem::new(js_items.get(i)) as Rc<dyn IAsyncDataTransferItem>)
                .collect(),
            None => panic!("Cannot access a disposed object: BrowserClipboardDataTransfer"),
        };
        *self.items.borrow_mut() = Some(items.clone());
        items
    }
}

impl PlatformAsyncDataTransferImpl for BrowserClipboardDataTransfer {
    fn provide_formats(&self) -> Result<Vec<DataFormat>, ClipboardError> {
        let mut formats: Vec<DataFormat> = Vec::new();
        for item in self.items() {
            for format in item.try_formats()?.iter() {
                if !formats.contains(format) {
                    formats.push(format.clone());
                }
            }
        }
        Ok(formats)
    }

    fn provide_items(&self) -> Result<Vec<Rc<dyn IAsyncDataTransferItem>>, ClipboardError> {
        Ok(self.items())
    }

    fn dispose(&self, _owner: &PlatformAsyncDataTransfer) {
        // The objects of the page are released with the last reference to them.
        let js_items = self.js_items.borrow_mut().take();
        drop(js_items);
        let items = self.items.borrow_mut().take();
        drop(items);
    }
}
