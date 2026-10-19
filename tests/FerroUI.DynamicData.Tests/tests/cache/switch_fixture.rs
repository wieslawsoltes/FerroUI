//! `SwitchFixture` (`DynamicData.Tests/Cache/SwitchFixture.cs`): the change sets of the latest of
//! a sequence of caches or of sequences of change sets. The tests of the lifetime of the
//! subscriptions (`SwitchFixture.SubscriptionLifetime.cs`) are in
//! `switch_fixture_subscription_lifetime.rs`.

use crate::domain::Person;
use crate::utilities::RawAnonymousObservable;
use ferroui_dynamic_data::cache::{Change, ChangeReason, ChangeSet, IConnectableCache, IObservableCache, ObservableCacheEx, SourceCache, SourceCacheEx, SwitchCacheEx};
use ferroui_reactive::disposables::Disposable;
use ferroui_reactive::subjects::{BehaviorSubject, Subject};
use ferroui_reactive::Observable;
use ferroui_reactive_testing::new_error;
use ferroui_system::{IDisposable, IObservable, IObserver};
use std::cell::RefCell;
use std::rc::Rc;

type People = SourceCache<Rc<Person>, String>;

/// The change sets of a cache of people (`IObservable<IChangeSet<Person, string>>`).
type PeopleChanges = Rc<dyn IObservable<ChangeSet<Rc<Person>, String>>>;

fn new_people() -> People {
    SourceCache::new(|p: &Rc<Person>| p.name())
}

/// `Enumerable.Range(from, count).Select(i => new Person("Person" + i, i))`.
fn people_range(from: i32, count: i32) -> Vec<Rc<Person>> {
    (from..from + count).map(|i| Person::new(format!("Person{i}"), i)).collect()
}

/// A change set that adds `person`.
fn add_person(name: &str, age: i32) -> ChangeSet<Rc<Person>, String> {
    ChangeSet::from(vec![Change::new(ChangeReason::Add, name.to_owned(), Person::new(name, age))])
}

#[test]
fn clears_for_new_source() {
    let source = new_people();
    let switchable = BehaviorSubject::new(source.clone());
    let results = switchable.as_observable().switch_cache().as_aggregator();

    let inital = people_range(1, 100);
    source.add_or_update_items(inital.clone());

    assert_eq!(results.data().count(), 100);

    let new_source = new_people();
    switchable.on_next(new_source.clone());

    assert_eq!(results.data().count(), 0);

    new_source.add_or_update_items(inital);
    assert_eq!(results.data().count(), 100);

    let next_updates = people_range(101, 100);
    new_source.add_or_update_items(next_updates);
    assert_eq!(results.data().count(), 200);

    results.dispose();
    new_source.dispose();
    switchable.dispose();
    source.dispose();
}

#[test]
fn poulates_first_source() {
    let source = new_people();
    let switchable = BehaviorSubject::new(source.clone());
    let results = switchable.as_observable().switch_cache().as_aggregator();

    let inital = people_range(1, 100);
    source.add_or_update_items(inital);

    assert_eq!(results.data().count(), 100);

    results.dispose();
    switchable.dispose();
    source.dispose();
}

#[test]
fn propagates_outer_errors() {
    let source = new_people();
    let switchable = BehaviorSubject::new(source.clone());
    let results = switchable.as_observable().switch_cache().as_aggregator();

    let inital = people_range(1, 100);
    source.add_or_update_items(inital);

    let error = new_error("Test");
    switchable.on_error(error.clone());

    assert!(results.error().is_some_and(|actual| Rc::ptr_eq(&actual, &error)));

    results.dispose();
    switchable.dispose();
    source.dispose();
}

#[test]
fn propagates_inner_errors() {
    let source = new_people();
    let switchable = BehaviorSubject::<PeopleChanges>::new(source.connect());
    let results = switchable.as_observable().switch_cache().as_aggregator();

    let inital = people_range(1, 100);
    source.add_or_update_items(inital);

    let source2 = BehaviorSubject::new(ChangeSet::<Rc<Person>, String>::empty());

    switchable.on_next(source2.as_observable());

    let error = new_error("Test");
    source2.on_error(error.clone());

    assert!(results.error().is_some_and(|actual| Rc::ptr_eq(&actual, &error)));

    source2.dispose();
    results.dispose();
    switchable.dispose();
    source.dispose();
}

#[test]
fn completes_when_sources_and_inner_complete() {
    let source = new_people();
    let switchable = BehaviorSubject::<PeopleChanges>::new(source.connect());
    let results = switchable.as_observable().switch_cache().as_aggregator();

    source.add_or_update_items(people_range(1, 100));

    switchable.on_completed();
    assert!(!results.is_completed(), "the inner sequence is still running");

    source.dispose();

    assert!(results.is_completed(), "both the sources and the inner sequence have completed");
    assert!(results.error().is_none());
    assert_eq!(results.data().count(), 100, "all data should have been received before completion");

    results.dispose();
    switchable.dispose();
}

#[test]
fn does_not_complete_while_inner_is_still_running() {
    let source = new_people();
    let switchable = BehaviorSubject::<PeopleChanges>::new(source.connect());
    let results = switchable.as_observable().switch_cache().as_aggregator();

    switchable.on_completed();
    source.add_or_update(Person::new("Person1", 1));

    assert!(!results.is_completed(), "the inner sequence has not completed");
    assert_eq!(results.data().count(), 1, "changes should still flow after the sources sequence completes");

    results.dispose();
    switchable.dispose();
    source.dispose();
}

#[test]
fn does_not_complete_when_only_a_superseded_inner_completes() {
    let first = new_people();
    let second = new_people();
    let switchable = BehaviorSubject::<PeopleChanges>::new(first.connect());
    let results = switchable.as_observable().switch_cache().as_aggregator();

    switchable.on_next(second.connect());
    switchable.on_completed();

    first.dispose();

    assert!(!results.is_completed(), "the superseded sequence is not the current one");

    second.add_or_update(Person::new("Person1", 1));
    assert_eq!(results.data().count(), 1, "the current sequence should still be delivering");

    second.dispose();
    assert!(results.is_completed(), "the current sequence has now completed");

    results.dispose();
    switchable.dispose();
}

#[test]
fn completes_when_sources_and_inner_complete_synchronously() {
    let results = Observable::return_(Observable::empty::<ChangeSet<Rc<Person>, String>>()).switch_cache().as_aggregator();

    assert!(results.is_completed(), "everything completed during subscription");
    assert!(results.error().is_none());

    results.dispose();
}

#[test]
fn delivers_changes_emitted_before_synchronous_completion() {
    let change = add_person("Person1", 1);
    let results = Observable::return_(Observable::return_(change)).switch_cache().as_aggregator();

    assert_eq!(results.data().count(), 1, "changes emitted before a synchronous completion must not be lost");
    assert!(results.is_completed(), "the source completed");
    assert!(results.error().is_none());

    results.dispose();
}

#[test]
fn ignores_changes_from_a_superseded_source() {
    let first = Subject::<ChangeSet<Rc<Person>, String>>::new();
    let second = Subject::<ChangeSet<Rc<Person>, String>>::new();
    let first_changes: PeopleChanges = first.clone();
    let switchable = BehaviorSubject::new(first_changes);
    let results = switchable.as_observable().switch_cache().as_aggregator();

    first.on_next(add_person("Person1", 1));
    assert_eq!(results.data().count(), 1);

    let second_changes: PeopleChanges = second.clone();
    switchable.on_next(second_changes);
    assert_eq!(results.data().count(), 0, "moving to a new source drops what the previous one contributed");

    first.on_next(add_person("Person2", 2));

    assert_eq!(results.data().count(), 0, "a superseded source must not be able to write into the result");
    assert!(results.error().is_none());

    results.dispose();
    switchable.dispose();
    second.dispose();
    first.dispose();
}

#[test]
fn propagates_inner_errors_raised_synchronously() {
    let error = new_error("Test");
    let results = Observable::return_(Observable::throw::<ChangeSet<Rc<Person>, String>>(error.clone())).switch_cache().as_aggregator();

    assert!(results.error().is_some_and(|actual| Rc::ptr_eq(&actual, &error)), "the error was raised during subscription");

    results.dispose();
}

// Not ported: DoesNotHoldALockWhileDeliveringDownstream - real threads

#[test]
fn ignores_errors_from_a_superseded_source() {
    // Switching away from a source means anything it produces afterwards belongs to a source that
    // is no longer selected, and that includes its failures. Ordinarily disposal stops a
    // superseded source being heard from again, but disposal cannot reach a notification that is
    // already in flight, so the operator has to discard it on arrival. The raw observable hands
    // back the observer directly, which is how that in-flight failure is reproduced here without
    // needing a race to land.
    let superseded_observer: Rc<RefCell<Option<Rc<dyn IObserver<ChangeSet<Rc<Person>, String>>>>>> = Rc::new(RefCell::new(None));
    let captured = superseded_observer.clone();
    let superseded = RawAnonymousObservable::create(move |observer: Rc<dyn IObserver<ChangeSet<Rc<Person>, String>>>| {
        *captured.borrow_mut() = Some(observer);
        Disposable::empty()
    });

    let switchable = Subject::<PeopleChanges>::new();
    let current = Subject::<ChangeSet<Rc<Person>, String>>::new();

    let results = switchable.as_observable().switch_cache().as_aggregator();

    switchable.on_next(superseded);
    let current_changes: PeopleChanges = current.clone();
    switchable.on_next(current_changes);

    let observer = superseded_observer.borrow().clone();
    assert!(observer.is_some(), "the superseded source should have been subscribed");
    observer.expect("the superseded source was subscribed").on_error(new_error("Test"));

    assert!(results.error().is_none(), "the failed source had already been switched away from");

    current.on_next(add_person("a", 1));

    assert_eq!(results.data().count(), 1, "the selected source should still be delivering");

    results.dispose();
    current.dispose();
    switchable.dispose();
}
