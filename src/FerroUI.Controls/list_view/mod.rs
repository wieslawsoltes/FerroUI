//! WinUI's `ListViewBase`, `ListView` and their items and panels (#443, LV-1 and LV-2 of
//! docs/porting/winui-calculator-gaps.md, section 6).

mod items_stack_panel;
mod list_view;
mod list_view_base;

#[cfg(test)]
mod list_view_tests;

pub use items_stack_panel::{GroupHeaderPlacement, ItemsStackPanel, ItemsUpdatingScrollMode};
pub use list_view::{ListView, ListViewBaseItem, ListViewItem, ListViewItemTemplateSettings};
pub use list_view_base::{
    ChoosingItemContainerEventArgs, ContainerContentChangingEventArgs, IncrementalLoadingTrigger, ItemClickEventArgs,
    ItemIndexRange, ListViewBase, ListViewReorderMode, ListViewSelectionMode, ScrollIntoViewAlignment,
};
