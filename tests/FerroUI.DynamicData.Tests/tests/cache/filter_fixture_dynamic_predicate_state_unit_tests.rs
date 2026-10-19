//! `FilterFixture.DynamicPredicateState.UnitTests`
//! (`DynamicData.Tests/Cache/FilterFixture.DynamicPredicateState.UnitTests.cs`): the filter of
//! caches by a predicate of a stream of states (`filter_with_state`, `filter_with_state_with`),
//! with the tests of `FilterFixture.Base`.
//!
//! Upstream's states of type `object` (`new object()`) are the unit `()`.

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

fn changes_are_made_before_initial_predicate_state_value_items_are_excluded(empty_changeset_policy: EmptyChangesetPolicy) {
    // Setup
    let source = TestSourceCache::new(Item::select_id);

    // UUT Initialization
    let (subscription, results) = source
        .connect()
        .filter_with_state_with(
            Observable::never::<()>(),
            |_: &(), item: &Rc<Item>| item.is_included(),
            empty_changeset_policy == EmptyChangesetPolicy::SuppressEmptyChangesets,
        )
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
    } else {
        assert!(results.recorded_change_sets().is_empty(), "empty changesets should be suppressed");
    }
    assert!(results.recorded_items().is_empty(), "the predicate has not initialized");
    assert!(!results.has_completed(), "the source has not completed");

    // Final verification
    assert_should_not_support_sorting(&results, "sorting is not supported by filter operators");

    subscription.dispose();
    source.dispose();
}

#[test]
fn changes_are_made_before_initial_predicate_state_value_items_are_excluded_case1() {
    changes_are_made_before_initial_predicate_state_value_items_are_excluded(EmptyChangesetPolicy::IncludeEmptyChangesets);
}

#[test]
fn changes_are_made_before_initial_predicate_state_value_items_are_excluded_case2() {
    changes_are_made_before_initial_predicate_state_value_items_are_excluded(EmptyChangesetPolicy::SuppressEmptyChangesets);
}

// Not ported: PredicateIsNull_ThrowsException - argument validation

fn predicate_state_completes_after_initial_value_completion_waits_for_source_completion(completion_strategy: StreamCompletionStrategy) {
    // Setup
    let source = TestSourceCache::new(Item::select_id);

    source.edit(|updater| {
        updater.add_or_update_items(&mut [Item::new(1, true), Item::new(2, true), Item::new(3, true), Item::new(4, false), Item::new(5, false), Item::new(6, false)].into_iter())
    });

    let subject = Subject::<()>::new();
    let predicate_state: Rc<dyn IObservable<()>> =
        if completion_strategy == StreamCompletionStrategy::Asynchronous { subject.as_observable() } else { Observable::return_(()) };

    // UUT Initialization & Action
    let (subscription, results) = source
        .connect()
        .filter_with_state(predicate_state, |_: &(), item: &Rc<Item>| item.is_included())
        .validate_synchronization()
        .validate_cache_change_sets(|item: &Rc<Item>| item.id())
        .record_cache_items();

    if completion_strategy == StreamCompletionStrategy::Asynchronous {
        subject.on_next(());
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
    source.dispose();
}

#[test]
fn predicate_state_completes_after_initial_value_completion_waits_for_source_completion_case1() {
    predicate_state_completes_after_initial_value_completion_waits_for_source_completion(StreamCompletionStrategy::Asynchronous);
}

#[test]
fn predicate_state_completes_after_initial_value_completion_waits_for_source_completion_case2() {
    predicate_state_completes_after_initial_value_completion_waits_for_source_completion(StreamCompletionStrategy::Immediate);
}

fn predicate_state_completes_before_initial_value_completion_propagates_if_empty_changesets_are_suppressed(
    completion_strategy: StreamCompletionStrategy,
    empty_changeset_policy: EmptyChangesetPolicy,
) {
    // Setup
    let source = TestSourceCache::new(Item::select_id);

    let subject = Subject::<()>::new();
    let predicate_state: Rc<dyn IObservable<()>> =
        if completion_strategy == StreamCompletionStrategy::Asynchronous { subject.as_observable() } else { Observable::empty::<()>() };

    // UUT Initialization & Action
    let (subscription, results) = source
        .connect()
        .filter_with_state_with(predicate_state, |_: &(), item: &Rc<Item>| item.is_included(), empty_changeset_policy == EmptyChangesetPolicy::SuppressEmptyChangesets)
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
fn predicate_state_completes_before_initial_value_completion_propagates_if_empty_changesets_are_suppressed_case1() {
    predicate_state_completes_before_initial_value_completion_propagates_if_empty_changesets_are_suppressed(
        StreamCompletionStrategy::Immediate,
        EmptyChangesetPolicy::IncludeEmptyChangesets,
    );
}

#[test]
fn predicate_state_completes_before_initial_value_completion_propagates_if_empty_changesets_are_suppressed_case2() {
    predicate_state_completes_before_initial_value_completion_propagates_if_empty_changesets_are_suppressed(
        StreamCompletionStrategy::Immediate,
        EmptyChangesetPolicy::SuppressEmptyChangesets,
    );
}

#[test]
fn predicate_state_completes_before_initial_value_completion_propagates_if_empty_changesets_are_suppressed_case3() {
    predicate_state_completes_before_initial_value_completion_propagates_if_empty_changesets_are_suppressed(
        StreamCompletionStrategy::Asynchronous,
        EmptyChangesetPolicy::IncludeEmptyChangesets,
    );
}

#[test]
fn predicate_state_completes_before_initial_value_completion_propagates_if_empty_changesets_are_suppressed_case4() {
    predicate_state_completes_before_initial_value_completion_propagates_if_empty_changesets_are_suppressed(
        StreamCompletionStrategy::Asynchronous,
        EmptyChangesetPolicy::SuppressEmptyChangesets,
    );
}

fn predicate_state_fails_error_propagates(completion_strategy: StreamCompletionStrategy) {
    // Setup
    let source = TestSourceCache::new(Item::select_id);

    let error = new_error("Test");

    let subject = Subject::<()>::new();
    let predicate_state: Rc<dyn IObservable<()>> =
        if completion_strategy == StreamCompletionStrategy::Asynchronous { subject.as_observable() } else { Observable::throw::<()>(error.clone()) };

    // UUT Initialization & Action
    let (subscription, results) = source
        .connect()
        .filter_with_state(predicate_state, |_: &(), item: &Rc<Item>| item.is_included())
        .validate_synchronization()
        .validate_cache_change_sets(Item::select_id)
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
fn predicate_state_fails_error_propagates_case1() {
    predicate_state_fails_error_propagates(StreamCompletionStrategy::Asynchronous);
}

#[test]
fn predicate_state_fails_error_propagates_case2() {
    predicate_state_fails_error_propagates(StreamCompletionStrategy::Immediate);
}

// Not ported: PredicateStateIsNull_ThrowsException - argument validation

fn predicate_state_changes_items_are_re_filtered(empty_changeset_policy: EmptyChangesetPolicy) {
    // Setup
    let source = TestSourceCache::new(Item::select_id);
    let predicate_state = BehaviorSubject::new(0x5);

    source.edit(|updater| {
        updater.add_or_update_items(&mut [Item::new(1, true), Item::new(2, true), Item::new(3, true), Item::new(4, false), Item::new(5, false), Item::new(6, false)].into_iter())
    });

    // UUT Initialization
    let (subscription, results) = source
        .connect()
        .filter_with_state_with(
            predicate_state.as_observable(),
            Item::filter_by_id_inclusion_mask,
            empty_changeset_policy == EmptyChangesetPolicy::SuppressEmptyChangesets,
        )
        .validate_synchronization()
        .validate_cache_change_sets(Item::select_id)
        .record_cache_items();

    assert!(results.error().is_none());
    assert_eq!(results.recorded_change_sets().len(), 1, "an initial changeset was published");
    let mask = predicate_state.value().expect("the subject has a value");
    let matching: Vec<Rc<Item>> = source.items().into_iter().filter(|item| Item::filter_by_id_inclusion_mask(&mask, item)).collect();
    assert_equivalent_because(&results.recorded_items(), &matching, "all matching items should have propagated");
    assert!(!results.has_completed(), "the source has not completed");

    // UUT Action
    predicate_state.on_next(0xA);

    assert!(results.error().is_none());
    assert_eq!(results.recorded_change_sets().iter().skip(1).count(), 1, "1 predicate change occurred");
    let mask = predicate_state.value().expect("the subject has a value");
    let matching: Vec<Rc<Item>> = source.items().into_iter().filter(|item| Item::filter_by_id_inclusion_mask(&mask, item)).collect();
    assert_equivalent_because(&results.recorded_items(), &matching, "newly-matching items should have been added, and newly-excluded items should have been removed");
    assert!(!results.has_completed(), "the source has not completed");

    // Final verification
    assert_should_not_support_sorting(&results, "sorting is not supported by filter operators");

    subscription.dispose();
    predicate_state.dispose();
    source.dispose();
}

#[test]
fn predicate_state_changes_items_are_re_filtered_case1() {
    predicate_state_changes_items_are_re_filtered(EmptyChangesetPolicy::IncludeEmptyChangesets);
}

#[test]
fn predicate_state_changes_items_are_re_filtered_case2() {
    predicate_state_changes_items_are_re_filtered(EmptyChangesetPolicy::SuppressEmptyChangesets);
}

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
        .filter_with_state_with(
            Observable::return_(()).concat(Observable::never()),
            |_: &(), item: &Rc<Item>| item.is_included(),
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

    let predicate_state = BehaviorSubject::new(());

    // UUT Initialization & Action
    if completion_strategy == StreamCompletionStrategy::Immediate {
        source.complete();
    }

    let (subscription, results) = source
        .connect()
        .filter_with_state(predicate_state.as_observable(), |_: &(), item: &Rc<Item>| item.is_included())
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
    predicate_state.on_completed();

    assert!(results.error().is_none());
    assert_eq!(results.recorded_change_sets().iter().skip(1).count(), 0, "no source operations were performed");
    assert_equivalent_because(&results.recorded_items(), &included_items(source.items()), "no changes should have been made");
    assert!(results.has_completed(), "all source streams have completed");

    // Final verification
    assert_should_not_support_sorting(&results, "sorting is not supported by filter operators");

    subscription.dispose();
    predicate_state.dispose();
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
    let predicate_state = BehaviorSubject::new(());

    // UUT Initialization
    let (subscription, results) = source
        .as_observable()
        .filter_with_state(predicate_state.as_observable(), |_: &(), item: &Rc<Item>| item.is_included())
        .validate_synchronization()
        .validate_cache_change_sets(Item::select_id)
        .record_cache_items();

    assert!(results.error().is_none());
    assert!(results.recorded_change_sets().is_empty(), "no source operations were performed");
    assert!(!results.has_completed(), "the source has not completed");

    // UUT Action
    subscription.dispose();

    assert!(!source.has_observers(), "subscription disposal should propagate to all sources");
    assert!(!predicate_state.has_observers(), "subscription disposal should propagate to all sources");

    predicate_state.dispose();
    source.dispose();
}

/// `BuildUut`.
fn build_uut(
    source: Rc<dyn IObservable<ChangeSet<Rc<Item>, i32>>>,
    predicate: ItemPredicate,
    suppress_empty_change_sets: bool,
) -> Rc<dyn IObservable<ChangeSet<Rc<Item>, i32>>> {
    source.filter_with_state_with(Observable::return_(()), move |_: &(), item: &Rc<Item>| predicate(item), suppress_empty_change_sets)
}

super::filter_fixture_base::filter_fixture_base_tests!(build_uut);
