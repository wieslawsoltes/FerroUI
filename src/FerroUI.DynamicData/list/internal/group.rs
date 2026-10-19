//! `Group<TObject, TGroup>` (`List/Internal/Group.cs`).

use crate::list::{IExtendedList, IGroup, IObservableList, SourceList};
use ferroui_system::IDisposable;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

struct Inner<T, G> {
    group_key: G,
    source: SourceList<T>,
}

/// A group of the items of a list that share a group key (`Group<TObject, TGroup>`, the `IGroup`
/// of the change sets of `GroupOn`): the grouping edits its list, and its observers watch it. A
/// clone is the same group.
///
/// Deviation (DEVIATIONS.md, DynamicData): upstream's class is internal and its change sets hold
/// the interface `IGroup`; here they hold this type, so it is public.
pub struct Group<T, G> {
    inner: Rc<Inner<T, G>>,
}

impl<T, G> Clone for Group<T, G> {
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone() }
    }
}

impl<T: Clone + PartialEq + 'static, G> Group<T, G> {
    /// An empty group of `group_key` (`Group(TGroup)`).
    pub(crate) fn new(group_key: G) -> Self {
        Self { inner: Rc::new(Inner { group_key, source: SourceList::new() }) }
    }

    /// The group key (`GroupKey`).
    pub fn group_key(&self) -> &G {
        &self.inner.group_key
    }

    /// The items of the group (`List`).
    pub fn list(&self) -> Rc<dyn IObservableList<T>> {
        Rc::new(self.inner.source.clone())
    }

    /// The number of items.
    pub(crate) fn count(&self) -> usize {
        self.inner.source.count()
    }

    /// Edits the list of the group (`Edit`).
    pub(crate) fn edit(&self, edit_action: impl FnOnce(&mut dyn IExtendedList<T>)) {
        self.inner.source.edit(edit_action);
    }

    /// Whether `self` and `other` are the same group (`ReferenceEquals`).
    pub fn ptr_eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner)
    }
}

impl<T: Clone + PartialEq + 'static, G> IGroup<T, G> for Group<T, G> {
    fn group_key(&self) -> &G {
        &self.inner.group_key
    }

    fn list(&self) -> Rc<dyn IObservableList<T>> {
        Rc::new(self.inner.source.clone())
    }
}

impl<T: Clone + PartialEq + 'static, G> IDisposable for Group<T, G> {
    /// Ends the list of the group, which completes its observers (`Dispose`).
    fn dispose(&self) {
        self.inner.source.dispose();
    }
}

impl<T, G: PartialEq> PartialEq for Group<T, G> {
    /// Groups of the same key are equal (`Equals`).
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner) || self.inner.group_key == other.inner.group_key
    }
}

impl<T, G: Eq> Eq for Group<T, G> {}

impl<T, G: Hash> Hash for Group<T, G> {
    /// The hash of the group key (`GetHashCode`).
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.inner.group_key.hash(state);
    }
}

impl<T: Clone + PartialEq + 'static, G: fmt::Display> fmt::Display for Group<T, G> {
    /// `ToString`: `Group of {GroupKey} ({List.Count} records)`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Group of {} ({} records)", self.inner.group_key, self.inner.source.count())
    }
}

impl<T: Clone + PartialEq + 'static, G: fmt::Debug> fmt::Debug for Group<T, G> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Group of {:?} ({} records)", self.inner.group_key, self.inner.source.count())
    }
}
