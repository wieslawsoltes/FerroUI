//! `SumFixture.ForList` (`DynamicData.Tests/AggregationTests/SumFixture.ForList.cs`): the running
//! sum of the values of the items of a list (`Sum` on list change sets, `sum_list` here), its
//! emissions on adds, removes, replacements and clears, the propagation of completion and errors,
//! and the overloads for each number type.

use crate::utilities::{ObservableExtensions, TestSourceList};
use ferroui_dynamic_data::aggregation::AggregationListEx;
use ferroui_dynamic_data::list::{IObservableList, SourceListEditConvenienceEx};
use ferroui_reactive_testing::new_error;
use ferroui_system::IDisposable;
use std::rc::Rc;

/// A source with the items 10, 20 and 30 (`source.AddRange(new[] { 10, 20, 30 })`).
fn source_with_three_items() -> TestSourceList<i32> {
    let source = TestSourceList::<i32>::new();
    source.add_range([10, 20, 30]);
    source
}

fn items_are_added_sum_reflects_all_items(item_count: usize, expected_sum: i32) {
    let items = [10, 20, 30];
    let source = TestSourceList::<i32>::new();

    // UUT Construction
    let (subscription, results) = source.connect().sum_list(|x: &i32| *x).validate_synchronization().record_values();

    assert!(results.error().is_none(), "no errors should have occurred");
    assert!(!results.has_completed(), "the source can still publish notifications");
    assert!(results.recorded_values().is_empty(), "no items have been added to the source");

    // UUT Action
    source.add_range(items.iter().copied().take(item_count));

    assert!(results.error().is_none(), "no errors should have occurred");
    assert!(!results.has_completed(), "the source can still publish notifications");
    assert_eq!(results.recorded_values(), vec![expected_sum], "an AddRange produces a single changeset; the sum of the first {item_count} items should be {expected_sum}");

    subscription.dispose();
    source.dispose();
}

#[test]
fn items_are_added_sum_reflects_all_items_case1() {
    items_are_added_sum_reflects_all_items(1, 10);
}

#[test]
fn items_are_added_sum_reflects_all_items_case2() {
    items_are_added_sum_reflects_all_items(3, 60);
}

fn item_is_removed_sum_reflects_removal(removal_index: usize, expected_sum: i32) {
    let source = source_with_three_items();

    // UUT Construction
    let (subscription, results) = source.connect().sum_list(|x: &i32| *x).validate_synchronization().record_values();

    assert!(results.error().is_none(), "no errors should have occurred");
    assert!(!results.has_completed(), "the source can still publish notifications");
    assert_eq!(results.recorded_values(), vec![60], "one changeset was published containing all pre-existing items; the sum of items 10 + 20 + 30 is 60");

    // UUT Action
    source.remove_at(removal_index);

    assert!(results.error().is_none(), "no errors should have occurred");
    assert!(!results.has_completed(), "the source can still publish notifications");
    let values = results.recorded_values();
    assert_eq!(values.len(), 2, "one additional sum value should have been emitted after the removal");
    assert_eq!(values.last().copied(), Some(expected_sum), "removing item at index {removal_index} should leave a sum of {expected_sum}");

    subscription.dispose();
    source.dispose();
}

#[test]
fn item_is_removed_sum_reflects_removal_case1() {
    item_is_removed_sum_reflects_removal(0, 50);
}

#[test]
fn item_is_removed_sum_reflects_removal_case2() {
    item_is_removed_sum_reflects_removal(1, 40);
}

#[test]
fn item_is_removed_sum_reflects_removal_case3() {
    item_is_removed_sum_reflects_removal(2, 30);
}

#[test]
fn item_is_replaced_sum_reflects_replacement() {
    let source = source_with_three_items();

    // UUT Construction
    let (subscription, results) = source.connect().sum_list(|x: &i32| *x).validate_synchronization().record_values();

    assert_eq!(results.recorded_values(), vec![60], "one changeset was published containing all pre-existing items; the sum of items 10 + 20 + 30 is 60");

    // UUT Action: replace item at index 1 (value 20) with 50
    source.replace_at(1, 50);

    assert!(results.error().is_none(), "no errors should have occurred");
    assert!(!results.has_completed(), "the source can still publish notifications");
    let values = results.recorded_values();
    assert_eq!(values.len(), 2, "one additional sum value should have been emitted after the replacement");
    assert_eq!(values.last().copied(), Some(90), "replacing 20 with 50 should change the sum from 60 to 90");

    subscription.dispose();
    source.dispose();
}

#[test]
fn items_are_cleared_sum_returns_to_zero() {
    let source = source_with_three_items();

    // UUT Construction
    let (subscription, results) = source.connect().sum_list(|x: &i32| *x).validate_synchronization().record_values();

    assert_eq!(results.recorded_values(), vec![60], "one changeset was published containing all pre-existing items; the sum of items 10 + 20 + 30 is 60");

    // UUT Action
    source.clear();

    assert!(results.error().is_none(), "no errors should have occurred");
    assert!(!results.has_completed(), "the source can still publish notifications");
    let values = results.recorded_values();
    assert_eq!(values.len(), 2, "one additional sum value should have been emitted after clearing");
    assert_eq!(values.last().copied(), Some(0), "all items were removed so the sum should return to zero");

    subscription.dispose();
    source.dispose();
}

#[test]
fn source_is_empty_no_sum_emitted() {
    let source = TestSourceList::<i32>::new();

    // UUT Construction
    let (subscription, results) = source.connect().sum_list(|x: &i32| *x).validate_synchronization().record_values();

    assert!(results.error().is_none(), "no errors should have occurred");
    assert!(!results.has_completed(), "the source can still publish notifications");
    assert!(results.recorded_values().is_empty(), "no items were added so no sum values should have been emitted");

    subscription.dispose();
    source.dispose();
}

#[test]
fn source_completes_after_emitting_completion_propagates() {
    let source = source_with_three_items();

    // UUT Construction
    let (subscription, results) = source.connect().sum_list(|x: &i32| *x).validate_synchronization().record_values();

    assert!(results.error().is_none(), "no errors should have occurred");
    assert!(!results.has_completed(), "the source can still publish notifications");
    assert_eq!(results.recorded_values().len(), 1, "one changeset was published containing all pre-existing items");

    // UUT Action
    source.complete();

    assert!(results.error().is_none(), "no errors should have occurred");
    assert!(results.has_completed(), "the source has completed");

    subscription.dispose();
    source.dispose();
}

#[test]
fn source_completes_without_emitting_completion_propagates() {
    let source = TestSourceList::<i32>::new();

    // UUT Construction
    let (subscription, results) = source.connect().sum_list(|x: &i32| *x).validate_synchronization().record_values();

    assert!(results.recorded_values().is_empty(), "no items were added to the source");

    // UUT Action
    source.complete();

    assert!(results.error().is_none(), "no errors should have occurred");
    assert!(results.has_completed(), "the source has completed");
    assert!(results.recorded_values().is_empty(), "no items were added so no sum values should have been emitted");

    subscription.dispose();
    source.dispose();
}

#[test]
fn source_completes_immediately_initial_sum_and_completion_propagate() {
    let source = source_with_three_items();
    source.complete();

    // UUT Construction: source is already completed, with pre-existing items.
    // Subscription should produce both an initial sum and a completion, synchronously.
    let (subscription, results) = source.connect().sum_list(|x: &i32| *x).validate_synchronization().record_values();

    assert!(results.error().is_none(), "no errors should have occurred");
    assert!(results.has_completed(), "the source was already completed at the time of subscription");
    assert_eq!(
        results.recorded_values(),
        vec![60],
        "an initial sum value should still be emitted, even when the source completes immediately upon subscription; the sum of items 10 + 20 + 30 is 60"
    );

    subscription.dispose();
    source.dispose();
}

#[test]
fn source_completes_immediately_without_emitting_completion_propagates() {
    let source = TestSourceList::<i32>::new();

    source.complete();

    // UUT Construction: source is already completed, with no items.
    let (subscription, results) = source.connect().sum_list(|x: &i32| *x).validate_synchronization().record_values();

    assert!(results.error().is_none(), "no errors should have occurred");
    assert!(results.has_completed(), "the source was already completed at the time of subscription");
    assert!(results.recorded_values().is_empty(), "no items were added so no sum values should have been emitted");

    subscription.dispose();
    source.dispose();
}

#[test]
fn source_errors_after_emitting_error_propagates() {
    let source = source_with_three_items();

    // UUT Construction
    let (subscription, results) = source.connect().sum_list(|x: &i32| *x).validate_synchronization().record_values();

    assert!(results.error().is_none(), "no errors should have occurred");
    assert_eq!(results.recorded_values().len(), 1, "one changeset was published containing all pre-existing items");

    // UUT Action
    let error = new_error("Test error");
    source.set_error(error.clone());

    assert!(results.error().is_some_and(|e| Rc::ptr_eq(&e, &error)), "the error from the source should propagate to the subscriber");
    assert!(!results.has_completed(), "an error is not a completion");

    subscription.dispose();
    source.dispose();
}

#[test]
fn source_fails_immediately_error_propagates() {
    let source = source_with_three_items();
    let error = new_error("Test error");
    source.set_error(error.clone());

    // UUT Construction: source is already in error state.
    // The error should propagate synchronously upon subscription.
    let (subscription, results) = source.connect().sum_list(|x: &i32| *x).validate_synchronization().record_values();

    assert!(results.error().is_some_and(|e| Rc::ptr_eq(&e, &error)), "the error from the source should propagate to the subscriber immediately upon subscription");
    assert!(!results.has_completed(), "an error is not a completion");

    subscription.dispose();
    source.dispose();
}

fn items_are_added_sum_is_correct_for_int(values: &[i32], expected_sum: i32) {
    let source = TestSourceList::<i32>::new();

    source.add_range(values.iter().copied());

    let (subscription, results) = source.connect().sum_list(|x: &i32| *x).record_values();

    assert_eq!(results.recorded_values().last().copied(), Some(expected_sum), "the int sum of {values:?} is {expected_sum}");

    subscription.dispose();
    source.dispose();
}

#[test]
fn items_are_added_sum_is_correct_for_int_case1() {
    items_are_added_sum_is_correct_for_int(&[10, 20, 30], 60);
}

#[test]
fn items_are_added_sum_is_correct_for_int_case2() {
    items_are_added_sum_is_correct_for_int(&[i32::MAX], i32::MAX);
}

#[test]
fn items_are_added_sum_is_correct_for_int_case3() {
    items_are_added_sum_is_correct_for_int(&[i32::MIN], i32::MIN);
}

#[test]
fn items_are_added_sum_is_correct_for_int_case4() {
    items_are_added_sum_is_correct_for_int(&[i32::MAX, -1], i32::MAX - 1);
}

#[test]
fn items_are_added_sum_is_correct_for_int_case5() {
    items_are_added_sum_is_correct_for_int(&[i32::MIN, 1], i32::MIN + 1);
}

#[test]
fn items_are_added_sum_is_correct_for_nullable_int() {
    let source = source_with_three_items();

    let (subscription, results) = source.connect().sum_list(|x: &i32| Some(*x)).record_values();

    assert_eq!(results.recorded_values().last().copied(), Some(60), "the nullable int sum of items 10 + 20 + 30 is 60");

    subscription.dispose();
    source.dispose();
}

#[test]
fn items_are_added_sum_is_correct_for_long() {
    let source = source_with_three_items();

    let (subscription, results) = source.connect().sum_list(|x: &i32| *x as i64).record_values();

    assert_eq!(results.recorded_values().last().copied(), Some(60_i64), "the long sum of items 10 + 20 + 30 is 60");

    subscription.dispose();
    source.dispose();
}

#[test]
fn items_are_added_sum_is_correct_for_nullable_long() {
    let source = source_with_three_items();

    let (subscription, results) = source.connect().sum_list(|x: &i32| Some(*x as i64)).record_values();

    assert_eq!(results.recorded_values().last().copied(), Some(60_i64), "the nullable long sum of items 10 + 20 + 30 is 60");

    subscription.dispose();
    source.dispose();
}

#[test]
fn items_are_added_sum_is_correct_for_double() {
    let source = source_with_three_items();

    let (subscription, results) = source.connect().sum_list(|x: &i32| *x as f64).record_values();

    assert_eq!(results.recorded_values().last().copied(), Some(60.0_f64), "the double sum of items 10 + 20 + 30 is 60");

    subscription.dispose();
    source.dispose();
}

#[test]
fn items_are_added_sum_is_correct_for_nullable_double() {
    let source = source_with_three_items();

    let (subscription, results) = source.connect().sum_list(|x: &i32| Some(*x as f64)).record_values();

    assert_eq!(results.recorded_values().last().copied(), Some(60.0_f64), "the nullable double sum of items 10 + 20 + 30 is 60");

    subscription.dispose();
    source.dispose();
}

// Not ported: ItemsAreAdded_SumIsCorrect_ForDecimal - no decimal type (phase 1b)

// Not ported: ItemsAreAdded_SumIsCorrect_ForNullableDecimal - no decimal type (phase 1b)

#[test]
fn items_are_added_sum_is_correct_for_float() {
    let source = source_with_three_items();

    let (subscription, results) = source.connect().sum_list(|x: &i32| *x as f32).record_values();

    assert_eq!(results.recorded_values().last().copied(), Some(60.0_f32), "the float sum of items 10 + 20 + 30 is 60");

    subscription.dispose();
    source.dispose();
}

#[test]
fn items_are_added_sum_is_correct_for_nullable_float() {
    let source = source_with_three_items();

    let (subscription, results) = source.connect().sum_list(|x: &i32| Some(*x as f32)).record_values();

    assert_eq!(results.recorded_values().last().copied(), Some(60.0_f32), "the nullable float sum of items 10 + 20 + 30 is 60");

    subscription.dispose();
    source.dispose();
}
