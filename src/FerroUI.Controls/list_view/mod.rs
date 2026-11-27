//! WinUI's `ListViewBase`, `ListView`, `GridView` and their items and panels (#443, LV-1 and LV-2 of
//! docs/porting/winui-calculator-gaps.md, section 6).

mod grid_view;
mod grouping;
mod items_stack_panel;
mod items_wrap_grid;
mod list_view;
mod list_view_base;
mod list_view_base_semantic_zoom;

#[cfg(test)]
mod grid_view_grouping_tests;
#[cfg(test)]
mod grouping_tests;
#[cfg(test)]
mod items_updating_scroll_mode_tests;
#[cfg(test)]
pub(crate) mod list_view_tests;

pub use grid_view::{GridView, GridViewHeaderItem, GridViewItem, GridViewItemTemplateSettings};
pub use grouping::{CollectionViewGroup, CollectionViewSource, GroupStyle, GroupedItemsView};
pub use items_stack_panel::{GroupHeaderPlacement, ItemsStackPanel, ItemsUpdatingScrollMode};
pub use items_wrap_grid::ItemsWrapGrid;
pub use list_view::{
    ListView, ListViewBaseHeaderItem, ListViewBaseItem, ListViewHeaderItem, ListViewItem, ListViewItemTemplateSettings,
};
pub use list_view_base::{
    ChoosingItemContainerEventArgs, ContainerContentChangingEventArgs, IncrementalLoadingTrigger, ItemClickEventArgs,
    ItemIndexRange, ListViewBase, ListViewReorderMode, ListViewSelectionMode, ScrollIntoViewAlignment,
};
