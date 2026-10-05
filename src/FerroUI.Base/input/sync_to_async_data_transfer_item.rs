use super::platform::ClipboardError;
use super::{DataFormat, IAsyncDataTransferItem, IDataTransferItem, LocalBoxFuture};
use std::any::Any;
use std::rc::Rc;

/// Wraps a synchronous data transfer item into an asynchronous one.
pub(crate) struct SyncToAsyncDataTransferItem {
    data_transfer_item: Rc<dyn IDataTransferItem>,
}

impl SyncToAsyncDataTransferItem {
    pub(crate) fn new(data_transfer_item: Rc<dyn IDataTransferItem>) -> Rc<Self> {
        Rc::new(Self { data_transfer_item })
    }
}

impl IDataTransferItem for SyncToAsyncDataTransferItem {
    fn formats(&self) -> Rc<[DataFormat]> {
        self.data_transfer_item.formats()
    }

    fn try_get_raw(&self, format: &DataFormat) -> Option<Rc<dyn Any>> {
        self.data_transfer_item.try_get_raw(format)
    }
}

impl IAsyncDataTransferItem for SyncToAsyncDataTransferItem {
    fn formats(&self) -> Rc<[DataFormat]> {
        self.data_transfer_item.formats()
    }

    fn try_get_raw_async(&self, format: &DataFormat) -> LocalBoxFuture<Result<Option<Rc<dyn Any>>, ClipboardError>> {
        Box::pin(std::future::ready(Ok(self.data_transfer_item.try_get_raw(format))))
    }
}
