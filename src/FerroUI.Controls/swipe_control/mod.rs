//! WinUI's `SwipeControl` and its items (#443, SW-1 of docs/porting/winui-calculator-gaps.md,
//! section 6.4).

mod swipe_control;
mod swipe_items;

#[cfg(test)]
mod swipe_control_tests;

pub use swipe_control::SwipeControl;
pub use swipe_items::{
    ExecuteItemsHaveOneItem, SwipeBehaviorOnInvoked, SwipeItem, SwipeItemInvokedEventArgs, SwipeItems, SwipeMode,
};
