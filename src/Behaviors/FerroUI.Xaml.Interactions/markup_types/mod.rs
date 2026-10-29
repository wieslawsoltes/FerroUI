//! Markup metadata of the types of this crate that are not declared with their type: the modes of
//! the action group (`enums.rs`) and the converters of the storage items (`converters.rs`). The
//! classes state their properties through their registered properties, as every class of the
//! framework.

use crate::{
    ActionTask, AsyncDataTransferRef, ClipboardRef, StorageFileRef, StorageFolderRef, StorageItemRef,
    StorageProviderRef,
};
use ferroui_base::data::core::ValueTypes;
use ferroui_base::metadata::MarkupType;
use ferroui_base::platform::storage::IStorageProvider;
use std::rc::Rc;

mod converters;
mod enums;

/// The lists of the types declared for markup.
const TYPE_LISTS: &[&[&MarkupType]] = &[converters::TYPES, enums::TYPES];

/// Adds the types of this crate to the process-wide table of types with markup metadata. Called
/// once by `register_types()`.
pub(crate) fn register() {
    for types in TYPE_LISTS {
        MarkupType::register_all(types);
    }
    #[cfg(feature = "file-system")]
    MarkupType::register_all(crate::file_system::MARKUP_TYPES);
    ValueTypes::register_global(register_value_types);
}

/// What the untyped value conversions must know about the types of this crate: the nullable forms
/// of the handles the properties hold, and of the converters.
fn register_value_types() {
    converters::register_value_types();
    enums::register_value_types();

    ValueTypes::register_nullable::<ActionTask>();
    ValueTypes::register_nullable::<AsyncDataTransferRef>();
    ValueTypes::register_nullable::<ClipboardRef>();
    ValueTypes::register_nullable::<StorageFileRef>();
    ValueTypes::register_nullable::<StorageFolderRef>();
    ValueTypes::register_nullable::<StorageItemRef>();
    ValueTypes::register_nullable::<StorageProviderRef>();
    // The storage provider of a top level (`{Binding $parent[TopLevel].StorageProvider}`, the
    // contract the framework declares to markup) as the handle the pickers hold.
    ValueTypes::register_cast::<Rc<dyn IStorageProvider>, StorageProviderRef>(|provider| {
        StorageProviderRef::new(provider.clone())
    });
    ValueTypes::register_conversion::<Rc<dyn IStorageProvider>, Option<StorageProviderRef>>(|provider| {
        Some(Some(StorageProviderRef::new(provider.clone())))
    });
}
