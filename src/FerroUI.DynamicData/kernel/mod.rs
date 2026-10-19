//! The kernel of DynamicData (`DynamicData/Kernel`): the extensions of `Option<T>` (upstream's
//! `Optional<T>`) and of its sequences, the status of a connection, the error of an item, the
//! pairs of an item with its index or a value, and the helpers of sequences and schedulers.
//!
//! Upstream's `Optional<T>` is Rust's `Option<T>` (docs/porting/reactive.md, section 5.4), so
//! `Optional.cs` itself has no counterpart; `ReferenceEqualityComparer` is `Rc::ptr_eq`,
//! `ReadOnlyCollectionLight` is `Vec<T>`, `EnumerableIList`/`EnumeratorIList`/`IEnumerableIList`
//! are the slices and `Vec`s of Rust, `ISupportsCapacity` is `Vec::with_capacity`, and
//! `ParallelEx` (PLINQ) is not ported.

mod connection_status;
mod enumerable_ex;
mod error;
mod internal_ex;
mod item_with_index;
mod item_with_value;
mod option_else;
mod option_extensions;
mod option_observable_extensions;

pub use connection_status::ConnectionStatus;
pub use enumerable_ex::{duplicates, index_of_many, index_of_many_with, with_index};
pub use error::Error;
pub(crate) use internal_ex::{dotnet_binary_search, return_with};
pub use internal_ex::{retry_with_back_off, schedule_recurring_action, schedule_recurring_action_with, to_unit};
pub use item_with_index::ItemWithIndex;
pub use item_with_value::ItemWithValue;
pub use option_else::OptionElse;
pub use option_extensions::{first_or_optional, lookup, remove_if_contained, select_values, OptionExtensions};
pub use option_observable_extensions::OptionObservableExtensions;
