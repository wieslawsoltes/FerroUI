use super::{ClipboardError, IClipboard};
use crate::input::{
    AsyncDataTransferExtensions, DataFormat, DataFormatOf, DataTransfer, DataTransferItem, IAsyncDataTransfer,
    LocalBoxFuture,
};
use crate::media::imaging::Bitmap;
use crate::platform::storage::IStorageItem;
use std::rc::Rc;

/// Disposes the data transfer returned by the clipboard when the operation
/// reading from it ends, however it ends.
struct Using(Rc<dyn IAsyncDataTransfer>);

impl Drop for Using {
    fn drop(&mut self) {
        self.0.dispose();
    }
}

/// Contains extension methods related to [`IClipboard`].
///
/// Every operation resolves to the [`ClipboardError`] of the clipboard or
/// of the data transfer it reads from, where the reference throws from the
/// awaited task.
pub trait ClipboardExtensions: IClipboard {
    /// Returns a list containing the formats currently available from the
    /// clipboard. It is empty if the clipboard is empty.
    fn get_data_formats_async(&self) -> LocalBoxFuture<Result<Rc<[DataFormat]>, ClipboardError>> {
        let data = self.try_get_data_async();

        Box::pin(async move {
            match data.await? {
                Some(data_transfer) => {
                    let data_transfer = Using(data_transfer);
                    data_transfer.0.try_formats()
                }
                None => Ok(Rc::from(Vec::new())),
            }
        })
    }

    /// Tries to get a value for a given format from the clipboard. The
    /// future resolves to `None` if the format is not supported.
    ///
    /// If the clipboard contains several items supporting the format, the
    /// first matching one will be returned.
    fn try_get_value_async<T: Clone + 'static>(
        &self,
        format: &DataFormatOf<T>,
    ) -> LocalBoxFuture<Result<Option<T>, ClipboardError>> {
        let data = self.try_get_data_async();
        let format = format.clone();

        Box::pin(async move {
            let Some(data_transfer) = data.await? else {
                return Ok(None);
            };
            let data_transfer = Using(data_transfer);
            let result = data_transfer.0.try_get_value_async(&format).await;
            result
        })
    }

    /// Tries to get multiple values for a given format from the clipboard.
    /// The future resolves to `None` if the format is not supported.
    fn try_get_values_async<T: Clone + 'static>(
        &self,
        format: &DataFormatOf<T>,
    ) -> LocalBoxFuture<Result<Option<Vec<T>>, ClipboardError>> {
        let data = self.try_get_data_async();
        let format = format.clone();

        Box::pin(async move {
            let Some(data_transfer) = data.await? else {
                return Ok(None);
            };
            let data_transfer = Using(data_transfer);
            let result = data_transfer.0.try_get_values_async(&format).await;
            result
        })
    }

    /// Places a single value on the clipboard in the specified format.
    ///
    /// By calling this method, the clipboard will be instantly cleared, and
    /// data will be lazily requested later when it is pasted.
    ///
    /// If `value` is `None`, nothing will be placed on the clipboard and it
    /// will be cleared instead.
    fn set_value_async<T: 'static>(
        &self,
        format: &DataFormatOf<T>,
        value: Option<T>,
    ) -> LocalBoxFuture<Result<(), ClipboardError>> {
        let Some(value) = value else {
            return self.clear_async();
        };

        let data_transfer = DataTransfer::new();
        data_transfer.add(DataTransferItem::create(format, Some(value)));
        self.set_data_async(Some(data_transfer))
    }

    /// Places multiple values on the clipboard in the specified format.
    ///
    /// By calling this method, the clipboard will be instantly cleared, and
    /// data will be lazily requested later when it is pasted.
    ///
    /// If `values` is `None` or empty, nothing will be placed on the
    /// clipboard and it will be cleared instead.
    fn set_values_async<T: 'static>(
        &self,
        format: &DataFormatOf<T>,
        values: Option<impl IntoIterator<Item = T>>,
    ) -> LocalBoxFuture<Result<(), ClipboardError>>
    where
        Self: Sized,
    {
        let Some(values) = values else {
            return self.clear_async();
        };

        let data_transfer = DataTransfer::new();

        for value in values {
            data_transfer.add(DataTransferItem::create(format, Some(value)));
        }

        if data_transfer.items().is_empty() {
            self.clear_async()
        } else {
            self.set_data_async(Some(data_transfer))
        }
    }

    /// Returns a text, if available, from the clipboard.
    fn try_get_text_async(&self) -> LocalBoxFuture<Result<Option<String>, ClipboardError>> {
        self.try_get_value_async(&DataFormat::text())
    }

    /// Returns a file (or folder), if available, from the clipboard.
    fn try_get_file_async(&self) -> LocalBoxFuture<Result<Option<Rc<dyn IStorageItem>>, ClipboardError>> {
        self.try_get_value_async(&DataFormat::file())
    }

    /// Returns the files (or folders), if available, from the clipboard.
    fn try_get_files_async(&self) -> LocalBoxFuture<Result<Option<Vec<Rc<dyn IStorageItem>>>, ClipboardError>> {
        self.try_get_values_async(&DataFormat::file())
    }

    /// Returns a bitmap, if available, from the clipboard.
    fn try_get_bitmap_async(&self) -> LocalBoxFuture<Result<Option<Rc<Bitmap>>, ClipboardError>> {
        self.try_get_value_async(&DataFormat::bitmap())
    }

    /// Places a text on the clipboard.
    ///
    /// By calling this method, the clipboard will be instantly cleared.
    ///
    /// If `text` is `None`, nothing will be placed on the clipboard and it
    /// will be cleared instead.
    fn set_text_async(&self, text: Option<&str>) -> LocalBoxFuture<Result<(), ClipboardError>> {
        self.set_value_async(&DataFormat::text(), text.map(str::to_string))
    }

    /// Places a file (or folder) on the clipboard.
    ///
    /// By calling this method, the clipboard will be instantly cleared.
    ///
    /// If `file` is `None`, nothing will be placed on the clipboard and it
    /// will be cleared instead.
    fn set_file_async(&self, file: Option<Rc<dyn IStorageItem>>) -> LocalBoxFuture<Result<(), ClipboardError>> {
        self.set_value_async(&DataFormat::file(), file)
    }

    /// Places a list of files (or folders) on the clipboard.
    ///
    /// By calling this method, the clipboard will be instantly cleared.
    ///
    /// If `files` is `None` or empty, nothing will be placed on the
    /// clipboard and it will be cleared instead.
    fn set_files_async(
        &self,
        files: Option<impl IntoIterator<Item = Rc<dyn IStorageItem>>>,
    ) -> LocalBoxFuture<Result<(), ClipboardError>>
    where
        Self: Sized,
    {
        self.set_values_async(&DataFormat::file(), files)
    }

    /// Places a bitmap on the clipboard.
    ///
    /// By calling this method, the clipboard will be instantly cleared.
    ///
    /// If `bitmap` is `None`, nothing will be placed on the clipboard and
    /// it will be cleared instead.
    fn set_bitmap_async(&self, bitmap: Option<Rc<Bitmap>>) -> LocalBoxFuture<Result<(), ClipboardError>> {
        self.set_value_async(&DataFormat::bitmap(), bitmap)
    }
}

impl<I: IClipboard + ?Sized> ClipboardExtensions for I {}
