//! The view models of the demo (namespace `TreeDataGridDemo.ViewModels`): one module per
//! reference file.

use crate::markup::XamlClass;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::metadata::{MarkupType, MarkupTyped};
use ferroui_base::TypeInfo;

mod core_demo_view_model;
mod countries_page_view_model;
mod drag_drop_page_view_model;
mod files_page_view_model;
mod main_window_view_model;
mod people_xaml_page_view_model;
mod template_column_bug_page_view_model;
mod wikipedia_page_view_model;
// The pseudo-random number generator of the runtime library (not a file of the demo).
pub(crate) mod random;

pub use core_demo_view_model::{CoreDemoViewModel, DemoPerson, DemoTemplateItem, ExpansionState};
pub use countries_page_view_model::CountriesPageViewModel;
pub use drag_drop_page_view_model::DragDropPageViewModel;
pub use files_page_view_model::FilesPageViewModel;
pub use main_window_view_model::MainWindowViewModel;
pub use people_xaml_page_view_model::PeopleXamlPageViewModel;
pub use template_column_bug_page_view_model::TemplateColumnBugPageViewModel;
pub use wikipedia_page_view_model::{LoadingTask, WikipediaPageViewModel};

/// The classes of this namespace (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[];

/// The classes of this namespace that have a document (`&X::XAML_CLASS`).
pub(crate) const CLASSES: &[&XamlClass] = &[];

/// The types of this namespace declared with `ferro_markup_type!` / `ferro_markup_enum!`
/// (`<X as MarkupTyped>::MARKUP`).
pub(crate) const MARKUP_TYPES: &[&MarkupType] = &[
    <CountriesPageViewModel as MarkupTyped>::MARKUP,
    <MainWindowViewModel as MarkupTyped>::MARKUP,
    <PeopleXamlPageViewModel as MarkupTyped>::MARKUP,
    <TemplateColumnBugPageViewModel as MarkupTyped>::MARKUP,
    <WikipediaPageViewModel as MarkupTyped>::MARKUP,
    <DragDropPageViewModel as MarkupTyped>::MARKUP,
    <FilesPageViewModel as MarkupTyped>::MARKUP,
    <CoreDemoViewModel as MarkupTyped>::MARKUP,
];

/// What the untyped value conversions must know about the types of this namespace.
pub(crate) fn register_value_types() {
    ValueTypes::register_reference::<CountriesPageViewModel>();
    ValueTypes::register_reference::<MainWindowViewModel>();
    ValueTypes::register_reference::<PeopleXamlPageViewModel>();
    ValueTypes::register_reference::<TemplateColumnBugPageViewModel>();
    ValueTypes::register_reference::<WikipediaPageViewModel>();
    ValueTypes::register_reference::<DragDropPageViewModel>();
    ValueTypes::register_reference::<FilesPageViewModel>();
    ValueTypes::register_reference::<CoreDemoViewModel>();
    ValueTypes::register_reference::<DemoPerson>();
    ValueTypes::register_reference::<DemoTemplateItem>();
}

/// Makes the typed lists of the view models known to markup (`ferro_markup_list!`).
pub(crate) fn register_lists() {}
