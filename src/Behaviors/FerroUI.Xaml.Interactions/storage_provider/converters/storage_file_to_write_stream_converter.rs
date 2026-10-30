//! Port of `StorageProvider/Converters/StorageFileToWriteStreamConverter.cs`.

use crate::action_task::TaskHandle;
use crate::{StorageFileRef, StorageItemRef};
use ferroui_base::platform::storage::IStorageFile;
use ferroui_base::data::converters::IValueConverter;
use ferroui_base::data::core::ValueType;
use ferroui_base::data::{BindingError, BindingOperations};
use ferroui_base::utilities::CultureInfo;
use ferroui_base::BoxedValue;
use std::rc::Rc;

/// Converts a storage file to a write stream: the task of opening the stream
/// (`TaskHandle<std::io::Result<Box<dyn Write>>>`).
#[derive(Debug, Default)]
pub struct StorageFileToWriteStreamConverter;

impl StorageFileToWriteStreamConverter {
    /// Gets a shared instance of the converter (`StorageFileToWriteStreamConverter.Instance`).
    pub fn instance() -> Rc<StorageFileToWriteStreamConverter> {
        ferroui_base::ferro_thread_local! {
            static INSTANCE: Rc<StorageFileToWriteStreamConverter> = Rc::new(StorageFileToWriteStreamConverter);
        }
        INSTANCE.with(Rc::clone)
    }
}

// A converter is a reference type: an instance equals itself.
impl PartialEq for StorageFileToWriteStreamConverter {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl IValueConverter for StorageFileToWriteStreamConverter {
    fn convert(
        &self,
        value: Option<&BoxedValue>,
        _target_type: ValueType,
        _parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        if let Some(storage_file) = value.and_then(storage_file_of) {
            return Ok(Some(Rc::new(TaskHandle::spawn(storage_file.open_write_async()))));
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

/// The storage file an untyped value holds (`value is IStorageFile storageFile`).
fn storage_file_of(value: &BoxedValue) -> Option<Rc<dyn IStorageFile>> {
    if let Some(file) = value.downcast_ref::<StorageFileRef>() {
        return Some(file.0.clone());
    }

    if let Some(file) = value.downcast_ref::<Rc<dyn IStorageFile>>() {
        return Some(file.clone());
    }

    value.downcast_ref::<StorageItemRef>().and_then(|item| item.0.clone().as_storage_file())
}
