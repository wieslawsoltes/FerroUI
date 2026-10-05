//! The type table of this crate: its namespaces, its classes and what it
//! states about itself for markup.
//!
//! Types become known by name when they are initialised on a thread; this
//! list makes the ones that were never used known too (markup can name any
//! of them). Registration is explicit: an application (or the markup
//! loader) calls the `register_types()` of each crate it uses.
//!
//! Keep the list complete: every `ferro_class!` and `ferro_static_type!` of
//! the crate has an entry, under the namespace of the upstream type.

use ferroui_base::metadata::{MarkupAssembly, XmlnsDefinition, FERRO_XML_NAMESPACE};
use ferroui_base::{StaticType, TypeInfo};

/// The dotted namespaces of the modules of this crate. A type belongs to the
/// namespace of the longest module path that is a prefix of the path of its
/// declaring module.
const NAMESPACES: &[(&str, &str)] = &[
    ("ferroui_controls", "FerroUI.Controls"),
    ("ferroui_controls::animation", "FerroUI.Animation"),
    ("ferroui_controls::app_builder", "FerroUI"),
    ("ferroui_controls::application", "FerroUI"),
    ("ferroui_controls::application_lifetimes", "FerroUI.Controls.ApplicationLifetimes"),
    ("ferroui_controls::automation", "FerroUI.Automation"),
    ("ferroui_controls::automation::peers", "FerroUI.Automation.Peers"),
    ("ferroui_controls::automation::peers::embeddable_control_root_automation_peer", "FerroUI.Controls.Automation.Peers"),
    ("ferroui_controls::automation::peers::expander_automation_peer", "FerroUI.Controls.Automation.Peers"),
    ("ferroui_controls::automation::peers::image_automation_peer", "FerroUI.Controls.Automation.Peers"),
    ("ferroui_controls::automation::peers::interop_automation_peer", "FerroUI.Controls.Automation.Peers"),
    ("ferroui_controls::automation::peers::label_automation_peer", "FerroUI.Controls.Automation.Peers"),
    ("ferroui_controls::automation::peers::native_control_host_peer", "FerroUI.Controls.Automation.Peers"),
    ("ferroui_controls::automation::peers::progress_bar_automation_peer", "FerroUI.Controls.Automation.Peers"),
    ("ferroui_controls::automation::peers::radio_button_automation_peer", "FerroUI.Controls.Automation.Peers"),
    ("ferroui_controls::automation::peers::slider_automation_peer", "FerroUI.Controls.Automation.Peers"),
    ("ferroui_controls::automation::peers::thumb_automation_peer", "FerroUI.Controls.Automation.Peers"),
    ("ferroui_controls::automation::peers::user_control_automation_peer", "FerroUI.Controls.Automation.Peers"),
    ("ferroui_controls::automation::provider", "FerroUI.Automation.Provider"),
    ("ferroui_controls::chrome", "FerroUI.Controls.Chrome"),
    ("ferroui_controls::converters", "FerroUI.Controls.Converters"),
    ("ferroui_controls::diagnostics", "FerroUI.Diagnostics"),
    ("ferroui_controls::documents", "FerroUI.Controls.Documents"),
    ("ferroui_controls::embedding", "FerroUI.Controls.Embedding"),
    ("ferroui_controls::flyouts::flyout_base", "FerroUI.Controls.Primitives"),
    ("ferroui_controls::flyouts::popup_flyout_base", "FerroUI.Controls.Primitives"),
    ("ferroui_controls::metadata", "FerroUI.Controls.Metadata"),
    ("ferroui_controls::mixins", "FerroUI.Controls.Mixins"),
    ("ferroui_controls::notifications", "FerroUI.Controls.Notifications"),
    ("ferroui_controls::numeric_up_down", "FerroUI.Controls"),
    ("ferroui_controls::calendar", "FerroUI.Controls"),
    ("ferroui_controls::calendar::calendar_blackout_dates_collection", "FerroUI.Controls.Primitives"),
    ("ferroui_controls::calendar::calendar_button", "FerroUI.Controls.Primitives"),
    ("ferroui_controls::calendar::calendar_day_button", "FerroUI.Controls.Primitives"),
    ("ferroui_controls::calendar::calendar_extensions", "FerroUI.Controls.Primitives"),
    ("ferroui_controls::calendar::calendar_item", "FerroUI.Controls.Primitives"),
    ("ferroui_controls::calendar::selected_dates_collection", "FerroUI.Controls.Primitives"),
    ("ferroui_controls::calendar_date_picker", "FerroUI.Controls"),
    ("ferroui_controls::date_time_pickers", "FerroUI.Controls"),
    ("ferroui_controls::date_time_pickers::date_time_picker_panel", "FerroUI.Controls.Primitives"),
    ("ferroui_controls::date_time_pickers::picker_presenter_base", "FerroUI.Controls.Primitives"),
    ("ferroui_controls::page", "FerroUI.Controls"),
    ("ferroui_controls::pull_to_refresh", "FerroUI.Controls"),
    ("ferroui_controls::pull_to_refresh::refresh_info_provider", "FerroUI.Controls.PullToRefresh"),
    ("ferroui_controls::pull_to_refresh::scrollable_pull_gesture_recognizer", "FerroUI.Controls.PullToRefresh"),
    ("ferroui_controls::pips_pager", "FerroUI.Controls"),
    ("ferroui_controls::pips_pager::pips_pager_template_settings", "FerroUI.Controls.Primitives"),
    ("ferroui_controls::platform", "FerroUI.Controls.Platform"),
    ("ferroui_controls::platform::mac_os_properties", "FerroUI.Controls"),
    ("ferroui_controls::platform::x11_properties", "FerroUI.Controls"),
    ("ferroui_controls::platform::in_process_drag_source", "FerroUI.Platform"),
    ("ferroui_controls::embedding::offscreen", "FerroUI.Controls.Embedding.Offscreen"),
    ("ferroui_controls::url_opened_event_args", "FerroUI"),
    ("ferroui_controls::system_font_app_builder_extension", "FerroUI"),
    ("ferroui_controls::platform::platform_feedback", "FerroUI.Controls"),
    ("ferroui_controls::presenters", "FerroUI.Controls.Presenters"),
    ("ferroui_controls::primitives", "FerroUI.Controls.Primitives"),
    ("ferroui_controls::shapes", "FerroUI.Controls.Shapes"),
    ("ferroui_controls::split_view::split_view_template_settings", "FerroUI.Controls.Primitives"),
    ("ferroui_controls::templates", "FerroUI.Controls.Templates"),
    ("ferroui_controls::utils", "FerroUI.Controls.Utils"),
];

/// What this crate states about itself for markup: its assembly name and
/// the namespaces the XML namespace of the framework maps to.
pub static ASSEMBLY: MarkupAssembly = MarkupAssembly {
    name: "FerroUI.Controls",
    crate_name: "ferroui_controls",
    xmlns_definitions: &[
        XmlnsDefinition { xml_namespace: FERRO_XML_NAMESPACE, namespace: "FerroUI" },
        XmlnsDefinition { xml_namespace: FERRO_XML_NAMESPACE, namespace: "FerroUI.Automation" },
        XmlnsDefinition { xml_namespace: FERRO_XML_NAMESPACE, namespace: "FerroUI.Controls" },
        XmlnsDefinition { xml_namespace: FERRO_XML_NAMESPACE, namespace: "FerroUI.Controls.Embedding" },
        XmlnsDefinition { xml_namespace: FERRO_XML_NAMESPACE, namespace: "FerroUI.Controls.Presenters" },
        XmlnsDefinition { xml_namespace: FERRO_XML_NAMESPACE, namespace: "FerroUI.Controls.Primitives" },
        XmlnsDefinition { xml_namespace: FERRO_XML_NAMESPACE, namespace: "FerroUI.Controls.Shapes" },
        XmlnsDefinition { xml_namespace: FERRO_XML_NAMESPACE, namespace: "FerroUI.Controls.Templates" },
        XmlnsDefinition { xml_namespace: FERRO_XML_NAMESPACE, namespace: "FerroUI.Controls.Notifications" },
        XmlnsDefinition { xml_namespace: FERRO_XML_NAMESPACE, namespace: "FerroUI.Controls.Chrome" },
        XmlnsDefinition { xml_namespace: FERRO_XML_NAMESPACE, namespace: "FerroUI.Controls.Documents" },
    ],
    xmlns_prefixes: &[],
    metadata: &[],
};

macro_rules! types {
    ($($type_:ty),* $(,)?) => {
        &[$(<$type_ as StaticType>::TYPE),*]
    };
}

/// Registers the namespaces and the types of this crate (and of the crates
/// it is built on) with the table of known types ([`TypeInfo::find`]).
/// Cheap and idempotent; nothing is initialised until a type is used or
/// looked into.
pub fn register_types() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        ferroui_base::register_types();
        TypeInfo::register_namespaces(NAMESPACES);
        TypeInfo::register_all(TYPES);
        TypeInfo::register_rust_paths(crate::rust_paths::CLASS_RUST_PATHS);
        ferroui_base::metadata::register_type_rust_paths(crate::rust_paths::TYPE_RUST_PATHS);
        ferroui_base::metadata::MarkupType::register_rust_paths(crate::rust_paths::MARKUP_RUST_PATHS);
        MarkupAssembly::register(&ASSEMBLY);
        crate::markup_types::register();
    });
}

const TYPES: &[&TypeInfo] = types![
    // FerroUI
    crate::application::Application,
    // FerroUI.Animation
    crate::animation::ConnectedAnimationProxy,
    crate::animation::ConnectedAnimationService,
    // FerroUI.Automation
    crate::automation::AutomationProperties,
    // FerroUI.Automation.Peers
    crate::automation::peers::AutoCompleteBoxAutomationPeer,
    crate::automation::peers::AutomationPeer,
    crate::automation::peers::ButtonAutomationPeer,
    crate::automation::peers::CalendarAutomationPeer,
    crate::automation::peers::CalendarDatePickerAutomationPeer,
    crate::automation::peers::CalendarDayButtonAutomationPeer,
    crate::automation::peers::CarouselPageAutomationPeer,
    crate::automation::peers::ComboBoxAutomationPeer,
    crate::automation::peers::ContentControlAutomationPeer,
    crate::automation::peers::ContentPageAutomationPeer,
    crate::automation::peers::ControlAutomationPeer,
    crate::automation::peers::DatePickerAutomationPeer,
    crate::automation::peers::DrawerPageAutomationPeer,
    crate::automation::peers::ItemsControlAutomationPeer,
    crate::automation::peers::ListBoxAutomationPeer,
    crate::automation::peers::ListItemAutomationPeer,
    crate::automation::peers::MenuItemAutomationPeer,
    crate::automation::peers::NativeMenuBarAutomationPeer,
    crate::automation::peers::NavigationPageAutomationPeer,
    crate::automation::peers::NoneAutomationPeer,
    crate::automation::peers::NumericUpDownAutomationPeer,
    crate::automation::peers::PipsPagerAutomationPeer,
    crate::automation::peers::PopupAutomationPeer,
    crate::automation::peers::PopupRootAutomationPeer,
    crate::automation::peers::RangeBaseAutomationPeer,
    crate::automation::peers::ScrollBarAutomationPeer,
    crate::automation::peers::ScrollViewerAutomationPeer,
    crate::automation::peers::SelectingItemsControlAutomationPeer,
    crate::automation::peers::SplitButtonAutomationPeer,
    crate::automation::peers::TabbedPageAutomationPeer,
    crate::automation::peers::TextBlockAutomationPeer,
    crate::automation::peers::TextBoxAutomationPeer,
    crate::automation::peers::TimePickerAutomationPeer,
    crate::automation::peers::ToggleButtonAutomationPeer,
    crate::automation::peers::ToggleSplitButtonAutomationPeer,
    crate::automation::peers::ToolTipAutomationPeer,
    crate::automation::peers::TreeViewAutomationPeer,
    crate::automation::peers::TreeViewItemAutomationPeer,
    crate::automation::peers::UnrealizedElementAutomationPeer,
    crate::automation::peers::WindowAutomationPeer,
    crate::automation::peers::WindowBaseAutomationPeer,
    // FerroUI.Controls.Automation.Peers
    crate::automation::peers::EmbeddableControlRootAutomationPeer,
    crate::automation::peers::ExpanderAutomationPeer,
    crate::automation::peers::ImageAutomationPeer,
    crate::automation::peers::InteropAutomationPeer,
    crate::automation::peers::LabelAutomationPeer,
    crate::automation::peers::NativeControlHostPeer,
    crate::automation::peers::ProgressBarAutomationPeer,
    crate::automation::peers::RadioButtonAutomationPeer,
    crate::automation::peers::SliderAutomationPeer,
    crate::automation::peers::ThumbAutomationPeer,
    crate::automation::peers::UserControlAutomationPeer,
    // FerroUI.Controls
    crate::border::Border,
    crate::button::Button,
    crate::button_spinner::ButtonSpinner,
    crate::canvas::Canvas,
    crate::carousel::Carousel,
    crate::check_box::CheckBox,
    crate::column_definition::ColumnDefinition,
    crate::combo_box::ComboBox,
    crate::combo_box_item::ComboBoxItem,
    crate::content_control::ContentControl,
    crate::context_menu::ContextMenu,
    crate::control::Control,
    crate::data_validation_errors::DataValidationErrors,
    crate::decorator::Decorator,
    crate::definition_base::DefinitionBase,
    crate::design::Design,
    crate::dock_panel::DockPanel,
    crate::drop_down_button::DropDownButton,
    crate::expander::Expander,
    crate::experimental_acrylic_border::ExperimentalAcrylicBorder,
    crate::flyouts::Flyout,
    crate::flyouts::FlyoutPresenter,
    crate::flyouts::MenuFlyout,
    crate::flyouts::MenuFlyoutPresenter,
    crate::grid::Grid,
    crate::grid::GridLinesRenderer,
    crate::grid_splitter::GridSplitter,
    crate::group_box::GroupBox,
    crate::hotkey_manager::HotKeyManager,
    crate::hyperlink_button::HyperlinkButton,
    crate::icon_element::IconElement,
    crate::image::Image,
    crate::items_control::ItemsControl,
    crate::label::Label,
    crate::top_level_host_decorations::LayerWrapper,
    crate::layout_transform_control::LayoutTransformControl,
    crate::list_box::ListBox,
    crate::list_box_item::ListBoxItem,
    crate::platform::MacOSProperties,
    crate::masked_text_box::MaskedTextBox,
    crate::menu::Menu,
    crate::menu_base::MenuBase,
    crate::menu_item::MenuItem,
    crate::native_dock::NativeDock,
    crate::native_menu::NativeMenu,
    crate::native_menu_bar::NativeMenuBar,
    crate::native_menu_bar_presenter::NativeMenuBarPresenter,
    crate::native_menu_item::NativeMenuItem,
    crate::native_menu_item_base::NativeMenuItemBase,
    crate::native_menu_item_separator::NativeMenuItemSeparator,
    crate::panel::Panel,
    crate::path_icon::PathIcon,
    crate::platform::PlatformFeedback,
    crate::grid_splitter::PreviewAdorner,
    crate::progress_bar::ProgressBar,
    crate::progress_bar::ProgressBarTemplateSettings,
    crate::radio_button::RadioButton,
    crate::relative_panel::RelativePanel,
    crate::repeat_button::RepeatButton,
    crate::row_definition::RowDefinition,
    crate::scroll_viewer::ScrollViewer,
    crate::selectable_text_block::SelectableTextBlock,
    crate::separator::Separator,
    crate::slider::Slider,
    crate::spinner::Spinner,
    crate::split_button::SplitButton,
    crate::stack_panel::StackPanel,
    crate::tab_control::TabControl,
    crate::tab_item::TabItem,
    crate::text_block::TextBlock,
    crate::text_box::TextBox,
    crate::tick_bar::TickBar,
    crate::split_button::ToggleSplitButton,
    crate::tool_tip::ToolTip,
    crate::top_level::TopLevel,
    crate::top_level_host::TopLevelHost,
    crate::tray_icon::TrayIcon,
    crate::tree_view::TreeView,
    crate::tree_view_item::TreeViewItem,
    crate::user_control::UserControl,
    crate::viewbox::Viewbox,
    crate::viewbox::ViewboxContainer,
    crate::virtualizing_carousel_panel::VirtualizingCarouselPanel,
    crate::virtualizing_panel::VirtualizingPanel,
    crate::virtualizing_stack_panel::VirtualizingStackPanel,
    crate::window::Window,
    crate::window_base::WindowBase,
    crate::wrap_panel::WrapPanel,
    // FerroUI.Controls.Chrome
    crate::chrome::ResizeGripLayer,
    crate::chrome::WindowDecorationProperties,
    crate::chrome::WindowDrawnDecorations,
    crate::chrome::WindowDrawnDecorationsContent,
    // FerroUI.Controls.Documents
    crate::documents::Bold,
    crate::documents::Inline,
    crate::documents::InlineUIContainer,
    crate::documents::Italic,
    crate::documents::LineBreak,
    crate::documents::Run,
    crate::documents::Span,
    crate::documents::TextElement,
    crate::documents::Underline,
    // FerroUI.Controls.Embedding
    crate::embedding::EmbeddableControlRoot,
    // FerroUI.Controls.Notifications
    crate::notifications::ReversibleStackPanel,
    // FerroUI.Controls.Presenters
    crate::presenters::ContentPresenter,
    crate::presenters::ItemsPresenter,
    crate::presenters::panel_container_generator::PanelContainerGenerator,
    crate::presenters::ScrollContentPresenter,
    crate::presenters::TextPresenter,
    // FerroUI.Controls.Primitives
    crate::primitives::AccessText,
    crate::primitives::AdornerLayer,
    crate::flyouts::FlyoutBase,
    crate::primitives::HeaderedContentControl,
    crate::primitives::HeaderedItemsControl,
    crate::primitives::HeaderedSelectingItemsControl,
    crate::primitives::LightDismissOverlayLayer,
    crate::primitives::OverlayLayer,
    crate::primitives::OverlayPopupHost,
    crate::primitives::Popup,
    crate::flyouts::PopupFlyoutBase,
    crate::primitives::PopupOverlayLayer,
    crate::primitives::PopupRoot,
    crate::primitives::RangeBase,
    crate::primitives::ScrollBar,
    crate::primitives::SelectingItemsControl,
    crate::primitives::TabStrip,
    crate::primitives::TabStripItem,
    crate::primitives::TemplatedControl,
    crate::primitives::TextSearch,
    crate::primitives::TextSelectorLayer,
    crate::primitives::Thumb,
    crate::primitives::ToggleButton,
    crate::primitives::Track,
    crate::primitives::UniformGrid,
    crate::primitives::VisualLayerManager,
    // FerroUI.Controls.Shapes
    crate::shapes::Arc,
    crate::shapes::Ellipse,
    crate::shapes::Line,
    crate::shapes::Path,
    crate::shapes::Polygon,
    crate::shapes::Polyline,
    crate::shapes::Rectangle,
    crate::shapes::Sector,
    crate::shapes::Shape,
    // FerroUI.Controls.Utils
    crate::utils::BindingEvaluator,
    // --- numeric ---
    crate::numeric_up_down::NumericUpDown,
    // --- splitview ---
    crate::split_view::SplitView,
    crate::split_view::SplitViewTemplateSettings,
    // --- misc-controls ---
    crate::input_pane_aware_decorator::InputPaneAwareDecorator,
    crate::theme_variant_scope::ThemeVariantScope,
    crate::toggle_switch::ToggleSwitch,
    crate::transitioning_content_control::TransitioningContentControl,
    // --- commandbar ---
    crate::command_bar::CommandBar,
    crate::command_bar::CommandBarButton,
    crate::command_bar::CommandBarSeparator,
    crate::command_bar::CommandBarToggleButton,
    // --- autocomplete ---
    crate::auto_complete_box::AutoCompleteBox,
    // --- storage-misc ---
    crate::embedding::offscreen::OffscreenTopLevel,
    crate::native_control_host::NativeControlHost,
    crate::platform::X11Properties,
    // --- flexpanel ---
    crate::flex_panel::Flex,
    crate::flex_panel::FlexPanel,
    // --- notifications ---
    crate::notifications::NotificationCard,
    crate::notifications::WindowNotificationManager,
    // --- pulltorefresh ---
    crate::pull_to_refresh::RefreshContainer,
    crate::pull_to_refresh::RefreshVisualizer,
    // FerroUI.Controls.PullToRefresh
    crate::pull_to_refresh::RefreshInfoProvider,
    crate::pull_to_refresh::ScrollablePullGestureRecognizer,
    // --- calendar ---
    crate::calendar::Calendar,
    crate::calendar::CalendarButton,
    crate::calendar::CalendarDayButton,
    crate::calendar::CalendarItem,
    crate::calendar_date_picker::CalendarDatePicker,
    // --- datetimepickers ---
    crate::date_time_pickers::DatePicker,
    crate::date_time_pickers::DatePickerPresenter,
    crate::date_time_pickers::TimePicker,
    crate::date_time_pickers::TimePickerPresenter,
    // FerroUI.Controls.Primitives
    crate::date_time_pickers::DateTimePickerPanel,
    crate::date_time_pickers::PickerPresenterBase,
    // --- page-core ---
    crate::page::ContentPage,
    crate::page::NavigationPage,
    crate::page::Page,
    crate::page::PageNavigationHost,
    // --- page-multi ---
    crate::page::CarouselPage,
    crate::page::MultiPage,
    crate::page::SelectingMultiPage,
    crate::page::TabbedPage,
    // --- page-drawer ---
    crate::page::DrawerPage,
    // --- pipspager ---
    crate::pips_pager::PipsPager,
    // FerroUI.Controls.Primitives
    crate::pips_pager::PipsPagerTemplateSettings,
    // --- connected-animation ---
    // --- tableview ---
    crate::TableView,
    crate::TableViewCell,
    crate::TableViewColumn,
    crate::TableViewColumnHeader,
    crate::TableViewRow,
    // FerroUI.Controls.Presenters
    crate::presenters::TableViewCellsPresenter,
    crate::presenters::TableViewColumnHeadersPresenter,
    // --- textselection ---
    // FerroUI.Controls.Primitives
    crate::primitives::TextSelectionHandle,
    crate::primitives::TextSelectionHandleCanvas,
    // --- leftovers ---
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_types_of_this_crate_are_found_by_namespace_and_name() {
        register_types();
        register_types();

        let find = |namespace, name| TypeInfo::find(namespace, name).unwrap_or_else(|| panic!("{namespace}.{name}"));
        assert!(std::ptr::eq(find("FerroUI.Controls", "Border"), crate::Border::TYPE));
        assert!(std::ptr::eq(find("FerroUI.Controls", "Control"), crate::Control::TYPE));
        assert!(std::ptr::eq(find("FerroUI.Controls.Shapes", "Rectangle"), crate::shapes::Rectangle::TYPE));
        assert!(std::ptr::eq(find("FerroUI.Controls.Primitives", "TemplatedControl"), crate::primitives::TemplatedControl::TYPE));
        assert_eq!(crate::Border::TYPE.full_name(), "FerroUI.Controls.Border");
        // A file whose namespace differs from the one of its directory.
        assert_eq!(crate::flyouts::FlyoutBase::TYPE.full_name(), "FerroUI.Controls.Primitives.FlyoutBase");
        assert_eq!(crate::flyouts::Flyout::TYPE.full_name(), "FerroUI.Controls.Flyout");
        assert_eq!(crate::platform::PlatformFeedback::TYPE.full_name(), "FerroUI.Controls.PlatformFeedback");
        // Static types are found by name, and so are their properties, before any use.
        assert!(std::ptr::eq(find("FerroUI.Controls", "HotKeyManager"), crate::HotKeyManager::TYPE));
        assert!(std::ptr::eq(find("FerroUI.Controls", "ToolTip"), crate::tool_tip::ToolTip::TYPE));
        // The types of the crates this crate is built on are registered too.
        assert!(TypeInfo::find("FerroUI", "Visual").is_some());

        let assembly = MarkupAssembly::find("FerroUI.Controls").unwrap();
        assert!(std::ptr::eq(MarkupAssembly::of_module(crate::Border::TYPE.module_path()).unwrap(), assembly));
        assert!(assembly.xmlns_definitions.iter().any(|d| d.namespace == "FerroUI.Controls.Primitives"));
    }

    #[test]
    fn every_type_of_this_crate_has_a_namespace_and_a_handle() {
        register_types();

        for type_ in TYPES {
            assert!(type_.namespace().starts_with("FerroUI"), "{type_} has no namespace");
            if let Some(handle) = type_.handle() {
                let (found, nullable) = TypeInfo::find_by_handle(handle).unwrap();
                assert!(std::ptr::eq(found, *type_) && !nullable, "{type_}");
            }
        }
    }
}
