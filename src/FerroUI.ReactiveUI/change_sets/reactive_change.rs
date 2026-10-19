//! `ReactiveChange<T>` (`ReactiveUI.Core/ChangeSets/ReactiveChange.cs`).

use super::ReactiveChangeReason;

/// One change of a collection (`ReactiveChange<T>`): the reason, the item, and where they apply
/// the previous item and the indexes.
///
/// The indexes are `-1` when they are not known, as in `NotifyCollectionChangedEventArgs`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ReactiveChange<T> {
    reason: ReactiveChangeReason,
    current: T,
    previous: Option<T>,
    current_index: i32,
    previous_index: i32,
}

impl<T> ReactiveChange<T> {
    /// A change (`ReactiveChange(ReactiveChangeReason, T, T?, int, int)`): `previous` is the
    /// replaced item of a [`Replace`](ReactiveChangeReason::Replace), and `previous_index` the
    /// former index of a [`Move`](ReactiveChangeReason::Move).
    pub fn new(reason: ReactiveChangeReason, current: T, previous: Option<T>, current_index: i32, previous_index: i32) -> Self {
        Self { reason, current, previous, current_index, previous_index }
    }

    /// Why the change happened (`Reason`).
    pub fn reason(&self) -> ReactiveChangeReason {
        self.reason
    }

    /// The item added, replacing, moved or refreshed (`Current`).
    pub fn current(&self) -> &T {
        &self.current
    }

    /// The replaced item of a [`Replace`](ReactiveChangeReason::Replace) (`Previous`).
    pub fn previous(&self) -> Option<&T> {
        self.previous.as_ref()
    }

    /// The index of the item, or `-1` when it is not known (`CurrentIndex`).
    pub fn current_index(&self) -> i32 {
        self.current_index
    }

    /// The former index of a [`Move`](ReactiveChangeReason::Move), or `-1` (`PreviousIndex`).
    pub fn previous_index(&self) -> i32 {
        self.previous_index
    }
}
