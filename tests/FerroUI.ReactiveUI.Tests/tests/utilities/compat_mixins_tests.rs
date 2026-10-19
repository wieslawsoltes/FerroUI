//! `CompatMixinsTests` (`ReactiveUI.Tests/Utilities/CompatMixinsTests.cs`).
//!
//! `SkipLast` is `skip_last_items`, so that it does not shadow the `skip_last` of other traits.

use crate::mocks::CompatMixins;

#[test]
fn run_processes_all_items() {
    const MULTIPLIER: i32 = 2;
    const SEQUENCE_LENGTH: i32 = 5;
    let items = 1..=SEQUENCE_LENGTH;
    let mut processed_items = Vec::new();

    items.run(|x| processed_items.push(x * MULTIPLIER));

    assert_eq!(processed_items, (1..=SEQUENCE_LENGTH).map(|x| x * MULTIPLIER).collect::<Vec<_>>());
}

#[test]
fn skip_last_count_greater_than_collection_returns_empty() {
    const SKIP_COUNT: usize = 10;
    const SEQUENCE_LENGTH: i32 = 3;
    let items = 1..=SEQUENCE_LENGTH;

    let result = items.skip_last_items(SKIP_COUNT);

    assert!(result.is_empty());
}

#[test]
fn skip_last_removes_last_n_items() {
    const SKIP_COUNT: usize = 2;
    const SEQUENCE_LENGTH: i32 = 5;
    const EXPECTED_LENGTH: i32 = 3;
    let items = 1..=SEQUENCE_LENGTH;

    let result = items.skip_last_items(SKIP_COUNT);

    assert_eq!(result, (1..=EXPECTED_LENGTH).collect::<Vec<_>>());
}

#[test]
fn skip_last_zero_count_returns_all_items() {
    const SEQUENCE_LENGTH: i32 = 5;
    let items = 1..=SEQUENCE_LENGTH;

    let result = items.skip_last_items(0);

    assert_eq!(result, (1..=SEQUENCE_LENGTH).collect::<Vec<_>>());
}
