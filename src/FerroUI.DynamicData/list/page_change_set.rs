//! `IPageChangeSet<T>` and `PageChangeSet<T>` (`List/IPageChangeSet.cs`, `List/PageChangeSet.cs`).

use super::{ChangeSet, IChangeSet};
use crate::cache::PageResponse;
use std::fmt;
use std::ops::Deref;

/// A change set of a page of a list, with the description of the page (`IPageChangeSet<T>`).
pub trait IPageChangeSet<T>: IChangeSet<T> {
    /// The page (`Response`).
    fn response(&self) -> &PageResponse;
}

/// The changes of a page of a list, and the page (`PageChangeSet<T>`).
///
/// It dereferences to its [`ChangeSet`], as upstream's forwards the members of `IChangeSet<T>`.
pub struct PageChangeSet<T> {
    changes: ChangeSet<T>,
    response: PageResponse,
}

impl<T> Clone for PageChangeSet<T> {
    fn clone(&self) -> Self {
        Self { changes: self.changes.clone(), response: self.response }
    }
}

impl<T> PageChangeSet<T> {
    /// The changes of the page `response` (`PageChangeSet(IChangeSet<T>, IPageResponse)`).
    pub fn new(changes: ChangeSet<T>, response: PageResponse) -> Self {
        Self { changes, response }
    }

    /// The page (`Response`).
    pub fn response(&self) -> &PageResponse {
        &self.response
    }

    /// The changes.
    pub fn changes(&self) -> &ChangeSet<T> {
        &self.changes
    }
}

impl<T> Deref for PageChangeSet<T> {
    type Target = ChangeSet<T>;

    fn deref(&self) -> &Self::Target {
        &self.changes
    }
}

impl<'a, T> IntoIterator for &'a PageChangeSet<T> {
    type Item = &'a super::Change<T>;
    type IntoIter = std::slice::Iter<'a, super::Change<T>>;

    fn into_iter(self) -> Self::IntoIter {
        self.changes.changes().iter()
    }
}

impl<T: 'static> IChangeSet<T> for PageChangeSet<T> {
    fn change_set(&self) -> &ChangeSet<T> {
        &self.changes
    }
}

impl<T: 'static> IPageChangeSet<T> for PageChangeSet<T> {
    fn response(&self) -> &PageResponse {
        &self.response
    }
}

impl<T: PartialEq> PartialEq for PageChangeSet<T> {
    fn eq(&self, other: &Self) -> bool {
        self.changes == other.changes && self.response == other.response
    }
}

impl<T: fmt::Debug> fmt::Debug for PageChangeSet<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PageChangeSet").field("changes", &self.changes).field("response", &self.response).finish()
    }
}
