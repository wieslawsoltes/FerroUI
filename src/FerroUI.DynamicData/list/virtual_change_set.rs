//! `IVirtualChangeSet<T>` and `VirtualChangeSet<T>` (`List/IVirtualChangeSet.cs`, `List/VirtualChangeSet.cs`).

use super::{ChangeSet, IChangeSet};
use crate::cache::VirtualResponse;
use std::fmt;
use std::ops::Deref;

/// A change set of a window of a list, with the description of the window (`IVirtualChangeSet<T>`).
pub trait IVirtualChangeSet<T>: IChangeSet<T> {
    /// The window (`Response`).
    fn response(&self) -> &VirtualResponse;
}

/// The changes of a window of a list, and the window (`VirtualChangeSet<T>`).
///
/// It dereferences to its [`ChangeSet`], as upstream's forwards the members of `IChangeSet<T>`.
pub struct VirtualChangeSet<T> {
    changes: ChangeSet<T>,
    response: VirtualResponse,
}

impl<T> Clone for VirtualChangeSet<T> {
    fn clone(&self) -> Self {
        Self { changes: self.changes.clone(), response: self.response }
    }
}

impl<T> VirtualChangeSet<T> {
    /// The changes of the window `response` (`VirtualChangeSet(IChangeSet<T>, IVirtualResponse)`).
    pub fn new(changes: ChangeSet<T>, response: VirtualResponse) -> Self {
        Self { changes, response }
    }

    /// The window (`Response`).
    pub fn response(&self) -> &VirtualResponse {
        &self.response
    }

    /// The changes.
    pub fn changes(&self) -> &ChangeSet<T> {
        &self.changes
    }
}

impl<T> Deref for VirtualChangeSet<T> {
    type Target = ChangeSet<T>;

    fn deref(&self) -> &Self::Target {
        &self.changes
    }
}

impl<'a, T> IntoIterator for &'a VirtualChangeSet<T> {
    type Item = &'a super::Change<T>;
    type IntoIter = std::slice::Iter<'a, super::Change<T>>;

    fn into_iter(self) -> Self::IntoIter {
        self.changes.changes().iter()
    }
}

impl<T: 'static> IChangeSet<T> for VirtualChangeSet<T> {
    fn change_set(&self) -> &ChangeSet<T> {
        &self.changes
    }
}

impl<T: 'static> IVirtualChangeSet<T> for VirtualChangeSet<T> {
    fn response(&self) -> &VirtualResponse {
        &self.response
    }
}

impl<T: PartialEq> PartialEq for VirtualChangeSet<T> {
    fn eq(&self, other: &Self) -> bool {
        self.changes == other.changes && self.response == other.response
    }
}

impl<T: fmt::Debug> fmt::Debug for VirtualChangeSet<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VirtualChangeSet").field("changes", &self.changes).field("response", &self.response).finish()
    }
}
