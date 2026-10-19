//! The list side of DynamicData (`DynamicData/List`): changes of positions in a list, their change
//! sets, and the lists that produce them.
//!
//! The types keep upstream's names (`Change<T>`, `ChangeSet<T>`, `IChangeSet<T>`), which in .NET
//! differ from those of the cache side by their number of type parameters; here they are told
//! apart by their module, and the crate root re-exports those of the cache.

mod change;
mod change_aware_list;
mod change_set;
mod i_list;
pub(crate) mod internal;
mod item_change;
mod list_change_reason;
mod observable_list;
mod observable_list_ex;
mod ops;
mod range_change;
mod source_list;
pub mod testing;

pub use change::Change;
pub use change_aware_list::{ChangeAwareList, IExtendedList};
pub use change_set::{ChangeSet, IChangeSet};
pub use i_list::{binary_search_by, clear_or_remove_many, clone_changes, clone_changes_with, remove_many, replace, replace_or_add, IList};
pub use item_change::ItemChange;
pub use list_change_reason::{ChangeType, ListChangeReason};
pub use range_change::RangeChange;
pub use observable_list::ObservableList;
pub use observable_list_ex::ObservableListEx;
pub use source_list::{IObservableList, ISourceList, SourceList};
mod change_aware_list_with_ref_counts;
mod change_set_ex;
pub use change_set_ex::*;
mod i_change_set_adaptor;
pub use i_change_set_adaptor::*;
mod i_group;
pub use i_group::*;
mod i_grouping;
pub use i_grouping::*;
mod list_filter_policy;
pub use list_filter_policy::*;
mod page_change_set;
pub use page_change_set::*;
mod sort_exception;
pub use sort_exception::*;
mod sort_options;
pub use sort_options::*;
mod source_list_edit_convenience_ex;
pub use source_list_edit_convenience_ex::*;
mod source_list_ex;
pub use source_list_ex::*;
mod virtual_change_set;
pub use virtual_change_set::*;
pub use internal::{Group, ImmutableGroup};
pub use ops::flatten_buffer_result::{flatten_buffer_result, FlattenBufferResultListEx};
pub use ops::cast::{ObservableObjectListEx};
pub use ops::merge_change_sets::{ListChangeSetSourcesEx, ObservableListChangeSetsEx};
