use super::{DataFormat, DataFormatOf, IDataTransferItem};
use crate::media::imaging::Bitmap;
use crate::platform::storage::IStorageItem;
use std::rc::Rc;

/// Contains extension methods for [`IDataTransferItem`].
pub trait DataTransferItemExtensions: IDataTransferItem {
    /// Gets whether the item supports a specific format.
    fn contains(&self, format: &DataFormat) -> bool {
        self.formats().iter().any(|candidate| candidate == format)
    }

    /// Tries to get a value for a given format from the item. Returns
    /// `None` if the format is not supported.
    fn try_get_value<T: Clone + 'static>(&self, format: &DataFormatOf<T>) -> Option<T> {
        self.try_get_raw(format).and_then(|value| value.downcast_ref::<T>().cloned())
    }

    /// Returns a text, if available, from the item.
    fn try_get_text(&self) -> Option<String> {
        self.try_get_value(&DataFormat::text())
    }

    /// Returns a bitmap, if available, from the item.
    fn try_get_bitmap(&self) -> Option<Rc<Bitmap>> {
        self.try_get_value(&DataFormat::bitmap())
    }

    /// Returns a file (or folder), if available, from the item.
    ///
    /// See [`DataFormat::file`].
    fn try_get_file(&self) -> Option<Rc<dyn IStorageItem>> {
        self.try_get_value(&DataFormat::file())
    }
}

impl<I: IDataTransferItem + ?Sized> DataTransferItemExtensions for I {}
