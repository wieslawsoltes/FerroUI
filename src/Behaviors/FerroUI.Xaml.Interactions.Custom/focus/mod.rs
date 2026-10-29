//! The behaviors that move the keyboard focus to their control: when it is loaded, attached,
//! visible, pressed or flagged, to the selected item of an items control, and the focus trap
//! (`Focus/`).

mod auto_focus_behavior;
mod focus_behavior;
mod focus_behavior_base;
mod focus_control_behavior;
mod focus_on_attached_behavior;
mod focus_on_attached_to_visual_tree_behavior;
mod focus_on_pointer_moved_behavior;
mod focus_on_pointer_pressed_behavior;
mod focus_on_visible_behavior;
mod focus_selected_item_behavior;
mod focus_trap_behavior;

pub use auto_focus_behavior::AutoFocusBehavior;
pub use focus_behavior::FocusBehavior;
pub use focus_behavior_base::{
    FocusBehaviorBase, FocusBehaviorBaseImpl, FocusBehaviorBaseImplExt, FocusBehaviorBaseVTable,
};
pub use focus_control_behavior::FocusControlBehavior;
pub use focus_on_attached_behavior::FocusOnAttachedBehavior;
pub use focus_on_attached_to_visual_tree_behavior::FocusOnAttachedToVisualTreeBehavior;
pub use focus_on_pointer_moved_behavior::FocusOnPointerMovedBehavior;
pub use focus_on_pointer_pressed_behavior::FocusOnPointerPressedBehavior;
pub use focus_on_visible_behavior::FocusOnVisibleBehavior;
pub use focus_selected_item_behavior::FocusSelectedItemBehavior;
pub use focus_trap_behavior::FocusTrapBehavior;
