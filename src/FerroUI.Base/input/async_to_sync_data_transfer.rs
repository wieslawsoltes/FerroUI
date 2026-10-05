use super::async_to_sync_data_transfer_item::AsyncToSyncDataTransferItem;
use super::platform::ClipboardError;
use super::{DataFormat, IAsyncDataTransfer, IAsyncDataTransferItem, IDataTransfer, IDataTransferItem};
use crate::reactive::IDisposable;
use std::cell::RefCell;
use std::rc::Rc;

/// Wraps an asynchronous data transfer into a synchronous one.
pub(crate) struct AsyncToSyncDataTransfer {
    async_data_transfer: Rc<dyn IAsyncDataTransfer>,
    items: RefCell<Option<Rc<[Rc<dyn IDataTransferItem>]>>>,
}

impl AsyncToSyncDataTransfer {
    pub(crate) fn new(async_data_transfer: Rc<dyn IAsyncDataTransfer>) -> Rc<Self> {
        Rc::new(Self { async_data_transfer, items: RefCell::new(None) })
    }

    fn provide_items(&self) -> Rc<[Rc<dyn IDataTransferItem>]> {
        self.async_data_transfer
            .items()
            .iter()
            .map(|async_item| AsyncToSyncDataTransferItem::new(async_item.clone()) as Rc<dyn IDataTransferItem>)
            .collect()
    }
}

impl IDisposable for AsyncToSyncDataTransfer {
    fn dispose(&self) {
        self.async_data_transfer.dispose()
    }
}

impl IDataTransfer for AsyncToSyncDataTransfer {
    fn formats(&self) -> Rc<[DataFormat]> {
        self.async_data_transfer.formats()
    }

    fn items(&self) -> Rc<[Rc<dyn IDataTransferItem>]> {
        if let Some(items) = self.items.borrow().as_ref() {
            return items.clone();
        }

        let items = self.provide_items();
        *self.items.borrow_mut() = Some(items.clone());
        items
    }

    fn as_async_data_transfer(self: Rc<Self>) -> Option<Rc<dyn IAsyncDataTransfer>> {
        Some(self)
    }
}

impl IAsyncDataTransfer for AsyncToSyncDataTransfer {
    fn formats(&self) -> Rc<[DataFormat]> {
        self.async_data_transfer.formats()
    }

    fn items(&self) -> Rc<[Rc<dyn IAsyncDataTransferItem>]> {
        self.async_data_transfer.items()
    }

    fn try_formats(&self) -> Result<Rc<[DataFormat]>, ClipboardError> {
        self.async_data_transfer.try_formats()
    }

    fn try_items(&self) -> Result<Rc<[Rc<dyn IAsyncDataTransferItem>]>, ClipboardError> {
        self.async_data_transfer.try_items()
    }

    fn as_data_transfer(self: Rc<Self>) -> Option<Rc<dyn IDataTransfer>> {
        Some(self)
    }
}
