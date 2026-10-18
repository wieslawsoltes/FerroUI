//! `Selection/`: how the selections of the core handle the input of the control.
//!
//! The selection models are the core's (`ferroui_treedatagrid_core::selection`): the row and cell
//! selections of a source, the tree selection model, its indexes and change arguments. The
//! reference's copies of them in the control project are not ported; nor are its classic column
//! selection model (`TreeDataGridColumnSelectionModel`, `ITreeDataGridColumnSelectionModel`) and
//! the product of selected columns and rows (`SelectedCellIndexes`), which only its classic cell
//! selection used: the core's cell selection keeps its own selection of columns and computes the
//! selected cells (`TreeDataGridCellSelectionModel::selected_indexes`), and the arguments of its
//! changes are the core's `TreeDataGridCellSelectionChangedEventArgs`.
//!
//! This module holds the interactions of the control with those selections, and the typed
//! operations the untyped control needs of a source's model type
//! ([`register_tree_data_grid_model`]).

mod contract;
pub use contract::*;

mod tree_data_grid_cell_selection_interaction;
mod tree_data_grid_row_selection_interaction;
mod tree_data_grid_selection_support;

pub use tree_data_grid_cell_selection_interaction::TreeDataGridCellSelectionInteraction;
pub use tree_data_grid_row_selection_interaction::TreeDataGridRowSelectionInteraction;
pub use tree_data_grid_selection_support::{
    as_cell_selection, as_row_selection, clear_show_expander, is_source_registered, register_tree_data_grid_model,
    register_tree_data_grid_source, source_support, update_show_expander, ITreeDataGridModelSupport,
    ITreeDataGridSelectionFactory, ITreeDataGridSourceSupport, IUntypedCellSelection, IUntypedRowSelection,
};
