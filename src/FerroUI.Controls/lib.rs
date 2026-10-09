//! FerroUI controls library: the control base classes, panels, templating
//! and the windowing platform contracts.

pub mod animation;
pub mod application_lifetimes;
pub mod automation;
pub mod chrome;
pub mod diagnostics;
pub mod embedding;
pub mod converters;
pub mod metadata;
pub mod notifications;
pub mod platform;
pub mod presentation_source;
pub mod presenters;
pub mod primitives;
pub mod remote;
pub mod shapes;
pub mod templates;
pub mod utils;

mod acrylic_platform_compensation_levels;
mod border;
mod canvas;
mod column_definition;
mod column_definitions;
mod content_control;
mod control;
mod control_extensions;
mod controls;
mod decorator;
mod definition_base;
mod definition_list;
mod dock_panel;
mod grid;
mod grid_length;
mod i_global_data_templates;
mod layout_transform_control;
mod markup_types;
mod panel;
mod pixel_point_event_args;
mod placement_mode;
mod platform_inhibition_type;
mod register_types;
#[doc(hidden)]
pub use markup_types::{AddChildOf, FerroListOf};
mod rust_paths;
mod relative_panel;
mod request_bring_into_view_event_args;
mod row_definition;
mod row_definitions;
mod size_changed_event_args;
mod size_to_content;
mod screens;
mod top_level;
mod top_level_host;
mod top_level_host_decorations;
mod top_level_host_peers;
mod window_base;
mod stack_panel;
mod viewbox;
mod window_closing_event_args;
mod window_decorations;
mod window_edge;
mod window_resized_event_args;
mod window_startup_location;
mod window_state;
mod window_transparency_level;
mod wrap_panel;

pub use acrylic_platform_compensation_levels::AcrylicPlatformCompensationLevels;
pub use border::Border;
pub use canvas::{Canvas, CanvasImpl, CanvasImplExt, CanvasVTable};
pub use column_definition::ColumnDefinition;
pub use column_definitions::ColumnDefinitions;
pub use content_control::{ContentControl, ContentControlImpl, ContentControlImplExt, ContentControlVTable};
pub use control::{Control, ControlImpl, ControlImplExt, ControlVTable};
pub use controls::Controls;
pub use decorator::Decorator;
pub use definition_base::{DefinitionBase, DefinitionBaseImpl, DefinitionBaseImplExt, DefinitionBaseVTable};
pub use definition_list::DefinitionList;
pub use dock_panel::{Dock, DockPanel};
pub use grid::Grid;
pub use grid_length::{GridLength, GridUnitType};
pub use i_global_data_templates::IGlobalDataTemplates;
pub use layout_transform_control::LayoutTransformControl;
pub use notifications::ReversibleStackPanel;
pub use panel::{Panel, PanelImpl, PanelImplExt, PanelVTable};
pub use pixel_point_event_args::PixelPointEventArgs;
pub use placement_mode::PlacementMode;
pub use platform_inhibition_type::PlatformInhibitionType;
pub use platform::{FeedbackAction, FeedbackType, IPlatformFeedback, PlatformFeedback, PlatformFeedbackExtensions};
pub use relative_panel::RelativePanel;
pub use request_bring_into_view_event_args::RequestBringIntoViewEventArgs;
pub use row_definition::RowDefinition;
pub use row_definitions::RowDefinitions;
pub use size_changed_event_args::SizeChangedEventArgs;
pub use size_to_content::SizeToContent;
pub use screens::Screens;
pub use top_level::{TopLevel, TopLevelImpl, TopLevelImplExt, TopLevelVTable};
pub use window_base::{IgnoreVisibilityChangesGuard, WindowBase, WindowBaseImpl, WindowBaseImplExt, WindowBaseVTable};
pub use stack_panel::{StackPanel, StackPanelImpl, StackPanelImplExt, StackPanelVTable};
pub use templates::ITemplateOf;
pub use viewbox::Viewbox;
pub use window_closing_event_args::{WindowCloseReason, WindowClosingEventArgs};
pub use window_decorations::WindowDecorations;
pub use window_edge::WindowEdge;
pub use window_resized_event_args::{WindowResizeReason, WindowResizedEventArgs};
pub use window_startup_location::WindowStartupLocation;
pub use window_state::WindowState;
pub use window_transparency_level::{WindowTransparencyLevel, WindowTransparencyLevelCollection};
pub use wrap_panel::{WrapPanel, WrapPanelItemsAlignment};

mod button;
mod button_spinner;
mod spinner;
mod split_button;
mod hotkey_manager;
pub mod mixins;
mod i_clickable_control;
mod i_command_source;
mod check_box;
mod context_menu;
mod drop_down_button;
mod hyperlink_button;
mod i_menu;
mod i_menu_element;
mod i_menu_item;
mod menu;
mod menu_base;
mod menu_item;
mod menu_item_access_key_handler;
mod menu_item_toggle_type;
mod i_native_menu_exporter_events_impl_bridge;
mod i_native_menu_item_exporter_events_impl_bridge;
mod native_dock;
mod native_menu;
mod native_menu_bar;
mod native_menu_bar_presenter;
mod native_menu_export;
mod native_menu_item;
mod native_menu_item_base;
mod native_menu_item_separator;
#[cfg(test)]
mod native_menu_tests;
mod tray_icon;
#[cfg(test)]
mod tray_icon_tests;
mod radio_button;
mod radio_button_group_manager;
mod repeat_button;
pub use button::{Button, ButtonImpl, ButtonImplExt, ButtonVTable, ClickMode};
pub use hotkey_manager::HotKeyManager;
pub use i_clickable_control::{as_clickable_control, register_clickable_control};
pub use i_command_source::{as_command_source, register_command_source};
pub use button_spinner::{ButtonSpinner, ButtonSpinnerImpl, ButtonSpinnerImplExt, ButtonSpinnerVTable, Location};
pub use check_box::CheckBox;
pub use drop_down_button::DropDownButton;
pub use hyperlink_button::HyperlinkButton;
pub use split_button::{
    SplitButton, SplitButtonImpl, SplitButtonImplExt, SplitButtonVTable, ToggleSplitButton, ToggleSplitButtonImpl,
    ToggleSplitButtonImplExt, ToggleSplitButtonVTable,
};
pub use spinner::{
    SpinDirection, SpinEventArgs, Spinner, SpinnerImpl, SpinnerImplExt, SpinnerVTable, ValidSpinDirections,
};
pub use context_menu::ContextMenu;
pub use menu::Menu;
pub use menu_base::{MenuBase, MenuBaseImpl, MenuBaseImplExt, MenuBaseVTable};
pub use menu_item::{MenuItem, MenuItemImpl, MenuItemImplExt, MenuItemVTable};
pub use menu_item_toggle_type::MenuItemToggleType;
pub use i_native_menu_exporter_events_impl_bridge::INativeMenuExporterEventsImplBridge;
pub use i_native_menu_item_exporter_events_impl_bridge::INativeMenuItemExporterEventsImplBridge;
pub use native_dock::NativeDock;
pub use native_menu::NativeMenu;
pub use native_menu_bar::NativeMenuBar;
pub use native_menu_item::NativeMenuItem;
pub use native_menu_item_base::NativeMenuItemBase;
pub use native_menu_item_separator::NativeMenuItemSeparator;
pub use tray_icon::{TrayIcon, TrayIcons};
pub use platform::MacOSProperties;
pub use radio_button::RadioButton;
pub use repeat_button::RepeatButton;
#[cfg(test)]
mod button_tests;
#[cfg(test)]
mod control_focus_adorner_tests;
#[cfg(test)]
mod hotkey_manager_tests;
#[cfg(test)]
mod radio_button_tests;
#[cfg(test)]
pub(crate) mod mouse_test_helper;
#[cfg(test)]
pub(crate) mod test_command;
#[cfg(test)]
pub(crate) mod test_support_buttons;

mod expander;
mod grid_splitter;
mod group_box;
mod progress_bar;
mod separator;
mod slider;
pub use expander::{ExpandDirection, Expander, ExpanderImpl, ExpanderImplExt, ExpanderVTable};
pub use grid_splitter::{
    GridResizeBehavior, GridResizeDirection, GridSplitter, GridSplitterImpl, GridSplitterImplExt, GridSplitterVTable,
};
pub use group_box::GroupBox;
mod user_control;
pub use user_control::UserControl;
pub use progress_bar::{ProgressBar, ProgressBarTemplateSettings};
pub use separator::Separator;
pub use slider::{Slider, SliderImpl, SliderImplExt, SliderVTable, TickPlacement};
mod tick_bar;
pub use tick_bar::{TickBar, TickBarPlacement, TickList};
#[cfg(test)]
mod grid_splitter_tests;
#[cfg(test)]
mod slider_tests;
#[cfg(test)]
mod expander_tests;
#[cfg(test)]
mod progress_bar_tests;
#[cfg(test)]
mod tick_bar_tests;

mod i_scroll_anchor_provider;
mod scroll_changed_event_args;
mod scroll_viewer;
pub use i_scroll_anchor_provider::{as_scroll_anchor_provider, register_scroll_anchor_provider, IScrollAnchorProvider};
pub use scroll_changed_event_args::ScrollChangedEventArgs;
pub use scroll_viewer::{ScrollViewer, ScrollViewerImpl, ScrollViewerImplExt, ScrollViewerVTable};
#[cfg(test)]
pub(crate) mod test_support_scrolling;
#[cfg(test)]
mod bring_into_view_tests;
#[cfg(test)]
mod scroll_viewer_tests;
#[cfg(test)]
mod scroll_viewer_tests_i_logical_scrollable;

mod experimental_acrylic_border;
#[cfg(test)]
mod experimental_acrylic_border_tests;
mod icon_element;
mod image;
mod label;
mod i_tool_tip_service;
mod tool_tip;
mod tool_tip_service;
pub use i_tool_tip_service::IToolTipService;
pub use tool_tip::ToolTip;
pub use tool_tip_service::ToolTipService;
#[cfg(test)]
mod tool_tip_tests;
#[cfg(test)]
mod context_menu_tests;
#[cfg(test)]
mod menu_item_tests;
mod path_icon;
pub use experimental_acrylic_border::ExperimentalAcrylicBorder;
pub use icon_element::IconElement;
pub use image::Image;
pub use label::Label;
pub use path_icon::PathIcon;
#[cfg(test)]
mod image_tests;
#[cfg(test)]
mod label_tests;
#[cfg(test)]
pub(crate) mod test_support_shapes;
#[cfg(test)]
mod arrange_tests;
#[cfg(test)]
mod layout_manager_tests;
#[cfg(test)]
pub(crate) mod layout_test_control;
#[cfg(test)]
mod layoutable_tests;
#[cfg(test)]
mod layoutable_tests_effective_viewport_changed;
#[cfg(test)]
mod layoutable_tests_layout_rounding;
#[cfg(test)]
mod measure_tests;
#[cfg(test)]
mod render_tests_culling;
#[cfg(test)]
mod visual_extensions_get_transformed_bounds_tests;
#[cfg(test)]
mod visual_extensions_get_visuals_at_tests;
#[cfg(test)]
mod visual_extensions_tests;
#[cfg(test)]
mod visual_tests;
#[cfg(test)]
mod styled_element_tests;
#[cfg(test)]
mod input_element_focus_tests;
#[cfg(test)]
mod drawing_image_propagation_tests;
#[cfg(test)]
mod gestures_tests;
#[cfg(test)]
mod input_element_gesture_tests;
#[cfg(test)]
mod styled_element_tests_theming;
#[cfg(test)]
mod styled_element_tests_resources;

#[cfg(test)]
pub(crate) mod test_support;

#[cfg(test)]
mod border_tests;
#[cfg(test)]
mod canvas_tests;
#[cfg(test)]
mod classes_tests;
#[cfg(test)]
mod content_control_tests;
#[cfg(test)]
mod user_control_tests;
#[cfg(test)]
mod decorator_tests;
#[cfg(test)]
mod dock_panel_tests;
#[cfg(test)]
mod grid_length_tests;
#[cfg(test)]
pub(crate) mod grid_mocks;
#[cfg(test)]
mod grid_tests;
#[cfg(test)]
mod layout_transform_control_tests;
#[cfg(test)]
mod loaded_tests;
#[cfg(test)]
mod panel_tests;
#[cfg(test)]
mod relative_panel_tests;
#[cfg(test)]
mod stack_panel_tests;
#[cfg(test)]
mod viewbox_tests;
#[cfg(test)]
mod wrap_panel_tests;
#[cfg(test)]
mod name_scope_tests;
#[cfg(test)]
mod navigation_tests;

mod app_builder;
mod application;
mod design;
mod desktop_application_extensions;
mod logging_extensions;
mod shutdown_mode;
pub use app_builder::AppBuilder;
pub use application::{
    Application, ApplicationImpl, ApplicationImplExt, ApplicationVTable, NewApplication,
};
pub use design::{Design, PreviewTemplate, DesignStyleValue};
pub use design::PreviewTarget;
pub use shutdown_mode::ShutdownMode;
#[cfg(any(test, feature = "testing"))]
pub mod testing;
#[cfg(test)]
mod app_builder_tests;
#[cfg(test)]
pub(crate) mod compositor_hit_testing_tests;
#[cfg(test)]
mod application_tests;
#[cfg(test)]
mod design_tests;
#[cfg(test)]
mod element_ref_tests;
#[cfg(test)]
mod platform_feedback_items_tests;
#[cfg(test)]
mod top_level_tests;
#[cfg(test)]
mod top_level_tests_platform_features;

mod window;
mod window_icon;
pub use window::{Window, WindowClosingBehavior, WindowImpl, WindowImplExt, WindowVTable};
pub use window_icon::WindowIcon;
#[cfg(test)]
mod window_base_tests;
#[cfg(test)]
mod window_tests;
#[cfg(test)]
mod window_decorations_tests;

// --- text controls ---
pub mod documents;

mod pasting_from_clipboard_event_args;
mod selectable_text_block;
mod text_block;
mod text_changed_event_args;
mod text_changing_event_args;

pub use pasting_from_clipboard_event_args::PastingFromClipboardEventArgs;
pub use selectable_text_block::SelectableTextBlock;
pub use text_block::{InlinesTextSource, SimpleTextSource, TextBlock, TextBlockImpl, TextBlockImplExt, TextBlockVTable};
pub use text_changed_event_args::TextChangedEventArgs;
pub use text_changing_event_args::TextChangingEventArgs;

#[cfg(test)]
mod selectable_text_block_tests;
#[cfg(test)]
mod text_block_tests;
#[cfg(test)]
mod label_text_tests;
#[cfg(test)]
mod button_text_tests;

// --- flyouts ---
mod flyouts;
pub use flyouts::{Flyout, FlyoutPresenter, FlyoutShowMode, MenuFlyout, MenuFlyoutPresenter};
#[cfg(test)]
mod button_flyout_tests;

mod text_box;
mod text_box_text_input_method_client;
pub use text_box::*;
pub use text_box_text_input_method_client::TextBoxTextInputMethodClient;
#[cfg(test)]
mod text_box_tests;

mod masked_text_box;
pub use masked_text_box::MaskedTextBox;
#[cfg(test)]
mod masked_text_box_tests;
#[cfg(test)]
mod text_box_tests_input;
mod data_validation_errors;
pub use data_validation_errors::{DataValidationErrors, ErrorConverter};
#[cfg(test)]
mod text_box_tests_data_validation;


pub mod generators;
pub mod selection;

mod assigned_binding;
mod combo_box;
mod combo_box_item;
mod container_clearing_event_args;
mod container_index_changed_event_args;
mod container_prepared_event_args;
mod i_selectable;
mod item_collection;
mod items_control;
mod items_source;
mod items_source_view;
mod list_box;
mod list_box_item;
mod navigable_containers;
mod selection_changed_event_args;
mod selection_mode;
mod tab_control;
mod tab_item;
mod tree_view;
mod tree_view_item;
mod virtualizing_panel;
mod virtualizing_stack_panel;

pub use assigned_binding::{AssignedBinding};
pub use combo_box::{ComboBox};
pub use combo_box_item::{ComboBoxItem};
pub use container_clearing_event_args::{ContainerClearingEventArgs};
pub use container_index_changed_event_args::{ContainerIndexChangedEventArgs};
pub use container_prepared_event_args::{ContainerPreparedEventArgs};
pub use i_selectable::{ISelectable, register_selectable, as_selectable};
pub use item_collection::{ItemCollection};
pub use items_control::{ItemsControl, ItemsControlImpl, ItemsControlImplExt, ItemsControlVTable};
pub use items_source::{
    ItemsChangedEventArgs, ItemsChangedHandler, items_equal, reference_equals, boxed_reference_equals, box_item, unbox_item, IItemsList, ItemsSource,
};
pub use items_source_view::{ItemsSourceView, ItemsSourceViewOf};
pub use list_box::{ListBox};
pub use list_box_item::{ListBoxItem};
pub use navigable_containers::{register_navigable_container, as_navigable_container};
pub use selection_changed_event_args::{SelectionChangedEventArgs};
pub use selection_mode::SelectionMode;
pub use tab_control::{TabControl, TabControlImpl, TabControlImplExt, TabControlVTable};
pub use tab_item::{TabItem};
pub use tree_view::{TreeView, TreeViewImpl, TreeViewImplExt, TreeViewVTable};
pub use tree_view_item::{TreeViewItem, TreeViewItemImpl, TreeViewItemImplExt, TreeViewItemVTable};
pub use virtualizing_panel::{
    VirtualizingPanel, VirtualizingPanelImpl, VirtualizingPanelImplExt, VirtualizingPanelVTable,
};
pub use virtualizing_stack_panel::{VirtualizingStackPanel};

#[cfg(test)]
mod combo_box_tests;
#[cfg(test)]
mod items_control_tests;
#[cfg(test)]
mod items_source_view_tests;
#[cfg(test)]
mod tab_control_tests;
#[cfg(test)]
mod tree_view_tests;
#[cfg(test)]
mod virtualizing_stack_panel_tests;

mod carousel;
mod virtualizing_carousel_panel;

pub use carousel::{Carousel};
pub use virtualizing_carousel_panel::{VirtualizingCarouselPanel};
#[cfg(test)]
mod tree_view_bring_into_view_tests;



#[cfg(test)]
mod carousel_tests;
#[cfg(test)]
mod list_box_tests;
#[cfg(test)]
mod virtualizing_carousel_panel_tests;
#[cfg(test)]
mod list_box_tests_single;
#[cfg(test)]
mod list_box_tests_multiple;
#[cfg(test)]
mod list_box_virtualization_issue_tests;
pub use items_source::{ItemsView, ItemsViewIter, TypedItems};
#[cfg(test)]
mod data_validation_errors_tests;
#[cfg(test)]
mod reference_semantics_tests;

pub use register_types::register_types;

// --- numeric ---
pub mod numeric_up_down;
pub use numeric_up_down::{
    NumericUpDown, NumericUpDownImpl, NumericUpDownImplExt, NumericUpDownVTable, NumericUpDownValueChangedEventArgs,
};

// --- splitview ---
mod split_view;
pub use split_view::{
    SplitView, SplitViewDisplayMode, SplitViewImpl, SplitViewImplExt, SplitViewPanePlacement, SplitViewVTable,
};

// --- misc-controls ---
mod input_pane_aware_behavior;
mod input_pane_aware_decorator;
mod theme_variant_scope;
mod toggle_switch;
mod transition_completed_event_args;
mod transitioning_content_control;
pub use input_pane_aware_behavior::InputPaneAwareBehavior;
pub use input_pane_aware_decorator::InputPaneAwareDecorator;
pub use theme_variant_scope::ThemeVariantScope;
pub use toggle_switch::ToggleSwitch;
pub use transition_completed_event_args::TransitionCompletedEventArgs;
pub use transitioning_content_control::TransitioningContentControl;
#[cfg(test)]
mod theme_variant_tests;
#[cfg(test)]
mod toggle_switch_tests;
#[cfg(test)]
mod transitioning_content_control_tests;

// --- commandbar ---
pub mod command_bar;
pub use command_bar::{
    CommandBar, CommandBarButton, CommandBarDefaultLabelPosition, CommandBarElementCollection, CommandBarElementList,
    CommandBarOverflowButtonVisibility, CommandBarSeparator, CommandBarToggleButton, ICommandBarElement,
};

// --- autocomplete ---
mod auto_complete_box;
pub use auto_complete_box::{
    AutoCompleteAsyncPopulator, AutoCompleteBox, AutoCompleteBoxImpl, AutoCompleteBoxImplExt, AutoCompleteBoxVTable,
    AutoCompleteFilterMode, AutoCompleteFilterPredicate, AutoCompletePopulation, AutoCompleteSelector,
    PopulatedEventArgs, PopulatingEventArgs,
};

// --- storage-misc ---
mod border_visual;
mod i_content_control;
mod i_headered;
mod native_control_host;
mod system_font_app_builder_extension;
mod url_opened_event_args;
pub use i_content_control::{as_content_control, register_content_control, IContentControl};
pub use i_headered::{as_headered, register_headered, IHeadered};
pub use native_control_host::{
    NativeControlHost, NativeControlHostImpl, NativeControlHostImplExt, NativeControlHostVTable,
};
pub use platform::X11Properties;
pub use platform::Win32Properties;
pub use url_opened_event_args::UrlOpenedEventArgs;
#[cfg(test)]
mod storage_misc_tests;

// --- flexpanel ---
pub mod flex_panel;
pub use flex_panel::{
    Flex, FlexAlignContent, FlexAlignItems, FlexBasis, FlexBasisKind, FlexDirection, FlexJustifyContent, FlexPanel,
    FlexWrap,
};

// --- notifications ---
pub use notifications::{
    IManagedNotificationManager, INotification, INotificationManager, Notification, NotificationCard,
    NotificationOverrides, NotificationPosition, NotificationType, WindowNotificationManager,
};

// --- pulltorefresh ---
pub mod pull_to_refresh;
pub use pull_to_refresh::{
    RefreshCompletionDeferral, RefreshContainer, RefreshRequestedEventArgs, RefreshVisualizer,
    RefreshVisualizerOrientation, RefreshVisualizerState,
};

// --- calendar ---
mod calendar;
mod calendar_date_picker;
pub use calendar::{
    Calendar, CalendarDateChangedEventArgs, CalendarDateRange, CalendarMode, CalendarModeChangedEventArgs,
    CalendarSelectionMode,
};
pub use calendar_date_picker::{
    CalendarDatePicker, CalendarDatePickerDateValidationErrorEventArgs, CalendarDatePickerFormat,
    CalendarDatePickerImpl, CalendarDatePickerImplExt, CalendarDatePickerVTable,
};

// --- datetimepickers ---
pub mod date_time_pickers;
pub use date_time_pickers::{
    DatePicker, DatePickerImpl, DatePickerImplExt, DatePickerPresenter, DatePickerSelectedValueChangedEventArgs,
    TimePicker, TimePickerImpl, TimePickerImplExt, TimePickerPresenter, TimePickerSelectedValueChangedEventArgs,
};

// --- page-core ---
pub mod page;
pub use page::{
    BarLayoutBehavior, ContentPage, INavigation, ModalPoppedEventArgs, ModalPushedEventArgs, NavigatedFromEventArgs,
    NavigatedToEventArgs, NavigatingFromEventArgs, NavigatingTask, NavigationEventArgs, NavigationPage,
    NavigationType, Page,
    PageImpl, PageImplExt, PageInsertedEventArgs, PageNavigationExtensions, PageNavigationHost,
    PageRemovedEventArgs, PageVTable,
};

// --- page-multi ---
pub use page::{
    CarouselPage, MultiPage, MultiPageImpl, MultiPageImplExt, MultiPageVTable, PageList, PageSelectionChangedEventArgs,
    PagesChangedHandler, SelectingMultiPage, SelectingMultiPageImpl, SelectingMultiPageImplExt,
    SelectingMultiPageVTable, TabPlacement, TabbedPage,
};

// --- page-drawer ---
pub use page::{DrawerBehavior, DrawerClosingEventArgs, DrawerLayoutBehavior, DrawerPage, DrawerPlacement};

// --- pipspager ---
mod pips_pager;
pub use pips_pager::{PipsPager, PipsPagerSelectedIndexChangedEventArgs, PipsPagerTemplateSettings};

// --- connected-animation ---

// --- tableview ---
mod table_view;
mod table_view_cell;
mod table_view_column;
mod table_view_column_header;
mod table_view_row;
pub use table_view::TableView;
pub use table_view_cell::TableViewCell;
pub use table_view_column::TableViewColumn;
pub use table_view_column_header::TableViewColumnHeader;
pub use table_view_row::TableViewRow;
#[cfg(test)]
mod table_view_column_header_tests;
#[cfg(test)]
mod table_view_tests;

// --- textselection ---

// --- leftovers ---

