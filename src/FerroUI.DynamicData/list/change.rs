//! `Change<T>` (`List/Change.cs`).

use super::{ChangeType, ItemChange, ListChangeReason, RangeChange};
use std::fmt;

/// A change of a list (`Change<T>`): of one item, or of a range of items.
///
/// Upstream's change of one item has an empty range (`RangeChange<T>.Empty`), and a change of a
/// range has an empty item (`ItemChange<T>.Empty`, whose item is the default value of `T`); here
/// [`range`](Self::range) is empty for the change of an item, and the change of a range has no
/// item ([`try_item`](Self::try_item)).
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Change<T> {
    reason: ListChangeReason,
    item: Option<ItemChange<T>>,
    range: RangeChange<T>,
}

impl<T> Change<T> {
    /// The change of one item (`Change(ListChangeReason, T, int index = -1)`).
    pub fn new(reason: ListChangeReason, current: T, index: Option<usize>) -> Self {
        Self::with_previous(reason, current, None, index, None)
    }

    /// The change of a range (`Change(ListChangeReason, IEnumerable<T>, int index = -1)`).
    ///
    /// # Panics
    /// For a reason of one item (`IndexOutOfRangeException`).
    pub fn range_change(reason: ListChangeReason, items: Vec<T>, index: Option<usize>) -> Self {
        assert!(reason.change_type() == ChangeType::Range, "ListChangeReason must be a range type for a range change");
        Self { reason, item: None, range: RangeChange::new(items, index) }
    }

    /// A move (`Change(T, int currentIndex, int previousIndex)`).
    pub fn moved(current: T, current_index: usize, previous_index: usize) -> Self {
        Self {
            reason: ListChangeReason::Moved,
            item: Some(ItemChange::new(ListChangeReason::Moved, current, None, Some(current_index), Some(previous_index))),
            range: RangeChange::default(),
        }
    }

    /// The change of one item with every part (`Change(ListChangeReason, T, in Optional<T>, int,
    /// int)`).
    ///
    /// # Panics
    /// For `Add` with a previous item, `Replace` without one, and `Refresh` without an index
    /// (`ArgumentException`).
    pub fn with_previous(reason: ListChangeReason, current: T, previous: Option<T>, current_index: Option<usize>, previous_index: Option<usize>) -> Self {
        assert!(!(reason == ListChangeReason::Add && previous.is_some()), "For ListChangeReason.Add, a previous value cannot be specified");
        assert!(!(reason == ListChangeReason::Replace && previous.is_none()), "For ListChangeReason.Replace, must supply previous value");
        assert!(!(reason == ListChangeReason::Refresh && current_index.is_none()), "For ListChangeReason.Refresh, must supply an index");
        Self { reason, item: Some(ItemChange::new(reason, current, previous, current_index, previous_index)), range: RangeChange::default() }
    }

    /// The reason (`Reason`).
    pub fn reason(&self) -> ListChangeReason {
        self.reason
    }

    /// Whether the change is of an item or of a range (`Type`).
    pub fn change_type(&self) -> ChangeType {
        self.reason.change_type()
    }

    /// The change of the item (`Item`).
    ///
    /// # Panics
    /// For the change of a range, whose item upstream is empty.
    pub fn item(&self) -> &ItemChange<T> {
        self.item.as_ref().expect("the change of a range has no item change")
    }

    /// The change of the item, `None` for the change of a range.
    pub fn try_item(&self) -> Option<&ItemChange<T>> {
        self.item.as_ref()
    }

    /// The range (`Range`): empty for the change of an item.
    pub fn range(&self) -> &RangeChange<T> {
        &self.range
    }

    pub(crate) fn range_mut(&mut self) -> &mut RangeChange<T> {
        &mut self.range
    }
}

impl<T> fmt::Debug for Change<T>
where
    T: fmt::Debug,
{
    /// `ToString`, with the item or the range.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.item {
            Some(item) => write!(f, "{:?}. {:?}", self.reason, item),
            None => write!(f, "{:?}. {} changes {:?}", self.reason, self.range.count(), self.range.items()),
        }
    }
}
