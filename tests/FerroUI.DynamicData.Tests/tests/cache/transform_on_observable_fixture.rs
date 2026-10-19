//! `TransformOnObservableFixture` (`DynamicData.Tests/Cache/TransformOnObservableFixture.cs`): the
//! latest value of an observable of each item.

use crate::domain::{fakers, Animal, Faker, Randomizer};
use ferroui_dynamic_data::cache::testing::ChangeSetAggregator;
use ferroui_dynamic_data::cache::{ChangeReason, ChangeSet, IConnectableCache, IObservableCache, ObservableCacheEx, SourceCache};
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_reactive_testing::{new_error, TestScheduler};
use ferroui_system::{IDisposable, IObservable};
use std::rc::Rc;
use std::time::Duration;

const INITIAL_COUNT: usize = if cfg!(debug_assertions) { 7 } else { 103 };
const ADD_COUNT: usize = if cfg!(debug_assertions) { 5 } else { 53 };
const REMOVE_COUNT: usize = if cfg!(debug_assertions) { 3 } else { 37 };
const UPDATE_COUNT: usize = if cfg!(debug_assertions) { 2 } else { 31 };
const UPDATE_TIME: Duration = Duration::from_millis(50);

struct Fixture {
    animal_cache: SourceCache<Rc<Animal>, i32>,
    animal_results: ChangeSetAggregator<Rc<Animal>, i32>,
    animal_faker: Faker<Rc<Animal>>,
    randomizer: Randomizer,
}

impl Fixture {
    fn new() -> Self {
        let randomizer = Randomizer::new(0x2112_2112);
        let animal_faker = fakers::animal().with_seed(&randomizer);
        let animal_cache = SourceCache::new(|a: &Rc<Animal>| a.id());
        animal_cache.edit(|updater| updater.add_or_update_items(&mut animal_faker.generate(INITIAL_COUNT).into_iter()));
        let animal_results = animal_cache.connect().as_aggregator();
        Self { animal_cache, animal_results, animal_faker, randomizer }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.animal_cache.dispose();
        self.animal_results.dispose();
    }
}

fn name_of(animal: &Rc<Animal>, _id: &i32) -> Rc<dyn IObservable<String>> {
    Observable::return_(animal.name())
}

#[test]
fn result_contains_all_initial_children() {
    let fixture = Fixture::new();

    let results = fixture.animal_cache.connect().transform_on_observable_with_key(name_of).as_aggregator();

    assert_eq!(fixture.animal_results.data().count(), INITIAL_COUNT);
    assert_eq!(results.data().count(), INITIAL_COUNT);
    assert_eq!(results.messages().len(), 1, "The child observables fire on subscription so everything should appear as a single changeset");
}

#[test]
fn result_contains_added_values() {
    let fixture = Fixture::new();
    let results = fixture.animal_cache.connect().transform_on_observable_with_key(name_of).as_aggregator();

    fixture.animal_cache.edit(|updater| updater.add_or_update_items(&mut fixture.animal_faker.generate(ADD_COUNT).into_iter()));

    assert_eq!(fixture.animal_results.data().count(), INITIAL_COUNT + ADD_COUNT);
    assert_eq!(results.data().count(), fixture.animal_results.data().count());
    assert_eq!(results.messages().len(), 2, "Initial Adds and then the subsequent Additions should each be a single message");
}

#[test]
fn result_does_not_contain_removed_values() {
    let fixture = Fixture::new();
    let results = fixture.animal_cache.connect().transform_on_observable_with_key(name_of).as_aggregator();

    let removed = fixture.randomizer.list_items(&fixture.animal_cache.items(), REMOVE_COUNT);
    fixture.animal_cache.edit(|updater| updater.remove_keys(&mut removed.iter().map(|a| a.id())));

    assert_eq!(fixture.animal_results.data().count(), INITIAL_COUNT - REMOVE_COUNT);
    assert_eq!(results.data().count(), fixture.animal_results.data().count());
    assert_eq!(results.messages().len(), 2, "1 for Adds and 1 for Removes");
}

/// Upstream runs the intervals in real time and awaits the end of the sequence; the port runs them
/// on a `TestScheduler` to their end, which is the end of the sequence.
#[test]
fn result_updates_on_future_values() {
    let fixture = Fixture::new();
    let scheduler = TestScheduler::new();
    let interval_scheduler = scheduler.scheduler();

    // An observable that fires a wrong value on an interval a fixed number of times, then the
    // expected value before completing.
    let create_child_obs = move |a: &Rc<Animal>, id: &i32| -> Rc<dyn IObservable<String>> {
        let (name, id) = (a.name(), *id);
        Observable::interval_on(UPDATE_TIME, interval_scheduler.clone())
            .select(move |n: i64| format!("{name}-{id}-{n}"))
            .take(UPDATE_COUNT)
            .concat(Observable::return_(a.name()))
    };

    let shared = fixture.animal_cache.connect().transform_on_observable_with_key(create_child_obs).publish();
    let shared_source: Rc<dyn IObservable<ChangeSet<String, i32>>> = shared.clone();
    let results = shared_source.as_aggregator();
    let cleanup = shared.connect();
    let animals = fixture.animal_cache.items();
    fixture.animal_cache.dispose();

    scheduler.run();
    assert!(results.is_completed(), "the sequence ends with the source and the child observables");

    assert_eq!(fixture.animal_results.data().count(), INITIAL_COUNT);
    assert_eq!(results.data().count(), fixture.animal_results.data().count());
    assert_eq!(results.summary().overall.adds, INITIAL_COUNT);
    assert_eq!(results.summary().overall.updates, INITIAL_COUNT * UPDATE_COUNT, "Each item should update {UPDATE_COUNT} times");
    assert!(!results.messages().is_empty(), "The delay may cause the messages to appear as multiple changesets");
    for animal in &animals {
        assert_eq!(results.data().lookup(&animal.id()), Some(animal.name()));
    }
    cleanup.dispose();
}

/// `ResultCompletesOnlyWhenSourceAndAllChildrenComplete(bool completeSource, bool completeChildren)`.
fn result_completes_only_when_source_and_all_children_complete(complete_source: bool, complete_children: bool) {
    let fixture = Fixture::new();
    let create_child_obs = move |a: &Rc<Animal>, _id: &i32| -> Rc<dyn IObservable<String>> {
        if complete_children {
            Observable::return_(a.name())
        } else {
            Observable::return_(a.name()).concat(Observable::never())
        }
    };

    let results = fixture.animal_cache.connect().transform_on_observable_with_key(create_child_obs).as_aggregator();

    if complete_source {
        fixture.animal_cache.dispose();
    }

    assert_eq!(fixture.animal_results.is_completed(), complete_source);
    assert_eq!(results.is_completed(), complete_source && complete_children);
}

#[test]
fn result_completes_only_when_source_and_all_children_complete_case1() {
    result_completes_only_when_source_and_all_children_complete(false, false);
}

#[test]
fn result_completes_only_when_source_and_all_children_complete_case2() {
    result_completes_only_when_source_and_all_children_complete(false, true);
}

#[test]
fn result_completes_only_when_source_and_all_children_complete_case3() {
    result_completes_only_when_source_and_all_children_complete(true, false);
}

#[test]
fn result_completes_only_when_source_and_all_children_complete_case4() {
    result_completes_only_when_source_and_all_children_complete(true, true);
}

#[test]
fn result_fails_if_child_fails() {
    let fixture = Fixture::new();
    let expected_error = new_error("Expected");
    let throw_observable = Observable::throw::<ChangeSet<Rc<Animal>, i32>>(expected_error.clone());

    let results = fixture.animal_cache.connect().transform_on_observable(move |_: &Rc<Animal>| throw_observable.clone()).as_aggregator();

    assert!(results.error().is_some_and(|error| Rc::ptr_eq(&error, &expected_error)));
}

#[test]
fn result_fails_if_source_fails() {
    let fixture = Fixture::new();
    let expected_error = new_error("Expected");
    let throw_observable = Observable::throw::<ChangeSet<Rc<Animal>, i32>>(expected_error.clone());
    let results = fixture.animal_cache.connect().concat(throw_observable).transform_on_observable(|a: &Rc<Animal>| Observable::return_(a.clone())).as_aggregator();

    fixture.animal_cache.dispose();

    assert!(results.error().is_some_and(|error| Rc::ptr_eq(&error, &expected_error)));
}

/// `OrderOfChangesIsPreserved(bool removeFirst)`.
fn order_of_changes_is_preserved(remove_first: bool) {
    let fixture = Fixture::new();
    let results = fixture.animal_cache.connect().transform_on_observable(|a: &Rc<Animal>| Observable::return_(a.clone())).as_aggregator();
    let (first_reason, next_reason, expected_changes) =
        if remove_first { (ChangeReason::Remove, ChangeReason::Add, INITIAL_COUNT * 2) } else { (ChangeReason::Add, ChangeReason::Remove, INITIAL_COUNT * 3) };

    fixture.animal_cache.edit(|updater| {
        if remove_first {
            updater.clear();
            updater.add_or_update_items(&mut fixture.animal_faker.generate(INITIAL_COUNT).into_iter());
        } else {
            updater.add_or_update_items(&mut fixture.animal_faker.generate(INITIAL_COUNT).into_iter());
            updater.clear();
        }
    });

    let messages = results.messages();
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[1].count(), expected_changes);
    assert!(messages[1].iter().take(INITIAL_COUNT).all(|change| change.reason() == first_reason));
    assert!(messages[1].iter().skip(INITIAL_COUNT).all(|change| change.reason() == next_reason));
}

#[test]
fn order_of_changes_is_preserved_case1() {
    order_of_changes_is_preserved(true);
}

#[test]
fn order_of_changes_is_preserved_case2() {
    order_of_changes_is_preserved(false);
}
