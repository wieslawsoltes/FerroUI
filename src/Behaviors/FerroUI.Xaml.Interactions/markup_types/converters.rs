//! Markup metadata of the converters of the storage items (`StorageProvider/Converters/`): their
//! shared instances (`{x:Static StorageItemToPathConverter.Instance}`).

use crate::{StorageFileToReadStreamConverter, StorageFileToWriteStreamConverter, StorageItemToPathConverter};
use ferroui_base::data::converters::IValueConverter;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::ferro_markup_type;
use ferroui_base::metadata::{MarkupType, MarkupTyped};
use std::rc::Rc;

ferro_markup_type!(class StorageFileToReadStreamConverter {
    namespace: "FerroUI.Xaml.Interactions.Core",
    handles: [Rc<StorageFileToReadStreamConverter>, Option<Rc<StorageFileToReadStreamConverter>>],
    interfaces: [Rc<dyn IValueConverter>],
    constructors: [() => || Rc::new(StorageFileToReadStreamConverter)],
    fields: [Instance: Rc<StorageFileToReadStreamConverter> => StorageFileToReadStreamConverter::instance],
});

ferro_markup_type!(class StorageFileToWriteStreamConverter {
    namespace: "FerroUI.Xaml.Interactions.Core",
    handles: [Rc<StorageFileToWriteStreamConverter>, Option<Rc<StorageFileToWriteStreamConverter>>],
    interfaces: [Rc<dyn IValueConverter>],
    constructors: [() => || Rc::new(StorageFileToWriteStreamConverter)],
    fields: [Instance: Rc<StorageFileToWriteStreamConverter> => StorageFileToWriteStreamConverter::instance],
});

ferro_markup_type!(class StorageItemToPathConverter {
    namespace: "FerroUI.Xaml.Interactions.Core",
    handles: [Rc<StorageItemToPathConverter>, Option<Rc<StorageItemToPathConverter>>],
    interfaces: [Rc<dyn IValueConverter>],
    constructors: [() => || Rc::new(StorageItemToPathConverter)],
    fields: [Instance: Rc<StorageItemToPathConverter> => StorageItemToPathConverter::instance],
});

/// The converters.
pub(super) const TYPES: &[&MarkupType] = &[
    <StorageFileToReadStreamConverter as MarkupTyped>::MARKUP,
    <StorageFileToWriteStreamConverter as MarkupTyped>::MARKUP,
    <StorageItemToPathConverter as MarkupTyped>::MARKUP,
];

/// Registers the nullable handles of the converters and their casts to the converter contract.
pub(super) fn register_value_types() {
    ValueTypes::register_nullable::<Rc<StorageFileToReadStreamConverter>>();
    ValueTypes::register_cast::<Rc<StorageFileToReadStreamConverter>, Rc<dyn IValueConverter>>(|c| c.clone());
    ValueTypes::register_nullable::<Rc<StorageFileToWriteStreamConverter>>();
    ValueTypes::register_cast::<Rc<StorageFileToWriteStreamConverter>, Rc<dyn IValueConverter>>(|c| c.clone());
    ValueTypes::register_nullable::<Rc<StorageItemToPathConverter>>();
    ValueTypes::register_cast::<Rc<StorageItemToPathConverter>, Rc<dyn IValueConverter>>(|c| c.clone());
}
