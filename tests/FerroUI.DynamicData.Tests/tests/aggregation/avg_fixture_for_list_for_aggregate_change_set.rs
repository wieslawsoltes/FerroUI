//! `AvgFixture.ForList.ForAggregateChangeSet`
//! (`DynamicData.Tests/AggregationTests/AvgFixture.ForList.ForAggregateChangeSet.cs`): the running
//! average of the change sets of a list shaped for aggregations (`for_aggregation().avg(..)`,
//! `avg_with(..)`), which carries the behavioral coverage of the list forms of `Avg`.

use crate::domain::Person;
use crate::utilities::{ObservableExtensions, StreamCompletionStrategy, TestSourceList};
use ferroui_dynamic_data::aggregation::{AggregateChangeSetEx, AggregateChangeSets};
use ferroui_dynamic_data::list::{IObservableList, SourceListEditConvenienceEx};
use ferroui_reactive_testing::new_error;
use ferroui_system::IDisposable;
use std::rc::Rc;

/// `source.Connect().ForAggregation()`. The list form is called by its path, so that only the
/// aggregate form of `avg` is in scope for the tests.
fn for_aggregation<T: Clone + PartialEq + 'static>(source: &TestSourceList<T>) -> AggregateChangeSets<T> {
    ferroui_dynamic_data::aggregation::AggregationListEx::<T>::for_aggregation(&source.connect())
}

/// `CreatePopulatedSource`.
fn create_populated_source() -> TestSourceList<i32> {
    let source = TestSourceList::new();
    source.add_range(vec![10, 20, 30]);
    source
}

fn items_are_added_average_reflects_all_items(item_count: usize, expected_average: f64) {
    let values = [10, 20, 30];
    let source = TestSourceList::<i32>::new();

    let (subscription, results) = for_aggregation(&source).avg(|value: &i32| *value).validate_synchronization().record_values();

    assert!(results.error().is_none());
    assert!(!results.has_completed());
    // Initial-empty behavior is covered separately by the known-defect test.
    let initial_value_count = results.recorded_values().len();

    source.add_range(values[..item_count].iter().copied());

    assert!(results.error().is_none());
    assert!(!results.has_completed());
    let recorded = results.recorded_values();
    assert_eq!(recorded.len(), initial_value_count + 1, "AddRange produces one change set");
    assert_eq!(recorded.last().copied(), Some(expected_average));

    subscription.dispose();
    source.dispose();
}

#[test]
fn items_are_added_average_reflects_all_items_case1() {
    items_are_added_average_reflects_all_items(1, 10.0);
}

#[test]
fn items_are_added_average_reflects_all_items_case2() {
    items_are_added_average_reflects_all_items(2, 15.0);
}

#[test]
fn items_are_added_average_reflects_all_items_case3() {
    items_are_added_average_reflects_all_items(3, 20.0);
}

fn item_is_removed_average_reflects_removal(index: usize, expected_average: f64) {
    let source = create_populated_source();

    let (subscription, results) = for_aggregation(&source).avg(|value: &i32| *value).validate_synchronization().record_values();

    source.remove_at(index);

    assert!(results.error().is_none());
    assert_eq!(results.recorded_values(), vec![20.0f64, expected_average]);

    subscription.dispose();
    source.dispose();
}

#[test]
fn item_is_removed_average_reflects_removal_case1() {
    item_is_removed_average_reflects_removal(0, 25.0);
}

#[test]
fn item_is_removed_average_reflects_removal_case2() {
    item_is_removed_average_reflects_removal(1, 20.0);
}

#[test]
fn item_is_removed_average_reflects_removal_case3() {
    item_is_removed_average_reflects_removal(2, 15.0);
}

#[test]
fn item_is_replaced_average_reflects_replacement() {
    let source = create_populated_source();

    let (subscription, results) = for_aggregation(&source).avg(|value: &i32| *value).validate_synchronization().record_values();

    source.replace_at(1, 50);

    assert!(results.error().is_none());
    assert_eq!(results.recorded_values(), vec![20.0f64, 30.0f64]);

    subscription.dispose();
    source.dispose();
}

#[test]
fn items_are_removed_as_range_average_reflects_removals() {
    let source = TestSourceList::<i32>::new();
    source.add_range(vec![10, 20, 30, 100]);

    let (subscription, results) = for_aggregation(&source).avg(|value: &i32| *value).validate_synchronization().record_values();

    source.remove_range(1, 2);

    assert!(results.error().is_none());
    assert_eq!(results.recorded_values(), vec![40.0f64, 55.0f64]);

    subscription.dispose();
    source.dispose();
}

#[test]
fn items_are_cleared_configured_empty_value_is_emitted() {
    let source = create_populated_source();

    let (subscription, results) = for_aggregation(&source).avg_with(|value: &i32| *value, -1i32).validate_synchronization().record_values();

    source.clear();

    assert!(results.error().is_none());
    assert_eq!(results.recorded_values(), vec![20.0f64, -1.0f64]);

    subscription.dispose();
    source.dispose();
}

#[test]
#[ignore = "Existing defect: Avg does not emit emptyValue when the source is initially empty. Re-enable once the operator has been rewritten and fixed."]
fn source_is_empty_configured_empty_value_is_emitted() {
    let source = TestSourceList::<i32>::new();

    let (subscription, results) = for_aggregation(&source).avg_with(|value: &i32| *value, -1i32).validate_synchronization().record_values();

    assert!(results.error().is_none());
    assert!(!results.has_completed());
    assert_eq!(results.recorded_values(), vec![-1.0f64]);

    subscription.dispose();
    source.dispose();
}

#[test]
fn move_does_not_change_average_but_still_emits() {
    let source = create_populated_source();

    let (subscription, results) = for_aggregation(&source).avg(|value: &i32| *value).validate_synchronization().record_values();

    source.move_item(2, 0);

    assert!(results.error().is_none());
    assert_eq!(results.recorded_values(), vec![20.0f64, 20.0f64]);

    subscription.dispose();
    source.dispose();
}

#[test]
#[ignore = "Existing defect: the legacy aggregate adapter discards Refresh details. Re-enable once the operator has been rewritten and fixed."]
fn item_is_refreshed_average_reevaluates_mutated_value() {
    let source = TestSourceList::<Rc<Person>>::new();
    let person = Person::new("A", 10);
    source.add(person.clone());

    let (subscription, results) = for_aggregation(&source).avg(|item: &Rc<Person>| item.age()).record_values();

    person.set_age(40);
    source.refresh_at(0);

    assert_eq!(results.recorded_values(), vec![10.0f64, 40.0f64]);

    subscription.dispose();
    source.dispose();
}

// Legacy behavior: nullable projections are coalesced to zero and remain in the denominator.
#[test]
fn legacy_nullable_values_are_counted_as_zero() {
    let source = TestSourceList::<Rc<Person>>::new();
    source.add_range(vec![Person::with_nullable_age("A", Some(10)), Person::with_nullable_age("B", None), Person::with_nullable_age("C", Some(20))]);

    let (subscription, results) = for_aggregation(&source).avg_with(|person: &Rc<Person>| person.age_nullable(), -1i32).record_values();

    assert_eq!(results.recorded_values(), vec![10.0f64], "null contributes zero while the item remains in the denominator");

    subscription.dispose();
    source.dispose();
}

// Legacy behavior: null coalescing is currently part of the public operator behavior.
#[test]
fn legacy_all_values_are_null_zero_is_emitted_instead_of_empty_value() {
    let source = TestSourceList::<Rc<Person>>::new();
    source.add_range(vec![Person::with_nullable_age("A", None), Person::with_nullable_age("B", None)]);

    let (subscription, results) = for_aggregation(&source).avg_with(|person: &Rc<Person>| person.age_nullable(), -1i32).record_values();

    assert_eq!(results.recorded_values(), vec![0.0f64], "a non-empty collection of null projections is not empty");

    subscription.dispose();
    source.dispose();
}

#[test]
fn numeric_overloads_produce_expected_averages() {
    let source = create_populated_source();
    let changes = for_aggregation(&source);

    let (int_subscription, ints) = changes.avg_with(|value: &i32| *value, -1i32).record_values();
    let (long_subscription, longs) = changes.avg_with(|value: &i32| i64::from(*value), -2i64).record_values();
    let (double_subscription, doubles) = changes.avg_with(|value: &i32| f64::from(*value), -3.0f64).record_values();
    // Not ported: the decimal overload - no decimal type (phase 1b)
    let (float_subscription, floats) = changes.avg_with(|value: &i32| *value as f32, -5.0f32).record_values();

    source.clear();

    assert_eq!(ints.recorded_values(), vec![20.0f64, -1.0f64]);
    assert_eq!(longs.recorded_values(), vec![20.0f64, -2.0f64]);
    assert_eq!(doubles.recorded_values(), vec![20.0f64, -3.0f64]);
    assert_eq!(floats.recorded_values(), vec![20.0f32, -5.0f32]);

    int_subscription.dispose();
    long_subscription.dispose();
    double_subscription.dispose();
    float_subscription.dispose();
    source.dispose();
}

#[test]
fn legacy_nullable_numeric_overloads_produce_expected_averages() {
    let source = TestSourceList::<Rc<Person>>::new();
    source.add_range(vec![Person::with_nullable_age("A", Some(10)), Person::with_nullable_age("B", None)]);
    let changes = for_aggregation(&source);

    let (int_subscription, ints) = changes.avg_with(|person: &Rc<Person>| person.age_nullable(), -1i32).record_values();
    let (long_subscription, longs) = changes.avg_with(|person: &Rc<Person>| person.age_nullable().map(i64::from), -2i64).record_values();
    let (double_subscription, doubles) = changes.avg_with(|person: &Rc<Person>| person.age_nullable().map(f64::from), -3.0f64).record_values();
    // Not ported: the decimal overload - no decimal type (phase 1b)
    let (float_subscription, floats) = changes.avg_with(|person: &Rc<Person>| person.age_nullable().map(|age: i32| age as f32), -5.0f32).record_values();

    source.clear();

    assert_eq!(ints.recorded_values(), vec![5.0f64, -1.0f64]);
    assert_eq!(longs.recorded_values(), vec![5.0f64, -2.0f64]);
    assert_eq!(doubles.recorded_values(), vec![5.0f64, -3.0f64]);
    assert_eq!(floats.recorded_values(), vec![5.0f32, -5.0f32]);

    int_subscription.dispose();
    long_subscription.dispose();
    double_subscription.dispose();
    float_subscription.dispose();
    source.dispose();
}

#[test]
fn integer_average_can_be_fractional() {
    let source = TestSourceList::<i32>::new();
    source.add_range(vec![10, 11]);

    let (subscription, results) = for_aggregation(&source).avg(|value: &i32| *value).record_values();

    assert_eq!(results.recorded_values(), vec![10.5f64]);

    subscription.dispose();
    source.dispose();
}

fn source_completes_completion_propagates(completion_strategy: StreamCompletionStrategy) {
    let source = create_populated_source();

    if completion_strategy == StreamCompletionStrategy::Immediate {
        source.complete();
    }

    let (subscription, results) = for_aggregation(&source).avg(|value: &i32| *value).validate_synchronization().record_values();

    if completion_strategy == StreamCompletionStrategy::Asynchronous {
        source.complete();
    }

    assert!(results.error().is_none());
    assert!(results.has_completed());
    assert_eq!(results.recorded_values(), vec![20.0f64]);

    subscription.dispose();
    source.dispose();
}

#[test]
fn source_completes_completion_propagates_case1() {
    source_completes_completion_propagates(StreamCompletionStrategy::Asynchronous);
}

#[test]
fn source_completes_completion_propagates_case2() {
    source_completes_completion_propagates(StreamCompletionStrategy::Immediate);
}

fn empty_source_completes_configured_empty_value_and_completion_propagate(completion_strategy: StreamCompletionStrategy) {
    let source = TestSourceList::<i32>::new();

    if completion_strategy == StreamCompletionStrategy::Immediate {
        source.complete();
    }

    let (subscription, results) = for_aggregation(&source).avg_with(|value: &i32| *value, -1i32).validate_synchronization().record_values();

    if completion_strategy == StreamCompletionStrategy::Asynchronous {
        source.complete();
    }

    assert!(results.error().is_none());
    assert!(results.has_completed());
    assert_eq!(results.recorded_values(), vec![-1.0f64]);

    subscription.dispose();
    source.dispose();
}

#[test]
#[ignore = "Existing defect: Avg does not emit emptyValue when the source is initially empty. Re-enable once the operator has been rewritten and fixed."]
fn empty_source_completes_configured_empty_value_and_completion_propagate_case1() {
    empty_source_completes_configured_empty_value_and_completion_propagate(StreamCompletionStrategy::Asynchronous);
}

#[test]
#[ignore = "Existing defect: Avg does not emit emptyValue when the source is initially empty. Re-enable once the operator has been rewritten and fixed."]
fn empty_source_completes_configured_empty_value_and_completion_propagate_case2() {
    empty_source_completes_configured_empty_value_and_completion_propagate(StreamCompletionStrategy::Immediate);
}

fn source_fails_error_propagates(completion_strategy: StreamCompletionStrategy) {
    let source = create_populated_source();
    let error = new_error("Test error");

    if completion_strategy == StreamCompletionStrategy::Immediate {
        source.set_error(error.clone());
    }

    let (subscription, results) = for_aggregation(&source).avg(|value: &i32| *value).validate_synchronization().record_values();

    if completion_strategy == StreamCompletionStrategy::Asynchronous {
        source.set_error(error.clone());
    }

    assert!(results.error().is_some_and(|actual| Rc::ptr_eq(&actual, &error)));
    assert!(!results.has_completed());
    if completion_strategy == StreamCompletionStrategy::Immediate {
        assert!(results.recorded_values().is_empty());
    } else {
        assert_eq!(results.recorded_values(), vec![20.0f64]);
    }

    subscription.dispose();
    source.dispose();
}

#[test]
fn source_fails_error_propagates_case1() {
    source_fails_error_propagates(StreamCompletionStrategy::Asynchronous);
}

#[test]
fn source_fails_error_propagates_case2() {
    source_fails_error_propagates(StreamCompletionStrategy::Immediate);
}

#[test]
fn disposed_subscription_receives_no_further_values() {
    let source = TestSourceList::<i32>::new();
    let (subscription, results) = for_aggregation(&source).avg(|value: &i32| *value).record_values();

    source.add(10);
    let value_count_at_disposal = results.recorded_values().len();
    subscription.dispose();
    source.add(20);

    let values = results.recorded_values();
    assert_eq!(values.len(), value_count_at_disposal);
    assert_eq!(values.last().copied(), Some(10.0f64));

    source.dispose();
}

#[test]
fn multiple_subscriptions_maintain_independent_state() {
    let source = TestSourceList::<i32>::new();
    source.add(10);
    let averages = for_aggregation(&source).avg(|value: &i32| *value);

    let (first_subscription, first) = averages.record_values();
    let (second_subscription, second) = averages.record_values();
    source.add(20);

    assert_eq!(first.recorded_values(), vec![10.0f64, 15.0f64]);
    assert_eq!(second.recorded_values(), vec![10.0f64, 15.0f64]);

    first_subscription.dispose();
    second_subscription.dispose();
    source.dispose();
}
