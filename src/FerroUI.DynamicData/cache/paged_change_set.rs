//! `IPagedChangeSet<TObject, TKey>` and `PagedChangeSet<TObject, TKey>`
//! (`Cache/IPagedChangeSet.cs`, `Cache/PagedChangeSet.cs`).

use super::{Change, ChangeSet, IChangeSet, ISortedChangeSet, KeyValueCollection, PageResponse};
use std::fmt;
use std::ops::Deref;

/// A sorted change set of one page of the items (`IPagedChangeSet<TObject, TKey>`).
pub trait IPagedChangeSet<T, K>: ISortedChangeSet<T, K> {
    /// The page (`Response`).
    fn response(&self) -> &PageResponse;
}

/// The changes of one page of a sorted cache, with the items of the page in order and the page
/// (`PagedChangeSet<TObject, TKey>`).
///
/// It dereferences to its [`ChangeSet`], as upstream's derives from it.
pub struct PagedChangeSet<T, K> {
    changes: ChangeSet<T, K>,
    sorted_items: KeyValueCollection<T, K>,
    response: PageResponse,
}

impl<T, K> Clone for PagedChangeSet<T, K> {
    fn clone(&self) -> Self {
        Self { changes: self.changes.clone(), sorted_items: self.sorted_items.clone(), response: self.response }
    }
}

impl<T, K> Default for PagedChangeSet<T, K> {
    fn default() -> Self {
        Self::empty()
    }
}

impl<T, K> PagedChangeSet<T, K> {
    /// The changes `updates` of the page `response`, which holds `sorted_items`
    /// (`PagedChangeSet(IKeyValueCollection, IEnumerable<Change>, IPageResponse)`).
    pub fn new(sorted_items: KeyValueCollection<T, K>, updates: ChangeSet<T, K>, response: PageResponse) -> Self {
        Self { changes: updates, sorted_items, response }
    }

    /// A change set without changes, of no page (`PagedChangeSet<TObject, TKey>.Empty`).
    pub fn empty() -> Self {
        Self::new(KeyValueCollection::empty(), ChangeSet::empty(), PageResponse::new(0, 0, 0, 0))
    }

    /// The items of the page, sorted (`SortedItems`).
    pub fn sorted_items(&self) -> &KeyValueCollection<T, K> {
        &self.sorted_items
    }

    /// The page (`Response`).
    pub fn response(&self) -> &PageResponse {
        &self.response
    }

    /// The changes.
    pub fn changes(&self) -> &ChangeSet<T, K> {
        &self.changes
    }
}

impl<T, K> Deref for PagedChangeSet<T, K> {
    type Target = ChangeSet<T, K>;

    fn deref(&self) -> &Self::Target {
        &self.changes
    }
}

impl<'a, T, K> IntoIterator for &'a PagedChangeSet<T, K> {
    type Item = &'a Change<T, K>;
    type IntoIter = std::slice::Iter<'a, Change<T, K>>;

    fn into_iter(self) -> Self::IntoIter {
        self.changes.changes().iter()
    }
}

impl<T: 'static, K: 'static> IChangeSet<T, K> for PagedChangeSet<T, K> {
    fn change_set(&self) -> &ChangeSet<T, K> {
        &self.changes
    }
}

impl<T: 'static, K: 'static> ISortedChangeSet<T, K> for PagedChangeSet<T, K> {
    fn sorted_items(&self) -> &KeyValueCollection<T, K> {
        &self.sorted_items
    }
}

impl<T: 'static, K: 'static> IPagedChangeSet<T, K> for PagedChangeSet<T, K> {
    fn response(&self) -> &PageResponse {
        &self.response
    }
}

impl<T: PartialEq, K: PartialEq> PartialEq for PagedChangeSet<T, K> {
    /// The same sorted items (`Equals`: `SortedItems.SequenceEqual(other.SortedItems)`).
    fn eq(&self, other: &Self) -> bool {
        self.sorted_items == other.sorted_items
    }
}

impl<T, K> fmt::Display for PagedChangeSet<T, K> {
    /// `ToString`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PagedChangeSet. Updates = {}, Response: {}, SortedItems: {}", self.changes.count(), self.response, self.sorted_items.count())
    }
}

impl<T: fmt::Debug, K: fmt::Debug> fmt::Debug for PagedChangeSet<T, K> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PagedChangeSet").field("changes", &self.changes).field("response", &self.response).field("sorted_items", &self.sorted_items).finish()
    }
}
