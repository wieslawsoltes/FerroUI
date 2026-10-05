use super::platform::ClipboardError;
use super::{DataFormat, DataFormatOf, IAsyncDataTransferItem, LocalBoxFuture};
use crate::media::imaging::Bitmap;
use crate::platform::storage::IStorageItem;
use std::rc::Rc;

/// Contains extension methods for [`IAsyncDataTransferItem`].
pub trait AsyncDataTransferItemExtensions: IAsyncDataTransferItem {
    /// Gets whether the item supports a specific format.
    fn contains(&self, format: &DataFormat) -> bool {
        self.formats().iter().any(|candidate| candidate == format)
    }

    /// Gets whether the item supports a specific format, or the failure of
    /// the platform that provides the formats of the item.
    fn try_contains(&self, format: &DataFormat) -> Result<bool, ClipboardError> {
        Ok(self.try_formats()?.iter().any(|candidate| candidate == format))
    }

    /// Tries to get a value for a given format from the item. The future
    /// resolves to `None` if the format is not supported, and to an error
    /// if the platform providing the value fails.
    fn try_get_value_async<T: Clone + 'static>(
        &self,
        format: &DataFormatOf<T>,
    ) -> LocalBoxFuture<Result<Option<T>, ClipboardError>> {
        let raw = self.try_get_raw_async(format);
        Box::pin(async move { Ok(raw.await?.and_then(|value| value.downcast_ref::<T>().cloned())) })
    }

    /// Returns a text, if available, from the item.
    fn try_get_text_async(&self) -> LocalBoxFuture<Result<Option<String>, ClipboardError>> {
        self.try_get_value_async(&DataFormat::text())
    }

    /// Returns a bitmap, if available, from the item.
    fn try_get_bitmap_async(&self) -> LocalBoxFuture<Result<Option<Rc<Bitmap>>, ClipboardError>> {
        self.try_get_value_async(&DataFormat::bitmap())
    }

    /// Returns a file (or folder), if available, from the item.
    ///
    /// See [`DataFormat::file`].
    fn try_get_file_async(&self) -> LocalBoxFuture<Result<Option<Rc<dyn IStorageItem>>, ClipboardError>> {
        self.try_get_value_async(&DataFormat::file())
    }
}

impl<I: IAsyncDataTransferItem + ?Sized> AsyncDataTransferItemExtensions for I {}
