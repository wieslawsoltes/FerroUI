//! `FilterFixture.DynamicPredicate.UnitTests`
//! (`DynamicData.Tests/Cache/FilterFixture.DynamicPredicate.UnitTests.cs`): the filter of caches
//! by a stream of predicates (`filter_dynamic`, `filter_dynamic_with`), with the tests of
//! `FilterFixture.Base`.

use super::filter_fixture::{EmptyChangesetPolicy, Item, ItemPredicate};
use crate::utilities::{assert_equivalent_because, assert_should_not_support_sorting, CacheObservableExtensions, ObservableExtensions, StreamCompletionStrategy, TestSourceCache};
use ferroui_dynamic_data::cache::{ChangeSet, IConnectableCache, IObservableCache, ObservableCacheEx};
use ferroui_reactive::subjects::{BehaviorSubject, Subject, SubjectBase};
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_reactive_testing::new_error;
use ferroui_system::{IDisposable, IObservable, IObserver};
use std::rc::Rc;

fn included_items(items: Vec<Rc<Item>>) -> Vec<Rc<Item>> {
    items.into_iter().filter(Item::filter_by_is_included).collect()
}

fn changes_are_made_before_initial_predicate_changed_value_items_are_excluded(empty_changeset_policy: EmptyChangesetPolicy) {
    // Setup
    let source = TestSourceCache::new(Item::select_id);

    // UUT Initialization
    let (subscription, results) = source
        .connect()
        .filter_dynamic_with(Observable::never::<ItemPredicate>(), empty_changeset_policy == EmptyChangesetPolicy::SuppressEmptyChangesets)
        .validate_synchronization()
        .validate_cache_change_sets(Item::select_id)
        .record_cache_items();

    assert!(results.error().is_none());
    assert!(results.recorded_change_sets().is_empty(), "no source operations were performed");
    assert!(!results.has_completed(), "the source has not completed");

    // UUT Action
    // Add changes
    source.edit(|updater| updater.add_or_update_items(&mut [Item::new(1, true), Item::new(2, true), Item::new(3, false), Item::new(4, false)].into_iter()));

    // Refresh changes, with no item mutations.
    source.edit(|updater| updater.refresh_all());

    // Refresh changes, with item mutations affecting filtering.
    for item in source.items() {
        item.set_is_included(!item.is_included());
    }
    source.edit(|updater| updater.refresh_all());

    // Remove changes
    source.edit(|updater| updater.remove_keys(&mut [2, 3].into_iter()));

    // Update changes, not affecting filtering
    source.edit(|updater| updater.add_or_update_items(&mut [Item::new(1, false), Item::new(4, true)].into_iter()));

    // Update changes, affecting filtering
    source.edit(|updater| updater.add_or_update_items(&mut [Item::new(1, true), Item::new(4, false)].into_iter()));

    if empty_changeset_policy == EmptyChangesetPolicy::IncludeEmptyChangesets {
        assert_eq!(results.recorded_change_sets().len(), 6, "6 source operations were performed");
        assert!(results.recorded_items().is_empty(), "the predicate has not initialized");
    } else {
        assert!(results.recorded_change_sets().is_empty(), "empty changesets should be suppressed");
    }
    assert!(!results.has_completed(), "the source has not completed");

    // Final verification
    assert_should_not_support_sorting(&results, "sorting is not supported by filter operators");

    subscription.dispose();
    source.dispose();
}

#[test]
fn changes_are_made_before_initial_predicate_changed_value_items_are_excluded_case1() {
    changes_are_made_before_initial_predicate_changed_value_items_are_excluded(EmptyChangesetPolicy::IncludeEmptyChangesets);
}

#[test]
fn changes_are_made_before_initial_predicate_changed_value_items_are_excluded_case2() {
    changes_are_made_before_initial_predicate_changed_value_items_are_excluded(EmptyChangesetPolicy::SuppressEmptyChangesets);
}

fn predicate_changed_changes_items_are_re_filtered(empty_changeset_policy: EmptyChangesetPolicy) {
    // Setup
    let source = TestSourceCache::new(Item::select_id);
    let predicate_changed = BehaviorSubject::<ItemPredicate>::new(Rc::new(Item::filter_by_is_included));

    source.edit(|updater| {
        updater.add_or_update_items(&mut [Item::new(1, true), Item::new(2, true), Item::new(3, true), Item::new(4, false), Item::new(5, false), Item::new(6, false)].into_iter())
    });

    // UUT Initialization
    let (subscription, results) = source
        .connect()
        .filter_dynamic_with(predicate_changed.as_observable(), empty_changeset_policy == EmptyChangesetPolicy::SuppressEmptyChangesets)
        .validate_synchronization()
        .validate_cache_change_sets(Item::select_id)
        .record_cache_items();

    assert!(results.error().is_none());
    assert_eq!(results.recorded_change_sets().len(), 1, "an initial changeset was published");
    assert_equivalent_because(&results.recorded_items(), &included_items(source.items()), "all matching items should have propagated");
    assert!(!results.has_completed(), "the source has not completed");

    // UUT Action
    predicate_changed.on_next(Rc::new(Item::filter_by_even_id));

    assert!(results.error().is_none());
    assert_eq!(results.recorded_change_sets().iter().skip(1).count(), 1, "1 predicate change occurred");
    let even: Vec<Rc<Item>> = source.items().into_iter().filter(Item::filter_by_even_id).collect();
    assert_equivalent_because(&results.recorded_items(), &even, "newly-matching items should have been added, and newly-excluded items should have been removed");
    assert!(!results.has_completed(), "the source has not completed");

    // Final verification
    assert_should_not_support_sorting(&results, "sorting is not supported by filter operators");

    subscription.dispose();
    predicate_changed.dispose();
    source.dispose();
}

#[test]
fn predicate_changed_changes_items_are_re_filtered_case1() {
    predicate_changed_changes_items_are_re_filtered(EmptyChangesetPolicy::IncludeEmptyChangesets);
}

#[test]
fn predicate_changed_changes_items_are_re_filtered_case2() {
    predicate_changed_changes_items_are_re_filtered(EmptyChangesetPolicy::SuppressEmptyChangesets);
}

fn predicate_changed_completes_after_initial_value_completion_waits_for_source_completion(completion_strategy: StreamCompletionStrategy) {
    // Setup
    let source = TestSourceCache::new(Item::select_id);

    source.edit(|updater| {
        updater.add_or_update_items(&mut [Item::new(1, true), Item::new(2, true), Item::new(3, true), Item::new(4, false), Item::new(5, false), Item::new(6, false)].into_iter())
    });

    let subject = Subject::<ItemPredicate>::new();
    let predicate_changed: Rc<dyn IObservable<ItemPredicate>> = if completion_strategy == StreamCompletionStrategy::Asynchronous {
        subject.as_observable()
    } else {
        Observable::return_::<ItemPredicate>(Rc::new(Item::filter_by_is_included))
    };

    // UUT Initialization & Action
    let (subscription, results) = source
        .connect()
        .filter_dynamic(predicate_changed)
        .validate_synchronization()
        .validate_cache_change_sets(|item: &Rc<Item>| item.id())
        .record_cache_items();

    if completion_strategy == StreamCompletionStrategy::Asynchronous {
        subject.on_next(Rc::new(Item::filter_by_is_included));
        subject.on_completed();
    }

    assert!(results.error().is_none());
    assert_eq!(results.recorded_change_sets().len(), 1, "an initial predicate, was published");
    assert_equivalent_because(&results.recorded_items(), &included_items(source.items()), "all matching items should have propagated");
    assert!(!results.has_completed(), "changes could still be generated by the source");

    // UUT Action
    source.complete();

    assert!(results.error().is_none());
    if completion_strategy == StreamCompletionStrategy::Asynchronous {
        assert_eq!(results.recorded_change_sets().iter().skip(2).count(), 0, "no source operations were performed");
    } else {
        assert_eq!(results.recorded_change_sets().iter().skip(1).count(), 0, "no source operations were performed");
    }
    assert!(results.has_completed(), "all source streams have completed");

    // Final verification
    assert_should_not_support_sorting(&results, "sorting is not supported by filter operators");

    subscription.dispose();
}

#[test]
fn predicate_changed_completes_after_initial_value_completion_waits_for_source_completion_case1() {
    predicate_changed_completes_after_initial_value_completion_waits_for_source_completion(StreamCompletionStrategy::Asynchronous);
}

#[test]
fn predicate_changed_completes_after_initial_value_completion_waits_for_source_completion_case2() {
    predicate_changed_completes_after_initial_value_completion_waits_for_source_completion(StreamCompletionStrategy::Immediate);
}

fn predicate_changed_completes_before_initial_value_completion_propagates_if_empty_changesets_are_suppressed(
    completion_strategy: StreamCompletionStrategy,
    empty_changeset_policy: EmptyChangesetPolicy,
) {
    // Setup
    let source = TestSourceCache::new(Item::select_id);

    let subject = Subject::<ItemPredicate>::new();
    let predicate_changed: Rc<dyn IObservable<ItemPredicate>> =
        if completion_strategy == StreamCompletionStrategy::Asynchronous { subject.as_observable() } else { Observable::empty::<ItemPredicate>() };

    // UUT Initialization & Action
    let (subscription, results) = source
        .connect()
        .filter_dynamic_with(predicate_changed, empty_changeset_policy == EmptyChangesetPolicy::SuppressEmptyChangesets)
        .validate_synchronization()
        .validate_cache_change_sets(|item: &Rc<Item>| item.id())
        .record_cache_items();

    if completion_strategy == StreamCompletionStrategy::Asynchronous {
        subject.on_completed();
    }

    assert!(results.error().is_none());
    assert!(results.recorded_change_sets().is_empty(), "no source operations were performed");
    if empty_changeset_policy == EmptyChangesetPolicy::IncludeEmptyChangesets {
        assert!(!results.has_completed(), "additional empty changesets can occur");
    } else {
        assert!(results.has_completed(), "only empty changesets can occur");
    }

    subscription.dispose();
    source.dispose();
}

#[test]
fn predicate_changed_completes_before_initial_value_completion_propagates_if_empty_changesets_are_suppressed_case1() {
    predicate_changed_completes_before_initial_value_completion_propagates_if_empty_changesets_are_suppressed(
        StreamCompletionStrategy::Immediate,
        EmptyChangesetPolicy::IncludeEmptyChangesets,
    );
}

#[test]
fn predicate_changed_completes_before_initial_value_completion_propagates_if_empty_changesets_are_suppressed_case2() {
    predicate_changed_completes_before_initial_value_completion_propagates_if_empty_changesets_are_suppressed(
        StreamCompletionStrategy::Immediate,
        EmptyChangesetPolicy::SuppressEmptyChangesets,
    );
}

#[test]
fn predicate_changed_completes_before_initial_value_completion_propagates_if_empty_changesets_are_suppressed_case3() {
    predicate_changed_completes_before_initial_value_completion_propagates_if_empty_changesets_are_suppressed(
        StreamCompletionStrategy::Asynchronous,
        EmptyChangesetPolicy::IncludeEmptyChangesets,
    );
}

#[test]
fn predicate_changed_completes_before_initial_value_completion_propagates_if_empty_changesets_are_suppressed_case4() {
    predicate_changed_completes_before_initial_value_completion_propagates_if_empty_changesets_are_suppressed(
        StreamCompletionStrategy::Asynchronous,
        EmptyChangesetPolicy::SuppressEmptyChangesets,
    );
}

fn predicate_changed_fails_error_propagates(completion_strategy: StreamCompletionStrategy) {
    // Setup
    let source = TestSourceCache::new(Item::select_id);

    let error = new_error("Test");

    let subject = Subject::<ItemPredicate>::new();
    let predicate_changed: Rc<dyn IObservable<ItemPredicate>> =
        if completion_strategy == StreamCompletionStrategy::Asynchronous { subject.as_observable() } else { Observable::throw::<ItemPredicate>(error.clone()) };

    // UUT Initialization & Action
    let (subscription, results) = source
        .connect()
        .filter_dynamic(predicate_changed)
        .validate_synchronization()
        .validate_cache_change_sets(|item: &Rc<Item>| item.id())
        .record_cache_items();

    if completion_strategy == StreamCompletionStrategy::Asynchronous {
        subject.on_error(error.clone());
    }

    assert!(results.error().is_some_and(|actual| Rc::ptr_eq(&actual, &error)), "errors should propagate downstream");
    assert!(results.recorded_change_sets().is_empty(), "no source operations were performed");

    subscription.dispose();
    source.dispose();
}

#[test]
fn predicate_changed_fails_error_propagates_case1() {
    predicate_changed_fails_error_propagates(StreamCompletionStrategy::Asynchronous);
}

#[test]
fn predicate_changed_fails_error_propagates_case2() {
    predicate_changed_fails_error_propagates(StreamCompletionStrategy::Immediate);
}

// Not ported: PredicateChangedIsNull_ThrowsException - argument validation

fn source_completes_when_empty_completion_propagates_when_empty_changesets_are_suppressed(
    completion_strategy: StreamCompletionStrategy,
    empty_changeset_policy: EmptyChangesetPolicy,
) {
    // Setup
    let source = TestSourceCache::new(Item::select_id);

    // UUT Initialization & Action
    if completion_strategy == StreamCompletionStrategy::Immediate {
        source.complete();
    }

    let (subscription, results) = source
        .connect()
        .filter_dynamic_with(
            Observable::return_::<ItemPredicate>(Rc::new(Item::filter_by_is_included)).concat(Observable::never()),
            empty_changeset_policy == EmptyChangesetPolicy::SuppressEmptyChangesets,
        )
        .validate_synchronization()
        .validate_cache_change_sets(Item::select_id)
        .record_cache_items();

    if completion_strategy == StreamCompletionStrategy::Asynchronous {
        source.complete();
    }

    assert!(results.error().is_none());
    assert!(results.recorded_change_sets().is_empty(), "no source operations were performed");
    if empty_changeset_policy == EmptyChangesetPolicy::IncludeEmptyChangesets {
        assert!(!results.has_completed(), "the source has completed, but further empty changesets can occur");
    } else {
        assert!(results.has_completed(), "the source has completed, and no further changesets can occur");
    }

    // Final verification
    assert_should_not_support_sorting(&results, "sorting is not supported by filter operators");

    subscription.dispose();
    source.dispose();
}

#[test]
fn source_completes_when_empty_completion_propagates_when_empty_changesets_are_suppressed_case1() {
    source_completes_when_empty_completion_propagates_when_empty_changesets_are_suppressed(
        StreamCompletionStrategy::Asynchronous,
        EmptyChangesetPolicy::IncludeEmptyChangesets,
    );
}

#[test]
fn source_completes_when_empty_completion_propagates_when_empty_changesets_are_suppressed_case2() {
    source_completes_when_empty_completion_propagates_when_empty_changesets_are_suppressed(
        StreamCompletionStrategy::Asynchronous,
        EmptyChangesetPolicy::SuppressEmptyChangesets,
    );
}

#[test]
fn source_completes_when_empty_completion_propagates_when_empty_changesets_are_suppressed_case3() {
    source_completes_when_empty_completion_propagates_when_empty_changesets_are_suppressed(
        StreamCompletionStrategy::Immediate,
        EmptyChangesetPolicy::IncludeEmptyChangesets,
    );
}

#[test]
fn source_completes_when_empty_completion_propagates_when_empty_changesets_are_suppressed_case4() {
    source_completes_when_empty_completion_propagates_when_empty_changesets_are_suppressed(
        StreamCompletionStrategy::Immediate,
        EmptyChangesetPolicy::SuppressEmptyChangesets,
    );
}

fn source_completes_when_not_empty_completion_waits_for_predicate_changed_completion(completion_strategy: StreamCompletionStrategy) {
    // Setup
    let source = TestSourceCache::new(Item::select_id);

    source.edit(|updater| {
        updater.add_or_update_items(&mut [Item::new(1, true), Item::new(2, true), Item::new(3, true), Item::new(4, false), Item::new(5, false), Item::new(6, false)].into_iter())
    });

    let predicate_changed = BehaviorSubject::<ItemPredicate>::new(Rc::new(Item::filter_by_is_included));

    // UUT Initialization & Action
    if completion_strategy == StreamCompletionStrategy::Immediate {
        source.complete();
    }

    let (subscription, results) = source
        .connect()
        .filter_dynamic(predicate_changed.as_observable())
        .validate_synchronization()
        .validate_cache_change_sets(Item::select_id)
        .record_cache_items();

    if completion_strategy == StreamCompletionStrategy::Asynchronous {
        source.complete();
    }

    assert!(results.error().is_none());
    assert_eq!(results.recorded_change_sets().len(), 1, "an initial changeset was published");
    assert_equivalent_because(&results.recorded_items(), &included_items(source.items()), "all matching items should have propagated");
    assert!(!results.has_completed(), "the collection could still change due to new predicates");

    // UUT Action
    predicate_changed.on_completed();

    assert!(results.error().is_none());
    assert_eq!(results.recorded_change_sets().iter().skip(1).count(), 0, "no source operations were performed");
    assert_equivalent_because(&results.recorded_items(), &included_items(source.items()), "no changes should have been made");
    assert!(results.has_completed(), "all source streams have completed");

    // Final Verification
    assert_should_not_support_sorting(&results, "sorting is not supported by filter operators");

    subscription.dispose();
    predicate_changed.dispose();
    source.dispose();
}

#[test]
fn source_completes_when_not_empty_completion_waits_for_predicate_changed_completion_case1() {
    source_completes_when_not_empty_completion_waits_for_predicate_changed_completion(StreamCompletionStrategy::Asynchronous);
}

#[test]
fn source_completes_when_not_empty_completion_waits_for_predicate_changed_completion_case2() {
    source_completes_when_not_empty_completion_waits_for_predicate_changed_completion(StreamCompletionStrategy::Immediate);
}

#[test]
fn subscription_is_disposed_subscription_disposal_propagates() {
    // Setup
    let source = Subject::<ChangeSet<Rc<Item>, i32>>::new();
    let predicate_changed = BehaviorSubject::<ItemPredicate>::new(Rc::new(Item::filter_by_is_included));

    // UUT Initialization
    let (subscription, results) = source
        .as_observable()
        .filter_dynamic(predicate_changed.as_observable())
        .validate_synchronization()
        .validate_cache_change_sets(Item::select_id)
        .record_cache_items();

    assert!(results.error().is_none());
    assert!(results.recorded_change_sets().is_empty(), "no source operations were performed");
    assert!(!results.has_completed(), "the source has not completed");

    // UUT Action
    subscription.dispose();

    assert!(!source.has_observers(), "subscription disposal should propagate to all sources");
    assert!(!predicate_changed.has_observers(), "subscription disposal should propagate to all sources");

    predicate_changed.dispose();
    source.dispose();
}

/// `BuildUut`.
fn build_uut(
    source: Rc<dyn IObservable<ChangeSet<Rc<Item>, i32>>>,
    predicate: ItemPredicate,
    suppress_empty_change_sets: bool,
) -> Rc<dyn IObservable<ChangeSet<Rc<Item>, i32>>> {
    source.filter_dynamic_with(Observable::return_(predicate), suppress_empty_change_sets)
}

super::filter_fixture_base::filter_fixture_base_tests!(build_uut);
