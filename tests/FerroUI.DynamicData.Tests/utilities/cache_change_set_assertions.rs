//! `CacheChangeSetAssertions` (`Utilities/CacheChangeSetAssertions.cs`).

use super::assert_equivalent_because;
use ferroui_dynamic_data::cache::{ChangeReason, ChangeSet};
use std::fmt::Debug;

/// Asserts that the refreshes of `change_set` are of `expected_items`, in any order
/// (`ShouldHaveRefreshed(this IChangeSet<TObject, TKey>, IEnumerable<TObject>, string because)`).
///
/// # Panics
/// When they are not.
#[track_caller]
pub fn assert_cache_should_have_refreshed<T: Clone + PartialEq + Debug, K>(change_set: &ChangeSet<T, K>, expected_items: &[T], because: &str) {
    let refreshed: Vec<T> = change_set.iter().filter(|change| change.reason() == ChangeReason::Refresh).map(|change| change.current().clone()).collect();
    assert_equivalent_because(&refreshed, expected_items, because);
}
