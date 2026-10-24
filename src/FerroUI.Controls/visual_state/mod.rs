//! WinUI's visual state manager (docs/porting/winui-calculator-gaps.md, section 5): visual state
//! groups and states with setters, beside the pseudo-class styles of the framework.

mod applied_setters;
mod visual_state;
mod visual_state_changed_event_args;
mod visual_state_group;
mod visual_state_manager;

#[cfg(test)]
mod visual_state_manager_tests;

pub(crate) use applied_setters::AppliedSetters;
pub use visual_state::VisualState;
pub use visual_state_changed_event_args::VisualStateChangedEventArgs;
pub use visual_state_group::VisualStateGroup;
pub use visual_state_manager::{VisualStateGroupCollection, VisualStateManager, VisualStateManagerImpl};
