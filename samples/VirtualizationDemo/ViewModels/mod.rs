//! The view models of the sample (namespace `VirtualizationDemo.ViewModels`): one module per upstream file.

use crate::markup::XamlClass;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::metadata::{MarkupType, MarkupTyped};
use ferroui_base::TypeInfo;

mod chat_page_view_model;
mod expander_item_view_model;
mod expander_page_view_model;
mod main_window_view_model;
mod playground_item_view_model;
mod playground_page_view_model;
// The pseudo-random number generator of the runtime library (not a file of the sample).
mod random;

pub use chat_page_view_model::ChatPageViewModel;
pub use expander_item_view_model::ExpanderItemViewModel;
pub use expander_page_view_model::{ExpanderItemList, ExpanderPageViewModel};
pub use main_window_view_model::MainWindowViewModel;
pub use playground_item_view_model::PlaygroundItemViewModel;
pub use playground_page_view_model::{PlaygroundItemList, PlaygroundPageViewModel};

/// The classes of this namespace (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[];

/// The classes of this namespace that have a document (`&X::XAML_CLASS`).
pub(crate) const CLASSES: &[&XamlClass] = &[];

/// The types of this namespace declared with `ferro_markup_type!` / `ferro_markup_enum!`
/// (`<X as MarkupTyped>::MARKUP`).
pub(crate) const MARKUP_TYPES: &[&MarkupType] = &[
    <ChatPageViewModel as MarkupTyped>::MARKUP,
    <ExpanderItemViewModel as MarkupTyped>::MARKUP,
    <ExpanderPageViewModel as MarkupTyped>::MARKUP,
    <MainWindowViewModel as MarkupTyped>::MARKUP,
    <PlaygroundItemViewModel as MarkupTyped>::MARKUP,
    <PlaygroundPageViewModel as MarkupTyped>::MARKUP,
];

/// What the untyped value conversions must know about the types of this namespace
/// (`ValueTypes::register_reference::<X>()`, nullable forms, casts to contracts).
pub(crate) fn register_value_types() {
    ValueTypes::register_reference::<ChatPageViewModel>();
    ValueTypes::register_reference::<ExpanderItemViewModel>();
    ValueTypes::register_reference::<ExpanderPageViewModel>();
    ValueTypes::register_reference::<MainWindowViewModel>();
    ValueTypes::register_reference::<PlaygroundItemViewModel>();
    ValueTypes::register_reference::<PlaygroundPageViewModel>();
}

/// Makes the typed lists of the view models known to markup (`ferro_markup_list!`).
pub(crate) fn register_lists() {
    ExpanderItemList::register();
    PlaygroundItemList::register();
}
