//! `ChangeSetMixinTest` (`ReactiveUI.Tests/ChangeSetMixinTest.cs`).

use ferroui_reactive::subjects::Subject;
use ferroui_reactive::ObservableExt;
use ferroui_reactiveui::*;
use ferroui_system::IObserver;
use std::cell::RefCell;
use std::rc::Rc;

/// `ReplacedItemValue`.
const REPLACED_ITEM_VALUE: i32 = 2;
/// `ExpectedCountChangeEmissions`.
const EXPECTED_COUNT_CHANGE_EMISSIONS: usize = 2;

fn add_change_set() -> ReactiveChangeSet<i32> {
    ReactiveChangeSet::new(vec![ReactiveChange::new(ReactiveChangeReason::Add, 1, None, 0, -1)])
}

fn update_change_set() -> ReactiveChangeSet<i32> {
    ReactiveChangeSet::new(vec![ReactiveChange::new(ReactiveChangeReason::Replace, REPLACED_ITEM_VALUE, Some(1), 0, 0)])
}

fn remove_change_set() -> ReactiveChangeSet<i32> {
    ReactiveChangeSet::new(vec![ReactiveChange::new(ReactiveChangeReason::Remove, 1, None, 0, -1)])
}

#[test]
fn when_count_changed_filters_to_only_count_changes() {
    let subject = Subject::<ReactiveChangeSet<i32>>::new();
    let results = Rc::new(RefCell::new(Vec::new()));
    let sink = results.clone();
    let _subscription = subject
        .as_observable()
        .when_count_changed()
        .observe_on(immediate_scheduler())
        .subscribe_next(move |change_set| sink.borrow_mut().push(change_set));

    let add = add_change_set();
    let update = update_change_set();
    let remove = remove_change_set();

    subject.on_next(add.clone());
    subject.on_next(update);
    subject.on_next(remove.clone());

    let results = results.borrow();
    assert_eq!(results.len(), EXPECTED_COUNT_CHANGE_EMISSIONS);
    assert!(results[0].ptr_eq(&add));
    assert!(results[1].ptr_eq(&remove));
}

fn immediate_scheduler() -> Rc<dyn ferroui_reactive::concurrency::IScheduler> {
    ferroui_reactive::concurrency::ImmediateScheduler::instance()
}

// Not ported: HasCountChanged_NullChangeSet_Throws - argument validation

#[test]
fn has_count_changed_with_adds_returns_true() {
    let change_set = add_change_set();

    let result = change_set.count_has_changed();

    assert!(result);
}

#[test]
fn has_count_changed_with_only_updates_returns_false() {
    let change_set = update_change_set();

    let result = change_set.count_has_changed();

    assert!(!result);
}

#[test]
fn has_count_changed_with_removes_returns_true() {
    let change_set = remove_change_set();

    let result = change_set.count_has_changed();

    assert!(result);
}
