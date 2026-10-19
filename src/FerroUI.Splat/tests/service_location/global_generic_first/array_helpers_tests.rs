//! Ported from `Splat.Tests/ServiceLocation/GlobalGenericFirst/ArrayHelpersTests.cs` of Splat.
//! Adapted: arrays are slices and `Vec`s; `null` values are `None`; the snapshot of an entry is an
//! `Rc<[T]>`, so `IsSameReferenceAs` compares its address; `Entry.Remove(value)` is
//! `remove_where` with an equality; a factory that throws panics.

use crate::service_location::common::{materialize_registrations, Entry, Registration};
use std::cell::Cell;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Rc;

const TWO_ITEMS: usize = 2;
const THREE_ITEMS: usize = 3;
const FIRST_VALUE: i32 = 1;
const SECOND_VALUE: i32 = 2;
const THIRD_VALUE: i32 = 3;
const SAMPLE_VALUE: i32 = 42;
const SEED_VALUE: i32 = 99;
const THIRD_INDEX: usize = 2;

fn instance(value: &str) -> Registration<String> {
    Registration::from_instance(Some(value.to_owned()))
}

#[test]
fn materialize_registrations_with_empty_array_returns_empty_array() {
    let registrations: [Registration<String>; 0] = [];
    let result = materialize_registrations(&registrations);
    assert_eq!(result.len(), 0);
}

#[test]
fn materialize_registrations_with_instance_registrations_returns_instances() {
    let registrations = [instance("one"), instance("two"), instance("three")];
    let result = materialize_registrations(&registrations);
    assert_eq!(result.len(), THREE_ITEMS);
    assert_eq!(result[0], "one");
    assert_eq!(result[1], "two");
    assert_eq!(result[THIRD_INDEX], "three");
}

#[test]
fn materialize_registrations_with_factory_registrations_invokes_factories() {
    let invocation_count = Rc::new(Cell::new(0usize));
    let factory = |value: i32| -> Registration<i32> {
        let count = invocation_count.clone();
        Registration::from_factory(Rc::new(move || {
            count.set(count.get() + 1);
            Some(value)
        }))
    };
    let registrations = [factory(FIRST_VALUE), factory(SECOND_VALUE), factory(THIRD_VALUE)];
    let result = materialize_registrations(&registrations);
    assert_eq!(result.len(), THREE_ITEMS);
    assert_eq!(result[0], FIRST_VALUE);
    assert_eq!(result[1], SECOND_VALUE);
    assert_eq!(result[THIRD_INDEX], THIRD_VALUE);
    assert_eq!(invocation_count.get(), THREE_ITEMS);
}

#[test]
fn materialize_registrations_with_mixed_registrations_returns_all_values() {
    let registrations = [
        instance("instance"),
        Registration::from_factory(Rc::new(|| Some("factory".to_owned()))),
        instance("another instance"),
    ];
    let result = materialize_registrations(&registrations);
    assert_eq!(result.len(), THREE_ITEMS);
    assert_eq!(result[0], "instance");
    assert_eq!(result[1], "factory");
    assert_eq!(result[THIRD_INDEX], "another instance");
}

#[test]
fn materialize_registrations_with_null_values_filters_out_nulls() {
    let registrations = [
        instance("valid"),
        Registration::from_instance(None),
        Registration::from_factory(Rc::new(|| None)),
        instance("another valid"),
    ];
    let result = materialize_registrations(&registrations);
    assert_eq!(result.len(), TWO_ITEMS);
    assert_eq!(result[0], "valid");
    assert_eq!(result[1], "another valid");
}

#[test]
fn materialize_registrations_with_all_null_values_returns_empty_array() {
    let registrations: [Registration<String>; 2] =
        [Registration::from_instance(None), Registration::from_factory(Rc::new(|| None))];
    let result = materialize_registrations(&registrations);
    assert_eq!(result.len(), 0);
}

#[test]
fn materialize_registrations_with_factory_that_throws_propagates_exception() {
    let registrations: [Registration<String>; 1] =
        [Registration::from_factory(Rc::new(|| -> Option<String> { panic!("Test exception") }))];
    let result = catch_unwind(AssertUnwindSafe(|| materialize_registrations(&registrations)));
    assert!(result.is_err());
}

#[test]
fn materialize_registrations_called_multiple_times_invokes_factories_each_time() {
    let invocation_count = Rc::new(Cell::new(0));
    let count = invocation_count.clone();
    let registrations = [Registration::from_factory(Rc::new(move || {
        count.set(count.get() + 1);
        Some(count.get())
    }))];
    let result1 = materialize_registrations(&registrations);
    let result2 = materialize_registrations(&registrations);
    assert_eq!(invocation_count.get(), 2);
    assert_eq!(result1[0], FIRST_VALUE);
    assert_eq!(result2[0], SECOND_VALUE);
}

#[test]
fn entry_add_increments_count_and_publishes_items() {
    let entry = Entry::new();
    entry.add(FIRST_VALUE);
    entry.add(SECOND_VALUE);
    assert_eq!(entry.count(), TWO_ITEMS);
    assert!(entry.has_items());
    let snapshot = entry.get_snapshot();
    assert_eq!(snapshot.len(), TWO_ITEMS);
    assert_eq!(snapshot[0], FIRST_VALUE);
    assert_eq!(snapshot[1], SECOND_VALUE);
}

#[test]
fn entry_has_items_is_false_when_empty() {
    let entry: Entry<i32> = Entry::new();
    assert!(!entry.has_items());
    assert_eq!(entry.count(), 0);
    assert_eq!(entry.get_snapshot().len(), 0);
}

#[test]
fn entry_get_snapshot_returns_cached_instance_when_unchanged() {
    let entry = Entry::new();
    entry.add(1);
    let first = entry.get_snapshot();
    let second = entry.get_snapshot();
    assert!(Rc::ptr_eq(&first, &second));
}

#[test]
fn entry_get_snapshot_rebuilds_after_mutation() {
    let entry = Entry::new();
    entry.add(FIRST_VALUE);
    let first = entry.get_snapshot();
    entry.add(SECOND_VALUE);
    let second = entry.get_snapshot();
    assert_eq!(first.len(), 1);
    assert_eq!(second.len(), TWO_ITEMS);
    assert!(!Rc::ptr_eq(&first, &second));
}

#[test]
fn entry_remove_current_removes_last_item_and_reports_not_empty() {
    let entry = Entry::new();
    entry.add(FIRST_VALUE);
    entry.add(SECOND_VALUE);
    let became_empty = entry.remove_current();
    assert!(!became_empty);
    assert_eq!(entry.count(), 1);
    let snapshot = entry.get_snapshot();
    assert_eq!(snapshot.len(), 1);
    assert_eq!(snapshot[0], FIRST_VALUE);
}

#[test]
fn entry_remove_current_reports_empty_when_last_item_removed() {
    let entry = Entry::new();
    entry.add(SAMPLE_VALUE);
    let became_empty = entry.remove_current();
    assert!(became_empty);
    assert!(!entry.has_items());
    assert_eq!(entry.get_snapshot().len(), 0);
}

#[test]
fn entry_remove_current_on_empty_entry_returns_false() {
    let entry: Entry<i32> = Entry::new();
    let became_empty = entry.remove_current();
    assert!(!became_empty);
    assert_eq!(entry.count(), 0);
}

#[test]
fn entry_remove_present_value_removes_it_and_reports_true() {
    let entry = Entry::new();
    entry.add(FIRST_VALUE);
    entry.add(SECOND_VALUE);
    let removed = entry.remove_where(|value| *value == FIRST_VALUE);
    assert!(removed);
    assert_eq!(entry.count(), 1);
    assert_eq!(entry.get_snapshot()[0], SECOND_VALUE);
}

#[test]
fn entry_remove_absent_value_returns_false() {
    let entry = Entry::new();
    entry.add(FIRST_VALUE);
    let removed = entry.remove_where(|value| *value == SECOND_VALUE);
    assert!(!removed);
    assert_eq!(entry.count(), 1);
}

#[test]
fn entry_remove_last_value_publishes_empty_snapshot() {
    let entry = Entry::new();
    entry.add(SAMPLE_VALUE);
    let removed = entry.remove_where(|value| *value == SAMPLE_VALUE);
    assert!(removed);
    assert!(!entry.has_items());
    assert_eq!(entry.get_snapshot().len(), 0);
}

#[test]
fn entry_clear_removes_all_items() {
    let entry = Entry::new();
    entry.add(FIRST_VALUE);
    entry.add(SECOND_VALUE);
    entry.add(THIRD_VALUE);
    entry.clear();
    assert!(!entry.has_items());
    assert_eq!(entry.count(), 0);
    assert_eq!(entry.get_snapshot().len(), 0);
}

#[test]
fn entry_copy_items_to_appends_all_items() {
    let entry = Entry::new();
    entry.add(FIRST_VALUE);
    entry.add(SECOND_VALUE);
    let mut destination = vec![SEED_VALUE];
    entry.copy_items_to(&mut destination);
    assert_eq!(destination.len(), THREE_ITEMS);
    assert_eq!(destination[0], SEED_VALUE);
    assert_eq!(destination[1], FIRST_VALUE);
    assert_eq!(destination[THIRD_INDEX], SECOND_VALUE);
}

// Not ported: AppendNullable_WithNullArray_CreatesNewArrayWithSingleItem - ArrayHelpers.AppendNullable is not ported (the resolvers do not use it)
// Not ported: AppendNullable_WithExistingArray_AppendsItem - ArrayHelpers.AppendNullable is not ported (the resolvers do not use it)
// Not ported: AppendNullable_WithMultipleItems_MaintainsOrder - ArrayHelpers.AppendNullable is not ported (the resolvers do not use it)
// Not ported: RemoveLast_WithNullArray_ReturnsEmptyArray - ArrayHelpers.RemoveLast is not ported (the resolvers do not use it)
// Not ported: RemoveLast_WithEmptyArray_ReturnsEmptyArray - ArrayHelpers.RemoveLast is not ported (the resolvers do not use it)
// Not ported: RemoveLast_WithSingleItem_ReturnsEmptyArray - ArrayHelpers.RemoveLast is not ported (the resolvers do not use it)
// Not ported: RemoveLast_WithMultipleItems_RemovesLastItem - ArrayHelpers.RemoveLast is not ported (the resolvers do not use it)
// Not ported: RemoveLast_DoesNotModifyOriginalArray - ArrayHelpers.RemoveLast is not ported (the resolvers do not use it)
