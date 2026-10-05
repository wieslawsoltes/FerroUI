//! Primitive controls: the building blocks of the control set.

mod headered_content_control;
mod i_scroll_snap_points_info;
pub mod popup_positioning;
mod adorner_helper;
mod adorner_layer;
mod light_dismiss_overlay_layer;
mod overlay_layer;
mod popup_overlay_layer;
mod text_selector_layer;
mod visual_layer_manager;
mod i_popup_host;
mod overlay_popup_host;
mod popup;
mod popup_root;
mod snap_points_alignment;
mod snap_points_type;
mod template_applied_event_args;
mod templated_control;
mod uniform_grid;
mod scroll_bar_visibility;

pub use headered_content_control::HeaderedContentControl;
pub use i_scroll_snap_points_info::{IScrollSnapPointsInfo, SnapPointsChangedHandler};
pub use adorner_helper::AdornerHelper;
pub use adorner_layer::AdornerLayer;
pub use light_dismiss_overlay_layer::LightDismissOverlayLayer;
pub use overlay_layer::OverlayLayer;
pub use popup_overlay_layer::PopupOverlayLayer;
pub use text_selector_layer::TextSelectorLayer;
pub use visual_layer_manager::VisualLayerManager;
pub use i_popup_host::IPopupHost;
pub use overlay_popup_host::OverlayPopupHost;
pub use popup::{CustomPopupPlacementCallbackValue, Popup};
pub use popup_root::PopupRoot;
pub use snap_points_alignment::SnapPointsAlignment;
pub use snap_points_type::SnapPointsType;
pub use template_applied_event_args::TemplateAppliedEventArgs;
pub use templated_control::{TemplatedControl, TemplatedControlImpl, TemplatedControlImplExt, TemplatedControlVTable};
pub use uniform_grid::UniformGrid;
pub use scroll_bar_visibility::ScrollBarVisibility;

#[cfg(test)]
mod templated_control_tests;
#[cfg(test)]
mod headered_content_control_tests;
#[cfg(test)]
mod uniform_grid_tests;
#[cfg(test)]
mod visual_layer_manager_tests;
#[cfg(test)]
mod adorner_layer_composition_tests;
#[cfg(test)]
mod popup_root_tests;
#[cfg(test)]
mod popup_tests;

mod toggle_button;
pub use toggle_button::{ToggleButton, ToggleButtonImpl, ToggleButtonImplExt, ToggleButtonVTable};
#[cfg(test)]
mod toggle_button_tests;

mod range_base;
mod range_base_value_changed_event_args;
mod scroll_bar;
mod scroll_event_type;
mod thumb;
mod track;
pub use range_base::RangeBase;
pub use range_base_value_changed_event_args::RangeBaseValueChangedEventArgs;
pub use scroll_bar::{ScrollBar, ScrollEventArgs};
pub use scroll_event_type::ScrollEventType;
pub use track::{Track, TrackImpl, TrackImplExt, TrackVTable};
pub use thumb::{Thumb, ThumbImpl, ThumbImplExt, ThumbVTable};
#[cfg(test)]
mod range_base_tests;
#[cfg(test)]
mod scroll_bar_tests;
#[cfg(test)]
mod thumb_tests;
#[cfg(test)]
mod track_tests;

mod i_logical_scrollable;
pub use ferroui_base::input::IScrollable;
pub use i_logical_scrollable::{as_logical_scrollable, register_logical_scrollable, ILogicalScrollable};
pub use i_scroll_snap_points_info::{as_scroll_snap_points_info, register_scroll_snap_points_info};

mod access_text;
mod selection_handle_type;
pub use access_text::AccessText;
pub use selection_handle_type::SelectionHandleType;
#[cfg(test)]
mod access_text_tests;
#[cfg(test)]
mod templated_control_text_tests;

mod headered_items_control;
mod headered_selecting_items_control;
mod item_selection_event_triggers;
mod selecting_items_control;
mod tab_strip;
mod tab_strip_item;
mod text_search;

pub use headered_items_control::{
    HeaderedItemsControl, HeaderedItemsControlImpl, HeaderedItemsControlImplExt, HeaderedItemsControlVTable,
};
pub use headered_selecting_items_control::{
    HeaderedSelectingItemsControl, HeaderedSelectingItemsControlImpl, HeaderedSelectingItemsControlImplExt,
    HeaderedSelectingItemsControlVTable,
};
pub use item_selection_event_triggers::{ItemSelectionEventTriggers};
pub use selecting_items_control::{
    SelectedItemsList, SelectingItemsControl, SelectingItemsControlImpl, SelectingItemsControlImplExt,
    SelectingItemsControlVTable,
};
pub use tab_strip::{TabStrip};
pub use tab_strip_item::{TabStripItem};
pub use text_search::{TextSearch};

#[cfg(test)]
mod headered_items_control_tests;
#[cfg(test)]
mod selecting_items_control_tests;
#[cfg(test)]
mod selecting_items_control_tests_auto_select;
#[cfg(test)]
mod selecting_items_control_tests_multiple;
#[cfg(test)]
mod selecting_items_control_tests_selected_value;
#[cfg(test)]
mod tab_strip_tests;
#[cfg(test)]
mod popup_tests_items_control;

pub use crate::flyouts::{
    FlyoutBase, FlyoutBaseImpl, FlyoutBaseImplExt, FlyoutBaseVTable, PopupFlyoutBase, PopupFlyoutBaseImpl,
    PopupFlyoutBaseImplExt, PopupFlyoutBaseVTable,
};
pub use crate::split_view::SplitViewTemplateSettings;

// --- datetimepickers ---
pub use crate::date_time_pickers::{
    DateTimePickerPanel, DateTimePickerPanelType, PickerPresenterBase, PickerPresenterBaseImpl,
    PickerPresenterBaseImplExt,
};

// --- calendar ---
pub use crate::calendar::{
    CalendarBlackoutDatesCollection, CalendarButton, CalendarDayButton, CalendarItem, SelectedDatesCollection,
};
