//! Port of `StorageProvider/Converters/StorageItemToPathConverter.cs`.

use crate::{StorageFileRef, StorageFolderRef, StorageItemRef};
use ferroui_base::platform::storage::IStorageItem;
use ferroui_base::utilities::Uri;
use ferroui_base::data::converters::IValueConverter;
use ferroui_base::data::core::ValueType;
use ferroui_base::data::{BindingError, BindingOperations};
use ferroui_base::utilities::CultureInfo;
use ferroui_base::BoxedValue;
use std::rc::Rc;

/// Converts a storage item to its path (a [`Uri`]).
#[derive(Debug, Default)]
pub struct StorageItemToPathConverter;

impl StorageItemToPathConverter {
    /// Gets a shared instance of the converter (`StorageItemToPathConverter.Instance`).
    pub fn instance() -> Rc<StorageItemToPathConverter> {
        ferroui_base::ferro_thread_local! {
            static INSTANCE: Rc<StorageItemToPathConverter> = Rc::new(StorageItemToPathConverter);
        }
        INSTANCE.with(Rc::clone)
    }
}

// A converter is a reference type: an instance equals itself.
impl PartialEq for StorageItemToPathConverter {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl IValueConverter for StorageItemToPathConverter {
    fn convert(
        &self,
        value: Option<&BoxedValue>,
        _target_type: ValueType,
        _parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        if let Some(path) = value.and_then(storage_item_path_of) {
            return Ok(Some(Rc::new(path)));
        }

        Ok(Some(BindingOperations::do_nothing()))
    }

    fn convert_back(
        &self,
        _value: Option<&BoxedValue>,
        _target_type: ValueType,
        _parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        Ok(Some(BindingOperations::do_nothing()))
    }
}

/// The path of the storage item an untyped value holds (`value is IStorageItem storageItem`).
fn storage_item_path_of(value: &BoxedValue) -> Option<Uri> {
    if let Some(item) = value.downcast_ref::<StorageItemRef>() {
        return Some(item.path());
    }

    if let Some(file) = value.downcast_ref::<StorageFileRef>() {
        return Some(file.path());
    }

    if let Some(folder) = value.downcast_ref::<StorageFolderRef>() {
        return Some(folder.path());
    }

    if let Some(item) = value.downcast_ref::<Rc<dyn IStorageItem>>() {
        return Some(item.path());
    }

    None
}
