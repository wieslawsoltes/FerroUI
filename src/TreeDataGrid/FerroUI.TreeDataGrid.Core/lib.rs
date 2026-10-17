//! The framework-neutral part of FerroUI's TreeDataGrid, ported from the `TreeDataGrid.Core`
//! project of TreeDataGrid (<https://github.com/wieslawsoltes/TreeDataGrid>): sources of flat and
//! hierarchical data, their columns and rows, sorting, expansion, and row and cell selection. The
//! control that shows them is `ferroui-controls-treedatagrid` (docs/porting/treedatagrid.md).

pub mod accessor;
mod cell_index;
mod grid_length;
mod index_path;
pub mod items_source_view;
pub mod model_collection;
pub mod models;
pub mod selection;
pub mod utils;

pub use accessor::{Accessor, PathSubscription, PropertyPathObserver, TreeModel};
pub use cell_index::CellIndex;
pub use grid_length::{GridLength, GridUnitType};
pub use index_path::IndexPath;
pub use items_source_view::TreeDataGridItemsSourceView;
pub use model_collection::{ModelCollection, Models, MutableModelCollection};

#[cfg(test)]
mod tests;
