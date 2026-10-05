//! Automation peers: the objects that expose elements to UI automation.

mod auto_complete_box_automation_peer;
mod automation_peer;
mod button_automation_peer;
mod calendar_automation_peer;
mod calendar_date_picker_automation_peer;
mod calendar_day_button_automation_peer;
#[cfg(test)]
mod calendar_day_button_automation_peer_tests;
mod carousel_page_automation_peer;
mod combo_box_automation_peer;
#[cfg(test)]
mod combo_box_automation_peer_tests;
#[cfg(test)]
mod complex_control_automation_peer_tests;
mod content_control_automation_peer;
mod content_page_automation_peer;
mod control_automation_peer;
#[cfg(test)]
mod control_automation_peer_tests;
mod date_picker_automation_peer;
mod drawer_page_automation_peer;
mod embeddable_control_root_automation_peer;
#[cfg(test)]
mod embeddable_control_root_automation_peer_tests;
mod expander_automation_peer;
#[cfg(test)]
mod foundation_automation_peer_tests;
mod image_automation_peer;
mod interop_automation_peer;
#[cfg(test)]
mod items_automation_peer_tests;
mod items_control_automation_peer;
mod label_automation_peer;
mod list_box_automation_peer;
mod list_item_automation_peer;
mod menu_item_automation_peer;
#[cfg(test)]
mod menu_item_automation_peer_tests;
mod native_control_host_peer;
mod native_menu_bar_automation_peer;
#[cfg(test)]
mod native_menu_bar_automation_peer_tests;
mod navigation_page_automation_peer;
mod none_automation_peer;
mod numeric_up_down_automation_peer;
#[cfg(test)]
mod page_automation_peer_tests;
mod pips_pager_automation_peer;
mod popup_automation_peer;
mod popup_root_automation_peer;
mod progress_bar_automation_peer;
mod radio_button_automation_peer;
mod range_base_automation_peer;
mod scroll_bar_automation_peer;
mod scroll_viewer_automation_peer;
mod selecting_items_control_automation_peer;
mod slider_automation_peer;
mod split_button_automation_peer;
#[cfg(test)]
mod split_button_automation_peer_tests;
mod tabbed_page_automation_peer;
mod text_block_automation_peer;
mod text_box_automation_peer;
mod thumb_automation_peer;
mod time_picker_automation_peer;
mod toggle_button_automation_peer;
mod toggle_split_button_automation_peer;
mod tool_tip_automation_peer;
mod tree_view_automation_peer;
mod tree_view_item_automation_peer;
mod unrealized_element_automation_peer;
mod user_control_automation_peer;
mod window_automation_peer;
#[cfg(test)]
mod window_automation_peer_tests;
mod window_base_automation_peer;

pub use auto_complete_box_automation_peer::AutoCompleteBoxAutomationPeer;
pub use automation_peer::{
    AutomationControlType, AutomationLandmarkType, AutomationPeer, AutomationPeerImpl, AutomationPeerImplExt,
    AutomationPeerVTable,
};
pub use button_automation_peer::ButtonAutomationPeer;
pub use calendar_automation_peer::CalendarAutomationPeer;
pub use calendar_date_picker_automation_peer::CalendarDatePickerAutomationPeer;
pub use calendar_day_button_automation_peer::CalendarDayButtonAutomationPeer;
pub use carousel_page_automation_peer::CarouselPageAutomationPeer;
pub use combo_box_automation_peer::ComboBoxAutomationPeer;
pub use content_control_automation_peer::ContentControlAutomationPeer;
pub use content_page_automation_peer::ContentPageAutomationPeer;
pub use control_automation_peer::{
    ControlAutomationPeer, ControlAutomationPeerImpl, ControlAutomationPeerImplExt, ControlAutomationPeerVTable,
};
pub use date_picker_automation_peer::DatePickerAutomationPeer;
pub use drawer_page_automation_peer::DrawerPageAutomationPeer;
pub use embeddable_control_root_automation_peer::EmbeddableControlRootAutomationPeer;
pub use expander_automation_peer::ExpanderAutomationPeer;
pub use image_automation_peer::ImageAutomationPeer;
pub use interop_automation_peer::InteropAutomationPeer;
pub use items_control_automation_peer::{
    ItemsControlAutomationPeer, ItemsControlAutomationPeerImpl, ItemsControlAutomationPeerImplExt,
    ItemsControlAutomationPeerVTable,
};
pub use label_automation_peer::LabelAutomationPeer;
pub use list_box_automation_peer::ListBoxAutomationPeer;
pub use list_item_automation_peer::ListItemAutomationPeer;
pub use menu_item_automation_peer::MenuItemAutomationPeer;
pub use native_control_host_peer::NativeControlHostPeer;
pub use native_menu_bar_automation_peer::NativeMenuBarAutomationPeer;
pub use navigation_page_automation_peer::NavigationPageAutomationPeer;
pub use none_automation_peer::NoneAutomationPeer;
pub use numeric_up_down_automation_peer::NumericUpDownAutomationPeer;
pub use pips_pager_automation_peer::PipsPagerAutomationPeer;
pub use popup_automation_peer::PopupAutomationPeer;
pub use popup_root_automation_peer::PopupRootAutomationPeer;
pub use progress_bar_automation_peer::ProgressBarAutomationPeer;
pub use radio_button_automation_peer::{
    RadioButtonAutomationPeer, RadioButtonAutomationPeerImpl, RadioButtonAutomationPeerImplExt,
    RadioButtonAutomationPeerVTable,
};
pub use range_base_automation_peer::{
    RangeBaseAutomationPeer, RangeBaseAutomationPeerImpl, RangeBaseAutomationPeerImplExt,
    RangeBaseAutomationPeerVTable,
};
pub use scroll_bar_automation_peer::ScrollBarAutomationPeer;
pub use scroll_viewer_automation_peer::ScrollViewerAutomationPeer;
pub use selecting_items_control_automation_peer::{
    SelectingItemsControlAutomationPeer, SelectingItemsControlAutomationPeerImpl,
    SelectingItemsControlAutomationPeerImplExt, SelectingItemsControlAutomationPeerVTable,
};
pub use slider_automation_peer::SliderAutomationPeer;
pub use split_button_automation_peer::SplitButtonAutomationPeer;
pub use tabbed_page_automation_peer::TabbedPageAutomationPeer;
pub use text_block_automation_peer::TextBlockAutomationPeer;
pub use text_box_automation_peer::{
    TextBoxAutomationPeer, TextBoxAutomationPeerImpl, TextBoxAutomationPeerImplExt, TextBoxAutomationPeerVTable,
};
pub use thumb_automation_peer::ThumbAutomationPeer;
pub use time_picker_automation_peer::TimePickerAutomationPeer;
pub use toggle_button_automation_peer::ToggleButtonAutomationPeer;
pub use toggle_split_button_automation_peer::ToggleSplitButtonAutomationPeer;
pub use tool_tip_automation_peer::ToolTipAutomationPeer;
pub use tree_view_automation_peer::TreeViewAutomationPeer;
pub use tree_view_item_automation_peer::TreeViewItemAutomationPeer;
pub use unrealized_element_automation_peer::UnrealizedElementAutomationPeer;
pub use user_control_automation_peer::UserControlAutomationPeer;
pub use window_automation_peer::WindowAutomationPeer;
pub use window_base_automation_peer::WindowBaseAutomationPeer;
