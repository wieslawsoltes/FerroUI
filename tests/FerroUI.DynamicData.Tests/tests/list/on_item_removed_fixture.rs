//! `OnItemRemovedFixture` (`DynamicData.Tests/List/OnItemRemovedFixture.cs`): the action of the
//! items removed from a list, run for removals, replacements and clears, and for the items left
//! when the subscription ends if asked, and the propagation of the end of the source.

use crate::utilities::{assert_equivalent_because, ListItemRecordingObserver, ListObservableExtensions, TestSourceList};
use ferroui_dynamic_data::list::{IObservableList, ObservableListEx};
use ferroui_reactive::ObservableExt;
use ferroui_system::{IDisposable, ObservableError};
use std::cell::RefCell;
use std::fmt;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Rc;

type Invocations = Rc<RefCell<Vec<i32>>>;
type Results = Rc<ListItemRecordingObserver<i32>>;

/// The error of the tests (`new Exception(message)`).
#[derive(Debug)]
struct TestError(&'static str);

impl fmt::Display for TestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

impl std::error::Error for TestError {}

fn new_error(message: &'static str) -> ObservableError {
    Rc::new(TestError(message))
}

/// `removeActionInvocations.Add`.
fn record_into(invocations: &Invocations) -> impl Fn(&i32) + 'static {
    let invocations = invocations.clone();
    move |item: &i32| invocations.borrow_mut().push(*item)
}

fn invocations_of(invocations: &Invocations) -> Vec<i32> {
    invocations.borrow().clone()
}

/// `Enumerable.Range(1, count)`.
fn range(count: i32) -> Vec<i32> {
    (1..=count).collect()
}

/// `source.AddRange(items)`.
fn add_range(source: &TestSourceList<i32>, items: Vec<i32>) {
    source.edit(move |list| list.add_range(items));
}

/// `source.Insert(index, item)`.
fn insert(source: &TestSourceList<i32>, index: usize, item: i32) {
    source.edit(move |list| list.insert(index, item));
}

/// `source.Move(original, destination)`.
fn move_item(source: &TestSourceList<i32>, original: usize, destination: usize) {
    source.edit(move |list| list.move_index(original, destination));
}

/// `source.RemoveAt(index)`.
fn remove_at(source: &TestSourceList<i32>, index: usize) {
    source.edit(move |list| list.remove_at(index));
}

/// `source.ReplaceAt(index, item)`.
fn replace_at(source: &TestSourceList<i32>, index: usize, item: i32) {
    source.edit(move |list| list.set(index, item));
}

/// `source.RemoveRange(index, count)`.
fn remove_range(source: &TestSourceList<i32>, index: usize, count: usize) {
    source.edit(move |list| list.remove_range(index, count));
}

/// `source.Clear()`.
fn clear(source: &TestSourceList<i32>) {
    source.edit(|list| list.clear());
}

/// `source.Items.Skip(index).Take(count).ToArray()`.
fn items_in_range(source: &TestSourceList<i32>, index: usize, count: usize) -> Vec<i32> {
    source.items().into_iter().skip(index).take(count).collect()
}

/// The operator under test with its default (`OnItemRemoved(removeAction)`), validated and
/// recorded.
fn subscribe(source: &TestSourceList<i32>, invocations: &Invocations) -> (Rc<dyn IDisposable>, Results) {
    source.connect().on_item_removed(record_into(invocations)).validate_list_change_sets().record_list_items()
}

/// The operator under test (`OnItemRemoved(removeAction, invokeOnUnsubscribe)`), validated and
/// recorded.
fn subscribe_with(source: &TestSourceList<i32>, invocations: &Invocations, invoke_on_unsubscribe: bool) -> (Rc<dyn IDisposable>, Results) {
    source.connect().on_item_removed_with(record_into(invocations), invoke_on_unsubscribe).validate_list_change_sets().record_list_items()
}

#[track_caller]
fn assert_running(results: &Results) {
    assert!(results.error().is_none(), "no errors should have occurred");
    assert!(!results.has_completed(), "the source can still publish notifications");
}

#[track_caller]
fn assert_items_propagated(results: &Results, source: &TestSourceList<i32>) {
    assert_eq!(results.recorded_items(), source.items(), "all collection changes should propagate downstream");
}

#[track_caller]
fn assert_single_change_set(results: &Results, because: &str) {
    assert_eq!(results.recorded_change_sets().len(), 1, "{because}");
}

/// The checks after the subscription to a list, and the reset of the records: one change set when
/// the list had initial items, none otherwise.
#[track_caller]
fn assert_initial_state(results: &Results, source: &TestSourceList<i32>, invocations: &Invocations, initial_item_count: i32) {
    assert_running(results);
    if initial_item_count == 0 {
        assert!(results.recorded_change_sets().is_empty(), "there were no initial items to be published");
    } else {
        assert_single_change_set(results, "the initial items should have been published");
    }
    assert_items_propagated(results, source);
    results.clear_change_sets();

    assert!(invocations_of(invocations).is_empty(), "no items have been removed from the collection");
}

/// The text of the panic of a subscriber that does not handle errors.
fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else if let Some(message) = payload.downcast_ref::<&str>() {
        (*message).to_owned()
    } else {
        String::new()
    }
}

// https://github.com/reactivemarbles/DynamicData/issues/1061
fn subscriber_does_not_handle_errors_error_bubbles_upstream(invoke_on_unsubscribe: bool) {
    let source = TestSourceList::<i32>::new();

    let subscription = source.connect().on_item_removed_with(|_: &i32| {}, invoke_on_unsubscribe).subscribe_ignore();

    let error = new_error("Test");

    // Deviation (DEVIATIONS.md, DynamicData): an error that the subscriber does not handle is a
    // panic, not a rethrown exception, so the test checks that the panic reaches the caller with
    // the message of the error instead of the identity of the error.
    let outcome = catch_unwind(AssertUnwindSafe(|| source.set_error(error.clone())));
    let payload = outcome.expect_err("errors not handled by the subscriber should propagate upstream to the caller");
    let message = panic_message(payload.as_ref());
    assert!(message.contains(&error.to_string()), "the error should be the one of the source, but the panic was {message:?}");

    subscription.dispose();
    source.dispose();
}

#[test]
fn subscriber_does_not_handle_errors_error_bubbles_upstream_case1() {
    subscriber_does_not_handle_errors_error_bubbles_upstream(true);
}

#[test]
fn subscriber_does_not_handle_errors_error_bubbles_upstream_case2() {
    subscriber_does_not_handle_errors_error_bubbles_upstream(false);
}

/// The removal of a range before the end of the subscription, as both tests of the end of the
/// subscription set it up; returns the subscription and the recorder.
fn subscribe_and_remove_range(
    source: &TestSourceList<i32>,
    remove_action_invocations: &Invocations,
    initial_item_count: i32,
    removal_index: usize,
    removal_count: usize,
    invoke_on_unsubscribe: bool,
) -> (Rc<dyn IDisposable>, Results) {
    if initial_item_count != 0 {
        add_range(source, range(initial_item_count));
    }

    // UUT Construction
    let (subscription, results) = subscribe_with(source, remove_action_invocations, invoke_on_unsubscribe);

    assert_initial_state(&results, source, remove_action_invocations, initial_item_count);

    // UUT Setup: Remove some items, to ensure correct tracking of remaining items.
    let removed_items = items_in_range(source, removal_index, removal_count);

    if removal_count != 0 {
        remove_range(source, removal_index, removal_count);
    }

    assert_running(&results);
    if removal_count == 0 {
        assert!(results.recorded_change_sets().is_empty(), "no items should have been removed");
    } else {
        assert_single_change_set(&results, &format!("{removal_count} item{} should have been removed", if removal_count == 1 { "" } else { "s" }));
    }
    assert_items_propagated(&results, source);

    assert_equivalent_because(&invocations_of(remove_action_invocations), &removed_items, "the removal action should be invoked for every removed item");
    remove_action_invocations.borrow_mut().clear();

    (subscription, results)
}

fn invoke_on_unsubscribe_is_requested_remove_action_is_invoked_for_each_remaining_item_on_completion(initial_item_count: i32, removal_index: usize, removal_count: usize) {
    let source = TestSourceList::<i32>::new();

    let remove_action_invocations: Invocations = Rc::new(RefCell::new(Vec::new()));

    let (subscription, _results) = subscribe_and_remove_range(&source, &remove_action_invocations, initial_item_count, removal_index, removal_count, true);

    // UUT Action
    subscription.dispose();

    assert_equivalent_because(&invocations_of(&remove_action_invocations), &source.items(), "the removal action should be invoked for all all remaining items");

    source.dispose();
}

#[test]
fn invoke_on_unsubscribe_is_requested_remove_action_is_invoked_for_each_remaining_item_on_completion_case1() {
    invoke_on_unsubscribe_is_requested_remove_action_is_invoked_for_each_remaining_item_on_completion(0, 0, 0);
}

#[test]
fn invoke_on_unsubscribe_is_requested_remove_action_is_invoked_for_each_remaining_item_on_completion_case2() {
    invoke_on_unsubscribe_is_requested_remove_action_is_invoked_for_each_remaining_item_on_completion(1, 0, 0);
}

#[test]
fn invoke_on_unsubscribe_is_requested_remove_action_is_invoked_for_each_remaining_item_on_completion_case3() {
    invoke_on_unsubscribe_is_requested_remove_action_is_invoked_for_each_remaining_item_on_completion(1, 0, 1);
}

#[test]
fn invoke_on_unsubscribe_is_requested_remove_action_is_invoked_for_each_remaining_item_on_completion_case4() {
    invoke_on_unsubscribe_is_requested_remove_action_is_invoked_for_each_remaining_item_on_completion(5, 0, 1);
}

#[test]
fn invoke_on_unsubscribe_is_requested_remove_action_is_invoked_for_each_remaining_item_on_completion_case5() {
    invoke_on_unsubscribe_is_requested_remove_action_is_invoked_for_each_remaining_item_on_completion(5, 2, 1);
}

#[test]
fn invoke_on_unsubscribe_is_requested_remove_action_is_invoked_for_each_remaining_item_on_completion_case6() {
    invoke_on_unsubscribe_is_requested_remove_action_is_invoked_for_each_remaining_item_on_completion(5, 1, 3);
}

#[test]
fn invoke_on_unsubscribe_is_requested_remove_action_is_invoked_for_each_remaining_item_on_completion_case7() {
    invoke_on_unsubscribe_is_requested_remove_action_is_invoked_for_each_remaining_item_on_completion(5, 0, 5);
}

fn invoke_on_unsubscribe_is_not_requested_remove_action_is_not_invoked_on_completion(initial_item_count: i32, removal_index: usize, removal_count: usize) {
    let source = TestSourceList::<i32>::new();

    let remove_action_invocations: Invocations = Rc::new(RefCell::new(Vec::new()));

    let (subscription, _results) = subscribe_and_remove_range(&source, &remove_action_invocations, initial_item_count, removal_index, removal_count, false);

    // UUT Action
    subscription.dispose();

    assert!(invocations_of(&remove_action_invocations).is_empty(), "the removal action should not be invoked upon unsubscription");

    source.dispose();
}

#[test]
fn invoke_on_unsubscribe_is_not_requested_remove_action_is_not_invoked_on_completion_case1() {
    invoke_on_unsubscribe_is_not_requested_remove_action_is_not_invoked_on_completion(0, 0, 0);
}

#[test]
fn invoke_on_unsubscribe_is_not_requested_remove_action_is_not_invoked_on_completion_case2() {
    invoke_on_unsubscribe_is_not_requested_remove_action_is_not_invoked_on_completion(1, 0, 0);
}

#[test]
fn invoke_on_unsubscribe_is_not_requested_remove_action_is_not_invoked_on_completion_case3() {
    invoke_on_unsubscribe_is_not_requested_remove_action_is_not_invoked_on_completion(1, 0, 1);
}

#[test]
fn invoke_on_unsubscribe_is_not_requested_remove_action_is_not_invoked_on_completion_case4() {
    invoke_on_unsubscribe_is_not_requested_remove_action_is_not_invoked_on_completion(5, 0, 1);
}

#[test]
fn invoke_on_unsubscribe_is_not_requested_remove_action_is_not_invoked_on_completion_case5() {
    invoke_on_unsubscribe_is_not_requested_remove_action_is_not_invoked_on_completion(5, 2, 1);
}

#[test]
fn invoke_on_unsubscribe_is_not_requested_remove_action_is_not_invoked_on_completion_case6() {
    invoke_on_unsubscribe_is_not_requested_remove_action_is_not_invoked_on_completion(5, 1, 3);
}

#[test]
fn invoke_on_unsubscribe_is_not_requested_remove_action_is_not_invoked_on_completion_case7() {
    invoke_on_unsubscribe_is_not_requested_remove_action_is_not_invoked_on_completion(5, 0, 5);
}

fn item_is_added_remove_action_is_not_invoked(initial_item_count: i32, insertion_index: usize) {
    let source = TestSourceList::<i32>::new();

    if initial_item_count != 0 {
        add_range(&source, range(initial_item_count));
    }

    let remove_action_invocations: Invocations = Rc::new(RefCell::new(Vec::new()));

    // UUT Construction
    let (subscription, results) = subscribe(&source, &remove_action_invocations);

    assert_initial_state(&results, &source, &remove_action_invocations, initial_item_count);

    // UUT Action
    insert(&source, insertion_index, initial_item_count);

    assert_running(&results);
    assert_single_change_set(&results, "an item was refreshed within the collection");
    assert_items_propagated(&results, &source);

    assert!(invocations_of(&remove_action_invocations).is_empty(), "no items were removed from the collection");

    // The disposal of the subscription runs the action for the items left; nothing is checked
    // after it, as upstream.
    subscription.dispose();
    source.dispose();
}

#[test]
fn item_is_added_remove_action_is_not_invoked_case1() {
    item_is_added_remove_action_is_not_invoked(0, 0);
}

#[test]
fn item_is_added_remove_action_is_not_invoked_case2() {
    item_is_added_remove_action_is_not_invoked(1, 0);
}

#[test]
fn item_is_added_remove_action_is_not_invoked_case3() {
    item_is_added_remove_action_is_not_invoked(1, 1);
}

#[test]
fn item_is_added_remove_action_is_not_invoked_case4() {
    item_is_added_remove_action_is_not_invoked(5, 0);
}

#[test]
fn item_is_added_remove_action_is_not_invoked_case5() {
    item_is_added_remove_action_is_not_invoked(5, 2);
}

#[test]
fn item_is_added_remove_action_is_not_invoked_case6() {
    item_is_added_remove_action_is_not_invoked(5, 5);
}

fn item_is_moved_remove_action_is_not_invoked(initial_item_count: i32, original_index: usize, destination_index: usize) {
    let source = TestSourceList::<i32>::new();

    add_range(&source, range(initial_item_count));

    let remove_action_invocations: Invocations = Rc::new(RefCell::new(Vec::new()));

    // UUT Construction
    let (subscription, results) = subscribe(&source, &remove_action_invocations);

    assert_initial_state(&results, &source, &remove_action_invocations, initial_item_count);

    // UUT Action
    move_item(&source, original_index, destination_index);

    assert_running(&results);
    assert_single_change_set(&results, "an item was moved within the collection");
    assert_items_propagated(&results, &source);

    assert!(invocations_of(&remove_action_invocations).is_empty(), "no items were removed from the collection");

    subscription.dispose();
    source.dispose();
}

#[test]
fn item_is_moved_remove_action_is_not_invoked_case1() {
    item_is_moved_remove_action_is_not_invoked(2, 0, 1);
}

#[test]
fn item_is_moved_remove_action_is_not_invoked_case2() {
    item_is_moved_remove_action_is_not_invoked(2, 1, 0);
}

#[test]
fn item_is_moved_remove_action_is_not_invoked_case3() {
    item_is_moved_remove_action_is_not_invoked(5, 0, 4);
}

#[test]
fn item_is_moved_remove_action_is_not_invoked_case4() {
    item_is_moved_remove_action_is_not_invoked(5, 4, 0);
}

#[test]
fn item_is_moved_remove_action_is_not_invoked_case5() {
    item_is_moved_remove_action_is_not_invoked(5, 1, 3);
}

#[test]
fn item_is_moved_remove_action_is_not_invoked_case6() {
    item_is_moved_remove_action_is_not_invoked(5, 3, 1);
}

fn item_is_removed_remove_action_is_invoked(initial_item_count: i32, removal_index: usize) {
    let source = TestSourceList::<i32>::new();

    if initial_item_count != 0 {
        add_range(&source, range(initial_item_count));
    }

    let remove_action_invocations: Invocations = Rc::new(RefCell::new(Vec::new()));

    // UUT Construction
    let (subscription, results) = subscribe(&source, &remove_action_invocations);

    assert_initial_state(&results, &source, &remove_action_invocations, initial_item_count);

    // UUT Action
    let removed_item = source.items()[removal_index];
    remove_at(&source, removal_index);

    assert_running(&results);
    assert_single_change_set(&results, "an item was removed from the collection");
    assert_items_propagated(&results, &source);

    assert_eq!(invocations_of(&remove_action_invocations).len(), 1, "an item was removed from the collection");
    assert_eq!(invocations_of(&remove_action_invocations)[0], removed_item, "an item was removed from the collection");

    subscription.dispose();
    source.dispose();
}

#[test]
fn item_is_removed_remove_action_is_invoked_case1() {
    item_is_removed_remove_action_is_invoked(1, 0);
}

#[test]
fn item_is_removed_remove_action_is_invoked_case2() {
    item_is_removed_remove_action_is_invoked(5, 0);
}

#[test]
fn item_is_removed_remove_action_is_invoked_case3() {
    item_is_removed_remove_action_is_invoked(5, 2);
}

#[test]
fn item_is_removed_remove_action_is_invoked_case4() {
    item_is_removed_remove_action_is_invoked(5, 4);
}

fn item_is_refreshed_remove_action_is_not_invoked(initial_item_count: i32, refresh_index: usize) {
    let source = TestSourceList::<i32>::new();

    add_range(&source, range(initial_item_count));

    let remove_action_invocations: Invocations = Rc::new(RefCell::new(Vec::new()));

    // UUT Construction
    let (subscription, results) = subscribe(&source, &remove_action_invocations);

    assert_initial_state(&results, &source, &remove_action_invocations, initial_item_count);

    // UUT Action
    source.refresh_at(refresh_index);

    assert_running(&results);
    assert_single_change_set(&results, "an item was refreshed within the collection");
    assert_items_propagated(&results, &source);

    assert!(invocations_of(&remove_action_invocations).is_empty(), "no items were removed from the collection");

    subscription.dispose();
    source.dispose();
}

#[test]
fn item_is_refreshed_remove_action_is_not_invoked_case1() {
    item_is_refreshed_remove_action_is_not_invoked(1, 0);
}

#[test]
fn item_is_refreshed_remove_action_is_not_invoked_case2() {
    item_is_refreshed_remove_action_is_not_invoked(5, 0);
}

#[test]
fn item_is_refreshed_remove_action_is_not_invoked_case3() {
    item_is_refreshed_remove_action_is_not_invoked(5, 2);
}

#[test]
fn item_is_refreshed_remove_action_is_not_invoked_case4() {
    item_is_refreshed_remove_action_is_not_invoked(5, 4);
}

fn item_is_replaced_remove_action_is_invoked_for_old_item(initial_item_count: i32, replacement_index: usize) {
    let source = TestSourceList::<i32>::new();

    add_range(&source, range(initial_item_count));

    let remove_action_invocations: Invocations = Rc::new(RefCell::new(Vec::new()));

    // UUT Construction
    let (subscription, results) = subscribe(&source, &remove_action_invocations);

    assert_initial_state(&results, &source, &remove_action_invocations, initial_item_count);

    // UUT Action
    let replaced_item = source.items()[replacement_index];
    replace_at(&source, replacement_index, initial_item_count);

    assert_running(&results);
    assert_single_change_set(&results, "an item was replaced within the collection");
    assert_items_propagated(&results, &source);

    assert_eq!(invocations_of(&remove_action_invocations).len(), 1, "an item was replaced within the collection");
    assert_eq!(invocations_of(&remove_action_invocations)[0], replaced_item, "an item was replaced within the collection");

    subscription.dispose();
    source.dispose();
}

#[test]
fn item_is_replaced_remove_action_is_invoked_for_old_item_case1() {
    item_is_replaced_remove_action_is_invoked_for_old_item(1, 0);
}

#[test]
fn item_is_replaced_remove_action_is_invoked_for_old_item_case2() {
    item_is_replaced_remove_action_is_invoked_for_old_item(5, 0);
}

#[test]
fn item_is_replaced_remove_action_is_invoked_for_old_item_case3() {
    item_is_replaced_remove_action_is_invoked_for_old_item(5, 2);
}

#[test]
fn item_is_replaced_remove_action_is_invoked_for_old_item_case4() {
    item_is_replaced_remove_action_is_invoked_for_old_item(5, 4);
}

fn item_range_is_removed_remove_action_is_invoked_for_each_item(initial_item_count: i32, removal_index: usize, removal_count: usize) {
    let source = TestSourceList::<i32>::new();

    add_range(&source, range(initial_item_count));

    let remove_action_invocations: Invocations = Rc::new(RefCell::new(Vec::new()));

    // UUT Construction
    let (subscription, results) = subscribe_with(&source, &remove_action_invocations, true);

    assert_initial_state(&results, &source, &remove_action_invocations, initial_item_count);

    // UUT Action
    let removed_items = items_in_range(&source, removal_index, removal_count);

    remove_range(&source, removal_index, removal_count);

    assert_running(&results);
    assert_single_change_set(&results, &format!("{removal_count} item{} should have been removed", if removal_count == 1 { "" } else { "s" }));
    assert_items_propagated(&results, &source);

    assert_equivalent_because(&invocations_of(&remove_action_invocations), &removed_items, "the removal action should be invoked for every removed item");

    subscription.dispose();
    source.dispose();
}

#[test]
fn item_range_is_removed_remove_action_is_invoked_for_each_item_case1() {
    item_range_is_removed_remove_action_is_invoked_for_each_item(1, 0, 1);
}

#[test]
fn item_range_is_removed_remove_action_is_invoked_for_each_item_case2() {
    item_range_is_removed_remove_action_is_invoked_for_each_item(5, 0, 1);
}

#[test]
fn item_range_is_removed_remove_action_is_invoked_for_each_item_case3() {
    item_range_is_removed_remove_action_is_invoked_for_each_item(5, 2, 1);
}

#[test]
fn item_range_is_removed_remove_action_is_invoked_for_each_item_case4() {
    item_range_is_removed_remove_action_is_invoked_for_each_item(5, 1, 3);
}

#[test]
fn item_range_is_removed_remove_action_is_invoked_for_each_item_case5() {
    item_range_is_removed_remove_action_is_invoked_for_each_item(5, 0, 5);
}

fn items_are_cleared_remove_action_is_invoked_for_each_item(initial_item_count: i32) {
    let source = TestSourceList::<i32>::new();

    add_range(&source, range(initial_item_count));

    let remove_action_invocations: Invocations = Rc::new(RefCell::new(Vec::new()));

    // UUT Construction
    let (subscription, results) = subscribe(&source, &remove_action_invocations);

    assert_initial_state(&results, &source, &remove_action_invocations, initial_item_count);

    // UUT Action
    let cleared_items = source.items();

    clear(&source);

    assert_running(&results);
    assert_single_change_set(&results, "all items in the collection should have been removed");
    assert_items_propagated(&results, &source);

    assert_equivalent_because(&invocations_of(&remove_action_invocations), &cleared_items, "the removal action should be invoked for every removed item");

    subscription.dispose();
    source.dispose();
}

#[test]
fn items_are_cleared_remove_action_is_invoked_for_each_item_case1() {
    items_are_cleared_remove_action_is_invoked_for_each_item(1);
}

#[test]
fn items_are_cleared_remove_action_is_invoked_for_each_item_case2() {
    items_are_cleared_remove_action_is_invoked_for_each_item(5);
}

fn source_completes_asynchronously_completion_propagates(invoke_on_unsubscribe: bool) {
    let source = TestSourceList::<i32>::new();

    add_range(&source, vec![1, 2, 3]);

    let remove_action_invocations: Invocations = Rc::new(RefCell::new(Vec::new()));

    // UUT Construction
    let (subscription, results) = subscribe_with(&source, &remove_action_invocations, invoke_on_unsubscribe);

    assert_initial_state(&results, &source, &remove_action_invocations, 3);

    // UUT Action
    source.complete();

    assert!(results.error().is_none(), "no errors should have occurred");
    assert!(results.has_completed(), "the source has completed");
    assert!(results.recorded_change_sets().is_empty(), "no changes were made to the collection");

    if invoke_on_unsubscribe {
        assert_equivalent_because(&invocations_of(&remove_action_invocations), &source.items(), "the operator was instructed to invoke the removal action all remaining items, upon stream completion");
    } else {
        assert!(invocations_of(&remove_action_invocations).is_empty(), "the operator was instructed to not invoke the removal action, upon stream completion");
    }

    subscription.dispose();
    source.dispose();
}

#[test]
fn source_completes_asynchronously_completion_propagates_case1() {
    source_completes_asynchronously_completion_propagates(true);
}

#[test]
fn source_completes_asynchronously_completion_propagates_case2() {
    source_completes_asynchronously_completion_propagates(false);
}

fn source_completes_immediately_completion_propagates(invoke_on_unsubscribe: bool) {
    let source = TestSourceList::<i32>::new();

    add_range(&source, vec![1, 2, 3]);
    source.complete();

    let remove_action_invocations: Invocations = Rc::new(RefCell::new(Vec::new()));

    // UUT Construction & Action
    let (subscription, results) = subscribe_with(&source, &remove_action_invocations, invoke_on_unsubscribe);

    assert!(results.error().is_none(), "no errors should have occurred");
    assert!(results.has_completed(), "the source has completed");
    assert_single_change_set(&results, "the initial items should have been published");
    assert_items_propagated(&results, &source);

    if invoke_on_unsubscribe {
        assert_equivalent_because(&invocations_of(&remove_action_invocations), &source.items(), "the operator was instructed to invoke the removal action all remaining items, upon stream completion");
    } else {
        assert!(invocations_of(&remove_action_invocations).is_empty(), "the operator was instructed to not invoke the removal action, upon stream completion");
    }

    subscription.dispose();
    source.dispose();
}

#[test]
fn source_completes_immediately_completion_propagates_case1() {
    source_completes_immediately_completion_propagates(true);
}

#[test]
fn source_completes_immediately_completion_propagates_case2() {
    source_completes_immediately_completion_propagates(false);
}

fn source_fails_asynchronously_completion_propagates(invoke_on_unsubscribe: bool) {
    let source = TestSourceList::<i32>::new();

    add_range(&source, vec![1, 2, 3]);

    let remove_action_invocations: Invocations = Rc::new(RefCell::new(Vec::new()));

    // UUT Construction
    let (subscription, results) = subscribe_with(&source, &remove_action_invocations, invoke_on_unsubscribe);

    assert_initial_state(&results, &source, &remove_action_invocations, 3);

    // UUT Action
    let error = new_error("");
    source.set_error(error.clone());

    let recorded_error = results.error().expect("errors within the stream should propagate");
    assert!(Rc::ptr_eq(&recorded_error, &error), "errors within the stream should propagate");
    assert!(results.recorded_change_sets().is_empty(), "no changes were made to the collection");

    if invoke_on_unsubscribe {
        assert_equivalent_because(&invocations_of(&remove_action_invocations), &source.items(), "the operator was instructed to invoke the removal action all remaining items, upon stream failure");
    } else {
        assert!(invocations_of(&remove_action_invocations).is_empty(), "the operator was instructed to not invoke the removal action, upon stream failure");
    }

    subscription.dispose();
    source.dispose();
}

#[test]
fn source_fails_asynchronously_completion_propagates_case1() {
    source_fails_asynchronously_completion_propagates(true);
}

#[test]
fn source_fails_asynchronously_completion_propagates_case2() {
    source_fails_asynchronously_completion_propagates(false);
}

fn source_fails_immediately_completion_propagates(invoke_on_unsubscribe: bool) {
    let source = TestSourceList::<i32>::new();

    add_range(&source, vec![1, 2, 3]);
    let error = new_error("");
    source.set_error(error.clone());

    let remove_action_invocations: Invocations = Rc::new(RefCell::new(Vec::new()));

    // UUT Construction & Action
    let (subscription, results) = subscribe_with(&source, &remove_action_invocations, invoke_on_unsubscribe);

    let recorded_error = results.error().expect("errors within the stream should propagate");
    assert!(Rc::ptr_eq(&recorded_error, &error), "errors within the stream should propagate");
    assert!(results.recorded_change_sets().is_empty(), "an error occurred during subscription");

    assert!(
        invocations_of(&remove_action_invocations).is_empty(),
        "{}",
        if invoke_on_unsubscribe { "the initial items in the collection were never published" } else { "the operator was instructed to not invoke the removal action, upon stream failure" }
    );

    subscription.dispose();
    source.dispose();
}

#[test]
fn source_fails_immediately_completion_propagates_case1() {
    source_fails_immediately_completion_propagates(true);
}

#[test]
fn source_fails_immediately_completion_propagates_case2() {
    source_fails_immediately_completion_propagates(false);
}
