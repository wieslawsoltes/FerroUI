//! `FilterImmutableFixture` (`DynamicData.Tests/Cache/FilterImmutableFixture.cs`): the filter of
//! items that never change, which caches nothing, drops moves and discards indexes.

use crate::utilities::{assert_equivalent_because, CacheObservableExtensions};
use ferroui_dynamic_data::cache::{Change, ChangeReason, ChangeSet, ObservableCacheEx};
use ferroui_reactive::disposables::Disposable;
use ferroui_reactive::subjects::Subject;
use ferroui_reactive::Observable;
use ferroui_reactive_testing::new_error;
use ferroui_system::{IObserver, ObservableError};
use std::rc::Rc;

/// `Item`: upstream's class with `required` init-only properties.
#[derive(Debug, PartialEq)]
struct Item {
    id: i32,
    is_included: bool,
}

impl Item {
    fn new(id: i32, is_included: bool) -> Rc<Self> {
        Rc::new(Self { id, is_included })
    }

    /// `Item.KeySelector`.
    fn key_selector(item: &Rc<Item>) -> i32 {
        item.id
    }

    /// `Item.Predicate`.
    fn predicate(item: &Rc<Item>) -> bool {
        item.is_included
    }
}

type Source = Rc<Subject<ChangeSet<Rc<Item>, i32>>>;

fn new_source() -> Source {
    Subject::<ChangeSet<Rc<Item>, i32>>::new()
}

#[test]
fn items_are_manipulated_unmatched_items_are_excluded_and_indexes_are_discarded() {
    let source = new_source();

    let (_subscription, results) = source.as_observable().filter_immutable(Item::predicate).validate_cache_change_sets(Item::key_selector).record_cache_items();

    // Add items
    let item1 = Item::new(1, true);
    let item2 = Item::new(2, false);
    source.on_next(ChangeSet::from(vec![
        Change::with_index(ChangeReason::Add, item1.id, item1.clone(), Some(0)),
        Change::with_index(ChangeReason::Add, item2.id, item2.clone(), Some(1)),
    ]));

    assert!(results.error().is_none());
    assert_eq!(results.recorded_change_sets().len(), 1, "1 source operation was performed");
    assert_equivalent_because(&results.recorded_items(), &[item1.clone()], "2 items were added, with 1 excluded");

    // Replace items, changing inclusion
    let item3 = Item::new(item1.id, false);
    let item4 = Item::new(item2.id, true);
    source.on_next(ChangeSet::from(vec![
        Change::with_previous(ChangeReason::Update, item3.id, item3.clone(), Some(item1.clone()), Some(0), Some(0)),
        Change::with_previous(ChangeReason::Update, item4.id, item4.clone(), Some(item2.clone()), Some(1), Some(1)),
    ]));

    assert!(results.error().is_none());
    assert_eq!(results.recorded_change_sets().len() - 1, 1, "1 source operation was performed");
    assert_equivalent_because(&results.recorded_items(), &[item4.clone()], "2 items were replaced, with 1 excluded");

    // Replace items, not changing inclusion
    let item5 = Item::new(item3.id, false);
    let item6 = Item::new(item4.id, true);
    source.on_next(ChangeSet::from(vec![
        Change::with_previous(ChangeReason::Update, item5.id, item5.clone(), Some(item3.clone()), Some(0), Some(0)),
        Change::with_previous(ChangeReason::Update, item6.id, item6.clone(), Some(item4.clone()), Some(1), Some(1)),
    ]));

    assert!(results.error().is_none());
    assert_eq!(results.recorded_change_sets().len() - 2, 1, "1 source operation was performed");
    assert_equivalent_because(&results.recorded_items(), &[item6.clone()], "2 items were replaced, with 1 excluded");

    // Refresh items
    source.on_next(ChangeSet::from(vec![
        Change::with_index(ChangeReason::Refresh, item5.id, item5.clone(), Some(0)),
        Change::with_index(ChangeReason::Refresh, item6.id, item6.clone(), Some(1)),
    ]));

    assert!(results.error().is_none());
    assert_eq!(results.recorded_change_sets().len() - 3, 1, "1 source operation was performed");
    assert_equivalent_because(&results.recorded_items(), &[item6.clone()], "2 items were refreshed, with 1 excluded");

    // Remove items
    source.on_next(ChangeSet::from(vec![
        Change::with_index(ChangeReason::Remove, item5.id, item5.clone(), Some(0)),
        Change::with_index(ChangeReason::Remove, item6.id, item6.clone(), Some(1)),
    ]));

    assert!(results.error().is_none());
    assert_eq!(results.recorded_change_sets().len() - 4, 1, "1 source operation was performed");
    assert!(results.recorded_items_by_key().is_empty(), "2 items were removed, with one excluded");

    for changes in results.recorded_change_sets() {
        for change in &changes {
            assert_eq!(change.current_index(), None, "indexes should not be preserved");
            assert_eq!(change.previous_index(), None, "indexes should not be preserved");
        }
    }
    assert!(!results.has_completed());
}

#[test]
fn items_are_moved_changes_are_not_propagated() {
    let source = new_source();

    let (_subscription, results) = source.as_observable().filter_immutable(Item::predicate).validate_cache_change_sets(Item::key_selector).record_cache_items();

    // Initial setup
    let item1 = Item::new(1, true);
    let item2 = Item::new(2, true);
    let item3 = Item::new(3, true);
    source.on_next(ChangeSet::from(vec![
        Change::with_index(ChangeReason::Add, item1.id, item1.clone(), Some(0)),
        Change::with_index(ChangeReason::Add, item2.id, item2.clone(), Some(1)),
        Change::with_index(ChangeReason::Add, item3.id, item3.clone(), Some(2)),
    ]));
    let change_sets_before_move = results.recorded_change_sets().len();

    // Move items
    source.on_next(ChangeSet::from(vec![Change::moved(item1.id, item1.clone(), 2, 0), Change::moved(item2.id, item2.clone(), 0, 1)]));

    assert!(results.error().is_none());
    assert_eq!(results.recorded_change_sets().len() - change_sets_before_move, 0, "move operations should not be propagated");
}

// Not ported: PredicateIsNull_ThrowsException - argument validation

#[test]
fn predicate_throws_exception_is_captured() {
    let source = new_source();

    let error = new_error("Exception");

    let raised = error.clone();
    let (_subscription, results) = source
        .as_observable()
        .try_filter_immutable(move |_: &Rc<Item>| -> Result<bool, ObservableError> { Err(raised.clone()) })
        .validate_cache_change_sets(Item::key_selector)
        .record_cache_items();

    let item1 = Item::new(1, true);
    source.on_next(ChangeSet::from(vec![Change::new(ChangeReason::Add, item1.id, item1.clone())]));

    assert!(results.error().is_some_and(|actual| Rc::ptr_eq(&actual, &error)));
    assert!(results.recorded_change_sets().is_empty(), "no source operations should have been processed");
    assert!(!results.has_completed());
}

#[test]
fn source_completes_completion_is_propagated() {
    let source = new_source();

    let (_subscription, results) = source.as_observable().filter_immutable(Item::predicate).validate_cache_change_sets(Item::key_selector).record_cache_items();

    let item1 = Item::new(1, true);
    source.on_next(ChangeSet::from(vec![Change::new(ChangeReason::Add, item1.id, item1.clone())]));
    source.on_completed();

    assert!(results.error().is_none());
    assert!(results.has_completed());
    assert_eq!(results.recorded_change_sets().len(), 1, "1 source operation was performed");
    assert_equivalent_because(&results.recorded_items(), &[item1.clone()], "1 item was added");

    // Make sure no extraneous notifications are published.
    let item2 = Item::new(2, true);
    source.on_next(ChangeSet::from(vec![Change::new(ChangeReason::Add, item2.id, item2.clone())]));

    assert_eq!(results.recorded_change_sets().len() - 1, 0, "no source operations should have been processed");
}

#[test]
fn source_completes_immediately_completion_is_propagated() {
    let item1 = Item::new(1, true);

    let emitted = item1.clone();
    let source = Observable::create(move |observer: Rc<dyn IObserver<ChangeSet<Rc<Item>, i32>>>| {
        observer.on_next(ChangeSet::from(vec![Change::new(ChangeReason::Add, emitted.id, emitted.clone())]));
        observer.on_completed();
        Disposable::empty()
    });

    let (_subscription, results) = source.filter_immutable(Item::predicate).validate_cache_change_sets(Item::key_selector).record_cache_items();

    assert!(results.error().is_none());
    assert!(results.has_completed());
    assert_eq!(results.recorded_change_sets().len(), 1, "1 source operation was performed");
    assert_equivalent_because(&results.recorded_items(), &[item1.clone()], "1 item was added");
}

#[test]
fn source_errors_error_is_propagated() {
    let source = new_source();

    let error = new_error("Exception");

    let (_subscription, results) = source.as_observable().filter_immutable(Item::predicate).validate_cache_change_sets(Item::key_selector).record_cache_items();

    let item1 = Item::new(1, true);
    source.on_next(ChangeSet::from(vec![Change::new(ChangeReason::Add, item1.id, item1.clone())]));
    source.on_error(error.clone());

    assert!(results.error().is_some_and(|actual| Rc::ptr_eq(&actual, &error)));
    assert!(!results.has_completed());
    assert_eq!(results.recorded_change_sets().len(), 1, "1 source operation was performed");
    assert_equivalent_because(&results.recorded_items(), &[item1.clone()], "1 item was added");

    // Make sure no extraneous notifications are published.
    let item2 = Item::new(2, true);
    source.on_next(ChangeSet::from(vec![Change::new(ChangeReason::Add, item2.id, item2.clone())]));

    assert_eq!(results.recorded_change_sets().len() - 1, 0, "no source operations should have been processed");
}

#[test]
fn source_errors_immediately_error_is_propagated() {
    let item1 = Item::new(1, true);
    let error = new_error("Exception");

    let (emitted, raised) = (item1.clone(), error.clone());
    let source = Observable::create(move |observer: Rc<dyn IObserver<ChangeSet<Rc<Item>, i32>>>| {
        observer.on_next(ChangeSet::from(vec![Change::new(ChangeReason::Add, emitted.id, emitted.clone())]));
        observer.on_error(raised.clone());
        Disposable::empty()
    });

    let (_subscription, results) = source.filter_immutable(Item::predicate).validate_cache_change_sets(Item::key_selector).record_cache_items();

    assert!(results.error().is_some_and(|actual| Rc::ptr_eq(&actual, &error)));
    assert!(!results.has_completed());
    assert_eq!(results.recorded_change_sets().len(), 1, "1 source operation was performed");
    assert_equivalent_because(&results.recorded_items(), &[item1.clone()], "1 item was added");
}

// Not ported: SourceIsNull_ThrowsException - argument validation

#[test]
fn suppress_empty_changesets_is_false_empty_changesets_are_published() {
    let source = new_source();

    let (_subscription, results) = source.as_observable().filter_immutable_with(Item::predicate, false).validate_cache_change_sets(Item::key_selector).record_cache_items();

    manipulate_excluded_items(&source);

    assert!(results.error().is_none());
    assert!(!results.has_completed());
    assert_eq!(results.recorded_change_sets().len(), 5, "5 source operations were performed");
    assert!(results.recorded_change_sets().iter().all(|changes| changes.count() == 0), "no included items were manipulated");
}

#[test]
fn suppress_empty_changesets_is_true_empty_changesets_are_not_published() {
    let source = new_source();

    let (_subscription, results) = source.as_observable().filter_immutable(Item::predicate).validate_cache_change_sets(Item::key_selector).record_cache_items();

    manipulate_excluded_items(&source);

    assert!(results.error().is_none());
    assert!(!results.has_completed());
    assert!(results.recorded_change_sets().is_empty(), "no source operations should have generated changes");
}

/// `ManipulateExcludedItems`.
fn manipulate_excluded_items(source: &Source) {
    let item1 = Item::new(1, false);
    source.on_next(ChangeSet::from(vec![Change::with_index(ChangeReason::Add, item1.id, item1.clone(), Some(0))]));

    let item2 = Item::new(item1.id, false);
    source.on_next(ChangeSet::from(vec![Change::with_previous(ChangeReason::Update, item2.id, item2.clone(), Some(item1.clone()), Some(0), Some(0))]));

    source.on_next(ChangeSet::from(vec![Change::with_index(ChangeReason::Refresh, item2.id, item2.clone(), Some(0))]));

    let item3 = Item::new(2, false);
    source.on_next(ChangeSet::from(vec![
        Change::with_index(ChangeReason::Add, item3.id, item3.clone(), Some(1)),
        Change::moved(item2.id, item2.clone(), 1, 0),
    ]));

    source.on_next(ChangeSet::from(vec![
        Change::with_index(ChangeReason::Remove, item2.id, item2.clone(), Some(1)),
        Change::with_index(ChangeReason::Remove, item3.id, item3.clone(), Some(0)),
    ]));
}

#[test]
fn update_previous_matched_current_does_not_emits_remove_carrying_previous_as_current() {
    // Per Change<T,K> contract, a Remove change carries the item that was removed in Current.
    // For an Update where Previous matched the predicate but Current does not, the item that
    // leaves the filtered view is Previous (it was downstream; Current never reached downstream).
    let source = new_source();

    let (_subscription, results) = source.as_observable().filter_immutable(Item::predicate).validate_cache_change_sets(Item::key_selector).record_cache_items();

    let included = Item::new(1, true);
    let excluded = Item::new(1, false);

    source.on_next(ChangeSet::from(vec![Change::with_index(ChangeReason::Add, included.id, included.clone(), Some(0))]));

    source.on_next(ChangeSet::from(vec![Change::with_previous(
        ChangeReason::Update,
        excluded.id,
        excluded.clone(),
        Some(included.clone()),
        Some(0),
        Some(0),
    )]));

    let recorded_change_sets = results.recorded_change_sets();
    let last_change_set = recorded_change_sets.last().expect("a change set was recorded");
    assert_eq!(last_change_set.count(), 1);

    let remove_change = &last_change_set.changes()[0];
    assert_eq!(remove_change.reason(), ChangeReason::Remove);
    assert!(
        Rc::ptr_eq(remove_change.current(), &included),
        "Remove.Current must carry the item that left downstream (the previously-matching value), not the new value that never reached downstream"
    );
}
