use super::{DataFormat, DataTransferItem, IAsyncDataTransfer, IAsyncDataTransferItem, IDataTransfer, IDataTransferItem};
use crate::reactive::IDisposable;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Default)]
struct Cache {
    formats: Option<Rc<[DataFormat]>>,
    items: Option<Rc<[Rc<DataTransferItem>]>>,
    sync_items: Option<Rc<[Rc<dyn IDataTransferItem>]>>,
    async_items: Option<Rc<[Rc<dyn IAsyncDataTransferItem>]>>,
}

/// A mutable implementation of [`IDataTransfer`] and
/// [`IAsyncDataTransfer`].
///
/// While it also implements [`IAsyncDataTransfer`], this type always
/// returns data synchronously. For advanced usages, consider implementing
/// [`IAsyncDataTransfer`] directly.
pub struct DataTransfer {
    items: RefCell<Vec<Rc<DataTransferItem>>>,
    cache: RefCell<Cache>,
}

impl DataTransfer {
    /// Creates an empty data transfer.
    pub fn new() -> Rc<DataTransfer> {
        Rc::new(DataTransfer { items: RefCell::new(Vec::new()), cache: RefCell::new(Cache::default()) })
    }

    /// Gets the formats supported by the items of this object, without
    /// duplicates.
    pub fn formats(&self) -> Rc<[DataFormat]> {
        if let Some(formats) = self.cache.borrow().formats.as_ref() {
            return formats.clone();
        }

        let mut formats: Vec<DataFormat> = Vec::new();
        for item in self.items().iter() {
            for format in item.formats().iter() {
                if !formats.contains(format) {
                    formats.push(format.clone());
                }
            }
        }

        let formats: Rc<[DataFormat]> = Rc::from(formats);
        self.cache.borrow_mut().formats = Some(formats.clone());
        formats
    }

    /// Gets a list of [`DataTransferItem`] contained in this object.
    pub fn items(&self) -> Rc<[Rc<DataTransferItem>]> {
        if let Some(items) = self.cache.borrow().items.as_ref() {
            return items.clone();
        }

        let items: Rc<[Rc<DataTransferItem>]> = Rc::from(self.items.borrow().as_slice());
        self.cache.borrow_mut().items = Some(items.clone());
        items
    }

    /// Adds an existing [`DataTransferItem`] to this object.
    pub fn add(&self, item: Rc<DataTransferItem>) {
        let old = std::mem::take(&mut *self.cache.borrow_mut());
        drop(old);
        self.items.borrow_mut().push(item);
    }
}

impl IDisposable for DataTransfer {
    fn dispose(&self) {}
}

impl IDataTransfer for DataTransfer {
    fn formats(&self) -> Rc<[DataFormat]> {
        DataTransfer::formats(self)
    }

    fn items(&self) -> Rc<[Rc<dyn IDataTransferItem>]> {
        if let Some(items) = self.cache.borrow().sync_items.as_ref() {
            return items.clone();
        }

        let items: Rc<[Rc<dyn IDataTransferItem>]> =
            self.items.borrow().iter().map(|item| item.clone() as Rc<dyn IDataTransferItem>).collect();
        self.cache.borrow_mut().sync_items = Some(items.clone());
        items
    }

    fn as_async_data_transfer(self: Rc<Self>) -> Option<Rc<dyn IAsyncDataTransfer>> {
        Some(self)
    }
}

impl IAsyncDataTransfer for DataTransfer {
    fn formats(&self) -> Rc<[DataFormat]> {
        DataTransfer::formats(self)
    }

    fn items(&self) -> Rc<[Rc<dyn IAsyncDataTransferItem>]> {
        if let Some(items) = self.cache.borrow().async_items.as_ref() {
            return items.clone();
        }

        let items: Rc<[Rc<dyn IAsyncDataTransferItem>]> =
            self.items.borrow().iter().map(|item| item.clone() as Rc<dyn IAsyncDataTransferItem>).collect();
        self.cache.borrow_mut().async_items = Some(items.clone());
        items
    }

    fn as_data_transfer(self: Rc<Self>) -> Option<Rc<dyn IDataTransfer>> {
        Some(self)
    }
}
