use super::async_to_sync_data_transfer::AsyncToSyncDataTransfer;
use super::platform::ClipboardError;
use super::sync_to_async_data_transfer::SyncToAsyncDataTransfer;
use super::{
    AsyncDataTransferItemExtensions, DataFormat, DataFormatOf, IAsyncDataTransfer, IAsyncDataTransferItem,
    IDataTransfer, LocalBoxFuture,
};
use crate::logging::{LogEventLevel, Logger};
use crate::media::imaging::Bitmap;
use std::rc::Rc;

/// The items of an asynchronous data transfer that support a format, in
/// order.
pub struct AsyncDataTransferItems {
    items: Rc<[Rc<dyn IAsyncDataTransferItem>]>,
    format: DataFormat,
    index: usize,
}

impl Iterator for AsyncDataTransferItems {
    type Item = Rc<dyn IAsyncDataTransferItem>;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(item) = self.items.get(self.index) {
            self.index += 1;
            if item.contains(&self.format) {
                return Some(item.clone());
            }
        }
        None
    }
}

impl dyn IAsyncDataTransfer {
    /// Returns the data transfer as a synchronous one: itself if it also
    /// implements [`IDataTransfer`], a wrapper waiting for the values
    /// otherwise.
    ///
    /// This is an implementation detail of the platform backends.
    pub fn to_synchronous(self: Rc<Self>, log_area: &'static str) -> Rc<dyn IDataTransfer> {
        if let Some(data_transfer) = self.clone().as_data_transfer() {
            return data_transfer;
        }

        if let Some(logger) = Logger::try_get(LogEventLevel::Warning, log_area) {
            logger.log(
                None,
                "Using a synchronous wrapper for IAsyncDataTransferItem. Consider implementing IDataTransfer instead.",
            );
        }

        AsyncToSyncDataTransfer::new(self)
    }
}

impl dyn IDataTransfer {
    /// Returns the data transfer as an asynchronous one: itself if it also
    /// implements [`IAsyncDataTransfer`], a wrapper otherwise.
    ///
    /// This is an implementation detail of the platform backends.
    pub fn to_asynchronous(self: Rc<Self>) -> Rc<dyn IAsyncDataTransfer> {
        match self.clone().as_async_data_transfer() {
            Some(data_transfer) => data_transfer,
            None => SyncToAsyncDataTransfer::new(self),
        }
    }
}

// Keep the `try_get_xxx_async` methods in sync with the `try_get_xxx` ones
// of `DataTransferExtensions`.

/// Contains extension methods for [`IAsyncDataTransfer`].
pub trait AsyncDataTransferExtensions: IAsyncDataTransfer {
    /// Gets whether the data transfer supports a specific format.
    fn contains(&self, format: &DataFormat) -> bool {
        self.formats().iter().any(|candidate| candidate == format)
    }

    /// Gets the list of [`IAsyncDataTransferItem`] contained in the data
    /// transfer, filtered by a given format.
    ///
    /// Some platforms (such as Windows and X11) may only support a single
    /// data item for all formats except files.
    fn get_items(&self, format: &DataFormat) -> AsyncDataTransferItems {
        AsyncDataTransferItems { items: self.items(), format: format.clone(), index: 0 }
    }

    /// Tries to get a value for a given format from the data transfer. The
    /// future resolves to `None` if the format is not supported, and to an
    /// error if the platform providing the items, their formats or the
    /// value fails.
    ///
    /// If the data transfer contains several items supporting the format,
    /// the first matching one will be returned.
    fn try_get_value_async<T: Clone + 'static>(
        &self,
        format: &DataFormatOf<T>,
    ) -> LocalBoxFuture<Result<Option<T>, ClipboardError>> {
        // The first item supporting the format is looked for before this
        // returns, as in the reference; the items after it are not asked
        // for their formats.
        let item = self.try_items().and_then(|items| {
            for item in items.iter() {
                if item.try_contains(format)? {
                    return Ok(Some(item.clone()));
                }
            }
            Ok(None)
        });

        match item {
            Ok(Some(item)) => item.try_get_value_async(format),
            Ok(None) => Box::pin(std::future::ready(Ok(None))),
            Err(error) => Box::pin(std::future::ready(Err(error))),
        }
    }

    /// Tries to get multiple values for a given format from the data
    /// transfer. The future resolves to `None` if the format is not
    /// supported, and to an error as soon as the platform providing the
    /// items, their formats or one of the values fails.
    fn try_get_values_async<T: Clone + 'static>(
        &self,
        format: &DataFormatOf<T>,
    ) -> LocalBoxFuture<Result<Option<Vec<T>>, ClipboardError>> {
        let items = self.try_items();
        let format = format.clone();

        Box::pin(async move {
            let mut results: Option<Vec<T>> = None;

            // The items are queried one after the other, each on the thread
            // the future is polled on; an item is asked for its formats
            // when the items before it have delivered their values.
            for item in items?.iter() {
                if !item.try_contains(&format)? {
                    continue;
                }

                let Some(result) = item.try_get_value_async(&format).await? else { continue };
                results.get_or_insert_with(Vec::new).push(result);
            }

            Ok(results)
        })
    }

    /// Returns a text, if available, from the data transfer.
    ///
    /// If the data transfer contains several items supporting
    /// [`DataFormat::text`], the first matching one will be returned.
    fn try_get_text_async(&self) -> LocalBoxFuture<Result<Option<String>, ClipboardError>> {
        self.try_get_value_async(&DataFormat::text())
    }

    /// Returns a bitmap, if available, from the data transfer.
    ///
    /// If the data transfer contains several items supporting
    /// [`DataFormat::bitmap`], the first matching one will be returned.
    fn try_get_bitmap_async(&self) -> LocalBoxFuture<Result<Option<Rc<Bitmap>>, ClipboardError>> {
        self.try_get_value_async(&DataFormat::bitmap())
    }
}

impl<I: IAsyncDataTransfer + ?Sized> AsyncDataTransferExtensions for I {}
