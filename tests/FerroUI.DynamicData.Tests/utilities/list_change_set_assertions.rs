//! `ListChangeSetAssertions` (`Utilities/ListChangeSetAssertions.cs`).

use super::assert_equivalent_because;
use ferroui_dynamic_data::list::{ChangeSet, ListChangeReason};
use std::fmt::Debug;

/// Asserts that the refreshes of `change_set` are of `expected_items`, in any order
/// (`ShouldHaveRefreshed(this IChangeSet<T>, IEnumerable<T>, string because)`).
///
/// # Panics
/// When they are not.
#[track_caller]
pub fn assert_list_should_have_refreshed<T: Clone + PartialEq + Debug>(change_set: &ChangeSet<T>, expected_items: &[T], because: &str) {
    let refreshed: Vec<T> = change_set.iter().filter(|change| change.reason() == ListChangeReason::Refresh).map(|change| change.item().current().clone()).collect();
    assert_equivalent_because(&refreshed, expected_items, because);
}
