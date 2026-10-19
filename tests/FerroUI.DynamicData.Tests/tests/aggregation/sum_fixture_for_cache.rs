//! `SumFixture.ForCache` (`DynamicData.Tests/AggregationTests/SumFixture.ForCache.cs`): the running
//! sum of the values of the items of a cache (`Sum` on cache change sets, `sum_cache` here), its
//! emissions on adds, removes, updates and batches, the propagation of completion and errors, and
//! the overloads for each number type.

use crate::domain::Person;
use crate::utilities::{ObservableExtensions, TestSourceCache};
use ferroui_dynamic_data::aggregation::AggregationCacheEx;
use ferroui_dynamic_data::cache::{IConnectableCache, SourceCacheEx};
use ferroui_reactive_testing::new_error;
use ferroui_system::IDisposable;
use std::rc::Rc;

/// The source of the tests (`new TestSourceCache<Person, string>(p => p.Name)`).
fn new_source() -> TestSourceCache<Rc<Person>, String> {
    TestSourceCache::new(|p: &Rc<Person>| p.name())
}

/// The name of the item at `index`: `((char)('A' + i)).ToString()`.
fn name_at(index: usize) -> String {
    char::from(b'A' + index as u8).to_string()
}

fn items_are_added_sum_reflects_all_items(item_count: usize, expected_sum: i32) {
    let ages = [10, 20, 30];
    let source = new_source();

    // UUT Construction
    let (subscription, results) = source.connect().sum_cache(|p: &Rc<Person>| p.age()).validate_synchronization().record_values();

    assert!(results.error().is_none(), "no errors should have occurred");
    assert!(!results.has_completed(), "the source can still publish notifications");
    assert!(results.recorded_values().is_empty(), "no items have been added to the source");

    // UUT Action
    for (i, age) in ages.iter().enumerate().take(item_count) {
        source.add_or_update(Person::new(name_at(i), *age));
    }

    assert!(results.error().is_none(), "no errors should have occurred");
    assert!(!results.has_completed(), "the source can still publish notifications");
    let values = results.recorded_values();
    assert_eq!(values.len(), item_count, "each AddOrUpdate should produce a new sum emission");
    assert_eq!(values.last().copied(), Some(expected_sum), "the sum of the first {item_count} ages should be {expected_sum}");

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

fn item_is_removed_sum_reflects_removal(key_to_remove: &str, expected_sum: i32) {
    let source = new_source();

    source.add_or_update(Person::new("A", 10));
    source.add_or_update(Person::new("B", 20));
    source.add_or_update(Person::new("C", 30));

    // UUT Construction
    let (subscription, results) = source.connect().sum_cache(|p: &Rc<Person>| p.age()).validate_synchronization().record_values();

    assert!(results.error().is_none(), "no errors should have occurred");
    assert!(!results.has_completed(), "the source can still publish notifications");
    assert_eq!(results.recorded_values(), vec![60], "one changeset was published containing all pre-existing items; the sum of ages 10 + 20 + 30 is 60");

    // UUT Action
    source.remove_key(key_to_remove.to_owned());

    assert!(results.error().is_none(), "no errors should have occurred");
    assert!(!results.has_completed(), "the source can still publish notifications");
    let values = results.recorded_values();
    assert_eq!(values.len(), 2, "one additional sum value should have been emitted after the removal");
    assert_eq!(values.last().copied(), Some(expected_sum), "removing '{key_to_remove}' should leave a sum of {expected_sum}");

    subscription.dispose();
    source.dispose();
}

#[test]
fn item_is_removed_sum_reflects_removal_case1() {
    item_is_removed_sum_reflects_removal("A", 50);
}

#[test]
fn item_is_removed_sum_reflects_removal_case2() {
    item_is_removed_sum_reflects_removal("B", 40);
}

#[test]
fn item_is_removed_sum_reflects_removal_case3() {
    item_is_removed_sum_reflects_removal("C", 30);
}

#[test]
fn item_is_updated_sum_reflects_new_value() {
    let source = new_source();

    source.add_or_update(Person::new("A", 10));
    source.add_or_update(Person::new("B", 20));

    // UUT Construction
    let (subscription, results) = source.connect().sum_cache(|p: &Rc<Person>| p.age()).validate_synchronization().record_values();

    assert!(results.error().is_none(), "no errors should have occurred");
    assert!(!results.has_completed(), "the source can still publish notifications");
    assert_eq!(results.recorded_values(), vec![30], "one changeset was published containing all pre-existing items; the sum of ages 10 + 20 is 30");

    // UUT Action: update "B" from age 20 to age 50 (same key, new value)
    source.add_or_update(Person::new("B", 50));

    assert!(results.error().is_none(), "no errors should have occurred");
    assert!(!results.has_completed(), "the source can still publish notifications");
    let values = results.recorded_values();
    assert_eq!(values.len(), 2, "one additional sum value should have been emitted after the update");
    assert_eq!(values.last().copied(), Some(60), "updating 'B' from 20 to 50 should change the sum from 30 to 60");

    subscription.dispose();
    source.dispose();
}

#[test]
fn multiple_changes_in_batch_single_sum_emitted() {
    let source = new_source();

    // UUT Construction
    let (subscription, results) = source.connect().sum_cache(|p: &Rc<Person>| p.age()).validate_synchronization().record_values();

    assert!(results.recorded_values().is_empty(), "no items have been added to the source");

    // UUT Action: add 3 items in a single batch
    source.edit(|updater| {
        updater.add_or_update(Person::new("A", 10));
        updater.add_or_update(Person::new("B", 20));
        updater.add_or_update(Person::new("C", 30));
    });

    assert!(results.error().is_none(), "no errors should have occurred");
    assert!(!results.has_completed(), "the source can still publish notifications");
    assert_eq!(results.recorded_values(), vec![60], "a batched edit should produce exactly one sum emission; the sum of ages 10 + 20 + 30 is 60");

    subscription.dispose();
    source.dispose();
}

#[test]
fn source_is_empty_no_sum_emitted() {
    let source = new_source();

    // UUT Construction
    let (subscription, results) = source.connect().sum_cache(|p: &Rc<Person>| p.age()).validate_synchronization().record_values();

    assert!(results.error().is_none(), "no errors should have occurred");
    assert!(!results.has_completed(), "the source can still publish notifications");
    assert!(results.recorded_values().is_empty(), "no items were added so no sum values should have been emitted");

    subscription.dispose();
    source.dispose();
}

#[test]
fn all_items_removed_sum_returns_to_zero() {
    let source = new_source();

    source.add_or_update(Person::new("A", 10));
    source.add_or_update(Person::new("B", 20));
    source.add_or_update(Person::new("C", 30));

    // UUT Construction
    let (subscription, results) = source.connect().sum_cache(|p: &Rc<Person>| p.age()).validate_synchronization().record_values();

    assert_eq!(results.recorded_values(), vec![60], "one changeset was published containing all pre-existing items; the sum of ages 10 + 20 + 30 is 60");

    // UUT Action: remove all items in a single batch
    source.edit(|updater| updater.clear());

    assert!(results.error().is_none(), "no errors should have occurred");
    assert!(!results.has_completed(), "the source can still publish notifications");
    let values = results.recorded_values();
    assert_eq!(values.len(), 2, "one additional sum value should have been emitted after clearing");
    assert_eq!(values.last().copied(), Some(0), "all items were removed so the sum should return to zero");

    subscription.dispose();
    source.dispose();
}

#[test]
fn source_completes_after_emitting_completion_propagates() {
    let source = new_source();

    source.add_or_update(Person::new("A", 10));

    // UUT Construction
    let (subscription, results) = source.connect().sum_cache(|p: &Rc<Person>| p.age()).validate_synchronization().record_values();

    assert!(results.error().is_none(), "no errors should have occurred");
    assert!(!results.has_completed(), "the source can still publish notifications");
    assert_eq!(results.recorded_values(), vec![10], "one changeset was published containing the pre-existing item; the sum of a single age of 10 is 10");

    // UUT Action
    source.complete();

    assert!(results.error().is_none(), "no errors should have occurred");
    assert!(results.has_completed(), "the source has completed");

    subscription.dispose();
    source.dispose();
}

#[test]
fn source_completes_without_emitting_completion_propagates() {
    let source = new_source();

    // UUT Construction
    let (subscription, results) = source.connect().sum_cache(|p: &Rc<Person>| p.age()).validate_synchronization().record_values();

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
    let source = new_source();

    source.add_or_update(Person::new("A", 10));
    source.add_or_update(Person::new("B", 20));
    source.add_or_update(Person::new("C", 30));
    source.complete();

    // UUT Construction: source is already completed, with pre-existing items.
    // Subscription should produce both an initial sum and a completion, synchronously.
    let (subscription, results) = source.connect().sum_cache(|p: &Rc<Person>| p.age()).validate_synchronization().record_values();

    assert!(results.error().is_none(), "no errors should have occurred");
    assert!(results.has_completed(), "the source was already completed at the time of subscription");
    assert_eq!(
        results.recorded_values(),
        vec![60],
        "an initial sum value should still be emitted, even when the source completes immediately upon subscription; the sum of ages 10 + 20 + 30 is 60"
    );

    subscription.dispose();
    source.dispose();
}

#[test]
fn source_completes_immediately_without_emitting_completion_propagates() {
    let source = new_source();

    source.complete();

    // UUT Construction: source is already completed, with no items.
    let (subscription, results) = source.connect().sum_cache(|p: &Rc<Person>| p.age()).validate_synchronization().record_values();

    assert!(results.error().is_none(), "no errors should have occurred");
    assert!(results.has_completed(), "the source was already completed at the time of subscription");
    assert!(results.recorded_values().is_empty(), "no items were added so no sum values should have been emitted");

    subscription.dispose();
    source.dispose();
}

#[test]
fn source_errors_after_emitting_error_propagates() {
    let source = new_source();

    source.add_or_update(Person::new("A", 10));

    // UUT Construction
    let (subscription, results) = source.connect().sum_cache(|p: &Rc<Person>| p.age()).validate_synchronization().record_values();

    assert!(results.error().is_none(), "no errors should have occurred");
    assert_eq!(results.recorded_values().len(), 1, "one changeset was published containing the pre-existing item");

    // UUT Action
    let error = new_error("Test error");
    source.set_error(error.clone());

    assert!(results.error().is_some_and(|e| Rc::ptr_eq(&e, &error)), "the error from the source should propagate to the subscriber");
    assert!(!results.has_completed(), "an error is not a completion");

    subscription.dispose();
    source.dispose();
}

#[test]
fn source_errors_without_emitting_error_propagates() {
    let source = new_source();

    // UUT Construction
    let (subscription, results) = source.connect().sum_cache(|p: &Rc<Person>| p.age()).validate_synchronization().record_values();

    assert!(results.recorded_values().is_empty(), "no items were added to the source");

    // UUT Action
    let error = new_error("Test error");
    source.set_error(error.clone());

    assert!(results.error().is_some_and(|e| Rc::ptr_eq(&e, &error)), "the error from the source should propagate to the subscriber");
    assert!(!results.has_completed(), "an error is not a completion");
    assert!(results.recorded_values().is_empty(), "no items were added so no sum values should have been emitted");

    subscription.dispose();
    source.dispose();
}

#[test]
fn source_fails_immediately_error_propagates() {
    let source = new_source();

    source.add_or_update(Person::new("A", 10));
    let error = new_error("Test error");
    source.set_error(error.clone());

    // UUT Construction: source is already in error state.
    // The error should propagate synchronously upon subscription.
    let (subscription, results) = source.connect().sum_cache(|p: &Rc<Person>| p.age()).validate_synchronization().record_values();

    assert!(results.error().is_some_and(|e| Rc::ptr_eq(&e, &error)), "the error from the source should propagate to the subscriber immediately upon subscription");
    assert!(!results.has_completed(), "an error is not a completion");

    subscription.dispose();
    source.dispose();
}

#[test]
fn nullable_values_are_treated_as_zero() {
    let source = new_source();

    source.add_or_update(Person::with_nullable_age("A", Some(10)));
    source.add_or_update(Person::with_nullable_age("B", None));
    source.add_or_update(Person::with_nullable_age("C", Some(30)));

    // UUT Construction
    let (subscription, results) = source.connect().sum_cache(|p: &Rc<Person>| p.age_nullable()).validate_synchronization().record_values();

    assert!(results.error().is_none(), "no errors should have occurred");
    assert!(!results.has_completed(), "the source can still publish notifications");
    assert_eq!(
        results.recorded_values(),
        vec![40],
        "one changeset was published containing all pre-existing items; null values should be treated as zero, so the sum should be 10 + 0 + 30 = 40"
    );

    subscription.dispose();
    source.dispose();
}

fn items_are_added_sum_is_correct_for_int(ages: &[i32], expected_sum: i32) {
    let source = new_source();

    for (i, age) in ages.iter().enumerate() {
        source.add_or_update(Person::new(name_at(i), *age));
    }

    let (subscription, results) = source.connect().sum_cache(|p: &Rc<Person>| p.age()).record_values();

    assert_eq!(results.recorded_values().last().copied(), Some(expected_sum), "the int sum of {ages:?} is {expected_sum}");

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

/// The source with the persons A, B and C of ages 10, 20 and 30.
fn source_with_three_persons() -> TestSourceCache<Rc<Person>, String> {
    let source = new_source();
    source.add_or_update(Person::new("A", 10));
    source.add_or_update(Person::new("B", 20));
    source.add_or_update(Person::new("C", 30));
    source
}

#[test]
fn items_are_added_sum_is_correct_for_nullable_int() {
    let source = new_source();

    source.add_or_update(Person::with_nullable_age("A", Some(10)));
    source.add_or_update(Person::with_nullable_age("B", Some(20)));
    source.add_or_update(Person::with_nullable_age("C", Some(30)));

    let (subscription, results) = source.connect().sum_cache(|p: &Rc<Person>| p.age_nullable()).record_values();

    assert_eq!(results.recorded_values().last().copied(), Some(60), "the nullable int sum of ages 10 + 20 + 30 is 60");

    subscription.dispose();
    source.dispose();
}

#[test]
fn items_are_added_sum_is_correct_for_long() {
    let source = source_with_three_persons();

    let (subscription, results) = source.connect().sum_cache(|p: &Rc<Person>| p.age() as i64).record_values();

    assert_eq!(results.recorded_values().last().copied(), Some(60_i64), "the long sum of ages 10 + 20 + 30 is 60");

    subscription.dispose();
    source.dispose();
}

#[test]
fn items_are_added_sum_is_correct_for_nullable_long() {
    let source = source_with_three_persons();

    let (subscription, results) = source.connect().sum_cache(|p: &Rc<Person>| Some(p.age() as i64)).record_values();

    assert_eq!(results.recorded_values().last().copied(), Some(60_i64), "the nullable long sum of ages 10 + 20 + 30 is 60");

    subscription.dispose();
    source.dispose();
}

#[test]
fn items_are_added_sum_is_correct_for_double() {
    let source = source_with_three_persons();

    let (subscription, results) = source.connect().sum_cache(|p: &Rc<Person>| p.age() as f64).record_values();

    assert_eq!(results.recorded_values().last().copied(), Some(60.0_f64), "the double sum of ages 10 + 20 + 30 is 60");

    subscription.dispose();
    source.dispose();
}

#[test]
fn items_are_added_sum_is_correct_for_nullable_double() {
    let source = source_with_three_persons();

    let (subscription, results) = source.connect().sum_cache(|p: &Rc<Person>| Some(p.age() as f64)).record_values();

    assert_eq!(results.recorded_values().last().copied(), Some(60.0_f64), "the nullable double sum of ages 10 + 20 + 30 is 60");

    subscription.dispose();
    source.dispose();
}

// Not ported: ItemsAreAdded_SumIsCorrect_ForDecimal - no decimal type (phase 1b)

// Not ported: ItemsAreAdded_SumIsCorrect_ForNullableDecimal - no decimal type (phase 1b)

#[test]
fn items_are_added_sum_is_correct_for_float() {
    let source = source_with_three_persons();

    let (subscription, results) = source.connect().sum_cache(|p: &Rc<Person>| p.age() as f32).record_values();

    assert_eq!(results.recorded_values().last().copied(), Some(60.0_f32), "the float sum of ages 10 + 20 + 30 is 60");

    subscription.dispose();
    source.dispose();
}

#[test]
fn items_are_added_sum_is_correct_for_nullable_float() {
    let source = source_with_three_persons();

    let (subscription, results) = source.connect().sum_cache(|p: &Rc<Person>| Some(p.age() as f32)).record_values();

    assert_eq!(results.recorded_values().last().copied(), Some(60.0_f32), "the nullable float sum of ages 10 + 20 + 30 is 60");

    subscription.dispose();
    source.dispose();
}
