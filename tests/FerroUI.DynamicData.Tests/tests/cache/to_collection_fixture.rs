//! `ToCollectionFixture` of caches (`DynamicData.Tests/Cache/ToCollectionFixture.cs`).

use crate::utilities::{assert_equivalent, ObservableExtensions, StreamCompletionStrategy, TestSourceCache};
use ferroui_dynamic_data::{ChangeSet, IConnectableCache, IObservableCache, ObservableCacheEx, SourceCache};
use ferroui_reactive::subjects::{Subject, SubjectBase};
use ferroui_system::IDisposable;
use std::rc::Rc;

/// `TestItem`: a record with an id and a version.
#[derive(Clone, Debug, PartialEq)]
struct TestItem {
    id: i32,
    version: i32,
}

impl TestItem {
    fn new(id: i32) -> Self {
        Self { id, version: 0 }
    }

    fn select_id(item: &TestItem) -> i32 {
        item.id
    }
}

#[test]
fn when_changes_are_made_result_matches_source_and_prior_results_are_not_mutated() {
    // Setup
    let source = SourceCache::new(TestItem::select_id);

    // UUT Initialization
    let mut prior_results: Vec<Vec<TestItem>> = Vec::new();
    let (subscription, results) = source.connect().to_collection().record_values();

    assert!(results.error().is_none(), "no errors should have occurred");
    // Upstream disabled due to an existing defect: an initial snapshot should always be published.
    assert!(!results.has_completed(), "the source has not completed");

    let check = |prior_results: &mut Vec<Vec<TestItem>>| {
        assert!(results.error().is_none(), "no errors should have occurred");
        let recorded = results.recorded_values();
        assert_eq!(recorded.len() - prior_results.len(), 1, "a single source operation was performed");
        assert_equivalent(recorded.last().expect("a snapshot"), &source.items());
        assert!(!results.has_completed(), "the source has not completed");
        for (result, prior_result) in recorded.iter().zip(prior_results.iter()) {
            assert_equivalent(result, prior_result);
        }
        prior_results.push(recorded.last().cloned().expect("a snapshot"));
    };

    // UUT Action (add items)
    source.edit(|updater| updater.add_or_update_items(&mut vec![TestItem::new(1), TestItem::new(2), TestItem::new(3)].into_iter()));
    check(&mut prior_results);

    // UUT Action (replace items)
    source.edit(|updater| {
        updater.add_or_update_items(&mut vec![TestItem { id: 1, version: 1 }, TestItem { id: 2, version: 1 }, TestItem { id: 3, version: 1 }].into_iter())
    });
    check(&mut prior_results);

    // UUT Action (remove items)
    let keys = source.keys();
    source.edit(|updater| updater.remove_keys(&mut keys.into_iter()));
    check(&mut prior_results);

    subscription.dispose();
    source.dispose();
}

fn when_source_completes_completion_propagates(completion_strategy: StreamCompletionStrategy) {
    // Setup
    let source = TestSourceCache::new(TestItem::select_id);

    // UUT Initialization & Action
    if completion_strategy == StreamCompletionStrategy::Immediate {
        source.complete();
    }

    let (subscription, results) = source.connect_with(None, false).to_collection().record_values();

    if completion_strategy == StreamCompletionStrategy::Asynchronous {
        source.complete();
    }

    assert!(results.error().is_none());
    assert_eq!(results.recorded_values().len(), 1, "an initial snapshot should always be published");
    assert!(results.recorded_values().last().expect("a snapshot").is_empty(), "no items were added to the source");
    assert!(results.has_completed(), "the source has completed");
    subscription.dispose();
}

#[test]
fn when_source_completes_completion_propagates_case1() {
    when_source_completes_completion_propagates(StreamCompletionStrategy::Asynchronous);
}

#[test]
fn when_source_completes_completion_propagates_case2() {
    when_source_completes_completion_propagates(StreamCompletionStrategy::Immediate);
}

fn when_source_fails_error_propagates(completion_strategy: StreamCompletionStrategy) {
    // Setup
    let source = TestSourceCache::new(TestItem::select_id);

    // UUT Initialization & Action
    let error: ferroui_system::ObservableError = Rc::new(std::io::Error::other("Test"));
    if completion_strategy == StreamCompletionStrategy::Immediate {
        source.set_error(error.clone());
    }

    let (subscription, results) = source.connect().to_collection().record_values();

    if completion_strategy == StreamCompletionStrategy::Asynchronous {
        source.set_error(error.clone());
    }

    let recorded_error = results.error().expect("errors should propagate");
    assert!(Rc::ptr_eq(&recorded_error, &error), "errors should propagate");
    // Upstream disabled due to an existing defect: an initial snapshot should always be published.
    subscription.dispose();
}

#[test]
fn when_source_fails_error_propagates_case1() {
    when_source_fails_error_propagates(StreamCompletionStrategy::Asynchronous);
}

#[test]
fn when_source_fails_error_propagates_case2() {
    when_source_fails_error_propagates(StreamCompletionStrategy::Immediate);
}

// Not ported: WhenSourceIsNull_ThrowsException - argument validation.

#[test]
fn when_subscription_is_disposed_subscription_disposal_propagates() {
    // Setup
    let source = Subject::<ChangeSet<TestItem, i32>>::new();

    // UUT Initialization
    let (subscription, results) = source.as_observable().to_collection().record_values();

    assert!(results.error().is_none());
    // Upstream disabled due to an existing defect: an initial snapshot should always be published.
    assert!(!results.has_completed(), "the source has not completed");

    // UUT Action
    subscription.dispose();

    assert!(!source.has_observers(), "subscription disposal should propagate to the source");
}
