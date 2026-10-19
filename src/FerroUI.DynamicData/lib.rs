//! ferroui-dynamic-data
//!
//! A port of DynamicData (reactivemarbles/DynamicData) on top of `ferroui-reactive`: change sets
//! that describe the edits of a collection, observable caches (`SourceCache`, keyed) and lists
//! (`SourceList`), and the operators that transform, filter, sort, group, join and bind them. It is
//! single-threaded like the observables it is built on (docs/porting/reactive.md, sections 5.1 and
//! 5.4).
//!
//! - [`cache`]: the keyed side (`Change<T, K>`, `ChangeSet<T, K>`, `ChangeAwareCache`, `SourceCache`, ...).
//! - [`list`]: the side of positions (`list::Change<T>`, `list::ChangeSet<T>`, `ChangeAwareList`, ...).

pub mod binding;
pub mod cache;
pub mod diagnostics;
pub(crate) mod internal;
pub mod list;

pub use cache::{
    Change, ChangeAwareCache, ChangeReason, ChangeSet, ICache, ICacheUpdater, IChangeSet, IConnectableCache, IObservableCache, IQuery, ISourceCache, ISourceUpdater,
    ObservableCache, ObservableCacheEx, SourceCache,
};

#[cfg(test)]
mod tests;
mod as_disposable;
pub use as_disposable::*;
mod dynamic_data_options;
pub use dynamic_data_options::*;
mod enumerable_ex;
pub use enumerable_ex::*;
pub mod kernel;
mod observable_items_ex;
pub use observable_items_ex::*;
pub mod aggregation;
pub mod experimental;
mod observable_change_set;
pub use observable_change_set::*;
mod as_async_disposable;
pub use as_async_disposable::*;
