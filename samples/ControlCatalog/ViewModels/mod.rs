//! The view models of the catalog (namespace `ControlCatalog.ViewModels`): one module per upstream file.

use crate::markup::XamlClass;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::metadata::{MarkupType, MarkupTyped};
use ferroui_base::TypeInfo;

mod application_view_model;
mod combo_box_page_view_model;
mod context_page_view_model;
mod cursor_page_view_model;
mod data_validation_view_model;
mod expander_page_view_model;
mod flex_item_view_model;
mod flex_view_model;
mod list_box_page_view_model;
mod main_window_view_model;
mod main_window_view_model_page_list;
mod menu_item_view_model;
mod menu_page_view_model;
mod notification_view_model;
mod platform_information_view_model;
mod platform_settings_view_model;
pub(crate) mod random;
mod refresh_container_view_model;
mod section_view_model;
mod settings_view_model;
mod split_view_page_view_model;
mod tab_control_page_view_model;
mod table_view_page_view_model;
mod transitioning_content_control_page_view_model;
mod tree_view_page_view_model;
mod wrap_panel_page_view_model;

pub use application_view_model::ApplicationViewModel;
pub use combo_box_page_view_model::{ComboBoxPageViewModel, IdAndName};
pub use context_page_view_model::ContextPageViewModel;
pub use cursor_page_view_model::{CursorPageViewModel, StandardCursorModel};
pub use data_validation_view_model::DataValidationViewModel;
pub use expander_page_view_model::ExpanderPageViewModel;
pub use flex_item_view_model::FlexItemViewModel;
pub use flex_view_model::FlexViewModel;
pub use list_box_page_view_model::{ItemModel, ListBoxPageViewModel};
pub use main_window_view_model::MainWindowViewModel;
pub use main_window_view_model_page_list::UnavailablePage;
pub use menu_item_view_model::MenuItemViewModel;
pub use menu_page_view_model::MenuPageViewModel;
pub use notification_view_model::NotificationViewModel;
pub use platform_information_view_model::PlatformInformationViewModel;
pub use platform_settings_view_model::PlatformSettingsViewModel;
pub use refresh_container_view_model::RefreshContainerViewModel;
pub use section_view_model::SectionViewModel;
pub use settings_view_model::SettingsViewModel;
pub use split_view_page_view_model::SplitViewPageViewModel;
pub use tab_control_page_view_model::{TabControlPageViewModel, TabControlPageViewModelItem};
pub use table_view_page_view_model::{Country, TableViewPageViewModel};
pub use transitioning_content_control_page_view_model::{
    CustomTransition, PageTransition, TransitioningContentControlPageViewModel,
};
pub use tree_view_page_view_model::{Node, TreeViewPageViewModel};
pub use wrap_panel_page_view_model::{WrapPanelItemViewModel, WrapPanelPageViewModel};

/// The classes of this namespace (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[];

/// The classes of this namespace that have a document (`&X::XAML_CLASS`).
pub(crate) const CLASSES: &[&XamlClass] = &[];

/// The types of this namespace declared with `ferro_markup_type!` / `ferro_markup_enum!`
/// (`<X as MarkupTyped>::MARKUP`).
pub(crate) const MARKUP_TYPES: &[&MarkupType] = &[
    <NotificationViewModel as MarkupTyped>::MARKUP,
    <FlexViewModel as MarkupTyped>::MARKUP,
    <FlexItemViewModel as MarkupTyped>::MARKUP,
    <ApplicationViewModel as MarkupTyped>::MARKUP,
    <ComboBoxPageViewModel as MarkupTyped>::MARKUP,
    <ContextPageViewModel as MarkupTyped>::MARKUP,
    <Country as MarkupTyped>::MARKUP,
    <CursorPageViewModel as MarkupTyped>::MARKUP,
    <DataValidationViewModel as MarkupTyped>::MARKUP,
    <ExpanderPageViewModel as MarkupTyped>::MARKUP,
    <IdAndName as MarkupTyped>::MARKUP,
    <ItemModel as MarkupTyped>::MARKUP,
    <ListBoxPageViewModel as MarkupTyped>::MARKUP,
    <MainWindowViewModel as MarkupTyped>::MARKUP,
    <MenuItemViewModel as MarkupTyped>::MARKUP,
    <MenuPageViewModel as MarkupTyped>::MARKUP,
    <PlatformInformationViewModel as MarkupTyped>::MARKUP,
    <PlatformSettingsViewModel as MarkupTyped>::MARKUP,
    <RefreshContainerViewModel as MarkupTyped>::MARKUP,
    <SectionViewModel as MarkupTyped>::MARKUP,
    <SettingsViewModel as MarkupTyped>::MARKUP,
    <SplitViewPageViewModel as MarkupTyped>::MARKUP,
    <StandardCursorModel as MarkupTyped>::MARKUP,
    <TabControlPageViewModel as MarkupTyped>::MARKUP,
    <TabControlPageViewModelItem as MarkupTyped>::MARKUP,
    <TableViewPageViewModel as MarkupTyped>::MARKUP,
    <CustomTransition as MarkupTyped>::MARKUP,
    <Node as MarkupTyped>::MARKUP,
    <PageTransition as MarkupTyped>::MARKUP,
    <TransitioningContentControlPageViewModel as MarkupTyped>::MARKUP,
    <TreeViewPageViewModel as MarkupTyped>::MARKUP,
    <WrapPanelItemViewModel as MarkupTyped>::MARKUP,
    <WrapPanelPageViewModel as MarkupTyped>::MARKUP,
];

/// What the untyped value conversions must know about the types of this namespace
/// (`ValueTypes::register_reference::<X>()`, nullable forms, casts to contracts).
pub(crate) fn register_value_types() {
    ValueTypes::register_reference::<NotificationViewModel>();
    ValueTypes::register_reference::<FlexViewModel>();
    ValueTypes::register_reference::<FlexItemViewModel>();
    ValueTypes::register_reference::<ApplicationViewModel>();
    ValueTypes::register_reference::<ComboBoxPageViewModel>();
    ValueTypes::register_reference::<ContextPageViewModel>();
    ValueTypes::register_reference::<Country>();
    ValueTypes::register_reference::<CursorPageViewModel>();
    ValueTypes::register_reference::<DataValidationViewModel>();
    ValueTypes::register_reference::<ExpanderPageViewModel>();
    ValueTypes::register_reference::<IdAndName>();
    ValueTypes::register_reference::<ItemModel>();
    ValueTypes::register_reference::<ListBoxPageViewModel>();
    ValueTypes::register_reference::<MainWindowViewModel>();
    ValueTypes::register_reference::<MenuItemViewModel>();
    ValueTypes::register_reference::<MenuPageViewModel>();
    ValueTypes::register_reference::<PlatformInformationViewModel>();
    ValueTypes::register_reference::<PlatformSettingsViewModel>();
    ValueTypes::register_reference::<RefreshContainerViewModel>();
    ValueTypes::register_reference::<SectionViewModel>();
    ValueTypes::register_reference::<SettingsViewModel>();
    ValueTypes::register_reference::<SplitViewPageViewModel>();
    ValueTypes::register_reference::<StandardCursorModel>();
    ValueTypes::register_reference::<TabControlPageViewModel>();
    ValueTypes::register_reference::<TabControlPageViewModelItem>();
    ValueTypes::register_reference::<TableViewPageViewModel>();
    ValueTypes::register_reference::<CustomTransition>();
    ValueTypes::register_reference::<Node>();
    ValueTypes::register_reference::<PageTransition>();
    ValueTypes::register_reference::<TransitioningContentControlPageViewModel>();
    ValueTypes::register_reference::<TreeViewPageViewModel>();
    ValueTypes::register_reference::<WrapPanelItemViewModel>();
    ValueTypes::register_reference::<WrapPanelPageViewModel>();

    // `ToString()` of the items that are shown as text.
    ValueTypes::register_display::<ItemModel>();
    ValueTypes::register_display::<Node>();
    ValueTypes::register_display::<PageTransition>();
}

/// `System.Diagnostics.Debug.WriteLine(message)`: a line of the debug
/// output, in builds with debug assertions only.
pub(crate) fn debug_write_line(message: &str) {
    if cfg!(debug_assertions) {
        eprintln!("{message}");
    }
}

/// Makes the typed lists of the view models known to markup (`ferro_markup_list!`).
pub(crate) fn register_lists() {
    cursor_page_view_model::StandardCursorList::register();
    tree_view_page_view_model::NodeList::register();
    wrap_panel_page_view_model::WrapPanelItemList::register();
}
