//! `ItemWithValue<TObject, TValue>` (`Kernel/ItemWithValue.cs`).

use std::fmt;

/// An item and a value that goes with it (`ItemWithValue<TObject, TValue>`): equal when both
/// parts are.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ItemWithValue<T, V> {
    item: T,
    value: V,
}

impl<T, V> ItemWithValue<T, V> {
    /// The pair (`ItemWithValue(TObject, TValue)`).
    pub fn new(item: T, value: V) -> Self {
        Self { item, value }
    }

    /// The item (`Item`).
    pub fn item(&self) -> &T {
        &self.item
    }

    /// The value (`Value`).
    pub fn value(&self) -> &V {
        &self.value
    }

    /// The item and the value.
    pub fn into_parts(self) -> (T, V) {
        (self.item, self.value)
    }
}

impl<T: fmt::Display, V: fmt::Display> fmt::Display for ItemWithValue<T, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.item, self.value)
    }
}
