//! `CacheItemRecordingObserverAssertions` (`Utilities/CacheItemRecordingObserverAssertions.cs`).

use super::CacheItemRecordingObserver;
use std::hash::Hash;

/// Asserts that the recorded stream has no indexes: no change of any change set has a current or
/// previous index, and no item was recorded in sorted order (`ShouldNotSupportSorting(this
/// CacheItemRecordingObserver<TObject, TKey>, string because)`).
///
/// # Panics
/// When a change has an index, or a sorted item was recorded.
#[track_caller]
pub fn assert_should_not_support_sorting<T: Clone + 'static, K: Eq + Hash + Clone + 'static>(results: &CacheItemRecordingObserver<T, K>, because: &str) {
    for (number, change_set) in results.recorded_change_sets().iter().enumerate() {
        for change in change_set {
            assert_eq!(change.current_index(), None, "change set #{number}: the current index should be omitted {because}");
            assert_eq!(change.previous_index(), None, "change set #{number}: the previous index should be omitted {because}");
        }
    }
    assert!(results.recorded_items_sorted().is_empty(), "no item should be recorded in sorted order {because}");
}
