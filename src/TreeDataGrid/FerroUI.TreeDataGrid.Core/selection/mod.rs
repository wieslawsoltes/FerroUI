//! `Selection/`: the tree selection model and the row and cell selection of a source.
//!
//! # Virtual members
//!
//! The selection classes of the reference form a small hierarchy with virtual members; as in the
//! framework's selection, they are plain shared structs whose virtual members are modelled
//! explicitly:
//!
//! - [`SelectionNodeBase`] holds the state of the base of the nodes; its inherent methods are the
//!   base implementations, and [`SelectionNodeBaseImpl`] the overridable members, implemented by
//!   [`TreeSelectionNode`]. The base holds the node weakly, for the virtual calls and as the
//!   listener of the changes of its source.
//! - [`TreeSelectionModelBase`] owns the root node; its overridable members (`GetChildren`,
//!   `TryGetItemAt`, `OnSourceCollectionChangeFinished`, `OnNodeCollectionReset`) are
//!   [`TreeSelectionModelHooks`], implemented by the type deriving from the model and registered
//!   weakly with [`TreeSelectionModelBase::set_hooks`].
//!
//! Items are `Option<Rc<M>>` (the reference's `T?`), untyped items `Option<BoxedValue>`
//! (`object?`); the untyped source of the contracts is [`UntypedModels`].

mod i_tree_data_grid_selection;
mod i_tree_selection_model;
mod index_range;
mod index_ranges;
mod selection_node_base;
mod tree_selected_indexes;
mod tree_selected_items;
mod tree_selection_changed_items;
mod tree_selection_model_base;
mod tree_selection_model_indexes_changed_event_args;
mod tree_selection_model_selection_changed_event_args;
mod tree_selection_model_source_reset_event_args;
mod tree_selection_node;
mod tree_data_grid_cell_selection_model;
mod tree_data_grid_row_selection_model;

pub use i_tree_data_grid_selection::{downcast_selection, ITreeDataGridSelection};
pub use i_tree_selection_model::{
    box_model, downcast_models, erase_models, IReadOnlyList, ITreeSelectionModel, ReadOnlyList, UntypedModels,
};
pub use index_range::{IndexRange, IndexRangeIndices};
pub use index_ranges::IndexRanges;
pub use selection_node_base::{CollectionChangeState, SelectionNodeBase, SelectionNodeBaseImpl};
pub use tree_selected_indexes::TreeSelectedIndexes;
pub use tree_selected_items::TreeSelectedItems;
pub use tree_selection_changed_items::TreeSelectionChangedItems;
pub use tree_selection_model_base::{
    typed_source, BatchUpdateOperation, Operation, TreeSelectionModelBase, TreeSelectionModelHooks,
};
pub use tree_selection_model_indexes_changed_event_args::TreeSelectionModelIndexesChangedEventArgs;
pub use tree_selection_model_selection_changed_event_args::{
    TreeSelectionModelSelectionChangedEventArgs, TreeSelectionModelSelectionChangedEventArgsOf, UntypedItems,
};
pub use tree_selection_model_source_reset_event_args::TreeSelectionModelSourceResetEventArgs;
pub use tree_selection_node::TreeSelectionNode;
pub use tree_data_grid_cell_selection_model::{
    ColumnSelectionItem, TreeDataGridCellSelectionChangedEventArgs, TreeDataGridCellSelectionModel,
};
pub use tree_data_grid_row_selection_model::TreeDataGridRowSelectionModel;
