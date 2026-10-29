//! The actions of the crate (`Actions/`).

mod add_class_action;
mod call_method_async_action;
mod change_ferro_property_action;
mod delayed_show_control_action;
mod focus_control_action;
mod focus_next_element_action;
mod hide_control_action;
mod hide_flyout_action;
mod hide_popup_action;
mod launch_uri_action;
mod popup_action;
mod remove_class_action;
mod remove_element_action;
mod screenshot_action;
mod scroll_to_control_action;
mod set_enabled_action;
mod set_theme_variant_action;
mod show_context_menu_action;
mod show_control_action;
mod show_flyout_action;
mod show_popup_action;
mod toggle_class_action;

pub use add_class_action::AddClassAction;
pub use call_method_async_action::CallMethodAsyncAction;
pub use change_ferro_property_action::ChangeFerroPropertyAction;
pub use delayed_show_control_action::DelayedShowControlAction;
pub use focus_control_action::FocusControlAction;
pub use focus_next_element_action::FocusNextElementAction;
pub use hide_control_action::HideControlAction;
pub use hide_flyout_action::HideFlyoutAction;
pub use hide_popup_action::HidePopupAction;
pub use launch_uri_action::LaunchUriAction;
pub use popup_action::PopupAction;
pub use remove_class_action::RemoveClassAction;
pub use remove_element_action::RemoveElementAction;
pub use screenshot_action::ScreenshotAction;
pub use scroll_to_control_action::ScrollToControlAction;
pub use set_enabled_action::SetEnabledAction;
pub use set_theme_variant_action::SetThemeVariantAction;
pub use show_context_menu_action::ShowContextMenuAction;
pub use show_control_action::ShowControlAction;
pub use show_flyout_action::ShowFlyoutAction;
pub use show_popup_action::ShowPopupAction;
pub use toggle_class_action::ToggleClassAction;

use ferroui_base::{BoxedValue, Ref, StyledElement, Visual};
use ferroui_controls::Control;
use ferroui_xaml_interactivity::helpers::member_lookup::object_of;

/// Not from the reference: the sender of an action as a control (`sender as Control`).
fn control_of(sender: Option<&BoxedValue>) -> Option<Ref<Control>> {
    sender.and_then(object_of).and_then(|sender| sender.cast::<Control>())
}

/// Not from the reference: the sender of an action as a styled element
/// (`sender as StyledElement`).
fn styled_element_of(sender: Option<&BoxedValue>) -> Option<Ref<StyledElement>> {
    sender.and_then(object_of).and_then(|sender| sender.cast::<StyledElement>())
}

/// Not from the reference: the sender of an action as a visual (`sender as Visual`).
fn visual_of(sender: Option<&BoxedValue>) -> Option<Ref<Visual>> {
    sender.and_then(object_of).and_then(|sender| sender.cast::<Visual>())
}
