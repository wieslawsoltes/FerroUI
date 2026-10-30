//! Hierarchical selection services of the WinUI repeater family.

mod index_path;
mod markup;
mod selected_items;
mod selected_items_markup;
mod selection_model;
mod selection_model_children_requested_event_args;
mod selection_node;
mod selection_tree_helper;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod selected_items_binding_tests;

pub use index_path::{IndexPath, SelectionError};
pub use markup::register_markup_types;
pub use selected_items::{SelectedItems, SelectedItemsIter};
pub use selected_items_markup::{SelectedIndicesView, SelectedItemsView};
pub use selection_model::{SelectionModel, SelectionModelSelectionChangedEventArgs};
pub use selection_model_children_requested_event_args::SelectionModelChildrenRequestedEventArgs;

