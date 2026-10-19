//! `FilterFixture.Static` (`DynamicData.Tests/List/FilterFixture.Static.cs`): the filter of a
//! list by a predicate that does not change (`Filter(Func<T, bool>)`, `Filter.Static`).

use super::filter_fixture::{is_same_error, Item, SourceType};
use crate::domain::{fakers, Person, Randomizer};
use crate::utilities::{assert_equivalent_because, assert_list_should_have_refreshed, ListObservableExtensions, ObservableExtensions, TestSourceList};
use ferroui_dynamic_data::list::{remove_many, ChangeSet, IObservableList, ObservableListEx};
use ferroui_reactive::subjects::{Subject, SubjectBase};
use ferroui_reactive::ObservableExt;
use ferroui_reactive_testing::new_error;
use ferroui_system::IDisposable;
use std::rc::Rc;

const AUTO_REFRESH_SEED: i32 = 0x1165;

/// The six items that most tests start with: the first three included, the last three not.
fn initial_items() -> Vec<Rc<Item>> {
    vec![Item::new(1, true), Item::new(2, true), Item::new(3, true), Item::new(4, false), Item::new(5, false), Item::new(6, false)]
}

/// `source.Items.Where(Item.FilterByIsIncluded)`.
fn included_items(source: &TestSourceList<Rc<Item>>) -> Vec<Rc<Item>> {
    source.items().into_iter().filter(Item::filter_by_is_included).collect()
}

#[test]
fn duplicate_items_are_added_items_are_tracked_separately() {
    // Setup
    let source = TestSourceList::<i32>::new();

    source.edit(|list| list.add_range(vec![1, 2, 3, 4, 3, 2]));

    // UUT Initialization
    let (subscription, results) = source.connect().filter(|item: &i32| item % 2 == 0).validate_synchronization().validate_list_change_sets().record_list_items();

    assert!(results.error().is_none());
    assert_eq!(results.recorded_change_sets().len(), 1, "there were initial items to publish");
    assert_equivalent_because(&results.recorded_items(), &[2, 4, 2], "all matching items should have been added");
    assert!(!results.has_completed(), "the source has not completed");

    // UUT Action
    source.edit(|list| {
        list.remove(&2);
    });

    assert!(results.error().is_none());
    assert_eq!(results.recorded_change_sets().iter().skip(1).count(), 1, "an operation was performed upon an included item");
    assert_equivalent_because(&results.recorded_items(), &[4, 2], "only one of the duplicate items was removed");
    assert!(!results.has_completed(), "the source has not completed");

    // UUT Action
    source.edit(|list| {
        list.remove(&3);
    });

    assert!(results.error().is_none());
    assert_eq!(results.recorded_change_sets().iter().skip(2).count(), 0, "an operation was performed upon an excluded item");
    assert!(!results.has_completed(), "the source has not completed");

    // UUT Action
    source.edit(|list| {
        list.remove(&2);
    });

    assert!(results.error().is_none());
    assert_eq!(results.recorded_change_sets().iter().skip(2).count(), 1, "an operation was performed upon an included item");
    assert_equivalent_because(&results.recorded_items(), &[4], "the second duplicate item was removed");
    assert!(!results.has_completed(), "the source has not completed");

    subscription.dispose();
    source.dispose();
}

#[test]
fn excluded_item_is_added_no_changes_are_made() {
    // Setup
    let source = TestSourceList::<Rc<Item>>::new();

    // UUT Initialization
    let (subscription, results) = source.connect().filter(Item::filter_by_is_included).validate_synchronization().validate_list_change_sets().record_list_items();

    assert!(results.error().is_none());
    assert!(results.recorded_change_sets().is_empty(), "there were no initial items to publish");
    assert!(results.recorded_items().is_empty(), "no items have been added to the source");
    assert!(!results.has_completed(), "the source has not completed");

    // UUT Action
    source.edit(|list| list.add(Item::new(1, false)));

    assert!(results.error().is_none());
    assert_eq!(results.recorded_change_sets().iter().skip(1).count(), 0, "empty changesets should be suppressed");
    assert!(!results.has_completed(), "the source has not completed");

    subscription.dispose();
    source.dispose();
}

#[test]
fn excluded_item_is_removed_no_changes_are_made() {
    // Setup
    let source = TestSourceList::<Rc<Item>>::new();

    source.edit(|list| list.add_range(initial_items()));

    // UUT Initialization
    let (subscription, results) = source.connect().filter(Item::filter_by_is_included).validate_synchronization().validate_list_change_sets().record_list_items();

    assert!(results.error().is_none());
    assert_eq!(results.recorded_change_sets().len(), 1, "the initial changeset should propagate");
    assert_eq!(results.recorded_items(), included_items(&source), "all matching items should have been added");
    assert!(!results.has_completed(), "the source has not completed");

    // UUT Action
    source.edit(|list| list.remove_at(5));

    assert!(results.error().is_none());
    assert_eq!(results.recorded_change_sets().iter().skip(1).count(), 0, "empty changesets should be suppressed");
    assert!(!results.has_completed(), "the source has not completed");

    subscription.dispose();
    source.dispose();
}

#[test]
fn excluded_items_are_removed_no_changes_are_made() {
    // Setup
    let source = TestSourceList::<Rc<Item>>::new();

    source.edit(|list| list.add_range(initial_items()));

    // UUT Initialization
    let (subscription, results) = source.connect().filter(Item::filter_by_is_included).validate_synchronization().validate_list_change_sets().record_list_items();

    assert!(results.error().is_none());
    assert_eq!(results.recorded_change_sets().len(), 1, "the initial changeset should propagate");
    assert_eq!(results.recorded_items(), included_items(&source), "all matching items should have been added");
    assert!(!results.has_completed(), "the source has not completed");

    // UUT Action
    let excluded_items: Vec<Rc<Item>> = source.items().into_iter().filter(|item| !item.is_included()).collect();
    source.edit(|list| remove_many(list, &excluded_items));

    assert!(results.error().is_none());
    assert_eq!(results.recorded_change_sets().iter().skip(1).count(), 0, "empty changesets should be suppressed");
    assert!(!results.has_completed(), "the source has not completed");

    subscription.dispose();
    source.dispose();
}

#[test]
fn items_are_added_matching_items_propagate() {
    // Setup
    let source = TestSourceList::<Rc<Item>>::new();

    // UUT Initialization
    let (subscription, results) = source.connect().filter(Item::filter_by_is_included).validate_synchronization().validate_list_change_sets().record_list_items();

    assert!(results.error().is_none());
    assert!(results.recorded_change_sets().is_empty(), "there were no initial items to publish");
    assert!(results.recorded_items().is_empty(), "no items have been added to the source");
    assert!(!results.has_completed(), "the source has not completed");

    // UUT Action
    source.edit(|list| list.add_range(initial_items()));

    assert!(results.error().is_none());
    assert_eq!(results.recorded_change_sets().len(), 1, "1 source operation was performed");
    assert_eq!(results.recorded_items(), included_items(&source), "all matching items should have been added");
    assert!(!results.has_completed(), "the source has not completed");

    subscription.dispose();
    source.dispose();
}

#[test]
fn items_are_moved_matching_movements_propagate() {
    // Setup
    let source = TestSourceList::<Rc<Item>>::new();

    source.edit(|list| list.add_range(initial_items()));

    // UUT Initialization
    let (subscription, results) = source.connect().filter(Item::filter_by_is_included).validate_synchronization().validate_list_change_sets().record_list_items();

    assert!(results.error().is_none());
    assert_eq!(results.recorded_change_sets().len(), 1, "the initial changeset should propagate");
    assert_eq!(results.recorded_items(), included_items(&source), "all matching items should have been added");
    assert!(!results.has_completed(), "the source has not completed");

    // UUT Action: Moves for matching items,
    source.edit(|items| {
        items.move_index(2, 0);
        items.move_index(1, 5);
    });

    assert!(results.error().is_none());
    assert_eq!(results.recorded_change_sets().iter().skip(1).count(), 1, "1 source operation was performed");
    assert_eq!(results.recorded_items(), included_items(&source), "all matching items should have been moved, accordingly");
    assert!(!results.has_completed(), "the source has not completed");

    // UUT Action: Moves for excluded items
    source.edit(|items| {
        items.move_index(4, 1);
        items.move_index(3, 5);
    });

    assert!(results.error().is_none());
    assert_eq!(results.recorded_change_sets().iter().skip(2).count(), 0, "empty changesets should be suppressed");
    assert!(!results.has_completed(), "the source has not completed");

    subscription.dispose();
    source.dispose();
}

#[test]
fn items_are_refreshed_items_are_re_filtered_or_refreshed() {
    // Setup
    let source = TestSourceList::<Rc<Item>>::new();

    source.edit(|list| list.add_range(initial_items()));

    // UUT Initialization
    let (subscription, results) = source.connect().filter(Item::filter_by_is_included).validate_synchronization().validate_list_change_sets().record_list_items();

    assert!(results.error().is_none());
    assert_eq!(results.recorded_change_sets().len(), 1, "the initial changeset should propagate");
    assert_equivalent_because(&results.recorded_items(), &included_items(&source), "all matching items should have propagated");
    assert!(!results.has_completed(), "the source has not completed");

    // UUT Action (add items)
    for item in source.items() {
        item.set_is_included(true);
    }

    source.refresh();

    assert!(results.error().is_none());
    assert_eq!(results.recorded_change_sets().iter().skip(1).count(), 1, "1 source operation was performed");
    assert_list_should_have_refreshed(&results.recorded_change_sets()[1], &source.items()[..3], "all unchanged items should have been refreshed");
    assert_equivalent_because(&results.recorded_items(), &included_items(&source), "all newly-matching items should have been added");
    assert!(!results.has_completed(), "the source has not completed");

    // UUT Action (remove items)
    for item in source.items().into_iter().take(3) {
        item.set_is_included(false);
    }

    source.refresh();

    assert!(results.error().is_none());
    assert_eq!(results.recorded_change_sets().iter().skip(2).count(), 1, "1 source operation was performed");
    assert_list_should_have_refreshed(&results.recorded_change_sets()[2], &source.items()[3..], "all unchanged items should have been refreshed");
    assert_equivalent_because(&results.recorded_items(), &included_items(&source), "all newly-excluded items should have been removed");
    assert!(!results.has_completed(), "the source has not completed");

    subscription.dispose();
    source.dispose();
}

#[test]
fn items_are_replaced_items_are_re_filtered() {
    // Setup
    let source = TestSourceList::<Rc<Item>>::new();

    source.edit(|list| list.add_range(initial_items()));

    // UUT Initialization
    let (subscription, results) = source.connect().filter(Item::filter_by_is_included).validate_synchronization().validate_list_change_sets().record_list_items();

    assert!(results.error().is_none());
    assert_eq!(results.recorded_change_sets().len(), 1, "the initial changeset should propagate");
    assert_eq!(results.recorded_items(), included_items(&source), "all matching items should have propagated");
    assert!(!results.has_completed(), "the source has not completed");

    // UUT Action (add and replace items)
    source.edit(|items| {
        items.set(0, Item::new(1, true));
        items.set(1, Item::new(2, true));
        items.set(2, Item::new(3, true));
        items.set(3, Item::new(4, true));
        items.set(4, Item::new(5, true));
        items.set(5, Item::new(6, true));
    });

    assert!(results.error().is_none());
    assert_eq!(results.recorded_change_sets().iter().skip(1).count(), 1, "1 source operation was performed");
    assert_eq!(results.recorded_items(), included_items(&source), "all newly-matching items should have been added");
    assert!(!results.has_completed(), "the source has not completed");

    // UUT Action (remove and replace items)
    source.edit(|items| {
        items.set(0, Item::new(1, false));
        items.set(1, Item::new(2, false));
        items.set(2, Item::new(3, false));
        items.set(3, Item::new(4, true));
        items.set(4, Item::new(5, true));
        items.set(5, Item::new(6, true));
    });

    assert!(results.error().is_none());
    assert_eq!(results.recorded_change_sets().iter().skip(2).count(), 1, "1 source operation was performed");
    assert_eq!(results.recorded_items(), included_items(&source), "all newly-excluded items should have been removed");
    assert!(!results.has_completed(), "the source has not completed");

    subscription.dispose();
    source.dispose();
}

#[test]
fn matching_item_is_added_item_propagates() {
    // Setup
    let source = TestSourceList::<Rc<Item>>::new();

    // UUT Initialization
    let (subscription, results) = source.connect().filter(Item::filter_by_is_included).validate_synchronization().validate_list_change_sets().record_list_items();

    assert!(results.error().is_none());
    assert!(results.recorded_change_sets().is_empty(), "there were no initial items to publish");
    assert!(results.recorded_items().is_empty(), "no items have been added to the source");
    assert!(!results.has_completed(), "the source has not completed");

    // UUT Action
    source.edit(|list| list.add(Item::new(1, true)));

    assert!(results.error().is_none());
    assert_eq!(results.recorded_change_sets().len(), 1, "1 source operation was performed");
    assert_equivalent_because(&results.recorded_items(), &source.items(), "the matching item should have been added");
    assert!(!results.has_completed(), "the source has not completed");

    subscription.dispose();
    source.dispose();
}

#[test]
fn matching_item_is_removed_removal_propagates() {
    // Setup
    let source = TestSourceList::<Rc<Item>>::new();

    source.edit(|list| list.add_range(initial_items()));

    // UUT Initialization
    let (subscription, results) = source.connect().filter(Item::filter_by_is_included).validate_synchronization().validate_list_change_sets().record_list_items();

    assert!(results.error().is_none());
    assert_eq!(results.recorded_change_sets().len(), 1, "the initial changeset should propagate");
    assert_eq!(results.recorded_items(), included_items(&source), "all matching items should have been added");
    assert!(!results.has_completed(), "the source has not completed");

    // UUT Action
    let _removed_item = source.items()[2].clone();
    source.edit(|list| list.remove_at(2));

    assert!(results.error().is_none());
    assert_eq!(results.recorded_change_sets().iter().skip(1).count(), 1, "1 source operation was performed");
    assert_eq!(results.recorded_items(), included_items(&source), "a matching item was removed");
    assert!(!results.has_completed(), "the source has not completed");

    subscription.dispose();
    source.dispose();
}

#[test]
fn matching_items_are_removed_removals_propagate() {
    // Setup
    let source = TestSourceList::<Rc<Item>>::new();

    source.edit(|list| list.add_range(initial_items()));

    // UUT Initialization
    let (subscription, results) = source.connect().filter(Item::filter_by_is_included).validate_synchronization().validate_list_change_sets().record_list_items();

    assert!(results.error().is_none());
    assert_eq!(results.recorded_change_sets().len(), 1, "the initial changeset should propagate");
    assert_eq!(results.recorded_items(), included_items(&source), "all matching items should have propagated");
    assert!(!results.has_completed(), "the source has not completed");

    // UUT Action
    let matching_items = included_items(&source);
    source.edit(|list| remove_many(list, &matching_items));

    assert!(results.error().is_none());
    assert_eq!(results.recorded_change_sets().iter().skip(1).count(), 1, "1 source operation was performed");
    assert!(results.recorded_items().is_empty(), "all matching items were removed");
    assert!(!results.has_completed(), "the source has not completed");

    subscription.dispose();
    source.dispose();
}

// Not ported: PredicateIsNull_ThrowsException - argument validation

fn source_completes_completion_propagates(source_type: SourceType) {
    // Setup
    let source = TestSourceList::<Rc<Item>>::new();

    source.edit(|list| list.add_range(vec![Item::new(1, true)]));

    if source_type == SourceType::Immediate {
        source.complete();
    }

    // UUT Initialization & Action
    let (subscription, results) = source.connect().filter(Item::filter_by_is_included).validate_synchronization().validate_list_change_sets().record_list_items();

    if source_type == SourceType::Asynchronous {
        source.complete();
    }

    assert!(results.error().is_none());
    assert_eq!(results.recorded_change_sets().len(), 1, "the initial item should have been published");
    assert_equivalent_because(&results.recorded_items(), &source.items(), "the initial item should have been published");
    assert!(results.has_completed(), "the source has completed");

    subscription.dispose();
    source.dispose();
}

#[test]
fn source_completes_completion_propagates_case1() {
    source_completes_completion_propagates(SourceType::Asynchronous);
}

#[test]
fn source_completes_completion_propagates_case2() {
    source_completes_completion_propagates(SourceType::Immediate);
}

fn source_fails_error_propagates(source_type: SourceType) {
    let source = TestSourceList::<Rc<Item>>::new();

    source.edit(|list| list.add_range(vec![Item::new(1, true)]));

    let error = new_error("Test");

    if source_type == SourceType::Immediate {
        source.set_error(error.clone());
    }

    let (subscription, results) = source.connect().filter(Item::filter_by_is_included).validate_synchronization().validate_list_change_sets().record_list_items();

    if source_type == SourceType::Asynchronous {
        source.set_error(error.clone());
    }

    assert!(is_same_error(results.error(), &error));
    if source_type == SourceType::Asynchronous {
        assert_eq!(results.recorded_change_sets().len(), 1, "the initial item should have been published");
        assert_equivalent_because(&results.recorded_items(), &source.items(), "the initial item should have been published");
    } else {
        assert!(results.recorded_change_sets().is_empty(), "an error occurred during initialization");
    }

    subscription.dispose();
    source.dispose();
}

#[test]
fn source_fails_error_propagates_case1() {
    source_fails_error_propagates(SourceType::Asynchronous);
}

#[test]
fn source_fails_error_propagates_case2() {
    source_fails_error_propagates(SourceType::Immediate);
}

// Not ported: SourceIsNull_ThrowsException - argument validation

#[test]
fn subscription_is_disposed_subscription_disposal_propagates() {
    // Setup
    let source = Subject::<ChangeSet<Rc<Item>>>::new();

    // UUT Initialization
    let (subscription, results) = source.as_observable().filter(Item::filter_by_is_included).validate_synchronization().validate_list_change_sets().record_list_items();

    assert!(results.error().is_none());
    assert!(results.recorded_change_sets().is_empty(), "no initial changeset occurred");
    assert!(results.recorded_items().is_empty(), "the source has not initialized");
    assert!(!results.has_completed(), "the source has not completed");

    // UUT Action
    subscription.dispose();

    assert!(!source.has_observers(), "subscription disposal should propagate upstream");

    source.dispose();
}

/// List filtering must bind exactly the matching items, so an item that stops matching is removed
/// rather than retained as a stale extra entry.
#[test]
fn auto_refresh_filter_update_collection_updated() {
    // Setup: every generated item starts below a derived threshold, so all of them match initially.
    let randomizer = Randomizer::new(AUTO_REFRESH_SEED);
    let source = TestSourceList::<Rc<Person>>::new();
    let people: Vec<Rc<Person>> = fakers::person().use_seed(randomizer.int()).generate(randomizer.int_range(3, 8) as usize);
    let exclusive_age = people.iter().map(|person| person.age()).max().expect("people were generated") + 1;

    // UUT Initialization
    let (bound, collection) = source
        .connect()
        .auto_refresh(ferroui_system::prop!(Person => age))
        .filter(move |person: &Rc<Person>| person.age() < exclusive_age)
        .validate_synchronization()
        .validate_list_change_sets()
        .bind_read_only();
    let subscription = bound.subscribe_ignore();

    // UUT Action
    source.edit(|list| list.add_range(people.clone()));

    assert_equivalent_because(&collection.to_vec(), &people, "every item matches the predicate");

    // UUT Action
    people[0].set_age(exclusive_age);

    assert_equivalent_because(
        &collection.to_vec(),
        &people.iter().filter(|person| person.age() < exclusive_age).cloned().collect::<Vec<_>>(),
        "an item that stops matching must be removed, not retained as a stale extra entry",
    );

    // UUT Action
    for person in &people {
        person.set_age(exclusive_age);
    }

    assert!(collection.is_empty(), "no item matches the predicate any longer");

    subscription.dispose();
    source.dispose();
}
