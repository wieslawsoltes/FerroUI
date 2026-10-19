//! `EditDiffChangeSetOptionalFixture` (`DynamicData.Tests/Cache/EditDiffChangeSetOptionalFixture.cs`):
//! `EditDiff` of a sequence of options (`IObservable<Optional<TObject>>`), which is
//! `edit_diff_optional` here.
//!
//! The two theories subscribe to the source sequence itself, not to the result of `EditDiff`, as
//! upstream's do.

use ferroui_dynamic_data::cache::{IObservableCache, ObservableCacheEx};
use ferroui_dynamic_data::kernel::OptionObservableExtensions;
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::{IObservable, ObservableError};
use std::cell::{Cell, RefCell};
use std::fmt;
use std::rc::Rc;

// Upstream's `MaxItems`, which no test of this fixture uses, is left out.

/// `s_noPerson`: `Optional.None<Person>()`.
const NO_PERSON: Option<Rc<Person>> = None;

/// The person of the fixture (`Person`), a class without `Equals`: equal only to itself, as
/// upstream's default equality of a class is that of references.
#[derive(Debug)]
struct Person {
    id: i32,
    name: String,
}

impl PartialEq for Person {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

/// `PersonComparer`: people of the same name and id are equal.
fn person_comparer(x: &Rc<Person>, y: &Rc<Person>) -> bool {
    x.name == y.name && x.id == y.id
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

/// `CreatePerson`: `Optional.Some(new Person(id, name))`.
fn create_person(id: i32, name: &str) -> Option<Rc<Person>> {
    Some(Rc::new(Person { id, name: name.to_owned() }))
}

// Not ported: NullChecksArePerformed - argument validation

#[test]
fn optional_some_creates_add_change() {
    // having
    let optional = create_person(0, "Name");
    let opt_observable = Observable::return_(optional);

    // when
    let observable_change_set = opt_observable.edit_diff_optional(|p: &Rc<Person>| p.id);
    let results = observable_change_set.as_aggregator();

    // then
    assert_eq!(results.data().count(), 1);
    assert_eq!(results.messages().len(), 1);
}

#[test]
fn optional_none_creates_remove_change() {
    // having
    let optional = create_person(0, "Name");
    let opt_observable = Observable::to_observable(vec![optional, NO_PERSON]);

    // when
    let observable_change_set = opt_observable.edit_diff_optional(|p: &Rc<Person>| p.id);
    let results = observable_change_set.as_aggregator();

    // then
    let messages = results.messages();
    assert_eq!(results.data().count(), 0);
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0].adds(), 1);
    assert_eq!(messages[1].removes(), 1);
    assert_eq!(messages[1].updates(), 0);
}

#[test]
fn optional_some_with_same_key_creates_update_change() {
    // having
    let optional1 = create_person(0, "Name");
    let optional2 = create_person(0, "Update");
    let opt_observable = Observable::to_observable(vec![optional1, optional2]);

    // when
    let observable_change_set = opt_observable.edit_diff_optional(|p: &Rc<Person>| p.id);
    let results = observable_change_set.as_aggregator();

    // then
    let messages = results.messages();
    assert_eq!(results.data().count(), 1);
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0].adds(), 1);
    assert_eq!(messages[1].removes(), 0);
    assert_eq!(messages[1].updates(), 1);
}

#[test]
fn optional_some_with_same_reference_creates_no_changes() {
    // having
    let optional = create_person(0, "Name");
    let opt_observable = Observable::to_observable(vec![optional.clone(), optional]);

    // when
    let observable_change_set = opt_observable.edit_diff_optional(|p: &Rc<Person>| p.id);
    let results = observable_change_set.as_aggregator();

    // then
    let summary = results.summary();
    assert_eq!(results.data().count(), 1);
    assert_eq!(results.messages().len(), 1);
    assert_eq!(summary.overall.adds, 1);
    assert_eq!(summary.overall.removes, 0);
    assert_eq!(summary.overall.updates, 0);
}

#[test]
fn optional_some_with_same_creates_no_changes() {
    // having
    let optional1 = create_person(0, "Name");
    let optional2 = create_person(0, "Name");
    let opt_observable = Observable::to_observable(vec![optional1, optional2]);

    // when
    let observable_change_set = opt_observable.edit_diff_optional_with_comparer(|p: &Rc<Person>| p.id, person_comparer);
    let results = observable_change_set.as_aggregator();

    // then
    let summary = results.summary();
    assert_eq!(results.data().count(), 1);
    assert_eq!(results.messages().len(), 1);
    assert_eq!(summary.overall.adds, 1);
    assert_eq!(summary.overall.removes, 0);
    assert_eq!(summary.overall.updates, 0);
}

#[test]
fn optional_some_with_different_key_creates_add_remove_changes() {
    // having
    let optional1 = create_person(0, "Name");
    let optional2 = create_person(1, "Update");
    let opt_observable = Observable::to_observable(vec![optional1, optional2]);

    // when
    let observable_change_set = opt_observable.edit_diff_optional(|p: &Rc<Person>| p.id);
    let results = observable_change_set.as_aggregator();

    // then
    let messages = results.messages();
    assert_eq!(results.data().count(), 1);
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0].adds(), 1);
    assert_eq!(messages[1].removes(), 1);
    assert_eq!(messages[1].updates(), 0);
}

/// `ResultCompletesIfAndOnlyIfSourceCompletes(bool completeSource)`.
fn result_completes_if_and_only_if_source_completes(complete_source: bool) {
    // having
    let optional = create_person(0, "Name");
    let mut opt_observable: Rc<dyn IObservable<Option<Rc<Person>>>> = Observable::return_(optional);
    if !complete_source {
        opt_observable = opt_observable.concat(Observable::never());
    }
    let completed = Rc::new(Cell::new(false));

    // when
    let on_completed = completed.clone();
    let _results = opt_observable.subscribe_next_completed(|_| {}, move || on_completed.set(true));

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
    let optional = create_person(0, "Name");
    let mut opt_observable: Rc<dyn IObservable<Option<Rc<Person>>>> = Observable::return_(optional);
    let test_exception: ObservableError = Rc::new(TestError);
    if fail_source {
        opt_observable = opt_observable.concat(Observable::throw(test_exception.clone()));
    }
    let received_error: Rc<RefCell<Option<ObservableError>>> = Rc::new(RefCell::new(None));

    // when
    let on_error = received_error.clone();
    let _results = opt_observable.subscribe_next_error(|_| {}, move |error| *on_error.borrow_mut() = Some(error));

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
