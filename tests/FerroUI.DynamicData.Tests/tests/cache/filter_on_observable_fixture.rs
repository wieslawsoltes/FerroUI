//! `FilterOnObservableFixture` (`DynamicData.Tests/Cache/FilterOnObservableFixture.cs`): the filter
//! of each item by the latest value of an observable of the item.

use crate::domain::Person;
use ferroui_dynamic_data::cache::testing::ChangeSetAggregator;
use ferroui_dynamic_data::cache::{IConnectableCache, IObservableCache, ObservableCacheEx, SourceCache};
use ferroui_reactive::subjects::Subject;
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_reactive_testing::{duration_to_ticks, TestScheduler};
use ferroui_system::{IDisposable, IObservable, IObserver};
use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

const MAGIC_NUMBER: usize = 37;

/// The fixture (`FilterOnObservableFixture` with its constructor and `Dispose`).
struct Fixture {
    source_results: ChangeSetAggregator<Rc<Person>, String>,
    source: SourceCache<Rc<Person>, String>,
}

impl Fixture {
    fn new() -> Self {
        let source = SourceCache::new(|p: &Rc<Person>| p.name());
        let source_results = source.connect().as_aggregator();
        Self { source_results, source }
    }

    /// `AddPeople`.
    fn add_people(&self, count: usize) -> Vec<Rc<Person>> {
        let people: Vec<Rc<Person>> = (0..count as i32).map(new_person).collect();
        self.source.edit(|updater| updater.add_or_update_items(&mut people.iter().cloned()));
        people
    }

    /// `AddPerson`.
    fn add_person(&self, n: i32) -> Rc<Person> {
        let p = new_person(n);
        self.source.edit(|updater| updater.add_or_update(p.clone()));
        p
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.source.dispose();
        self.source_results.dispose();
    }
}

/// `NewPerson`.
fn new_person(n: i32) -> Rc<Person> {
    Person::new(format!("Name{n}"), n)
}

#[test]
fn factory_is_invoked() {
    let fixture = Fixture::new();

    // having
    let invoked = Rc::new(Cell::new(false));
    let val = Rc::new(Cell::new(-1));
    let (factory_invoked, factory_val) = (invoked.clone(), val.clone());
    let factory = move |p: &Rc<Person>| -> Rc<dyn IObservable<bool>> {
        factory_invoked.set(true);
        factory_val.set(p.age());
        Observable::return_(true)
    };
    let sub = fixture.source.connect().filter_on_observable(factory).subscribe_ignore();

    // when
    fixture.add_person(MAGIC_NUMBER as i32);

    // then
    assert_eq!(fixture.source_results.data().count(), 1);
    assert!(invoked.get());
    assert_eq!(val.get(), MAGIC_NUMBER as i32, "Was value added to cache");
    // Not ported: the ArgumentNullException of a null factory - argument validation
    sub.dispose();
}

#[test]
fn factory_with_key_is_invoked() {
    let fixture = Fixture::new();

    // having
    let invoked = Rc::new(Cell::new(false));
    let val = Rc::new(Cell::new(-1));
    let (factory_invoked, factory_val) = (invoked.clone(), val.clone());
    let factory = move |p: &Rc<Person>, _name: &String| -> Rc<dyn IObservable<bool>> {
        factory_invoked.set(true);
        factory_val.set(p.age());
        Observable::return_(true)
    };
    let sub = fixture.source.connect().filter_on_observable_with_key(factory).subscribe_ignore();

    // when
    fixture.add_person(MAGIC_NUMBER as i32);

    // then
    assert_eq!(fixture.source_results.data().count(), 1);
    assert!(invoked.get());
    assert_eq!(val.get(), MAGIC_NUMBER as i32, "Was value added to cache");
    // Not ported: the ArgumentNullException of a null factory and of a null source - argument
    // validation
    sub.dispose();
}

#[test]
fn filtered_out_if_no_observable_value() {
    let fixture = Fixture::new();

    // having
    let filter_stats = fixture.source.connect().filter_on_observable(|_: &Rc<Person>| Observable::never::<bool>()).as_aggregator();

    // when
    fixture.add_people(MAGIC_NUMBER);

    // then
    assert_eq!(fixture.source_results.data().count(), MAGIC_NUMBER);
    assert_eq!(fixture.source_results.messages()[0].adds(), MAGIC_NUMBER);
    assert_eq!(fixture.source_results.messages().len(), 1, "Should have all been added at once");
    assert_eq!(filter_stats.messages().len(), 0, "All items should be filtered out");
}

#[test]
fn observable_filter_used_to_determine_inclusion() {
    let fixture = Fixture::new();

    // having
    let predicate = |p: &Rc<Person>| p.age() % 2 == 0;
    let filter_factory = move |p: &Rc<Person>| Observable::return_(predicate(p));
    let mut pass_count = 0;
    let mut fail_count = 0;
    let filter_stats = fixture.source.connect().filter_on_observable(filter_factory).as_aggregator();

    // when
    for p in fixture.add_people(MAGIC_NUMBER) {
        if predicate(&p) {
            pass_count += 1;
        } else {
            fail_count += 1;
        }
    }

    // then
    assert_eq!(fixture.source_results.data().count(), pass_count + fail_count);
    assert_eq!(filter_stats.data().count(), pass_count);
}

#[test]
fn observable_filter_triggers_add_and_remove() {
    let fixture = Fixture::new();

    // having
    let filter_subject = Subject::<bool>::new();

    let subject = filter_subject.clone();
    let filter_stats = fixture.source.connect().filter_on_observable(move |_: &Rc<Person>| subject.as_observable()).as_aggregator();

    fixture.add_people(MAGIC_NUMBER);

    // when
    filter_subject.on_next(true);
    filter_subject.on_next(false);

    // then
    assert_eq!(fixture.source_results.data().count(), MAGIC_NUMBER);
    assert_eq!(fixture.source_results.messages().len(), 1, "Should have all been added at once");
    assert_eq!(filter_stats.data().count(), 0);
    assert_eq!(filter_stats.messages().len(), MAGIC_NUMBER * 2, "Each should be added and removed");
    assert_eq!(filter_stats.summary().overall.adds, MAGIC_NUMBER);
    assert_eq!(filter_stats.summary().overall.removes, MAGIC_NUMBER);
}

#[test]
fn observable_filter_duplicate_values_have_no_effect() {
    let fixture = Fixture::new();

    // having
    let filter_subject = Subject::<bool>::new();

    let subject = filter_subject.clone();
    let filter_stats = fixture.source.connect().filter_on_observable(move |_: &Rc<Person>| subject.as_observable()).as_aggregator();

    fixture.add_people(MAGIC_NUMBER);

    // when
    filter_subject.on_next(false);
    filter_subject.on_next(false);
    filter_subject.on_next(false);
    filter_subject.on_next(true);
    filter_subject.on_next(true);
    filter_subject.on_next(true);

    // then
    assert_eq!(fixture.source_results.data().count(), MAGIC_NUMBER);
    assert_eq!(fixture.source_results.messages().len(), 1, "Should have all been added at once");
    assert_eq!(filter_stats.data().count(), MAGIC_NUMBER);
    assert_eq!(filter_stats.messages().len(), MAGIC_NUMBER, "Each should be added individually");
    assert_eq!(filter_stats.summary().overall.adds, MAGIC_NUMBER);
}

#[test]
fn observable_filter_changes_can_be_buffered() {
    let fixture = Fixture::new();

    // having
    let scheduler = TestScheduler::new();
    let filter_subject = Subject::<bool>::new();

    let subject = filter_subject.clone();
    let filter_stats = fixture
        .source
        .connect()
        .filter_on_observable_with(move |_: &Rc<Person>| subject.as_observable(), Some(Duration::from_secs(1)), Some(scheduler.scheduler()))
        .as_aggregator();

    fixture.add_people(MAGIC_NUMBER);

    // when
    filter_subject.on_next(true);
    scheduler.advance_by(duration_to_ticks(Duration::from_secs(2)));

    // then
    assert_eq!(fixture.source_results.data().count(), MAGIC_NUMBER);
    assert_eq!(fixture.source_results.messages().len(), 1, "Should have all been added at once");
    assert_eq!(filter_stats.data().count(), MAGIC_NUMBER);
    assert_eq!(filter_stats.messages().len(), 1, "Should have all been added at once");
    assert_eq!(filter_stats.summary().overall.adds, MAGIC_NUMBER);
}
