//! The framework-neutral part of FerroUI's TreeDataGrid, ported from the `TreeDataGrid.Core`
//! project of TreeDataGrid (<https://github.com/wieslawsoltes/TreeDataGrid>): sources of flat and
//! hierarchical data, their columns and rows, sorting, expansion, and row and cell selection. The
//! control that shows them is `ferroui-controls-treedatagrid` (docs/porting/treedatagrid.md).

pub mod accessor;
mod cell_index;
mod flat_tree_data_grid_source;
mod grid_length;
mod hierarchical_tree_data_grid_source;
mod i_tree_data_grid_source;
mod index_path;
pub mod items_source_view;
mod markup_types;
pub use markup_types::register_types;
pub mod model_collection;
pub mod models;
pub mod selection;
pub mod utils;

pub use accessor::{Accessor, PathSubscription, PropertyPathObserver, TreeModel};
pub use cell_index::CellIndex;
pub use flat_tree_data_grid_source::FlatTreeDataGridSource;
pub use hierarchical_tree_data_grid_source::{HierarchicalTreeDataGridSource, RowEventHandler};
pub use i_tree_data_grid_source::{
    downcast_source, ITreeDataGridSource, ITreeDataGridSourceOf, RowDropPosition, RowMoveEffects,
};
pub use grid_length::{GridLength, GridUnitType};
pub use index_path::IndexPath;
pub use items_source_view::TreeDataGridItemsSourceView;
pub use model_collection::{ModelCollection, Models, MutableModelCollection};

#[cfg(test)]
mod tests;
