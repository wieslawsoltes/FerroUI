use super::{DataFormat, DataFormatOf, DataTransferItemExtensions, IDataTransfer, IDataTransferItem};
use crate::media::imaging::Bitmap;
use std::rc::Rc;

/// The items of a data transfer that support a format, in order.
pub struct DataTransferItems {
    items: Rc<[Rc<dyn IDataTransferItem>]>,
    format: DataFormat,
    index: usize,
}

impl Iterator for DataTransferItems {
    type Item = Rc<dyn IDataTransferItem>;

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

// Keep the `try_get_xxx` methods in sync with the `try_get_xxx_async` ones
// of `AsyncDataTransferExtensions`.

/// Contains extension methods for [`IDataTransfer`].
pub trait DataTransferExtensions: IDataTransfer {
    /// Gets whether the data transfer supports a specific format.
    fn contains(&self, format: &DataFormat) -> bool {
        self.formats().iter().any(|candidate| candidate == format)
    }

    /// Gets the list of [`IDataTransferItem`] contained in the data
    /// transfer, filtered by a given format.
    ///
    /// Some platforms (such as Windows and X11) may only support a single
    /// data item for all formats except files.
    fn get_items(&self, format: &DataFormat) -> DataTransferItems {
        DataTransferItems { items: self.items(), format: format.clone(), index: 0 }
    }

    /// Tries to get a value for a given format from the data transfer.
    /// Returns `None` if the format is not supported.
    ///
    /// If the data transfer contains several items supporting the format,
    /// the first matching one will be returned.
    fn try_get_value<T: Clone + 'static>(&self, format: &DataFormatOf<T>) -> Option<T> {
        self.get_items(format).next().and_then(|item| item.try_get_value(format))
    }

    /// Tries to get multiple values for a given format from the data
    /// transfer. Returns `None` if the format is not supported.
    fn try_get_values<T: Clone + 'static>(&self, format: &DataFormatOf<T>) -> Option<Vec<T>> {
        let mut results: Option<Vec<T>> = None;

        for item in self.get_items(format) {
            let Some(result) = item.try_get_value(format) else { continue };
            results.get_or_insert_with(Vec::new).push(result);
        }

        results
    }

    /// Returns a text, if available, from the data transfer.
    ///
    /// If the data transfer contains several items supporting
    /// [`DataFormat::text`], the first matching one will be returned.
    fn try_get_text(&self) -> Option<String> {
        self.try_get_value(&DataFormat::text())
    }

    /// Returns a bitmap, if available, from the data transfer.
    ///
    /// If the data transfer contains several items supporting
    /// [`DataFormat::bitmap`], the first matching one will be returned.
    fn try_get_bitmap(&self) -> Option<Rc<Bitmap>> {
        self.try_get_value(&DataFormat::bitmap())
    }
}

impl<I: IDataTransfer + ?Sized> DataTransferExtensions for I {}
