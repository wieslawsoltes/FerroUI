//! `CacheChangeSetEx` (`Cache/CacheChangeSetEx.cs`): the transformation of the items of a change
//! set.
//!
//! `ToConcreteType` has no counterpart: every change set of the port is the concrete
//! `ChangeSet<T, K>` (`IChangeSet::change_set`).

use super::{Change, ChangeSet};

/// The change set with each item, and each previous item, converted by `transformer`; the
/// reasons, keys and indexes stay (`Transform(IChangeSet<TSource, TKey>, Func<TSource,
/// TDestination>)`).
pub(crate) fn transform_change_set<T, U, K: Clone>(source: &ChangeSet<T, K>, transformer: &dyn Fn(&T) -> U) -> ChangeSet<U, K> {
    source
        .iter()
        .map(|change| {
            Change::with_previous(
                change.reason(),
                change.key().clone(),
                transformer(change.current()),
                change.previous().map(transformer),
                change.current_index(),
                change.previous_index(),
            )
        })
        .collect()
}
