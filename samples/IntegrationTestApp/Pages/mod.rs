//! The pages of the sample (namespace `IntegrationTestApp.Pages`): one module per upstream file.
//!
//! The class of `Pages/EmbeddingPage.xaml` is a class of the root namespace in the upstream
//! sample: its module (`embedding_page.rs` of this directory) is declared by the root of the
//! crate.

use crate::markup::XamlClass;
use ferroui_base::metadata::MarkupType;
use ferroui_base::TypeInfo;

mod automation_page;
mod button_page;
mod check_box_page;
mod combo_box_page;
mod context_menu_page;
mod desktop_page;
mod drag_drop_page;
mod gestures_page;
mod keyboard_page;
mod list_box_page;
mod menu_page;
mod pointer_page;
mod popups_page;
mod radio_button_page;
mod screens_page;
mod scroll_bar_page;
mod slider_page;
mod window_decorations_page;
mod window_page;

pub use automation_page::AutomationPage;
pub use button_page::ButtonPage;
pub use check_box_page::CheckBoxPage;
pub use combo_box_page::ComboBoxPage;
pub use context_menu_page::ContextMenuPage;
pub use desktop_page::DesktopPage;
pub use drag_drop_page::DragDropPage;
pub use gestures_page::GesturesPage;
pub use keyboard_page::KeyboardPage;
pub use list_box_page::{ListBoxItemList, ListBoxPage};
pub use menu_page::MenuPage;
pub use pointer_page::PointerPage;
pub use popups_page::PopupsPage;
pub use radio_button_page::RadioButtonPage;
pub use screens_page::ScreensPage;
pub use scroll_bar_page::ScrollBarPage;
pub use slider_page::SliderPage;
pub use window_decorations_page::WindowDecorationsPage;
pub use window_page::WindowPage;

/// The classes of this namespace (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[
    AutomationPage::TYPE,
    ButtonPage::TYPE,
    CheckBoxPage::TYPE,
    ComboBoxPage::TYPE,
    ContextMenuPage::TYPE,
    DesktopPage::TYPE,
    DragDropPage::TYPE,
    GesturesPage::TYPE,
    KeyboardPage::TYPE,
    ListBoxPage::TYPE,
    MenuPage::TYPE,
    PointerPage::TYPE,
    PopupsPage::TYPE,
    RadioButtonPage::TYPE,
    ScreensPage::TYPE,
    ScrollBarPage::TYPE,
    SliderPage::TYPE,
    WindowDecorationsPage::TYPE,
    WindowPage::TYPE,
];

/// The classes of this namespace that have a document (`&X::XAML_CLASS`).
pub(crate) const CLASSES: &[&XamlClass] = &[
    &AutomationPage::XAML_CLASS,
    &ButtonPage::XAML_CLASS,
    &CheckBoxPage::XAML_CLASS,
    &ComboBoxPage::XAML_CLASS,
    &ContextMenuPage::XAML_CLASS,
    &DesktopPage::XAML_CLASS,
    &DragDropPage::XAML_CLASS,
    &GesturesPage::XAML_CLASS,
    &KeyboardPage::XAML_CLASS,
    &ListBoxPage::XAML_CLASS,
    &MenuPage::XAML_CLASS,
    &PointerPage::XAML_CLASS,
    &PopupsPage::XAML_CLASS,
    &RadioButtonPage::XAML_CLASS,
    &ScreensPage::XAML_CLASS,
    &ScrollBarPage::XAML_CLASS,
    &SliderPage::XAML_CLASS,
    &WindowDecorationsPage::XAML_CLASS,
    &WindowPage::XAML_CLASS,
];

/// The types of this namespace declared with `ferro_markup_type!` / `ferro_markup_enum!`
/// (`<X as MarkupTyped>::MARKUP`).
pub(crate) const MARKUP_TYPES: &[&MarkupType] = &[];

/// What the untyped value conversions must know about the types of this namespace
/// (`ValueTypes::register_reference::<X>()`, nullable forms, casts to contracts).
pub(crate) fn register_value_types() {}

/// Makes the typed lists of this namespace known to markup (`ferro_markup_list!`).
pub(crate) fn register_lists() {
    ListBoxItemList::register();
}
