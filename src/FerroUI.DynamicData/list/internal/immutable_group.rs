//! `ImmutableGroup<TObject, TGroupKey>` (`List/Internal/ImmutableGroup.cs`).

use crate::list::IGrouping;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

struct Inner<T, G> {
    key: G,
    items: Vec<T>,
}

/// The state of a group of a list at one moment (`ImmutableGroup<TObject, TGroupKey>`, the
/// `IGrouping` of the change sets of `GroupWithImmutableState`): a copy of its items. A clone is
/// the same state.
///
/// Deviation (DEVIATIONS.md, DynamicData): upstream's class is internal and its change sets hold
/// the interface `IGrouping`; here they hold this type, so it is public.
pub struct ImmutableGroup<T, G> {
    inner: Rc<Inner<T, G>>,
}

impl<T, G> Clone for ImmutableGroup<T, G> {
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone() }
    }
}

impl<T, G> ImmutableGroup<T, G> {
    /// The state of the group `key` with the items (`ImmutableGroup(TGroupKey, IList<TObject>)`,
    /// which copies them).
    pub(crate) fn new(key: G, items: Vec<T>) -> Self {
        Self { inner: Rc::new(Inner { key, items }) }
    }

    /// Whether `self` and `other` are the same state (`ReferenceEquals`).
    pub fn ptr_eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner)
    }
}

impl<T: Clone, G> IGrouping<T, G> for ImmutableGroup<T, G> {
    fn count(&self) -> usize {
        self.inner.items.len()
    }

    fn items(&self) -> Vec<T> {
        self.inner.items.clone()
    }

    fn key(&self) -> &G {
        &self.inner.key
    }
}

impl<T, G: PartialEq> PartialEq for ImmutableGroup<T, G> {
    /// States of the same group key are equal (`Equals`).
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner) || self.inner.key == other.inner.key
    }
}

impl<T, G: Eq> Eq for ImmutableGroup<T, G> {}

impl<T, G: Hash> Hash for ImmutableGroup<T, G> {
    /// The hash of the group key (`GetHashCode`).
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.inner.key.hash(state);
    }
}

impl<T, G: fmt::Display> fmt::Display for ImmutableGroup<T, G> {
    /// `ToString`: `Grouping for: {Key} ({Count} items)`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Grouping for: {} ({} items)", self.inner.key, self.inner.items.len())
    }
}

impl<T, G: fmt::Debug> fmt::Debug for ImmutableGroup<T, G> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Grouping for: {:?} ({} items)", self.inner.key, self.inner.items.len())
    }
}
