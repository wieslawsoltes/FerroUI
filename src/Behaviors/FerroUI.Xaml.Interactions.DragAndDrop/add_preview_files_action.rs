//! Port of `AddPreviewFilesAction.cs`.

use crate::values::{drag_event_args_of, object_of, StorageItemValue};
use ferroui_base::collections::FerroList;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::input::{DataFormat, DataTransferExtensions};
use ferroui_base::platform::storage::IStorageItem;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, ElementRef, FerroObjectImpl,
    FerroProperty, Ref, StyledElementImpl, StyledProperty,
};
use ferroui_controls::{ItemsControl, ItemsSource};
use ferroui_system::collections::object_model::ObservableCollection;
use ferroui_xaml_interactivity::{StyledElementAction, StyledElementActionImpl};
use std::rc::Rc;

/// Adds the files of a drag event (the parameter of the action) to the items of an
/// [`ItemsControl`]: the [`items_control`](Self::items_control), or else the sender.
#[repr(C)]
pub struct AddPreviewFilesAction {
    base: StyledElementAction,
}

ferro_class!(AddPreviewFilesAction: StyledElementAction);
ferro_class_info!(AddPreviewFilesAction {
    new: AddPreviewFilesAction::new,
    markup: {
        property_attributes: [
            ItemsControl: [ResolveByName],
        ],
    },
});
ferroui_base::ferro_impl_classes!(AddPreviewFilesAction: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl AddPreviewFilesAction {
        /// Identifies the `ItemsControl` property.
        ///
        /// Deviation (DEVIATIONS.md, Behaviors): the items control is held by an element reference
        /// (a weak reference), as the framework holds the elements a property names.
        pub fn items_control_property() -> StyledProperty<Option<ElementRef<ItemsControl>>> {
            ValueTypes::register_element_ref::<ItemsControl>();
            FerroProperty::register::<AddPreviewFilesAction, _>("ItemsControl", None)
        }
    }
}

impl StyledElementActionImpl for AddPreviewFilesAction {
    fn execute(this: &Self, sender: Option<&BoxedValue>, parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        Some(Rc::new(this.execute_core(sender, parameter)))
    }
}

/// Not from the reference: replaces the items of `items_source` with `files` (`IList.Clear` and
/// `IList.Add` of the reference).
///
/// Deviation (DEVIATIONS.md, Behaviors): the reference fills any writable `IList`; an items source
/// of the framework is read only, so the port fills the lists whose type it knows: a `FerroList`
/// or an `ObservableCollection` of storage items (`StorageItemValue`) or of untyped values. Returns
/// whether the source is one of them.
pub(crate) fn fill_items_source(items_source: &ItemsSource, files: &[Rc<dyn IStorageItem>]) -> bool {
    let values = || files.iter().map(|file| StorageItemValue(file.clone()));
    let boxed = || values().map(|value| Some(Rc::new(value) as BoxedValue));

    if let Some(list) = items_source.downcast_ref::<FerroList<StorageItemValue>>() {
        list.clear();
        list.add_range(values());
        return true;
    }
    if let Some(list) = items_source.downcast_ref::<FerroList<Option<BoxedValue>>>() {
        list.clear();
        list.add_range(boxed());
        return true;
    }
    if let Some(list) = items_source.downcast_ref::<ObservableCollection<StorageItemValue>>() {
        list.clear();
        for value in values() {
            list.add(value);
        }
        return true;
    }
    if let Some(list) = items_source.downcast_ref::<ObservableCollection<Option<BoxedValue>>>() {
        list.clear();
        for value in boxed() {
            list.add(value);
        }
        return true;
    }
    false
}

impl AddPreviewFilesAction {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementAction::construct() }
    }

    /// Creates the action.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the items control used to display preview.
    pub fn items_control(&self) -> Option<Ref<ItemsControl>> {
        ElementRef::resolve(&self.get_value(Self::items_control_property()))
    }

    pub fn set_items_control(&self, value: Option<Ref<ItemsControl>>) {
        self.set_value(Self::items_control_property(), ElementRef::from_nullable(value))
    }

    fn execute_core(&self, sender: Option<&BoxedValue>, parameter: Option<&BoxedValue>) -> bool {
        let Some(e) = drag_event_args_of(parameter).filter(|_| self.is_enabled()) else {
            return false;
        };

        let Some(items_control) =
            self.items_control().or_else(|| object_of(sender).and_then(|sender| sender.cast::<ItemsControl>()))
        else {
            return false;
        };

        if !e.data_transfer().contains(&DataFormat::file()) {
            return false;
        }

        let Some(files) = e.data_transfer().try_get_files().filter(|files| !files.is_empty()) else {
            return false;
        };

        match items_control.items_source() {
            Some(items_source) => fill_items_source(&items_source, &files),
            None => false,
        }
    }
}
