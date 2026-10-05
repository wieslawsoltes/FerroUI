//! Selection models.
//!
//! # Inheritance among the selection classes
//!
//! The selection classes form a small hierarchy with virtual members
//! (node base, selection model, internal selection model) but are plain
//! shared structs, not part of the class model. Their virtual members are
//! modelled explicitly, one level at a time:
//!
//! - [`SelectionNodeBase`] holds the state of the base class. Its inherent
//!   methods are the base implementations of the virtual members, which are
//!   declared by [`SelectionNodeBaseImpl`]. The base holds a weak reference
//!   to the object deriving from it and makes every virtual call through
//!   that reference.
//! - [`SelectionModel`] embeds the node base and implements
//!   [`SelectionNodeBaseImpl`]. Its own implementation of each virtual
//!   member is the `base_*` method of the same name; the member can be
//!   overridden once more by a type deriving from the model, which
//!   implements [`SelectionModelImpl`] and registers itself with
//!   [`SelectionModel::set_derived`]. A virtual call reaches the override
//!   if there is one, the `base_*` method otherwise; an override calls the
//!   `base_*` method where the managed code calls the base implementation.
//! - [`InternalSelectionModel`] owns an untyped model, overrides the
//!   assignment of the source and the handling of source collection changes
//!   through [`SelectionModelImpl`], and dereferences to the model.
//!
//! The typed model is `SelectionModel<T>`; items are `Option<T>`, a null
//! item being `None`. The untyped model is `SelectionModel<BoxedValue>`.
//! [`ISelectionModel`] is the untyped contract implemented by every model.

mod i_selection_model;
mod index_range;
mod internal_selection_model;
mod read_only_selection_list_base;
mod selected_indexes;
mod selected_items;
mod selection_model;
mod selection_model_indexes_changed_event_args;
mod selection_model_selection_changed_event_args;
mod selection_node_base;

#[cfg(test)]
mod internal_selection_model_tests;
#[cfg(test)]
mod selection_model_tests_multiple;
#[cfg(test)]
mod selection_model_tests_single;

pub use i_selection_model::{selection_model_ptr_eq, BatchUpdateOperation, ISelectionModel, SelectionModelExtensions};
pub use internal_selection_model::{InternalSelectionModel, WritableSelectedItems};
pub use read_only_selection_list_base::{IReadOnlySelectionList, ReadOnlySelectionList, ReadOnlySelectionListBase};
pub use selected_indexes::SelectedIndexes;
pub use selected_items::{SelectedItems, SelectedItemsUntyped};
pub use selection_model::{SelectionModel, SelectionModelImpl};
pub use selection_model_indexes_changed_event_args::SelectionModelIndexesChangedEventArgs;
pub use selection_model_selection_changed_event_args::{
    SelectionModelSelectionChangedEventArgs, SelectionModelSelectionChangedEventArgsOf,
};
pub use selection_node_base::{CollectionChangeState, SelectionNodeBase, SelectionNodeBaseImpl};
