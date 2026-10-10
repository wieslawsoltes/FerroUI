//! The view models of the sample (namespace `BindingDemo.ViewModels`): one module per upstream file.

use crate::markup::XamlClass;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::data::model::ModelTypes;
use ferroui_base::metadata::{MarkupType, MarkupTyped};
use ferroui_base::TypeInfo;
use std::any::TypeId;

mod data_annotations_error_view_model;
mod exception_error_view_model;
mod indei_error_view_model;
mod main_window_view_model;
mod nested_command_view_model;
// The pseudo-random number generator of the runtime library (not a file of the sample).
mod random;
mod test_item;

pub use data_annotations_error_view_model::DataAnnotationsErrorViewModel;
pub use exception_error_view_model::ExceptionErrorViewModel;
pub use indei_error_view_model::IndeiErrorViewModel;
pub use main_window_view_model::{MainWindowViewModel, TestItem, TestItemOfString, TestItemOfStringList};
pub use nested_command_view_model::NestedCommandViewModel;

/// The classes of this namespace (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[];

/// The classes of this namespace that have a document (`&X::XAML_CLASS`).
pub(crate) const CLASSES: &[&XamlClass] = &[];

/// The types of this namespace declared with `ferro_markup_type!` / `ferro_markup_enum!`
/// (`<X as MarkupTyped>::MARKUP`).
pub(crate) const MARKUP_TYPES: &[&MarkupType] = &[
    <DataAnnotationsErrorViewModel as MarkupTyped>::MARKUP,
    <ExceptionErrorViewModel as MarkupTyped>::MARKUP,
    <IndeiErrorViewModel as MarkupTyped>::MARKUP,
    <MainWindowViewModel as MarkupTyped>::MARKUP,
    <NestedCommandViewModel as MarkupTyped>::MARKUP,
    <TestItemOfString as MarkupTyped>::MARKUP,
];

/// What the untyped value conversions must know about the types of this namespace
/// (`ValueTypes::register_reference::<X>()`, nullable forms, casts to contracts).
pub(crate) fn register_value_types() {
    ValueTypes::register_reference::<DataAnnotationsErrorViewModel>();
    ValueTypes::register_reference::<ExceptionErrorViewModel>();
    ValueTypes::register_reference::<IndeiErrorViewModel>();
    ValueTypes::register_reference::<MainWindowViewModel>();
    ValueTypes::register_reference::<NestedCommandViewModel>();
    ValueTypes::register_reference::<TestItem<String>>();
    // `IndeiErrorViewModel : INotifyDataErrorInfo`: the validation of a binding finds the
    // contract of a model object in the binding metadata of its type; the members of the
    // type are the ones of its markup metadata.
    if !ModelTypes::is_registered(TypeId::of::<IndeiErrorViewModel>()) {
        ModelTypes::register::<IndeiErrorViewModel>(|builder| builder.notify_data_error_info());
    }
}

/// Makes the typed lists of the view models known to markup (`ferro_markup_list!`).
pub(crate) fn register_lists() {
    TestItemOfStringList::register();
}
