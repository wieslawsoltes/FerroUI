use super::platform::ClipboardError;
use super::{DataFormat, DataFormatOf, IAsyncDataTransferItem, IDataTransferItem, LocalBoxFuture};
use crate::media::imaging::Bitmap;
use crate::platform::storage::IStorageItem;
use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Clone)]
enum DataAccessor {
    Value(Rc<dyn Any>),
    Lazy(Rc<dyn Fn() -> Option<Rc<dyn Any>>>),
}

impl DataAccessor {
    fn get_value(&self) -> Option<Rc<dyn Any>> {
        match self {
            DataAccessor::Value(value) => Some(value.clone()),
            DataAccessor::Lazy(get_value) => get_value(),
        }
    }
}

/// A mutable implementation of [`IDataTransferItem`] and
/// [`IAsyncDataTransferItem`]. This type also provides several functions to
/// easily create a `DataTransferItem` for common usages.
///
/// While it also implements [`IAsyncDataTransferItem`], this type always
/// returns data synchronously. For advanced usages, consider implementing
/// [`IAsyncDataTransferItem`] directly.
pub struct DataTransferItem {
    // Kept in the order the formats were first set; the common case is a
    // single format.
    accessors: RefCell<Vec<(DataFormat, DataAccessor)>>,
    formats: RefCell<Option<Rc<[DataFormat]>>>,
}

impl DataTransferItem {
    /// Creates an item without any format.
    pub fn new() -> Rc<DataTransferItem> {
        Rc::new(DataTransferItem { accessors: RefCell::new(Vec::new()), formats: RefCell::new(None) })
    }

    /// Gets the formats supported by this item.
    pub fn formats(&self) -> Rc<[DataFormat]> {
        if let Some(formats) = self.formats.borrow().as_ref() {
            return formats.clone();
        }

        let formats: Rc<[DataFormat]> = self.accessors.borrow().iter().map(|(format, _)| format.clone()).collect();
        *self.formats.borrow_mut() = Some(formats.clone());
        formats
    }

    /// Gets whether the item supports a specific format.
    pub fn contains(&self, format: &DataFormat) -> bool {
        self.accessors.borrow().iter().any(|(key, _)| key == format)
    }

    /// Tries to get a value for a given format. Returns `None` if the
    /// format is not supported.
    pub fn try_get_raw(&self, format: &DataFormat) -> Option<Rc<dyn Any>> {
        // The accessor runs user code: it is cloned out of the borrow first.
        let accessor = self.find_accessor(format)?;
        accessor.get_value()
    }

    /// Tries to get a typed value for a given format.
    pub fn try_get_value<T: Clone + 'static>(&self, format: &DataFormatOf<T>) -> Option<T> {
        self.try_get_raw(format).and_then(|value| value.downcast_ref::<T>().cloned())
    }

    fn find_accessor(&self, format: &DataFormat) -> Option<DataAccessor> {
        self.accessors.borrow().iter().find(|(key, _)| key == format).map(|(_, accessor)| accessor.clone())
    }

    /// Sets the value for a given format.
    ///
    /// If `value` is `None`, the format won't be part of the item.
    pub fn set<T: 'static>(&self, format: &DataFormatOf<T>, value: Option<T>) {
        match value {
            None => self.remove_core(format),
            Some(value) => self.set_core(format, DataAccessor::Value(Rc::new(value))),
        }
    }

    /// Sets a value created on demand for a given format.
    pub fn set_with<T: 'static>(&self, format: &DataFormatOf<T>, get_value: impl Fn() -> Option<T> + 'static) {
        self.set_core(
            format,
            DataAccessor::Lazy(Rc::new(move || get_value().map(|value| Rc::new(value) as Rc<dyn Any>))),
        );
    }

    fn set_core(&self, format: &DataFormat, accessor: DataAccessor) {
        let old = {
            let mut accessors = self.accessors.borrow_mut();
            match accessors.iter_mut().find(|(key, _)| key == format) {
                Some(entry) => Some(std::mem::replace(&mut entry.1, accessor)),
                None => {
                    accessors.push((format.clone(), accessor));
                    None
                }
            }
        };
        // Dropped outside of the borrow: the old value may run user code.
        drop(old);

        *self.formats.borrow_mut() = None;
    }

    fn remove_core(&self, format: &DataFormat) {
        let removed = {
            let mut accessors = self.accessors.borrow_mut();
            accessors.iter().position(|(key, _)| key == format).map(|index| accessors.remove(index))
        };

        if removed.is_some() {
            *self.formats.borrow_mut() = None;
        }
    }

    /// Sets the value for the [`DataFormat::text`] format.
    ///
    /// If `value` is `None`, the format won't be part of the item.
    pub fn set_text(&self, value: Option<&str>) {
        self.set(&DataFormat::text(), value.map(str::to_string))
    }

    /// Sets the value for the [`DataFormat::file`] format.
    ///
    /// If `value` is `None`, the format won't be part of the item.
    pub fn set_file(&self, value: Option<Rc<dyn IStorageItem>>) {
        self.set(&DataFormat::file(), value)
    }

    /// Sets the value for the [`DataFormat::bitmap`] format.
    ///
    /// If `value` is `None`, the format won't be part of the item.
    pub fn set_bitmap(&self, value: Option<Rc<Bitmap>>) {
        self.set(&DataFormat::bitmap(), value)
    }

    /// Creates a new item for a single format with a given value.
    ///
    /// If `value` is `None`, the format won't be part of the item.
    pub fn create<T: 'static>(format: &DataFormatOf<T>, value: Option<T>) -> Rc<DataTransferItem> {
        let item = DataTransferItem::new();
        item.set(format, value);
        item
    }

    /// Creates a new item for a single format with a given value created on
    /// demand.
    pub fn create_with<T: 'static>(
        format: &DataFormatOf<T>,
        get_value: impl Fn() -> Option<T> + 'static,
    ) -> Rc<DataTransferItem> {
        let item = DataTransferItem::new();
        item.set_with(format, get_value);
        item
    }

    /// Creates a new item with [`DataFormat::text`] as a single format.
    ///
    /// If `value` is `None`, the format won't be part of the item.
    pub fn create_text(value: Option<&str>) -> Rc<DataTransferItem> {
        Self::create(&DataFormat::text(), value.map(str::to_string))
    }

    /// Creates a new item with [`DataFormat::file`] as a single format.
    ///
    /// If `value` is `None`, the format won't be part of the item.
    pub fn create_file(value: Option<Rc<dyn IStorageItem>>) -> Rc<DataTransferItem> {
        Self::create(&DataFormat::file(), value)
    }
}

impl IDataTransferItem for DataTransferItem {
    fn formats(&self) -> Rc<[DataFormat]> {
        DataTransferItem::formats(self)
    }

    fn try_get_raw(&self, format: &DataFormat) -> Option<Rc<dyn Any>> {
        DataTransferItem::try_get_raw(self, format)
    }
}

impl IAsyncDataTransferItem for DataTransferItem {
    fn formats(&self) -> Rc<[DataFormat]> {
        DataTransferItem::formats(self)
    }

    fn try_get_raw_async(&self, format: &DataFormat) -> LocalBoxFuture<Result<Option<Rc<dyn Any>>, ClipboardError>> {
        Box::pin(std::future::ready(Ok(DataTransferItem::try_get_raw(self, format))))
    }
}
