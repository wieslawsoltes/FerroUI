//! WinUI's `Pivot` and its parts (#443, PV-1 to PV-3 of docs/porting/winui-calculator-gaps.md,
//! section 7.1).

mod pivot;
mod pivot_common;
mod pivot_curve_generator;
mod pivot_header_item;
mod pivot_header_manager;
mod pivot_header_panel;
mod pivot_item;
mod pivot_panel;
mod pivot_state_machine;

#[cfg(test)]
mod pivot_state_machine_tests;
#[cfg(test)]
mod pivot_tests;

pub use pivot::{Pivot, PivotHeaderFocusVisualPlacement, PivotSlideInAnimationGroup};
pub use pivot_header_item::PivotHeaderItem;
pub use pivot_header_panel::PivotHeaderPanel;
pub use pivot_item::{PivotItem, PivotItemEventArgs};
pub use pivot_panel::PivotPanel;
