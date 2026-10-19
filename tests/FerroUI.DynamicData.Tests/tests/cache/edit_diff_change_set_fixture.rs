//! `EditDiffChangeSetFixture` (`DynamicData.Tests/Cache/EditDiffChangeSetFixture.cs`): `EditDiff`
//! of a sequence of collections (`IObservable<IEnumerable<TObject>>`).
//!
//! The two theories subscribe to the source sequence itself, not to the result of `EditDiff`, as
//! upstream's do.

use ferroui_dynamic_data::cache::{IObservableCache, ObservableCacheEx};
use ferroui_dynamic_data::ObservableSequencesEx;
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::{IObservable, ObservableError};
use std::cell::{Cell, RefCell};
use std::fmt;
use std::rc::Rc;

const MAX_ITEMS: usize = 1097;

/// The person of the fixture (`Person`), a class without `Equals`: equal only to itself, as
/// upstream's default equality of a class is that of references.
#[derive(Debug)]
struct Person {
    id: i32,
    // Upstream's `Name`, which no test reads.
    #[allow(dead_code)]
    name: String,
}

impl PartialEq for Person {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

/// The error of the theories (`new Exception("Test")`).
#[derive(Debug)]
struct TestError;

impl fmt::Display for TestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Test")
    }
}

impl std::error::Error for TestError {}

/// `CreatePerson`.
fn create_person(id: i32, name: String) -> Rc<Person> {
    Rc::new(Person { id, name })
}

/// `CreatePeople`: the people of the ids `base_id` to `base_id + count - 1`, named `base_name`
/// and the id.
fn create_people(base_id: i32, count: usize, base_name: &str) -> Vec<Rc<Person>> {
    (base_id..base_id + count as i32).map(|i| create_person(i, format!("{base_name}{i}"))).collect()
}

// Not ported: NullChecksArePerformed - argument validation

#[test]
fn items_from_enumerable_are_added_to_change_set() {
    // having
    let enumerable = create_people(0, MAX_ITEMS, "Name");
    let enum_observable = Observable::return_(enumerable);

    // when
    let observable_change_set = enum_observable.edit_diff(|p: &Rc<Person>| p.id);
    let results = observable_change_set.as_aggregator();

    // then
    assert_eq!(results.data().count(), MAX_ITEMS);
    assert_eq!(results.messages().len(), 1);
}

#[test]
fn items_removed_from_enumerable_are_removed_from_change_set() {
    // having
    let enumerable = create_people(0, MAX_ITEMS, "Name");
    let enum_observable = Observable::to_observable(vec![enumerable, Vec::new()]);

    // when
    let observable_change_set = enum_observable.edit_diff(|p: &Rc<Person>| p.id);
    let results = observable_change_set.as_aggregator();

    // then
    let messages = results.messages();
    assert_eq!(results.data().count(), 0);
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0].adds(), MAX_ITEMS);
    assert_eq!(messages[1].removes(), MAX_ITEMS);
    assert_eq!(messages[1].updates(), 0);
}

#[test]
fn items_updated_are_updated_in_change_set() {
    // having
    let enumerable1 = create_people(0, MAX_ITEMS * 2, "Name");
    let enumerable2 = create_people(MAX_ITEMS as i32, MAX_ITEMS, "Update");
    let enum_observable = Observable::to_observable(vec![enumerable1, enumerable2]);

    // when
    let observable_change_set = enum_observable.edit_diff(|p: &Rc<Person>| p.id);
    let results = observable_change_set.as_aggregator();

    // then
    let messages = results.messages();
    assert_eq!(results.data().count(), MAX_ITEMS);
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0].adds(), MAX_ITEMS * 2);
    assert_eq!(messages[1].updates(), MAX_ITEMS);
    assert_eq!(messages[1].removes(), MAX_ITEMS);
}

/// `ResultCompletesIfAndOnlyIfSourceCompletes(bool completeSource)`.
fn result_completes_if_and_only_if_source_completes(complete_source: bool) {
    // having
    let enumerable = create_people(0, MAX_ITEMS, "Name");
    let mut enum_observable: Rc<dyn IObservable<Vec<Rc<Person>>>> = Observable::return_(enumerable);
    if !complete_source {
        enum_observable = enum_observable.concat(Observable::never());
    }
    let completed = Rc::new(Cell::new(false));

    // when
    let on_completed = completed.clone();
    let _results = enum_observable.subscribe_next_completed(|_| {}, move || on_completed.set(true));

    // then
    assert_eq!(completed.get(), complete_source);
}

/// `[InlineData(true)]`.
#[test]
fn result_completes_if_and_only_if_source_completes_case1() {
    result_completes_if_and_only_if_source_completes(true);
}

/// `[InlineData(false)]`.
#[test]
fn result_completes_if_and_only_if_source_completes_case2() {
    result_completes_if_and_only_if_source_completes(false);
}

/// `ResultFailsIfAndOnlyIfSourceFails(bool failSource)`.
fn result_fails_if_and_only_if_source_fails(fail_source: bool) {
    // having
    let enumerable = create_people(0, MAX_ITEMS, "Name");
    let mut enum_observable: Rc<dyn IObservable<Vec<Rc<Person>>>> = Observable::return_(enumerable);
    let test_exception: ObservableError = Rc::new(TestError);
    if fail_source {
        enum_observable = enum_observable.concat(Observable::throw(test_exception.clone()));
    }
    let received_error: Rc<RefCell<Option<ObservableError>>> = Rc::new(RefCell::new(None));

    // when
    let on_error = received_error.clone();
    let _results = enum_observable.subscribe_next_error(|_| {}, move |error| *on_error.borrow_mut() = Some(error));

    // then
    let received_error = received_error.borrow().clone();
    match fail_source {
        true => assert!(received_error.is_some_and(|error| Rc::ptr_eq(&error, &test_exception))),
        false => assert!(received_error.is_none()),
    }
}

/// `[InlineData(true)]`.
#[test]
fn result_fails_if_and_only_if_source_fails_case1() {
    result_fails_if_and_only_if_source_fails(true);
}

/// `[InlineData(false)]`.
#[test]
fn result_fails_if_and_only_if_source_fails_case2() {
    result_fails_if_and_only_if_source_fails(false);
}
