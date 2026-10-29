//! Port of `StorageProvider/Converters/StorageFileToReadStreamConverter.cs`.

use crate::action_task::TaskHandle;
use crate::{StorageFileRef, StorageItemRef};
use ferroui_base::platform::storage::IStorageFile;
use ferroui_base::data::converters::IValueConverter;
use ferroui_base::data::core::ValueType;
use ferroui_base::data::{BindingError, BindingOperations};
use ferroui_base::utilities::CultureInfo;
use ferroui_base::BoxedValue;
use std::rc::Rc;

/// Converts a storage file to a read stream: the task of opening the stream
/// (`TaskHandle<std::io::Result<Box<dyn Read>>>`).
#[derive(Debug, Default)]
pub struct StorageFileToReadStreamConverter;

impl StorageFileToReadStreamConverter {
    /// Gets a shared instance of the converter (`StorageFileToReadStreamConverter.Instance`).
    pub fn instance() -> Rc<StorageFileToReadStreamConverter> {
        ferroui_base::ferro_thread_local! {
            static INSTANCE: Rc<StorageFileToReadStreamConverter> = Rc::new(StorageFileToReadStreamConverter);
        }
        INSTANCE.with(Rc::clone)
    }
}

// A converter is a reference type: an instance equals itself.
impl PartialEq for StorageFileToReadStreamConverter {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl IValueConverter for StorageFileToReadStreamConverter {
    fn convert(
        &self,
        value: Option<&BoxedValue>,
        _target_type: ValueType,
        _parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        if let Some(storage_file) = value.and_then(storage_file_of) {
            return Ok(Some(Rc::new(TaskHandle::spawn(storage_file.open_read_async()))));
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
