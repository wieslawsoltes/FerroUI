use crate::data_transfer_item_to_frn_clipboard_data_item_wrapper::DataTransferItemToFrnClipboardDataItemWrapper;
use crate::interop::*;
use ferroui_base::input::IDataTransfer;
use ferroui_microcom::{ComPtr, HResult};
use std::cell::RefCell;
use std::rc::Rc;

/// Exposes a data transfer to native code as a clipboard data source. The
/// data transfer is disposed when native code releases the source.
pub(crate) struct DataTransferToFrnClipboardDataSourceWrapper {
    data_transfer: RefCell<Option<Rc<dyn IDataTransfer>>>,
    items: RefCell<Option<Vec<ComPtr<IFrnClipboardDataItem>>>>,
}

impl DataTransferToFrnClipboardDataSourceWrapper {
    pub(crate) fn new(data_transfer: Rc<dyn IDataTransfer>) -> Self {
        Self { data_transfer: RefCell::new(Some(data_transfer)), items: RefCell::new(None) }
    }

    fn data_transfer(&self) -> Rc<dyn IDataTransfer> {
        match self.data_transfer.borrow().clone() {
            Some(data_transfer) => data_transfer,
            None => panic!("Cannot access a disposed object: DataTransferToFrnClipboardDataSourceWrapper"),
        }
    }

    fn items(&self) -> Vec<ComPtr<IFrnClipboardDataItem>> {
        if let Some(items) = self.items.borrow().as_ref() {
            return items.clone();
        }

        let items: Vec<ComPtr<IFrnClipboardDataItem>> = self
            .data_transfer()
            .items()
            .iter()
            .map(|item| IFrnClipboardDataItem::from_impl(DataTransferItemToFrnClipboardDataItemWrapper::new(item.clone())))
            .collect();
        *self.items.borrow_mut() = Some(items.clone());
        if items.is_empty() {
            self.destroyed();
        }

        items
    }

    fn destroyed(&self) {
        let data_transfer = self.data_transfer.borrow_mut().take();
        if let Some(data_transfer) = data_transfer {
            data_transfer.dispose();
        }
    }
}

impl Drop for DataTransferToFrnClipboardDataSourceWrapper {
    fn drop(&mut self) {
        self.destroyed();
    }
}

impl IFrnClipboardDataSourceImpl for DataTransferToFrnClipboardDataSourceWrapper {
    fn get_item_count(&self) -> i32 {
        crate::callback_base::guard(0, || self.items().len() as i32)
    }

    fn get_item(&self, index: i32) -> Result<Option<ComPtr<IFrnClipboardDataItem>>, HResult> {
        crate::callback_base::guard(Err(HResult::FAIL), || match self.items().get(index as usize) {
            Some(item) if index >= 0 => Ok(Some(item.clone())),
            _ => panic!("Index was outside the bounds of the array."),
        })
    }
}
