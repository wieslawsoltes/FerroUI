//! The behaviors and triggers of keyboard and pointer input (`Input/`).

mod global_hotkey_behavior;
mod inactivity_trigger;
mod lose_focus_on_enter_behavior;
mod select_all_on_focus_behavior;

pub use global_hotkey_behavior::GlobalHotkeyBehavior;
pub use inactivity_trigger::InactivityTrigger;
pub use lose_focus_on_enter_behavior::LoseFocusOnEnterBehavior;
pub use select_all_on_focus_behavior::SelectAllOnFocusBehavior;
