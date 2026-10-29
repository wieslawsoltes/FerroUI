//! Shared list contracts bridging plain/custom lists and FerroUI observable collections.

mod list_handle;
pub use list_handle::{IList, ListComparer, ListHandle};

mod i_enumerable;
#[cfg(test)]
mod list_tests;
pub use i_enumerable::{IEnumerable, IEnumerator};
mod read_only_observable_collection;
pub use read_only_observable_collection::ReadOnlyObservableCollection;
mod read_only_list_handle;
#[cfg(test)]
mod read_only_tests;
pub use read_only_list_handle::{IReadOnlyList, ReadOnlyListChanged, ReadOnlyListHandle};
#[cfg(test)]
mod read_only_list_tests;
