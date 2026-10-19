//! `Change<TObject, TKey>` (`Cache/Change.cs`).

use super::ChangeReason;
use std::fmt;

/// A change of one item of a cache (`Change<TObject, TKey>`): its reason, its key, the item and,
/// for an update, the item it replaced; the indexes for a sorted collection.
///
/// Deviation (DEVIATIONS.md, DynamicData): the indexes are `Option<usize>`, where upstream writes
/// `-1` for no index.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Change<T, K> {
    reason: ChangeReason,
    key: K,
    current: T,
    previous: Option<T>,
    current_index: Option<usize>,
    previous_index: Option<usize>,
}

impl<T, K> Change<T, K> {
    /// A change without a previous item (`Change(ChangeReason, TKey, TObject, int index = -1)`).
    ///
    /// # Panics
    /// For `ChangeReason::Update`, which needs the previous item (`ArgumentException`).
    pub fn new(reason: ChangeReason, key: K, current: T) -> Self {
        Self::with_previous(reason, key, current, None, None, None)
    }

    /// A change with the index of the item (`Change(ChangeReason, TKey, TObject, int index)`).
    pub fn with_index(reason: ChangeReason, key: K, current: T, index: Option<usize>) -> Self {
        Self::with_previous(reason, key, current, None, index, None)
    }

    /// A change with every part (`Change(ChangeReason, TKey, TObject, in Optional<TObject>, int,
    /// int)`).
    ///
    /// # Panics
    /// For `ChangeReason::Add` with a previous item, and for `ChangeReason::Update` without one
    /// (`ArgumentException`).
    pub fn with_previous(
        reason: ChangeReason,
        key: K,
        current: T,
        previous: Option<T>,
        current_index: Option<usize>,
        previous_index: Option<usize>,
    ) -> Self {
        assert!(!(reason == ChangeReason::Add && previous.is_some()), "For ChangeReason.Add, a previous value cannot be specified");
        assert!(!(reason == ChangeReason::Update && previous.is_none()), "For ChangeReason.Update, must supply previous value");
        Self { reason, key, current, previous, current_index, previous_index }
    }

    /// An update of `key` from `previous` to `current` (`Change(ChangeReason.Update, key, current,
    /// previous)`).
    pub fn update(key: K, current: T, previous: T) -> Self {
        Self::with_previous(ChangeReason::Update, key, current, Some(previous), None, None)
    }

    /// A move within a sorted collection (`Change(TKey, TObject, int currentIndex, int
    /// previousIndex)`).
    pub fn moved(key: K, current: T, current_index: usize, previous_index: usize) -> Self {
        Self {
            reason: ChangeReason::Moved,
            key,
            current,
            previous: None,
            current_index: Some(current_index),
            previous_index: Some(previous_index),
        }
    }

    /// The key of the item (`Key`).
    pub fn key(&self) -> &K {
        &self.key
    }

    /// The reason of the change (`Reason`).
    pub fn reason(&self) -> ChangeReason {
        self.reason
    }

    /// The item (`Current`).
    pub fn current(&self) -> &T {
        &self.current
    }

    /// The item that an update replaced (`Previous`).
    pub fn previous(&self) -> Option<&T> {
        self.previous.as_ref()
    }

    /// The index of the item in a sorted collection (`CurrentIndex`; `-1` upstream when none).
    pub fn current_index(&self) -> Option<usize> {
        self.current_index
    }

    /// The previous index of the item (`PreviousIndex`).
    pub fn previous_index(&self) -> Option<usize> {
        self.previous_index
    }

    /// The parts of the change, taken apart.
    pub fn into_parts(self) -> (ChangeReason, K, T, Option<T>) {
        (self.reason, self.key, self.current, self.previous)
    }
}

impl<T: fmt::Debug, K: fmt::Debug> fmt::Debug for Change<T, K> {
    /// `ToString`: the reason, the key, the item and the previous item.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}, Key: {:?}, Current: {:?}, Previous: {:?}", self.reason, self.key, self.current, self.previous)
    }
}
