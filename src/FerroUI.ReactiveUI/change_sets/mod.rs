//! The change sets of ReactiveUI (`ReactiveUI.Core/ChangeSets`, `ReactiveUI.Core/Mixins/ChangeSetMixins.cs`
//! and `Shared/Platform/ChangeSetBinder.cs`): the changes of a notifying collection as a stream of
//! batches of per-item changes, the light layer that replaced DynamicData inside ReactiveUI.

mod change_set_binder;
mod change_set_extensions;
mod change_set_mixins;
mod collection_changed;
mod collection_changed_extensions;
mod i_reactive_change_set;
mod i_reactive_change_set_t;
mod reactive_change;
mod reactive_change_reason;
mod reactive_change_set_t;

pub use change_set_binder::{ChangeSetBinder, ReactiveChangeHandler};
pub use change_set_extensions::{ChangeSetExtensions, ChangeSetSource};
pub use change_set_mixins::{ChangeSetCountMixins, ChangeSetMixins};
pub use collection_changed::CollectionChanged;
pub use collection_changed_extensions::CollectionChangedExtensions;
pub use i_reactive_change_set::IReactiveChangeSet;
pub use i_reactive_change_set_t::IReactiveChangeSetOf;
pub use reactive_change::ReactiveChange;
pub use reactive_change_reason::ReactiveChangeReason;
pub use reactive_change_set_t::ReactiveChangeSet;
