//! The pages of the catalog (namespace `ControlCatalog.Pages`): one module
//! per upstream code-behind file, next to its document.

use crate::markup::XamlClass;
use ferroui_base::metadata::MarkupType;
use ferroui_base::TypeInfo;

mod accelerator_page;
mod acrylic_page;
mod adorner_layer_page;
mod bitmap_cache_page;
mod border_page;
mod button_spinner_page;
mod buttons_page;
mod canvas_page;
mod check_box_page;
mod combo_box_page;
mod command_bar_page;
mod container_query_page;
mod content_demo_page;
mod context_flyout_page;
mod context_menu_page;
mod cursor_page;
mod custom_drawing;
mod custom_drawing_example_control;
mod data_validation_page;
mod expander_page;
mod flyouts_page;
mod focus_page;
mod headered_content_page;
mod home_page;
mod image_page;
mod labels_page;
mod layout_transform_control_page;
mod list_box_page;
mod menu_page;
mod native_embed_page;
mod platform_info_page;
mod platform_settings_page;
mod pointer_canvas;
mod pointer_contacts_tab;
mod progress_bar_page;
mod radio_button_page;
mod relative_panel_page;
mod screen_page;
mod section_page;
mod settings_page;
mod slider_page;
mod split_view_page;
mod tab_control_page;
mod tab_strip_page;
mod text_block_page;
mod text_box_page;
mod theme_page;
mod toggle_switch_page;
mod tool_tip_page;
mod transitioning_content_control_page;
mod tree_view_page;
mod viewbox_page;
mod window_customizations_page;
mod wrap_panel_page;

#[path = "TextBox/mod.rs"]
mod text_box;
#[path = "CommandBar/mod.rs"]
mod command_bar;
#[path = "Gestures/mod.rs"]
mod gestures;
#[path = "ContentPage/mod.rs"]
mod content_page;
#[path = "NavigationPage/mod.rs"]
mod navigation_page;
#[path = "OpenGl/mod.rs"]
pub mod open_gl;

pub use accelerator_page::AcceleratorPage;
pub use acrylic_page::AcrylicPage;
pub use adorner_layer_page::AdornerLayerPage;
pub use bitmap_cache_page::BitmapCachePage;
pub use border_page::BorderPage;
pub use button_spinner_page::ButtonSpinnerPage;
pub use buttons_page::{ButtonsPage, RelayCommand};
pub use canvas_page::CanvasPage;
pub use check_box_page::CheckBoxPage;
pub use combo_box_page::ComboBoxPage;
pub use command_bar::{
    CommandBarCustomizationPage,
    CommandBarDynamicOverflowPage,
    CommandBarEventsPage,
    CommandBarFirstLookPage,
    CommandBarKeyboardPage,
    CommandBarLabelPositionPage,
    CommandBarOverflowPage,
    CommandBarTogglePage,
};
pub use command_bar_page::CommandBarPage;
pub use container_query_page::ContainerQueryPage;
pub use content_demo_page::ContentDemoPage;
pub use content_page::{
    ContentPageCommandBarPage,
    ContentPageCustomizationPage,
    ContentPageEventsPage,
    ContentPageFirstLookPage,
    ContentPageSafeAreaPage,
};
pub use context_flyout_page::ContextFlyoutPage;
pub use context_menu_page::ContextMenuPage;
pub use cursor_page::CursorPage;
pub use custom_drawing::CustomDrawing;
pub use custom_drawing_example_control::CustomDrawingExampleControl;
pub use data_validation_page::DataValidationPage;
pub use expander_page::ExpanderPage;
pub use flyouts_page::FlyoutsPage;
pub use focus_page::FocusPage;
pub use gestures::{GesturePinchRotationPage, GesturePinchZoomPage};
pub use headered_content_page::HeaderedContentPage;
pub use home_page::HomePage;
pub use image_page::ImagePage;
pub use labels_page::LabelsPage;
pub use layout_transform_control_page::LayoutTransformControlPage;
pub use list_box_page::ListBoxPage;
pub use menu_page::MenuPage;
pub use native_embed_page::{EmbedSample, INativeDemoControl, NativeEmbedPage};
pub use navigation_page::*;
pub use platform_info_page::PlatformInfoPage;
pub use platform_settings_page::PlatformSettingsPage;
pub use pointer_canvas::PointerCanvas;
pub use pointer_contacts_tab::PointerContactsTab;
pub use progress_bar_page::ProgressBarPage;
pub use radio_button_page::RadioButtonPage;
pub use relative_panel_page::RelativePanelPage;
pub use screen_page::ScreenPage;
pub use section_page::SectionPage;
pub use settings_page::SettingsPage;
pub use slider_page::SliderPage;
pub use split_view_page::SplitViewPage;
pub use tab_control_page::TabControlPage;
pub use tab_strip_page::TabStripPage;
pub use text_block_page::TextBlockPage;
pub use text_box::{
    TextBoxEditingPage,
    TextBoxFirstLookPage,
    TextBoxFontsPage,
    TextBoxInputPage,
    TextBoxMultilinePage,
    TextBoxPlaceholderPage,
    TextBoxSelectionPage,
    TextBoxTextLayoutPage,
    TextBoxValidationPage,
};
pub use text_box_page::TextBoxPage;
pub use theme_page::ThemePage;
pub use toggle_switch_page::ToggleSwitchPage;
pub use tool_tip_page::ToolTipPage;
pub use transitioning_content_control_page::TransitioningContentControlPage;
pub use tree_view_page::TreeViewPage;
pub use viewbox_page::ViewboxPage;
pub use window_customizations_page::WindowCustomizationsPage;
pub use wrap_panel_page::WrapPanelPage;

/// The classes of this namespace (`X::TYPE`), per directory.
pub(crate) const TYPES: &[&[&TypeInfo]] = &[ROOT_TYPES, open_gl::TYPES, text_box::TYPES, command_bar::TYPES, gestures::TYPES, content_page::TYPES, navigation_page::TYPES];

/// The classes of this namespace that have a document (`&X::XAML_CLASS`), per directory.
pub(crate) const CLASSES: &[&[&XamlClass]] = &[ROOT_CLASSES, open_gl::CLASSES, text_box::CLASSES, command_bar::CLASSES, gestures::CLASSES, content_page::CLASSES, navigation_page::CLASSES];

/// The types of this namespace declared with `ferro_markup_type!` / `ferro_markup_enum!`
/// (`<X as MarkupTyped>::MARKUP`), per directory.
pub(crate) const MARKUP_TYPES: &[&[&MarkupType]] = &[ROOT_MARKUP_TYPES, text_box::MARKUP_TYPES, command_bar::MARKUP_TYPES, gestures::MARKUP_TYPES, content_page::MARKUP_TYPES, navigation_page::MARKUP_TYPES];

/// What the untyped value conversions must know about the types of this namespace.
pub(crate) fn register_value_types() {}

/// The classes of the files directly under `Pages/`.
const ROOT_TYPES: &[&TypeInfo] = &[
    AcceleratorPage::TYPE,
    AcrylicPage::TYPE,
    AdornerLayerPage::TYPE,
    BitmapCachePage::TYPE,
    BorderPage::TYPE,
    ButtonSpinnerPage::TYPE,
    ButtonsPage::TYPE,
    CanvasPage::TYPE,
    CheckBoxPage::TYPE,
    ComboBoxPage::TYPE,
    CommandBarPage::TYPE,
    ContainerQueryPage::TYPE,
    ContentDemoPage::TYPE,
    ContextFlyoutPage::TYPE,
    ContextMenuPage::TYPE,
    CursorPage::TYPE,
    CustomDrawing::TYPE,
    CustomDrawingExampleControl::TYPE,
    DataValidationPage::TYPE,
    EmbedSample::TYPE,
    ExpanderPage::TYPE,
    FlyoutsPage::TYPE,
    FocusPage::TYPE,
    HeaderedContentPage::TYPE,
    HomePage::TYPE,
    ImagePage::TYPE,
    LabelsPage::TYPE,
    LayoutTransformControlPage::TYPE,
    ListBoxPage::TYPE,
    MenuPage::TYPE,
    NativeEmbedPage::TYPE,
    PlatformInfoPage::TYPE,
    PlatformSettingsPage::TYPE,
    PointerCanvas::TYPE,
    PointerContactsTab::TYPE,
    ProgressBarPage::TYPE,
    RadioButtonPage::TYPE,
    RelativePanelPage::TYPE,
    ScreenPage::TYPE,
    SectionPage::TYPE,
    SettingsPage::TYPE,
    SliderPage::TYPE,
    SplitViewPage::TYPE,
    TabControlPage::TYPE,
    TabStripPage::TYPE,
    TextBlockPage::TYPE,
    TextBoxPage::TYPE,
    ThemePage::TYPE,
    ToggleSwitchPage::TYPE,
    ToolTipPage::TYPE,
    TransitioningContentControlPage::TYPE,
    TreeViewPage::TYPE,
    ViewboxPage::TYPE,
    WindowCustomizationsPage::TYPE,
    WrapPanelPage::TYPE,
];

const ROOT_CLASSES: &[&XamlClass] = &[
    &AcceleratorPage::XAML_CLASS,
    &AcrylicPage::XAML_CLASS,
    &AdornerLayerPage::XAML_CLASS,
    &BitmapCachePage::XAML_CLASS,
    &BorderPage::XAML_CLASS,
    &ButtonSpinnerPage::XAML_CLASS,
    &ButtonsPage::XAML_CLASS,
    &CanvasPage::XAML_CLASS,
    &CheckBoxPage::XAML_CLASS,
    &ComboBoxPage::XAML_CLASS,
    &CommandBarPage::XAML_CLASS,
    &ContainerQueryPage::XAML_CLASS,
    &ContentDemoPage::XAML_CLASS,
    &ContextFlyoutPage::XAML_CLASS,
    &ContextMenuPage::XAML_CLASS,
    &CursorPage::XAML_CLASS,
    &CustomDrawing::XAML_CLASS,
    &DataValidationPage::XAML_CLASS,
    &ExpanderPage::XAML_CLASS,
    &FlyoutsPage::XAML_CLASS,
    &FocusPage::XAML_CLASS,
    &HeaderedContentPage::XAML_CLASS,
    &HomePage::XAML_CLASS,
    &ImagePage::XAML_CLASS,
    &LabelsPage::XAML_CLASS,
    &LayoutTransformControlPage::XAML_CLASS,
    &ListBoxPage::XAML_CLASS,
    &MenuPage::XAML_CLASS,
    &NativeEmbedPage::XAML_CLASS,
    &PlatformInfoPage::XAML_CLASS,
    &PlatformSettingsPage::XAML_CLASS,
    &ProgressBarPage::XAML_CLASS,
    &RadioButtonPage::XAML_CLASS,
    &RelativePanelPage::XAML_CLASS,
    &SectionPage::XAML_CLASS,
    &SettingsPage::XAML_CLASS,
    &SliderPage::XAML_CLASS,
    &SplitViewPage::XAML_CLASS,
    &TabControlPage::XAML_CLASS,
    &TabStripPage::XAML_CLASS,
    &TextBlockPage::XAML_CLASS,
    &TextBoxPage::XAML_CLASS,
    &ThemePage::XAML_CLASS,
    &ToggleSwitchPage::XAML_CLASS,
    &ToolTipPage::XAML_CLASS,
    &TransitioningContentControlPage::XAML_CLASS,
    &TreeViewPage::XAML_CLASS,
    &ViewboxPage::XAML_CLASS,
    &WindowCustomizationsPage::XAML_CLASS,
    &WrapPanelPage::XAML_CLASS,
];

const ROOT_MARKUP_TYPES: &[&MarkupType] = &[];
