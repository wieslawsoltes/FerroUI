//! `IVirtualChangeSet<TObject, TKey>` and `VirtualChangeSet<TObject, TKey>`
//! (`Cache/IVirtualChangeSet.cs`, `Cache/VirtualChangeSet.cs`).

use super::{Change, ChangeSet, IChangeSet, ISortedChangeSet, KeyValueCollection, VirtualResponse};
use std::fmt;
use std::ops::Deref;

/// A sorted change set of a window of the items (`IVirtualChangeSet<TObject, TKey>`).
pub trait IVirtualChangeSet<T, K>: ISortedChangeSet<T, K> {
    /// The window (`Response`).
    fn response(&self) -> &VirtualResponse;
}

/// The changes of a window of a sorted cache, with the items of the window in order and the
/// window (`VirtualChangeSet<TObject, TKey>`).
///
/// It dereferences to its [`ChangeSet`], as upstream's derives from it.
pub struct VirtualChangeSet<T, K> {
    changes: ChangeSet<T, K>,
    sorted_items: KeyValueCollection<T, K>,
    response: VirtualResponse,
}

impl<T, K> Clone for VirtualChangeSet<T, K> {
    fn clone(&self) -> Self {
        Self { changes: self.changes.clone(), sorted_items: self.sorted_items.clone(), response: self.response }
    }
}

impl<T, K> Default for VirtualChangeSet<T, K> {
    fn default() -> Self {
        Self::empty()
    }
}

impl<T, K> VirtualChangeSet<T, K> {
    /// The changes `items` of the window `response`, which holds `sorted_items`
    /// (`VirtualChangeSet(IEnumerable<Change>, IKeyValueCollection, IVirtualResponse)`).
    pub fn new(items: ChangeSet<T, K>, sorted_items: KeyValueCollection<T, K>, response: VirtualResponse) -> Self {
        Self { changes: items, sorted_items, response }
    }

    /// A change set without changes, of no window (`VirtualChangeSet<TObject, TKey>.Empty`).
    pub fn empty() -> Self {
        Self::new(ChangeSet::empty(), KeyValueCollection::empty(), VirtualResponse::new(0, 0, 0))
    }

    /// The items of the window, sorted (`SortedItems`).
    pub fn sorted_items(&self) -> &KeyValueCollection<T, K> {
        &self.sorted_items
    }

    /// The window (`Response`).
    pub fn response(&self) -> &VirtualResponse {
        &self.response
    }

    /// The changes.
    pub fn changes(&self) -> &ChangeSet<T, K> {
        &self.changes
    }
}

impl<T, K> Deref for VirtualChangeSet<T, K> {
    type Target = ChangeSet<T, K>;

    fn deref(&self) -> &Self::Target {
        &self.changes
    }
}

impl<'a, T, K> IntoIterator for &'a VirtualChangeSet<T, K> {
    type Item = &'a Change<T, K>;
    type IntoIter = std::slice::Iter<'a, Change<T, K>>;

    fn into_iter(self) -> Self::IntoIter {
        self.changes.changes().iter()
    }
}

impl<T: 'static, K: 'static> IChangeSet<T, K> for VirtualChangeSet<T, K> {
    fn change_set(&self) -> &ChangeSet<T, K> {
        &self.changes
    }
}

impl<T: 'static, K: 'static> ISortedChangeSet<T, K> for VirtualChangeSet<T, K> {
    fn sorted_items(&self) -> &KeyValueCollection<T, K> {
        &self.sorted_items
    }
}

impl<T: 'static, K: 'static> IVirtualChangeSet<T, K> for VirtualChangeSet<T, K> {
    fn response(&self) -> &VirtualResponse {
        &self.response
    }
}

impl<T, K> PartialEq for VirtualChangeSet<T, K> {
    /// The same window and the same sorted items (`Equals`: `Response.Equals(other.Response) &&
    /// Equals(SortedItems, other.SortedItems)`, the second by reference).
    fn eq(&self, other: &Self) -> bool {
        self.response == other.response && self.sorted_items.ptr_eq(&other.sorted_items)
    }
}

impl<T, K> fmt::Display for VirtualChangeSet<T, K> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "VirtualChangeSet. Updates = {}, Response: {}, SortedItems: {}", self.changes.count(), self.response, self.sorted_items.count())
    }
}

impl<T: fmt::Debug, K: fmt::Debug> fmt::Debug for VirtualChangeSet<T, K> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VirtualChangeSet").field("changes", &self.changes).field("response", &self.response).field("sorted_items", &self.sorted_items).finish()
    }
}
