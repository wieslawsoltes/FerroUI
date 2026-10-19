//! `FilterFixture.WithPredicateState` (`DynamicData.Tests/List/FilterFixture.WithPredicateState.cs`):
//! the filter of a list by a predicate of the latest value of a state
//! (`Filter<T, TState>(IObservable<TState>, Func<TState, T, bool>, ...)`, `Filter.WithPredicateState`).
//!
//! Upstream's `Subject<object>` states, whose values only signal, are subjects of `()`.

use super::filter_fixture::is_same_error;
use crate::utilities::{assert_equivalent_because, ListItemRecordingObserver, ListObservableExtensions, ObservableExtensions, TestSourceList};
use ferroui_dynamic_data::list::{replace, Change, ChangeSet, IObservableList, ListChangeReason, ListFilterPolicy, ObservableListEx, SourceList};
use ferroui_reactive::subjects::{BehaviorSubject, Subject, SubjectBase};
use ferroui_reactive::Observable;
use ferroui_reactive_testing::new_error;
use ferroui_system::{IDisposable, IObserver};
use std::cell::Cell;
use std::fmt;
use std::rc::Rc;

/// The item of these tests (the private class `Item` of `WithPredicateState`): a class, not a
/// record, so it compares by reference. It is shared as an `Rc<Item>`, and the identity of the
/// `Item` in the `Rc` is the reference.
struct Item {
    id: i32,
    is_included: Cell<bool>,
}

impl Item {
    /// `new Item() { Id = id, IsIncluded = is_included }`.
    fn new(id: i32, is_included: bool) -> Rc<Self> {
        Rc::new(Self { id, is_included: Cell::new(is_included) })
    }

    fn is_included(&self) -> bool {
        self.is_included.get()
    }

    fn set_is_included(&self, value: bool) {
        self.is_included.set(value);
    }
}

impl PartialEq for Item {
    /// Reference equality, as a class that does not override `Equals`.
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl fmt::Display for Item {
    /// `ToString`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{ Id = {}, IsIncluded = {} }}", self.id, self.is_included())
    }
}

impl fmt::Debug for Item {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

/// `ShouldBeValid`.
#[track_caller]
fn should_be_valid(results: &ListItemRecordingObserver<Rc<Item>>, expected_filtered_items: &[Rc<Item>]) {
    assert!(results.error().is_none(), "no errors should have occurred");
    assert!(!results.has_completed(), "no completion events should have occurred");
    assert_equivalent_because(&results.recorded_items(), expected_filtered_items, "all filtered items should match the filter predicate");
}

fn changes_are_made_after_initial_predicate_state_items_are_filtered(filter_policy: ListFilterPolicy) {
    let source = TestSourceList::<Rc<Item>>::new();
    let predicate_state = Subject::<()>::new();

    let (subscription, results) = source
        .connect()
        .filter_with_state_with(predicate_state.as_observable(), |_: &(), item: &Rc<Item>| item.is_included(), filter_policy, true)
        .validate_synchronization()
        .validate_list_change_sets()
        .record_list_items();

    let enumerate_filtered_items = || source.items().into_iter().filter(|item| item.is_included()).collect::<Vec<_>>();

    // Set initial state
    predicate_state.on_next(());

    assert!(results.recorded_change_sets().is_empty(), "no source operations have been performed");

    // Test Add, with an included item
    let item1 = Item::new(1, true);
    source.edit(|list| list.add(item1.clone()));

    assert_eq!(results.recorded_change_sets().len(), 1, "one source operation was performed, with one included item added");
    should_be_valid(&results, &enumerate_filtered_items());

    // Test Add, with an excluded item
    let item2 = Item::new(2, false);
    source.edit(|list| list.add(item2.clone()));

    assert_eq!(results.recorded_change_sets().iter().skip(1).count(), 0, "one source operation was performed, but no included items were affected");

    // Test AddRange, with both included and excluded items
    let item3 = Item::new(3, false);
    let item4 = Item::new(4, true);
    let item5 = Item::new(5, true);
    let item6 = Item::new(6, false);
    let item7 = Item::new(7, false);
    let item8 = Item::new(8, true);
    source.edit(|list| list.add_range(vec![item3.clone(), item4.clone(), item5.clone(), item6.clone(), item7.clone(), item8.clone()]));

    assert_eq!(results.recorded_change_sets().iter().skip(1).count(), 1, "one source operation was performed, with 3 included items added");
    should_be_valid(&results, &enumerate_filtered_items());

    // Test Refresh, with no item mutations.
    source.refresh_indexes(0..source.count());

    assert_eq!(results.recorded_change_sets().iter().skip(2).count(), 1, "one source operation was performed, with all included items affected");
    let change_sets = results.recorded_change_sets();
    let refreshes = change_sets.iter().nth(2).expect("a change set of refreshes");
    assert!(refreshes.iter().all(|change| change.reason() == ListChangeReason::Refresh), "all included items should have been refreshed");
    let refreshed_items: Vec<Rc<Item>> = refreshes.iter().map(|change| change.item().current().clone()).collect();
    assert_equivalent_because(&refreshed_items, &enumerate_filtered_items(), "all included items should have been refreshed");
    should_be_valid(&results, &enumerate_filtered_items());

    // Test Refresh, with item mutations affecting filtering.
    item1.set_is_included(!item1.is_included());
    item3.set_is_included(!item3.is_included());
    item5.set_is_included(!item5.is_included());
    item6.set_is_included(!item6.is_included());
    source.refresh_indexes(0..source.count());

    assert_eq!(results.recorded_change_sets().iter().skip(3).count(), 1, "one source operation was performed, with items being included and excluded");
    should_be_valid(&results, &enumerate_filtered_items());

    // Test Remove, with an included item
    source.edit(|list| list.remove_at(3));

    assert_eq!(results.recorded_change_sets().iter().skip(4).count(), 1, "one source operation was performed, with one included item affected");
    should_be_valid(&results, &enumerate_filtered_items());

    // Test Remove, with an excluded item
    source.edit(|list| list.remove_at(3));

    assert_eq!(results.recorded_change_sets().iter().skip(5).count(), 0, "one source operation was performed, but no included items were affected");

    // Test Remove, with both included and excluded items
    source.edit(|list| list.remove_range(2, 2));

    assert_eq!(results.recorded_change_sets().iter().skip(5).count(), 1, "one source operation was performed, with one included item affected");
    should_be_valid(&results, &enumerate_filtered_items());

    // Test Replace, not affecting filtering
    let item9 = Item::new(9, false);
    let item10 = Item::new(10, true);
    source.edit(|updater| {
        replace(&mut *updater, &item7, item9.clone());
        replace(&mut *updater, &item8, item10.clone());
    });

    assert_eq!(results.recorded_change_sets().iter().skip(6).count(), 1, "one source operation was performed, with one included item affected");
    should_be_valid(&results, &enumerate_filtered_items());

    // Test Replace, affecting filtering
    let item11 = Item::new(11, true);
    let item12 = Item::new(12, false);
    source.edit(|updater| {
        replace(&mut *updater, &item9, item11.clone());
        replace(&mut *updater, &item10, item12.clone());
    });

    assert_eq!(results.recorded_change_sets().iter().skip(7).count(), 1, "one source operation was performed, with one included item affected");
    should_be_valid(&results, &enumerate_filtered_items());

    // Test Move of an included item, relative to another included item
    let item13 = Item::new(13, true);
    source.edit(|list| list.add(item13.clone()));
    source.edit(|list| list.move_index(2, 4));

    match filter_policy {
        ListFilterPolicy::CalculateDiff => {
            assert_eq!(results.recorded_change_sets().iter().skip(8).count(), 2, "two source operations were performed");
        }
        ListFilterPolicy::ClearAndReplace => {
            assert_eq!(
                results.recorded_change_sets().iter().skip(8).count(),
                1,
                "two source operations were performed, one of which was a move, which are not propagated, as ordering is not preserved"
            );
        }
    }
    should_be_valid(&results, &enumerate_filtered_items());

    // Test Move of an excluded item
    source.edit(|list| list.move_index(4, 2));

    match filter_policy {
        ListFilterPolicy::CalculateDiff => {
            assert_eq!(results.recorded_change_sets().iter().skip(10).count(), 1, "one source operation was performed");
        }
        ListFilterPolicy::ClearAndReplace => {
            assert_eq!(
                results.recorded_change_sets().iter().skip(9).count(),
                0,
                "one source operation was performed, a move, which are not propagated, as ordering is not preserved"
            );
        }
    }

    // Test Clear, with included items
    source.edit(|list| list.clear());

    match filter_policy {
        ListFilterPolicy::CalculateDiff => {
            assert_eq!(results.recorded_change_sets().iter().skip(11).count(), 1, "one source operation was performed, with all included items affected");
        }
        ListFilterPolicy::ClearAndReplace => {
            assert_eq!(results.recorded_change_sets().iter().skip(9).count(), 1, "one source operation was performed, with all included items affected");
        }
    }
    should_be_valid(&results, &enumerate_filtered_items());

    // Test Clear, with only excluded items
    source.edit(|list| list.add(Item::new(14, false)));
    source.edit(|list| list.clear());

    match filter_policy {
        ListFilterPolicy::CalculateDiff => {
            assert_eq!(results.recorded_change_sets().iter().skip(12).count(), 0, "two source operations were performed, and neither affected included items");
        }
        ListFilterPolicy::ClearAndReplace => {
            assert_eq!(results.recorded_change_sets().iter().skip(10).count(), 0, "two source operations were performed, and neither affected included items");
        }
    }
    should_be_valid(&results, &enumerate_filtered_items());

    subscription.dispose();
    predicate_state.dispose();
    source.dispose();
}

#[test]
fn changes_are_made_after_initial_predicate_state_items_are_filtered_case1() {
    changes_are_made_after_initial_predicate_state_items_are_filtered(ListFilterPolicy::CalculateDiff);
}

#[test]
fn changes_are_made_after_initial_predicate_state_items_are_filtered_case2() {
    changes_are_made_after_initial_predicate_state_items_are_filtered(ListFilterPolicy::ClearAndReplace);
}

fn changes_are_made_after_multiple_predicate_state_changes_items_are_filtered_with_latest_predicate_state(filter_policy: ListFilterPolicy) {
    let source = SourceList::<Rc<Item>>::new();
    let predicate_state = BehaviorSubject::new(1);

    let (subscription, results) = source
        .connect()
        .filter_with_state_with(predicate_state.as_observable(), |predicate_state: &i32, item: &Rc<Item>| item.id == *predicate_state, filter_policy, true)
        .validate_synchronization()
        .validate_list_change_sets()
        .record_list_items();

    // Publish multiple state changes
    predicate_state.on_next(2);
    predicate_state.on_next(3);

    assert!(results.recorded_change_sets().is_empty(), "no source operations have been performed");

    // Test filtering of items, by state
    source.edit(|list| list.add_range(vec![Item::new(1, true), Item::new(2, true), Item::new(3, false), Item::new(4, false)]));

    assert_eq!(results.recorded_change_sets().len(), 1, "one source operation was performed");
    let state = predicate_state.value().expect("the subject has a value");
    should_be_valid(&results, &source.items().into_iter().filter(|item| item.id == state).collect::<Vec<_>>());

    subscription.dispose();
    predicate_state.dispose();
    source.dispose();
}

#[test]
fn changes_are_made_after_multiple_predicate_state_changes_items_are_filtered_with_latest_predicate_state_case1() {
    changes_are_made_after_multiple_predicate_state_changes_items_are_filtered_with_latest_predicate_state(ListFilterPolicy::CalculateDiff);
}

#[test]
fn changes_are_made_after_multiple_predicate_state_changes_items_are_filtered_with_latest_predicate_state_case2() {
    changes_are_made_after_multiple_predicate_state_changes_items_are_filtered_with_latest_predicate_state(ListFilterPolicy::ClearAndReplace);
}

fn changes_are_made_before_initial_predicate_state_items_are_filtered_on_predicate_state(filter_policy: ListFilterPolicy) {
    let source = TestSourceList::<Rc<Item>>::new();
    let predicate_state = Subject::<()>::new();

    let (subscription, results) = source
        .connect()
        .filter_with_state_with(predicate_state.as_observable(), |_: &(), item: &Rc<Item>| item.is_included(), filter_policy, true)
        .validate_synchronization()
        .validate_list_change_sets()
        .record_list_items();

    assert!(results.recorded_change_sets().is_empty(), "no source operations have been performed");

    // Test Add, with an included item
    let item1 = Item::new(1, true);
    source.edit(|list| list.add(item1.clone()));

    // Test Add, with an excluded item
    let item2 = Item::new(2, false);
    source.edit(|list| list.add(item2.clone()));

    // Test AddRange, with both included and excluded items
    let item3 = Item::new(3, false);
    let item4 = Item::new(4, true);
    let item5 = Item::new(5, true);
    let item6 = Item::new(6, false);
    let item7 = Item::new(7, false);
    let item8 = Item::new(8, true);
    source.edit(|list| list.add_range(vec![item3.clone(), item4.clone(), item5.clone(), item6.clone(), item7.clone(), item8.clone()]));

    // Test Refresh, with no item mutations.
    source.refresh_indexes(0..source.count());

    // Test Refresh, with item mutations affecting filtering.
    item1.set_is_included(!item1.is_included());
    item3.set_is_included(!item3.is_included());
    item5.set_is_included(!item5.is_included());
    item6.set_is_included(!item6.is_included());
    source.refresh_indexes(0..source.count());

    // Test Remove, with an included item
    source.edit(|list| list.remove_at(3));

    // Test Remove, with an excluded item
    source.edit(|list| list.remove_at(3));

    // Test Remove, with both included and excluded items
    source.edit(|list| list.remove_range(2, 2));

    // Test Replace, not affecting filtering
    let item9 = Item::new(9, false);
    let item10 = Item::new(10, true);
    source.edit(|updater| {
        replace(&mut *updater, &item7, item9.clone());
        replace(&mut *updater, &item8, item10.clone());
    });

    // Test Replace, affecting filtering
    let item11 = Item::new(11, true);
    let item12 = Item::new(12, false);
    source.edit(|updater| {
        replace(&mut *updater, &item9, item11.clone());
        replace(&mut *updater, &item10, item12.clone());
    });

    // Test Move of an included item, relative to another included item
    let item13 = Item::new(13, true);
    source.edit(|list| list.add(item13.clone()));
    source.edit(|list| list.move_index(2, 4));

    // Test Move of an excluded item
    source.edit(|list| list.move_index(4, 2));

    assert!(results.recorded_change_sets().is_empty(), "the predicate state has not initialized");

    // Set initial state
    predicate_state.on_next(());

    assert_eq!(results.recorded_change_sets().len(), 1, "one source operation was performed");
    should_be_valid(&results, &source.items().into_iter().filter(|item| item.is_included()).collect::<Vec<_>>());

    subscription.dispose();
    predicate_state.dispose();
    source.dispose();
}

#[test]
fn changes_are_made_before_initial_predicate_state_items_are_filtered_on_predicate_state_case1() {
    changes_are_made_before_initial_predicate_state_items_are_filtered_on_predicate_state(ListFilterPolicy::CalculateDiff);
}

#[test]
fn changes_are_made_before_initial_predicate_state_items_are_filtered_on_predicate_state_case2() {
    changes_are_made_before_initial_predicate_state_items_are_filtered_on_predicate_state(ListFilterPolicy::ClearAndReplace);
}

#[test]
fn filter_policy_is_clear_and_replace_re_filtering_preserves_order() {
    let source = SourceList::<Rc<Item>>::new();
    let predicate_state = BehaviorSubject::new(1);

    let (subscription, results) = source
        .connect()
        .filter_with_state_with(
            predicate_state.as_observable(),
            |predicate_state: &i32, item: &Rc<Item>| item.id == *predicate_state,
            ListFilterPolicy::ClearAndReplace,
            true,
        )
        .validate_synchronization()
        .validate_list_change_sets()
        .record_list_items();

    // Test filtering of items, by state
    source.edit(|list| {
        list.add_range(vec![
            Item::new(1, true),
            Item::new(2, true),
            Item::new(3, false),
            Item::new(4, false),
            Item::new(5, true),
            Item::new(6, false),
            Item::new(7, false),
            Item::new(8, true),
            Item::new(9, false),
            Item::new(10, true),
        ])
    });

    assert_eq!(results.recorded_change_sets().len(), 1, "one source operation was performed");

    // Capture the current set of filtered items, and publish a state change, to force a re-filter
    let prior_filtered_items = results.recorded_items();
    predicate_state.on_next(1);

    assert_eq!(results.recorded_change_sets().iter().skip(1).count(), 1, "one source operation was performed");
    let state = predicate_state.value().expect("the subject has a value");
    should_be_valid(&results, &source.items().into_iter().filter(|item| item.id == state).collect::<Vec<_>>());
    assert_eq!(results.recorded_items(), prior_filtered_items);

    subscription.dispose();
    predicate_state.dispose();
    source.dispose();
}

// Not ported: PredicateIsNull_ExceptionIsThrown - argument validation

fn predicate_state_changes_items_are_re_filtered(filter_policy: ListFilterPolicy) {
    let source = SourceList::<Rc<Item>>::new();
    let predicate_state = BehaviorSubject::new(1);

    let (subscription, results) = source
        .connect()
        .filter_with_state_with(predicate_state.as_observable(), |predicate_state: &i32, item: &Rc<Item>| item.id == *predicate_state, filter_policy, true)
        .validate_synchronization()
        .validate_list_change_sets()
        .record_list_items();

    let enumerate_filtered_items = || {
        let state = predicate_state.value().expect("the subject has a value");
        source.items().into_iter().filter(|item| item.id == state).collect::<Vec<_>>()
    };

    // Test filtering of items, by state
    source.edit(|list| list.add_range(vec![Item::new(1, true), Item::new(2, true), Item::new(3, false), Item::new(4, false)]));

    assert_eq!(results.recorded_change_sets().len(), 1, "one source operation was performed");
    should_be_valid(&results, &enumerate_filtered_items());

    // Publish a state change, to change the filtering
    predicate_state.on_next(2);

    assert_eq!(results.recorded_change_sets().iter().skip(1).count(), 1, "one source operation was performed");
    should_be_valid(&results, &enumerate_filtered_items());

    subscription.dispose();
    predicate_state.dispose();
    source.dispose();
}

#[test]
fn predicate_state_changes_items_are_re_filtered_case1() {
    predicate_state_changes_items_are_re_filtered(ListFilterPolicy::CalculateDiff);
}

#[test]
fn predicate_state_changes_items_are_re_filtered_case2() {
    predicate_state_changes_items_are_re_filtered(ListFilterPolicy::ClearAndReplace);
}

fn predicate_state_completes_after_initial_value_completion_waits_for_source_completion(filter_policy: ListFilterPolicy) {
    let source = Subject::<ChangeSet<Rc<Item>>>::new();

    let (subscription, results) = source
        .as_observable()
        .filter_with_state_with(Observable::return_(()), |_: &(), item: &Rc<Item>| item.is_included(), filter_policy, true)
        .validate_synchronization()
        .validate_list_change_sets()
        .record_list_items();

    assert!(!results.has_completed(), "changes could still be generated by the source");
    assert!(results.recorded_change_sets().is_empty(), "no source operations were performed");

    source.on_completed();

    assert!(results.has_completed(), "all input streams have completed");
    assert!(results.recorded_change_sets().is_empty(), "no source operations were performed");

    subscription.dispose();
    source.dispose();
}

#[test]
fn predicate_state_completes_after_initial_value_completion_waits_for_source_completion_case1() {
    predicate_state_completes_after_initial_value_completion_waits_for_source_completion(ListFilterPolicy::CalculateDiff);
}

#[test]
fn predicate_state_completes_after_initial_value_completion_waits_for_source_completion_case2() {
    predicate_state_completes_after_initial_value_completion_waits_for_source_completion(ListFilterPolicy::ClearAndReplace);
}

fn predicate_state_completes_immediately_completion_is_propagated(filter_policy: ListFilterPolicy) {
    let source = Subject::<ChangeSet<Rc<Item>>>::new();

    let (subscription, results) = source
        .as_observable()
        .filter_with_state_with(Observable::empty::<()>(), |_: &(), item: &Rc<Item>| item.is_included(), filter_policy, true)
        .validate_synchronization()
        .validate_list_change_sets()
        .record_list_items();

    assert!(
        results.has_completed(),
        "completion of the predicate state stream before it emits any values means that items can never be accepted by the filter predicate"
    );
    assert!(results.recorded_change_sets().is_empty(), "no source operations were performed");

    assert!(!source.has_observers(), "all subscriptions should have been disposed, during finalization of the stream");

    subscription.dispose();
    source.dispose();
}

#[test]
fn predicate_state_completes_immediately_completion_is_propagated_case1() {
    predicate_state_completes_immediately_completion_is_propagated(ListFilterPolicy::CalculateDiff);
}

#[test]
fn predicate_state_completes_immediately_completion_is_propagated_case2() {
    predicate_state_completes_immediately_completion_is_propagated(ListFilterPolicy::ClearAndReplace);
}

fn predicate_state_errors_error_is_propagated(filter_policy: ListFilterPolicy) {
    let source = Subject::<ChangeSet<Rc<Item>>>::new();
    let predicate_state = Subject::<()>::new();

    let (subscription, results) = source
        .as_observable()
        .filter_with_state_with(predicate_state.as_observable(), |_: &(), item: &Rc<Item>| item.is_included(), filter_policy, true)
        .validate_synchronization()
        .validate_list_change_sets()
        .record_list_items();

    let error = new_error("This is a test.");
    predicate_state.on_error(error.clone());

    assert!(is_same_error(results.error(), &error), "errors should be propagated");
    assert!(results.recorded_change_sets().is_empty(), "no source operations were performed");

    assert!(!source.has_observers(), "all subscriptions should have been disposed, during finalization of the stream");
    assert!(!predicate_state.has_observers(), "all subscriptions should have been disposed, during finalization  of the stream");

    subscription.dispose();
    predicate_state.dispose();
    source.dispose();
}

#[test]
fn predicate_state_errors_error_is_propagated_case1() {
    predicate_state_errors_error_is_propagated(ListFilterPolicy::CalculateDiff);
}

#[test]
fn predicate_state_errors_error_is_propagated_case2() {
    predicate_state_errors_error_is_propagated(ListFilterPolicy::ClearAndReplace);
}

fn predicate_state_errors_immediately_error_is_propagated(filter_policy: ListFilterPolicy) {
    let source = Subject::<ChangeSet<Rc<Item>>>::new();

    let error = new_error("This is a test.");

    let (subscription, results) = source
        .as_observable()
        .filter_with_state_with(Observable::throw::<()>(error.clone()), |_: &(), item: &Rc<Item>| item.is_included(), filter_policy, true)
        .validate_synchronization()
        .validate_list_change_sets()
        .record_list_items();

    assert!(is_same_error(results.error(), &error), "errors should be propagated");
    assert!(results.recorded_change_sets().is_empty(), "no source operations were performed");

    assert!(!source.has_observers(), "all subscriptions should have been disposed, during finalization of the stream");

    subscription.dispose();
    source.dispose();
}

#[test]
fn predicate_state_errors_immediately_error_is_propagated_case1() {
    predicate_state_errors_immediately_error_is_propagated(ListFilterPolicy::CalculateDiff);
}

#[test]
fn predicate_state_errors_immediately_error_is_propagated_case2() {
    predicate_state_errors_immediately_error_is_propagated(ListFilterPolicy::ClearAndReplace);
}

// Not ported: PredicateStateIsNull_ExceptionIsThrown - argument validation

// Not ported: SourceAndPredicateStateNotifyFromDifferentThreads_FilteringIsThreadSafe - real threads
// (with it, its helpers GenerateStressItemsAndChangeSets and GenerateRandomPredicateStates, which
// no other test uses)

fn source_completes_when_empty_completion_is_propagated(filter_policy: ListFilterPolicy) {
    let source = Subject::<ChangeSet<Rc<Item>>>::new();

    let predicate_state = Subject::<()>::new();

    let (subscription, results) = source
        .as_observable()
        .filter_with_state_with(predicate_state.as_observable(), |_: &(), item: &Rc<Item>| item.is_included(), filter_policy, true)
        .validate_synchronization()
        .validate_list_change_sets()
        .record_list_items();

    source.on_completed();

    assert!(results.has_completed(), "no further changes can occur when there are no items to be filtered or unfiltered");
    assert!(results.recorded_change_sets().is_empty(), "no source operations were performed");

    assert!(!predicate_state.has_observers(), "all subscriptions should have been disposed, during finalization  of the stream");

    subscription.dispose();
    predicate_state.dispose();
    source.dispose();
}

#[test]
fn source_completes_when_empty_completion_is_propagated_case1() {
    source_completes_when_empty_completion_is_propagated(ListFilterPolicy::CalculateDiff);
}

#[test]
fn source_completes_when_empty_completion_is_propagated_case2() {
    source_completes_when_empty_completion_is_propagated(ListFilterPolicy::ClearAndReplace);
}

fn source_completes_when_not_empty_completion_waits_for_state_completion(filter_policy: ListFilterPolicy) {
    let source = Subject::<ChangeSet<Rc<Item>>>::new();

    let predicate_state = Subject::<()>::new();

    let (subscription, results) = source
        .as_observable()
        .filter_with_state_with(predicate_state.as_observable(), |_: &(), item: &Rc<Item>| item.is_included(), filter_policy, true)
        .validate_synchronization()
        .validate_list_change_sets()
        .record_list_items();

    source.on_next(ChangeSet::from(vec![Change::new(ListChangeReason::Add, Item::new(1, true), Some(0))]));
    source.on_completed();

    assert!(!results.has_completed(), "changes could still be generated by changes in predicate state");
    assert!(results.recorded_change_sets().is_empty(), "the predicate has not initialized");

    predicate_state.on_completed();

    assert!(results.has_completed(), "all input streams have completed");
    assert!(results.recorded_change_sets().is_empty(), "the predicate never initialized");

    subscription.dispose();
    predicate_state.dispose();
    source.dispose();
}

#[test]
fn source_completes_when_not_empty_completion_waits_for_state_completion_case1() {
    source_completes_when_not_empty_completion_waits_for_state_completion(ListFilterPolicy::CalculateDiff);
}

#[test]
fn source_completes_when_not_empty_completion_waits_for_state_completion_case2() {
    source_completes_when_not_empty_completion_waits_for_state_completion(ListFilterPolicy::ClearAndReplace);
}

fn source_completes_immediately_completion_is_propagated(filter_policy: ListFilterPolicy) {
    let predicate_state = Subject::<()>::new();

    let (subscription, results) = Observable::empty::<ChangeSet<Rc<Item>>>()
        .filter_with_state_with(predicate_state.as_observable(), |_: &(), item: &Rc<Item>| item.is_included(), filter_policy, true)
        .validate_synchronization()
        .validate_list_change_sets()
        .record_list_items();

    assert!(results.has_completed(), "no further changes can occur when there are no items to be filtered or unfiltered");
    assert!(results.recorded_change_sets().is_empty(), "no source operations were performed");

    assert!(!predicate_state.has_observers(), "all subscriptions should have been disposed, during finalization  of the stream");

    subscription.dispose();
    predicate_state.dispose();
}

#[test]
fn source_completes_immediately_completion_is_propagated_case1() {
    source_completes_immediately_completion_is_propagated(ListFilterPolicy::CalculateDiff);
}

#[test]
fn source_completes_immediately_completion_is_propagated_case2() {
    source_completes_immediately_completion_is_propagated(ListFilterPolicy::ClearAndReplace);
}

fn source_errors_error_is_propagated(filter_policy: ListFilterPolicy) {
    let source = Subject::<ChangeSet<Rc<Item>>>::new();

    let predicate_state = Subject::<()>::new();

    let (subscription, results) = source
        .as_observable()
        .filter_with_state_with(predicate_state.as_observable(), |_: &(), item: &Rc<Item>| item.is_included(), filter_policy, true)
        .validate_synchronization()
        .validate_list_change_sets()
        .record_list_items();

    let error = new_error("This is a test.");
    source.on_error(error.clone());

    assert!(is_same_error(results.error(), &error), "errors should be propagated");
    assert!(results.recorded_change_sets().is_empty(), "no source operations were performed");

    assert!(!source.has_observers(), "all subscriptions should have been disposed, during finalization of the stream");
    assert!(!predicate_state.has_observers(), "all subscriptions should have been disposed, during finalization  of the stream");

    subscription.dispose();
    predicate_state.dispose();
    source.dispose();
}

#[test]
fn source_errors_error_is_propagated_case1() {
    source_errors_error_is_propagated(ListFilterPolicy::CalculateDiff);
}

#[test]
fn source_errors_error_is_propagated_case2() {
    source_errors_error_is_propagated(ListFilterPolicy::ClearAndReplace);
}

fn source_errors_immediately_error_is_propagated(filter_policy: ListFilterPolicy) {
    let predicate_state = Subject::<()>::new();

    let error = new_error("This is a test.");

    let (subscription, results) = Observable::throw::<ChangeSet<Rc<Item>>>(error.clone())
        .filter_with_state_with(predicate_state.as_observable(), |_: &(), item: &Rc<Item>| item.is_included(), filter_policy, true)
        .validate_synchronization()
        .validate_list_change_sets()
        .record_list_items();

    assert!(is_same_error(results.error(), &error), "errors should be propagated");
    assert!(results.recorded_change_sets().is_empty(), "no source operations were performed");

    assert!(!predicate_state.has_observers(), "all subscriptions should have been disposed, during finalization  of the stream");

    subscription.dispose();
    predicate_state.dispose();
}

#[test]
fn source_errors_immediately_error_is_propagated_case1() {
    source_errors_immediately_error_is_propagated(ListFilterPolicy::CalculateDiff);
}

#[test]
fn source_errors_immediately_error_is_propagated_case2() {
    source_errors_immediately_error_is_propagated(ListFilterPolicy::ClearAndReplace);
}

// Not ported: SourceIsNull_ExceptionIsThrown - argument validation

fn subscription_is_disposed_unsubscription_is_propagated(filter_policy: ListFilterPolicy) {
    let source = Subject::<ChangeSet<Rc<Item>>>::new();

    let predicate_state = Subject::<()>::new();

    let (subscription, results) = source
        .as_observable()
        .filter_with_state_with(predicate_state.as_observable(), |_: &(), item: &Rc<Item>| item.is_included(), filter_policy, true)
        .validate_synchronization()
        .validate_list_change_sets()
        .record_list_items();

    subscription.dispose();

    assert!(!source.has_observers(), "subscription disposal should be propagated to all input streams");
    assert!(!predicate_state.has_observers(), "subscription disposal should be propagated to all input streams");

    assert!(results.recorded_change_sets().is_empty(), "no source operations were performed");

    predicate_state.dispose();
    source.dispose();
}

#[test]
fn subscription_is_disposed_unsubscription_is_propagated_case1() {
    subscription_is_disposed_unsubscription_is_propagated(ListFilterPolicy::CalculateDiff);
}

#[test]
fn subscription_is_disposed_unsubscription_is_propagated_case2() {
    subscription_is_disposed_unsubscription_is_propagated(ListFilterPolicy::ClearAndReplace);
}

fn suppress_empty_change_sets_is_false_empty_changesets_are_propagated_and_only_final_completion_is_propagated(completion_order: &[&str]) {
    let source = Subject::<ChangeSet<Rc<Item>>>::new();

    let predicate_state = Subject::<()>::new();

    let (subscription, results) = source
        .as_observable()
        .filter_with_state_with(predicate_state.as_observable(), |_: &(), item: &Rc<Item>| item.is_included(), ListFilterPolicy::CalculateDiff, false)
        .validate_synchronization()
        .validate_list_change_sets()
        .record_list_items();

    // Initialize the predicate
    predicate_state.on_next(());

    assert_eq!(results.recorded_change_sets().len(), 1, "the predicate state was initialized");
    assert_eq!(results.recorded_change_sets()[0].count(), 0, "there are no items in the collection");
    should_be_valid(&results, &[]);

    // Publish an empty changeset
    source.on_next(ChangeSet::empty());

    assert_eq!(results.recorded_change_sets().iter().skip(1).count(), 1, "a source operation was performed");
    assert_eq!(results.recorded_change_sets().iter().nth(1).expect("a change set").count(), 0, "the source changeset was empty");
    should_be_valid(&results, &[]);

    // Publish a changeset with only excluded items
    source.on_next(ChangeSet::from(vec![Change::range_change(ListChangeReason::AddRange, vec![Item::new(1, false), Item::new(2, false), Item::new(3, false)], Some(0))]));

    assert_eq!(results.recorded_change_sets().iter().skip(2).count(), 1, "a source operation was performed");
    assert_eq!(results.recorded_change_sets().iter().nth(2).expect("a change set").count(), 0, "all source items were excluded");
    should_be_valid(&results, &[]);

    for (i, stream) in completion_order.iter().enumerate() {
        match *stream {
            "source" => source.on_completed(),
            "predicateState" => predicate_state.on_completed(),
            _ => {}
        }

        if i < completion_order.len() - 1 {
            assert!(!results.has_completed(), "not all input streams have completed");
        }
    }

    assert!(results.has_completed(), "all input streams have completed");

    subscription.dispose();
    predicate_state.dispose();
    source.dispose();
}

#[test]
fn suppress_empty_change_sets_is_false_empty_changesets_are_propagated_and_only_final_completion_is_propagated_case1() {
    suppress_empty_change_sets_is_false_empty_changesets_are_propagated_and_only_final_completion_is_propagated(&["source", "predicateState"]);
}

#[test]
fn suppress_empty_change_sets_is_false_empty_changesets_are_propagated_and_only_final_completion_is_propagated_case2() {
    suppress_empty_change_sets_is_false_empty_changesets_are_propagated_and_only_final_completion_is_propagated(&["predicateState", "source"]);
}
