use super::sync_to_async_data_transfer_item::SyncToAsyncDataTransferItem;
use super::{DataFormat, IAsyncDataTransfer, IAsyncDataTransferItem, IDataTransfer, IDataTransferItem};
use crate::reactive::IDisposable;
use std::cell::RefCell;
use std::rc::Rc;

/// Wraps a synchronous data transfer into an asynchronous one.
pub(crate) struct SyncToAsyncDataTransfer {
    data_transfer: Rc<dyn IDataTransfer>,
    items: RefCell<Option<Rc<[Rc<dyn IAsyncDataTransferItem>]>>>,
}

impl SyncToAsyncDataTransfer {
    pub(crate) fn new(data_transfer: Rc<dyn IDataTransfer>) -> Rc<Self> {
        Rc::new(Self { data_transfer, items: RefCell::new(None) })
    }

    fn provide_items(&self) -> Rc<[Rc<dyn IAsyncDataTransferItem>]> {
        self.data_transfer
            .items()
            .iter()
            .map(|item| SyncToAsyncDataTransferItem::new(item.clone()) as Rc<dyn IAsyncDataTransferItem>)
            .collect()
    }
}

impl IDisposable for SyncToAsyncDataTransfer {
    fn dispose(&self) {
        self.data_transfer.dispose()
    }
}

impl IDataTransfer for SyncToAsyncDataTransfer {
    fn formats(&self) -> Rc<[DataFormat]> {
        self.data_transfer.formats()
    }

    fn items(&self) -> Rc<[Rc<dyn IDataTransferItem>]> {
        self.data_transfer.items()
    }

    fn as_async_data_transfer(self: Rc<Self>) -> Option<Rc<dyn IAsyncDataTransfer>> {
        Some(self)
    }
}

impl IAsyncDataTransfer for SyncToAsyncDataTransfer {
    fn formats(&self) -> Rc<[DataFormat]> {
        self.data_transfer.formats()
    }

    fn items(&self) -> Rc<[Rc<dyn IAsyncDataTransferItem>]> {
        if let Some(items) = self.items.borrow().as_ref() {
            return items.clone();
        }

        let items = self.provide_items();
        *self.items.borrow_mut() = Some(items.clone());
        items
    }

    fn as_data_transfer(self: Rc<Self>) -> Option<Rc<dyn IDataTransfer>> {
        Some(self)
    }
}
