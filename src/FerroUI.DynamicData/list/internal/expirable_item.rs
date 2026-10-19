//! `ExpirableItem<TObject>` (`List/Internal/ExpirableItem.cs`).

use std::fmt;
use std::time::SystemTime;

/// An item, when it was added and its order of addition (`ExpirableItem<TObject>`): equal when the
/// three are.
#[derive(Clone)]
pub(crate) struct ExpirableItem<T> {
    pub(crate) item: T,
    pub(crate) expire_at: SystemTime,
    pub(crate) index: u64,
}

impl<T: PartialEq> PartialEq for ExpirableItem<T> {
    fn eq(&self, other: &Self) -> bool {
        self.item == other.item && self.expire_at == other.expire_at && self.index == other.index
    }
}

impl<T: fmt::Display> fmt::Display for ExpirableItem<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} @ {:?}", self.item, self.expire_at)
    }
}
