use super::platform::ClipboardError;
use super::DataFormat;
use std::any::Any;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;

/// The result of an asynchronous operation of the input system: a future
/// that is polled on the dispatcher thread (for instance from a future
/// started with `Dispatcher::invoke_async_task_local`).
pub type LocalBoxFuture<T> = Pin<Box<dyn Future<Output = T>>>;

/// Represent an item inside a [`IAsyncDataTransfer`](super::IAsyncDataTransfer).
/// An item may support several formats and can return the value of a given
/// format on demand.
///
/// See [`DataTransferItem`](super::DataTransferItem) for the mutable
/// implementation.
pub trait IAsyncDataTransferItem {
    /// Gets the formats supported by this item.
    fn formats(&self) -> Rc<[DataFormat]>;

    /// Gets the formats supported by this item, or the failure of the
    /// platform that provides them.
    ///
    /// [`formats`](Self::formats) has no way to report a failure (where the
    /// reference throws from the property, it panics); the asynchronous
    /// operations reading an item use this member instead, so that the
    /// failure reaches whoever awaits them. Items whose formats cannot fail
    /// keep the default.
    fn try_formats(&self) -> Result<Rc<[DataFormat]>, ClipboardError> {
        Ok(self.formats())
    }

    /// Tries to get a value for a given format. The future resolves to
    /// `None` if the format is not supported, and to an error if the
    /// platform providing the value fails.
    ///
    /// Implementations of this method are expected to return a value
    /// matching the exact data type of the underlying
    /// [`DataFormatOf`](super::DataFormatOf).
    ///
    /// To retrieve a typed value, use
    /// [`AsyncDataTransferItemExtensions::try_get_value_async`](super::AsyncDataTransferItemExtensions::try_get_value_async).
    fn try_get_raw_async(&self, format: &DataFormat) -> LocalBoxFuture<Result<Option<Rc<dyn Any>>, ClipboardError>>;
}
