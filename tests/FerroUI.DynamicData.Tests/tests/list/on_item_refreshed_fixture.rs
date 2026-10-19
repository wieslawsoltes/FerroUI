//! `OnItemRefreshedFixture` (`DynamicData.Tests/List/OnItemRefreshedFixture.cs`): the action of
//! the items refreshed in a list, run for refreshes only, and the propagation of the end of the
//! source.

use crate::utilities::{ListItemRecordingObserver, ListObservableExtensions, TestSourceList};
use ferroui_dynamic_data::list::{IObservableList, ObservableListEx};
use ferroui_system::{IDisposable, ObservableError};
use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

type Invocations = Rc<RefCell<Vec<i32>>>;
type Results = Rc<ListItemRecordingObserver<i32>>;

/// The error of the tests (`new Exception()`).
#[derive(Debug)]
struct TestError;

impl fmt::Display for TestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("the test error")
    }
}

impl std::error::Error for TestError {}

fn new_error() -> ObservableError {
    Rc::new(TestError)
}

/// `refreshActionInvocations.Add`.
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

/// The operator under test, validated and recorded.
fn subscribe(source: &TestSourceList<i32>, invocations: &Invocations) -> (Rc<dyn IDisposable>, Results) {
    source.connect().on_item_refreshed(record_into(invocations)).validate_list_change_sets().record_list_items()
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

/// The checks after the subscription to a list with initial items, and the reset of the records.
#[track_caller]
fn assert_initial_items_published(results: &Results, source: &TestSourceList<i32>, invocations: &Invocations, because: &str) {
    assert_running(results);
    assert_single_change_set(results, "the initial items should have been published");
    assert_items_propagated(results, source);
    results.clear_change_sets();

    assert!(invocations_of(invocations).is_empty(), "{because}");
}

fn item_is_added_refresh_action_is_not_invoked(initial_item_count: i32, insertion_index: usize) {
    let source = TestSourceList::<i32>::new();

    if initial_item_count != 0 {
        add_range(&source, range(initial_item_count));
    }

    let refresh_action_invocations: Invocations = Rc::new(RefCell::new(Vec::new()));

    // UUT Construction
    let (subscription, results) = subscribe(&source, &refresh_action_invocations);

    assert_running(&results);
    if initial_item_count == 0 {
        assert!(results.recorded_change_sets().is_empty(), "there were no initial items to be published");
    } else {
        assert_single_change_set(&results, "the initial items should have been published");
    }
    assert_items_propagated(&results, &source);
    results.clear_change_sets();

    assert!(invocations_of(&refresh_action_invocations).is_empty(), "no items were refreshed within the collection");

    // UUT Action
    insert(&source, insertion_index, initial_item_count);

    assert_running(&results);
    assert_single_change_set(&results, "an item was refreshed within the collection");
    assert_items_propagated(&results, &source);

    assert!(invocations_of(&refresh_action_invocations).is_empty(), "no items were refreshed within the collection");

    subscription.dispose();
    source.dispose();
}

#[test]
fn item_is_added_refresh_action_is_not_invoked_case1() {
    item_is_added_refresh_action_is_not_invoked(0, 0);
}

#[test]
fn item_is_added_refresh_action_is_not_invoked_case2() {
    item_is_added_refresh_action_is_not_invoked(1, 0);
}

#[test]
fn item_is_added_refresh_action_is_not_invoked_case3() {
    item_is_added_refresh_action_is_not_invoked(1, 1);
}

#[test]
fn item_is_added_refresh_action_is_not_invoked_case4() {
    item_is_added_refresh_action_is_not_invoked(5, 0);
}

#[test]
fn item_is_added_refresh_action_is_not_invoked_case5() {
    item_is_added_refresh_action_is_not_invoked(5, 2);
}

#[test]
fn item_is_added_refresh_action_is_not_invoked_case6() {
    item_is_added_refresh_action_is_not_invoked(5, 5);
}

fn item_is_moved_refresh_action_is_not_invoked(initial_item_count: i32, original_index: usize, destination_index: usize) {
    let source = TestSourceList::<i32>::new();

    add_range(&source, range(initial_item_count));

    let refresh_action_invocations: Invocations = Rc::new(RefCell::new(Vec::new()));

    // UUT Construction
    let (subscription, results) = subscribe(&source, &refresh_action_invocations);

    assert_initial_items_published(&results, &source, &refresh_action_invocations, "no items were refreshed within the collection");

    // UUT Action
    move_item(&source, original_index, destination_index);

    assert_running(&results);
    assert_single_change_set(&results, "an item was moved within the collection");
    assert_items_propagated(&results, &source);

    assert!(invocations_of(&refresh_action_invocations).is_empty(), "no items were removed from the collection");

    subscription.dispose();
    source.dispose();
}

#[test]
fn item_is_moved_refresh_action_is_not_invoked_case1() {
    item_is_moved_refresh_action_is_not_invoked(2, 0, 1);
}

#[test]
fn item_is_moved_refresh_action_is_not_invoked_case2() {
    item_is_moved_refresh_action_is_not_invoked(2, 1, 0);
}

#[test]
fn item_is_moved_refresh_action_is_not_invoked_case3() {
    item_is_moved_refresh_action_is_not_invoked(5, 0, 4);
}

#[test]
fn item_is_moved_refresh_action_is_not_invoked_case4() {
    item_is_moved_refresh_action_is_not_invoked(5, 4, 0);
}

#[test]
fn item_is_moved_refresh_action_is_not_invoked_case5() {
    item_is_moved_refresh_action_is_not_invoked(5, 1, 3);
}

#[test]
fn item_is_moved_refresh_action_is_not_invoked_case6() {
    item_is_moved_refresh_action_is_not_invoked(5, 3, 1);
}

// Upstream names this test "RefreshActionIsNotInvoked" although it checks that the action is run
// for the refreshed item; the name is kept.
fn item_is_refreshed_refresh_action_is_not_invoked(initial_item_count: i32, refresh_index: usize) {
    let source = TestSourceList::<i32>::new();

    add_range(&source, range(initial_item_count));

    let refresh_action_invocations: Invocations = Rc::new(RefCell::new(Vec::new()));

    // UUT Construction
    let (subscription, results) = subscribe(&source, &refresh_action_invocations);

    assert_initial_items_published(&results, &source, &refresh_action_invocations, "no items were refreshed within the collection");

    // UUT Action
    let refreshed_item = source.items()[refresh_index];
    source.refresh_at(refresh_index);

    assert_running(&results);
    assert_single_change_set(&results, "an item was refreshed within the collection");
    assert_items_propagated(&results, &source);

    assert_eq!(invocations_of(&refresh_action_invocations), vec![refreshed_item], "1 item were refreshed within the collection");

    subscription.dispose();
    source.dispose();
}

#[test]
fn item_is_refreshed_refresh_action_is_not_invoked_case1() {
    item_is_refreshed_refresh_action_is_not_invoked(1, 0);
}

#[test]
fn item_is_refreshed_refresh_action_is_not_invoked_case2() {
    item_is_refreshed_refresh_action_is_not_invoked(5, 0);
}

#[test]
fn item_is_refreshed_refresh_action_is_not_invoked_case3() {
    item_is_refreshed_refresh_action_is_not_invoked(5, 2);
}

#[test]
fn item_is_refreshed_refresh_action_is_not_invoked_case4() {
    item_is_refreshed_refresh_action_is_not_invoked(5, 4);
}

// Upstream names this test "RefreshActionIsInvoked" although it checks that the action is not
// run; the name is kept.
fn item_is_removed_refresh_action_is_invoked(initial_item_count: i32, removal_index: usize) {
    let source = TestSourceList::<i32>::new();

    if initial_item_count != 0 {
        add_range(&source, range(initial_item_count));
    }

    let refresh_action_invocations: Invocations = Rc::new(RefCell::new(Vec::new()));

    // UUT Construction
    let (subscription, results) = subscribe(&source, &refresh_action_invocations);

    assert_initial_items_published(&results, &source, &refresh_action_invocations, "no items were refreshed within the collection");

    // UUT Action
    let _removed_item = source.items()[removal_index];
    remove_at(&source, removal_index);

    assert_running(&results);
    assert_single_change_set(&results, "an item was removed from the collection");
    assert_items_propagated(&results, &source);

    assert!(invocations_of(&refresh_action_invocations).is_empty(), "no items were refreshed within the collection");

    subscription.dispose();
    source.dispose();
}

#[test]
fn item_is_removed_refresh_action_is_invoked_case1() {
    item_is_removed_refresh_action_is_invoked(1, 0);
}

#[test]
fn item_is_removed_refresh_action_is_invoked_case2() {
    item_is_removed_refresh_action_is_invoked(5, 0);
}

#[test]
fn item_is_removed_refresh_action_is_invoked_case3() {
    item_is_removed_refresh_action_is_invoked(5, 2);
}

#[test]
fn item_is_removed_refresh_action_is_invoked_case4() {
    item_is_removed_refresh_action_is_invoked(5, 4);
}

fn item_is_replaced_refresh_action_is_invoked_for_old_item(initial_item_count: i32, replacement_index: usize) {
    let source = TestSourceList::<i32>::new();

    add_range(&source, range(initial_item_count));

    let refresh_action_invocations: Invocations = Rc::new(RefCell::new(Vec::new()));

    // UUT Construction
    let (subscription, results) = subscribe(&source, &refresh_action_invocations);

    assert_initial_items_published(&results, &source, &refresh_action_invocations, "no items were refreshed within the collection");

    // UUT Action
    replace_at(&source, replacement_index, initial_item_count);

    assert_running(&results);
    assert_single_change_set(&results, "an item was replaced within the collection");
    assert_items_propagated(&results, &source);

    assert!(invocations_of(&refresh_action_invocations).is_empty(), "no items were refreshed within the collection");

    subscription.dispose();
    source.dispose();
}

#[test]
fn item_is_replaced_refresh_action_is_invoked_for_old_item_case1() {
    item_is_replaced_refresh_action_is_invoked_for_old_item(1, 0);
}

#[test]
fn item_is_replaced_refresh_action_is_invoked_for_old_item_case2() {
    item_is_replaced_refresh_action_is_invoked_for_old_item(5, 0);
}

#[test]
fn item_is_replaced_refresh_action_is_invoked_for_old_item_case3() {
    item_is_replaced_refresh_action_is_invoked_for_old_item(5, 2);
}

#[test]
fn item_is_replaced_refresh_action_is_invoked_for_old_item_case4() {
    item_is_replaced_refresh_action_is_invoked_for_old_item(5, 4);
}

fn item_range_is_removed_refresh_action_is_invoked_for_each_item(initial_item_count: i32, removal_index: usize, removal_count: usize) {
    let source = TestSourceList::<i32>::new();

    add_range(&source, range(initial_item_count));

    let refresh_action_invocations: Invocations = Rc::new(RefCell::new(Vec::new()));

    // UUT Construction
    let (subscription, results) = subscribe(&source, &refresh_action_invocations);

    assert_initial_items_published(&results, &source, &refresh_action_invocations, "no items were refreshed within the collection");

    // UUT Action
    remove_range(&source, removal_index, removal_count);

    assert_running(&results);
    assert_single_change_set(&results, &format!("{removal_count} item{} should have been removed", if removal_count == 1 { "" } else { "s" }));
    assert_items_propagated(&results, &source);

    assert!(invocations_of(&refresh_action_invocations).is_empty(), "no items were refreshed within the collection");

    subscription.dispose();
    source.dispose();
}

#[test]
fn item_range_is_removed_refresh_action_is_invoked_for_each_item_case1() {
    item_range_is_removed_refresh_action_is_invoked_for_each_item(1, 0, 1);
}

#[test]
fn item_range_is_removed_refresh_action_is_invoked_for_each_item_case2() {
    item_range_is_removed_refresh_action_is_invoked_for_each_item(5, 0, 1);
}

#[test]
fn item_range_is_removed_refresh_action_is_invoked_for_each_item_case3() {
    item_range_is_removed_refresh_action_is_invoked_for_each_item(5, 2, 1);
}

#[test]
fn item_range_is_removed_refresh_action_is_invoked_for_each_item_case4() {
    item_range_is_removed_refresh_action_is_invoked_for_each_item(5, 1, 3);
}

#[test]
fn item_range_is_removed_refresh_action_is_invoked_for_each_item_case5() {
    item_range_is_removed_refresh_action_is_invoked_for_each_item(5, 0, 5);
}

fn items_are_cleared_refresh_action_is_invoked_for_each_item(initial_item_count: i32) {
    let source = TestSourceList::<i32>::new();

    add_range(&source, range(initial_item_count));

    let refresh_action_invocations: Invocations = Rc::new(RefCell::new(Vec::new()));

    // UUT Construction
    let (subscription, results) = subscribe(&source, &refresh_action_invocations);

    assert_initial_items_published(&results, &source, &refresh_action_invocations, "no items were refreshed within the collection");

    // UUT Action
    clear(&source);

    assert_running(&results);
    assert_single_change_set(&results, "all items in the collection should have been removed");
    assert_items_propagated(&results, &source);

    assert!(invocations_of(&refresh_action_invocations).is_empty(), "no items were refreshed within the collection");

    subscription.dispose();
    source.dispose();
}

#[test]
fn items_are_cleared_refresh_action_is_invoked_for_each_item_case1() {
    items_are_cleared_refresh_action_is_invoked_for_each_item(1);
}

#[test]
fn items_are_cleared_refresh_action_is_invoked_for_each_item_case2() {
    items_are_cleared_refresh_action_is_invoked_for_each_item(5);
}

#[test]
fn source_completes_asynchronously_completion_propagates() {
    let source = TestSourceList::<i32>::new();

    add_range(&source, vec![1, 2, 3]);

    let refresh_action_invocations: Invocations = Rc::new(RefCell::new(Vec::new()));

    // UUT Construction
    let (subscription, results) = subscribe(&source, &refresh_action_invocations);

    assert_initial_items_published(&results, &source, &refresh_action_invocations, "no items have been removed from the collection");

    // UUT Action
    source.complete();

    assert!(results.error().is_none(), "no errors should have occurred");
    assert!(results.has_completed(), "the source has completed");
    assert!(results.recorded_change_sets().is_empty(), "no changes were made to the collection");

    assert!(invocations_of(&refresh_action_invocations).is_empty(), "no items were refreshed within the collection");

    subscription.dispose();
    source.dispose();
}

#[test]
fn source_completes_immediately_completion_propagates() {
    let source = TestSourceList::<i32>::new();

    add_range(&source, vec![1, 2, 3]);
    source.complete();

    let refresh_action_invocations: Invocations = Rc::new(RefCell::new(Vec::new()));

    // UUT Construction & Action
    let (subscription, results) = subscribe(&source, &refresh_action_invocations);

    assert!(results.error().is_none(), "no errors should have occurred");
    assert!(results.has_completed(), "the source has completed");
    assert_single_change_set(&results, "the initial items should have been published");
    assert_items_propagated(&results, &source);

    assert!(invocations_of(&refresh_action_invocations).is_empty(), "no items were refreshed within the collection");

    subscription.dispose();
    source.dispose();
}

#[test]
fn source_fails_asynchronously_completion_propagates() {
    let source = TestSourceList::<i32>::new();

    add_range(&source, vec![1, 2, 3]);

    let refresh_action_invocations: Invocations = Rc::new(RefCell::new(Vec::new()));

    // UUT Construction
    let (subscription, results) = subscribe(&source, &refresh_action_invocations);

    assert_initial_items_published(&results, &source, &refresh_action_invocations, "no items have been removed from the collection");

    // UUT Action
    let error = new_error();
    source.set_error(error.clone());

    let recorded_error = results.error().expect("errors within the stream should propagate");
    assert!(Rc::ptr_eq(&recorded_error, &error), "errors within the stream should propagate");
    assert!(results.recorded_change_sets().is_empty(), "no changes were made to the collection");

    assert!(invocations_of(&refresh_action_invocations).is_empty(), "no items were refreshed within the collection");

    subscription.dispose();
    source.dispose();
}

#[test]
fn source_fails_immediately_completion_propagates() {
    let source = TestSourceList::<i32>::new();

    add_range(&source, vec![1, 2, 3]);
    let error = new_error();
    source.set_error(error.clone());

    let refresh_action_invocations: Invocations = Rc::new(RefCell::new(Vec::new()));

    // UUT Construction & Action
    let (subscription, results) = subscribe(&source, &refresh_action_invocations);

    let recorded_error = results.error().expect("errors within the stream should propagate");
    assert!(Rc::ptr_eq(&recorded_error, &error), "errors within the stream should propagate");
    assert!(results.recorded_change_sets().is_empty(), "an error occurred during subscription");

    assert!(invocations_of(&refresh_action_invocations).is_empty(), "no items were refreshed within the collection");

    subscription.dispose();
    source.dispose();
}
