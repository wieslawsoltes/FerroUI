//! The comparisons of FluentAssertions that the assertions of the tests use (`BeEquivalentTo`):
//! not an upstream file of the utilities, the replacement of the assertion library
//! (docs/porting/reactive.md, section 5.7).

use std::fmt::Debug;

/// Whether `actual` and `expected` hold the same items, each as often, in any order
/// (`BeEquivalentTo` of collections of objects that override `Equals`).
pub fn is_equivalent<T: PartialEq>(actual: &[T], expected: &[T]) -> bool {
    if actual.len() != expected.len() {
        return false;
    }
    let mut matched = vec![false; expected.len()];
    actual.iter().all(|item| match (0..expected.len()).find(|&index| !matched[index] && expected[index] == *item) {
        Some(index) => {
            matched[index] = true;
            true
        }
        None => false,
    })
}

/// Asserts that `actual` and `expected` hold the same items in any order
/// (`actual.Should().BeEquivalentTo(expected)`).
///
/// # Panics
/// When they do not, with both collections in the message.
#[track_caller]
pub fn assert_equivalent<T: PartialEq + Debug>(actual: &[T], expected: &[T]) {
    assert_equivalent_because(actual, expected, "");
}

/// [`assert_equivalent`] with the reason of upstream's `because` in the message.
#[track_caller]
pub fn assert_equivalent_because<T: PartialEq + Debug>(actual: &[T], expected: &[T], because: &str) {
    assert!(is_equivalent(actual, expected), "Expected the collection to be equivalent to {expected:?}{}{because}, but found {actual:?}.", if because.is_empty() { "" } else { " because " });
}

/// Asserts that `actual` and `expected` hold the same items in the same order
/// (`BeEquivalentTo(expected, options => options.WithStrictOrdering())`, `Equal(expected)`).
///
/// # Panics
/// When they do not.
#[track_caller]
pub fn assert_equivalent_strict<T: PartialEq + Debug>(actual: &[T], expected: &[T]) {
    assert_eq!(actual, expected, "Expected the collection to be equal to {expected:?} in order, but found {actual:?}.");
}
