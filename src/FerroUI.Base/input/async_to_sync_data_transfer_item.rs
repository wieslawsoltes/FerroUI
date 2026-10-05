use super::platform::ClipboardError;
use super::{DataFormat, IAsyncDataTransferItem, IDataTransferItem, LocalBoxFuture};
use crate::threading::{Dispatcher, DispatcherFrame};
use std::any::Any;
use std::rc::Rc;
use std::sync::Arc;
use std::task::{Context, Poll, Wake, Waker};

/// Stops the nested dispatcher frame that waits for a future.
struct FrameWaker(Arc<DispatcherFrame>);

impl Wake for FrameWaker {
    fn wake(self: Arc<Self>) {
        self.0.set_continue(false);
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.0.set_continue(false);
    }
}

/// Waits for a future on the dispatcher thread without blocking it: the
/// dispatcher keeps processing its queue in a nested frame until the future
/// has completed.
///
/// # Panics
/// Panics when the future is not ready and the dispatcher cannot run nested
/// loops.
fn wait_for<T>(mut future: LocalBoxFuture<T>) -> T {
    // The common case: the value is available synchronously.
    if let Poll::Ready(value) = future.as_mut().poll(&mut Context::from_waker(Waker::noop())) {
        return value;
    }

    let dispatcher = Dispatcher::ui_thread();
    loop {
        let frame = DispatcherFrame::new();
        let waker = Waker::from(Arc::new(FrameWaker(frame.clone())));
        if let Poll::Ready(value) = future.as_mut().poll(&mut Context::from_waker(&waker)) {
            return value;
        }
        dispatcher.push_frame(&frame);
    }
}

/// Wraps an asynchronous data transfer item into a synchronous one.
pub(crate) struct AsyncToSyncDataTransferItem {
    async_data_transfer_item: Rc<dyn IAsyncDataTransferItem>,
}

impl AsyncToSyncDataTransferItem {
    pub(crate) fn new(async_data_transfer_item: Rc<dyn IAsyncDataTransferItem>) -> Rc<Self> {
        Rc::new(Self { async_data_transfer_item })
    }
}

impl IDataTransferItem for AsyncToSyncDataTransferItem {
    fn formats(&self) -> Rc<[DataFormat]> {
        self.async_data_transfer_item.formats()
    }

    /// A synchronous item has no way to report a failure: the failure of
    /// the wrapped item is a panic (where the reference throws the
    /// exception of the awaited task).
    fn try_get_raw(&self, format: &DataFormat) -> Option<Rc<dyn Any>> {
        match wait_for(self.async_data_transfer_item.try_get_raw_async(format)) {
            Ok(value) => value,
            Err(error) => panic!("{error}"),
        }
    }
}

impl IAsyncDataTransferItem for AsyncToSyncDataTransferItem {
    fn formats(&self) -> Rc<[DataFormat]> {
        self.async_data_transfer_item.formats()
    }

    fn try_formats(&self) -> Result<Rc<[DataFormat]>, ClipboardError> {
        self.async_data_transfer_item.try_formats()
    }

    fn try_get_raw_async(&self, format: &DataFormat) -> LocalBoxFuture<Result<Option<Rc<dyn Any>>, ClipboardError>> {
        self.async_data_transfer_item.try_get_raw_async(format)
    }
}
