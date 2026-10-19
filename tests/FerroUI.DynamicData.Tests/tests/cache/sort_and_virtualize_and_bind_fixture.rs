//! `SortAndVirtualizeAndBindFixture` and its siblings (`DynamicData.Tests/Cache/SortAndVirtualizeAndBindFixture.cs`).
//!
//! The abstract fixture of upstream runs its tests for each way of binding; here each one is a
//! module of the same tests (`sort_and_virtualize_and_bind_fixture!`).

use crate::domain::{Person, Randomizer};
use crate::utilities::assert_equivalent;
use ferroui_dynamic_data::binding::{SortAndBindOptions, SortExpressionComparer};
use ferroui_dynamic_data::cache::testing::ChangeSetAggregator;
use ferroui_dynamic_data::cache::VirtualRequest;
use ferroui_dynamic_data::{ChangeReason, IConnectableCache, ObservableCacheEx, SourceCache};
use ferroui_reactive::subjects::BehaviorSubject;
use ferroui_system::{IDisposable, IObservable, IObserver};
use std::cell::RefCell;
use std::cmp::Ordering;
use std::rc::Rc;

type Comparer = Rc<dyn Fn(&Rc<Person>, &Rc<Person>) -> Ordering>;

/// The items of the bound list, read when a test looks (`IList<Person>`).
type BoundList = Rc<dyn Fn() -> Vec<Rc<Person>>>;

/// `SetUpTests`.
type SetUp = fn(&SourceCache<Rc<Person>, String>, &Comparer, Rc<dyn IObservable<VirtualRequest>>) -> (ChangeSetAggregator<Rc<Person>, String>, BoundList);

/// `SortAndVirtualizeAndBindWithImplicitOptionsFixtureReadOnlyCollection`: no options, which come from the
/// context of `SortAndVirtualize`.
fn with_implicit_options_read_only_collection(
    source: &SourceCache<Rc<Person>, String>,
    comparer: &Comparer,
    virtual_requests: Rc<dyn IObservable<VirtualRequest>>,
) -> (ChangeSetAggregator<Rc<Person>, String>, BoundList) {
    let (bound, list) = source.connect().sort_and_virtualize(comparer.clone(), virtual_requests).bind_read_only();
    (bound.as_aggregator(), Rc::new(move || list.to_vec()))
}

/// `SortAndVirtualizeAndBindFixtureReadOnlyCollection`.
fn read_only_collection(source: &SourceCache<Rc<Person>, String>, comparer: &Comparer, virtual_requests: Rc<dyn IObservable<VirtualRequest>>) -> (ChangeSetAggregator<Rc<Person>, String>, BoundList) {
    let (bound, list) = source.connect().sort_and_virtualize(comparer.clone(), virtual_requests).bind_read_only_with_sort_and_bind_options(SortAndBindOptions::default());
    (bound.as_aggregator(), Rc::new(move || list.to_vec()))
}

/// `SortAndVirtualizeAndBindWithImplicitOptionsFixture`: no options, which come from the context of
/// `SortAndVirtualize`.
fn with_implicit_options(source: &SourceCache<Rc<Person>, String>, comparer: &Comparer, virtual_requests: Rc<dyn IObservable<VirtualRequest>>) -> (ChangeSetAggregator<Rc<Person>, String>, BoundList) {
    let list = Rc::new(RefCell::new(Vec::new()));
    let read = list.clone();
    (source.connect().sort_and_virtualize(comparer.clone(), virtual_requests).bind_to_list(&list).as_aggregator(), Rc::new(move || read.borrow().clone()))
}

/// `SortAndVirtualizeAndBindFixture`: `SortAndBind` after `SortAndVirtualize`.
fn sort_and_bind(source: &SourceCache<Rc<Person>, String>, comparer: &Comparer, virtual_requests: Rc<dyn IObservable<VirtualRequest>>) -> (ChangeSetAggregator<Rc<Person>, String>, BoundList) {
    let list = Rc::new(RefCell::new(Vec::new()));
    let read = list.clone();
    (
        source.connect().sort_and_virtualize(comparer.clone(), virtual_requests).sort_and_bind_by_default_with(&list, SortAndBindOptions::default()).as_aggregator(),
        Rc::new(move || read.borrow().clone()),
    )
}

/// `SortAndVirtualizeAndBindFixtureBase`.
struct Fixture {
    source: SourceCache<Rc<Person>, String>,
    comparer: Comparer,
    virtual_requests: Rc<BehaviorSubject<VirtualRequest>>,
    aggregator: ChangeSetAggregator<Rc<Person>, String>,
    list: BoundList,
}

impl Fixture {
    fn new(set_up: SetUp) -> Self {
        let source = SourceCache::new(|p: &Rc<Person>| p.name());
        let comparer = SortExpressionComparer::<Rc<Person>>::ascending(|p| p.age()).then_by_ascending(|p| p.name()).comparer();
        let virtual_requests = BehaviorSubject::new(VirtualRequest::new(0, 25));
        let (aggregator, list) = set_up(&source, &comparer, virtual_requests.as_observable());
        Self { source, comparer, virtual_requests, aggregator, list }
    }

    fn list(&self) -> Vec<Rc<Person>> {
        (self.list)()
    }

    fn sorted(&self, items: &[Rc<Person>]) -> Vec<Rc<Person>> {
        let mut items = items.to_vec();
        items.sort_by(|a, b| (self.comparer)(a, b));
        items
    }

    fn add_or_update(&self, people: &[Rc<Person>]) {
        self.source.edit(|updater| updater.add_or_update_items(&mut people.iter().cloned()));
    }

    /// `Enumerable.Range(1, 100).Select(i => new Person($"P{i:000}", i)).OrderBy(p => Guid.NewGuid())`.
    fn people() -> Vec<Rc<Person>> {
        let people: Vec<Rc<Person>> = (1..=100).map(|i| Person::new(format!("P{i:03}"), i)).collect();
        Randomizer::new_unseeded().shuffle(&people)
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.source.dispose();
        self.aggregator.dispose();
        self.virtual_requests.on_completed();
    }
}

fn take(items: Vec<Rc<Person>>, skip: usize, take: usize) -> Vec<Rc<Person>> {
    items.into_iter().skip(skip).take(take).collect()
}

fn initial_batches(fixture: &Fixture) {
    let people = Fixture::people();
    fixture.add_or_update(&people);

    // For the first batch, it should use the results of the subject of virtual requests (if a
    // behaviour subject is used).
    let expected_result = take(fixture.sorted(&people), 0, 25);
    assert_equivalent(&fixture.list(), &expected_result);

    fixture.virtual_requests.on_next(VirtualRequest::new(25, 50));
    let expected_result = take(fixture.sorted(&people), 25, 50);
    assert_equivalent(&fixture.list(), &expected_result);

    fixture.virtual_requests.on_next(VirtualRequest::new(40, 50));
    let expected_result = take(fixture.sorted(&people), 40, 50);
    assert_equivalent(&fixture.list(), &expected_result);
}

fn overlapping_shift(fixture: &Fixture) {
    let people = Fixture::people();
    fixture.add_or_update(&people);

    fixture.virtual_requests.on_next(VirtualRequest::new(10, 30));

    // For the first batch, it should use the results of the subject of virtual requests (if a
    // behaviour subject is used).
    let expected_result = take(fixture.sorted(&people), 10, 30);
    assert_equivalent(&fixture.list(), &expected_result);
}

fn add_first_in_range(fixture: &Fixture) {
    let mut people = Fixture::people();
    fixture.add_or_update(&people);

    // Insert right at the beginning.
    let person = Person::new("_FirstPerson", 1);
    fixture.add_or_update(std::slice::from_ref(&person));

    let messages = fixture.aggregator.messages();
    assert_eq!(messages.len(), 2);

    let changes = &messages[1];
    assert_eq!(changes.count(), 2);

    let first_change = &changes[0];
    assert_eq!(first_change.reason(), ChangeReason::Remove);
    assert_eq!(first_change.current(), &Person::new("P025", 25));

    let second_change = &changes[1];
    assert_eq!(second_change.reason(), ChangeReason::Add);
    assert_eq!(second_change.current(), &person);

    // Check the correctness of the resulting collection.
    people.push(person);

    let expected_result = take(fixture.sorted(&people), 0, 25);
    assert_eq!(fixture.list(), expected_result);
}

fn add_outside_of_range(fixture: &Fixture) {
    let mut people = Fixture::people();
    fixture.add_or_update(&people);

    // Insert right at the end.
    let person = Person::new("X_Last", 100);
    fixture.add_or_update(std::slice::from_ref(&person));

    // Only the initial message should have been received.
    assert_eq!(fixture.aggregator.messages().len(), 1);

    people.push(person);
    let expected_result = take(fixture.sorted(&people), 0, 25);
    assert_eq!(fixture.list(), expected_result);
}

fn update_move_out_of_range(fixture: &Fixture) {
    let people = Fixture::people();
    fixture.add_or_update(&people);

    // Change an item so that it moves from in range to out of range.
    let person = Person::new("P012", 50);
    fixture.add_or_update(std::slice::from_ref(&person));

    let messages = fixture.aggregator.messages();
    assert_eq!(messages.len(), 2);

    let changes = &messages[1];
    assert_eq!(changes.count(), 2);

    let first_change = &changes[0];
    assert_eq!(first_change.reason(), ChangeReason::Remove);
    assert_eq!(first_change.current(), &Person::new("P012", 50));

    let second_change = &changes[1];
    assert_eq!(second_change.reason(), ChangeReason::Add);
    assert_eq!(second_change.current(), &Person::new("P026", 26));

    // Check the correctness of the resulting collection.
    let mut people = fixture.sorted(&people);
    people[11] = person;

    let expected_result = take(fixture.sorted(&people), 0, 25);
    assert_eq!(fixture.list(), expected_result);
}

fn update_stay_range(fixture: &Fixture) {
    let people = Fixture::people();
    fixture.add_or_update(&people);

    // Update an item, but keep it within the expected range.
    let person = Person::new("P012", -1);
    fixture.add_or_update(std::slice::from_ref(&person));

    let messages = fixture.aggregator.messages();
    assert_eq!(messages.len(), 2);

    let changes = &messages[1];
    assert_eq!(changes.count(), 1);

    let first_change = &changes[0];
    assert_eq!(first_change.reason(), ChangeReason::Update);
    assert_eq!(first_change.current(), &Person::new("P012", -1));
    assert_eq!(first_change.previous(), Some(&Person::new("P012", 12)));

    // Check the correctness of the resulting collection.
    let mut people = fixture.sorted(&people);
    people[11] = person;

    let expected_result = take(fixture.sorted(&people), 0, 25);
    assert_eq!(fixture.list(), expected_result);
}

fn update_out_of_range(fixture: &Fixture) {
    let people = Fixture::people();
    fixture.add_or_update(&people);

    let person = Person::new("P050", 100);
    fixture.add_or_update(std::slice::from_ref(&person));

    // Only the initial message should have been received.
    assert_eq!(fixture.aggregator.messages().len(), 1);

    let expected_result = take(fixture.sorted(&people), 0, 25);
    assert_eq!(fixture.list(), expected_result);
}

fn remove_range(fixture: &Fixture) {
    let mut people = Fixture::people();
    fixture.add_or_update(&people);

    // Remove an element from the active range.
    let person = Person::new("P012", 12);
    fixture.source.edit(|updater| updater.remove_item(&person));

    let messages = fixture.aggregator.messages();
    assert_eq!(messages.len(), 2);

    let changes = &messages[1];
    assert_eq!(changes.count(), 2);

    let first_change = &changes[0];
    assert_eq!(first_change.reason(), ChangeReason::Remove);
    assert_eq!(first_change.current(), &person);

    let second_change = &changes[1];
    assert_eq!(second_change.reason(), ChangeReason::Add);
    assert_eq!(second_change.current(), &Person::new("P026", 26));

    // Check the correctness of the resulting collection.
    let index = people.iter().position(|p| p == &person).expect("the person is there");
    people.remove(index);

    let expected_result = take(fixture.sorted(&people), 0, 25);
    assert_eq!(fixture.list(), expected_result);
}

fn remove_out_of_range(fixture: &Fixture) {
    let people = Fixture::people();
    fixture.add_or_update(&people);

    let person = Person::new("P050", 50);
    fixture.source.edit(|updater| updater.remove_item(&person));

    // Only the initial message should have been received.
    assert_eq!(fixture.aggregator.messages().len(), 1);

    let expected_result = take(fixture.sorted(&people), 0, 25);
    assert_eq!(fixture.list(), expected_result);
}

fn refresh_in_range(fixture: &Fixture) {
    let people = Fixture::people();
    fixture.add_or_update(&people);

    let person = people.iter().find(|p| p.name() == "P012").cloned().expect("P012");
    fixture.source.edit(|updater| updater.refresh_item(&person));

    let messages = fixture.aggregator.messages();
    assert_eq!(messages.len(), 2);

    let changes = &messages[1];
    assert_eq!(changes.count(), 1);
    assert_eq!(changes[0].reason(), ChangeReason::Refresh);
}

fn refresh_with_inline_change_in_range(fixture: &Fixture) {
    let people = Fixture::people();
    fixture.add_or_update(&people);

    let person = people.iter().find(|p| p.name() == "P012").cloned().expect("P012");

    // The item moves within the range, so it is propagated as a refresh.
    person.set_age(5);
    fixture.source.edit(|updater| updater.refresh_item(&person));

    let messages = fixture.aggregator.messages();
    assert_eq!(messages.len(), 2);

    let changes = &messages[1];
    assert_eq!(changes.count(), 1);
    assert_eq!(changes[0].reason(), ChangeReason::Refresh);

    let expected_result = take(fixture.sorted(&people), 0, 25);
    assert_eq!(fixture.list(), expected_result);
}

fn refresh_with_inline_change_outside_range(fixture: &Fixture) {
    let people = Fixture::people();
    fixture.add_or_update(&people);

    let person = people.iter().find(|p| p.name() == "P012").cloned().expect("P012");

    // The item moves outside the range, resulting in a remove and an index shift.
    person.set_age(50);
    fixture.source.edit(|updater| updater.refresh_item(&person));

    let messages = fixture.aggregator.messages();
    assert_eq!(messages.len(), 2);

    let changes = &messages[1];
    assert_eq!(changes.count(), 2);

    let first_change = &changes[0];
    assert_eq!(first_change.reason(), ChangeReason::Remove);
    assert_eq!(first_change.current(), &Person::new("P012", 50));

    let second_change = &changes[1];
    assert_eq!(second_change.reason(), ChangeReason::Add);
    assert_eq!(second_change.current(), &Person::new("P026", 26));

    let expected_result = take(fixture.sorted(&people), 0, 25);
    assert_eq!(fixture.list(), expected_result);
}

/// The tests that both fixtures of windows share, inside the module of one way of binding.
macro_rules! window_tests {
    () => {
        #[test]
        fn add_first_in_range() {
            super::add_first_in_range(&fixture());
        }

        #[test]
        fn add_outside_of_range() {
            super::add_outside_of_range(&fixture());
        }

        #[test]
        fn update_move_out_of_range() {
            super::update_move_out_of_range(&fixture());
        }

        #[test]
        fn update_stay_range() {
            super::update_stay_range(&fixture());
        }

        #[test]
        fn update_out_of_range() {
            super::update_out_of_range(&fixture());
        }

        #[test]
        fn remove_range() {
            super::remove_range(&fixture());
        }

        #[test]
        fn remove_out_of_range() {
            super::remove_out_of_range(&fixture());
        }

        #[test]
        fn refresh_in_range() {
            super::refresh_in_range(&fixture());
        }

        #[test]
        fn refresh_with_inline_change_in_range() {
            super::refresh_with_inline_change_in_range(&fixture());
        }

        #[test]
        fn refresh_with_inline_change_outside_range() {
            super::refresh_with_inline_change_outside_range(&fixture());
        }
    };
}

/// The tests of the abstract fixture for one way of binding.
macro_rules! sort_and_virtualize_and_bind_fixture {
    ($module:ident, $set_up:expr) => {
        mod $module {
            use super::Fixture;

            fn fixture() -> Fixture {
                Fixture::new($set_up)
            }

            #[test]
            fn initial_batches() {
                super::initial_batches(&fixture());
            }

            #[test]
            fn overlapping_shift() {
                super::overlapping_shift(&fixture());
            }

            window_tests!();
        }
    };
}

sort_and_virtualize_and_bind_fixture!(sort_and_virtualize_and_bind_with_implicit_options_fixture_read_only_collection, super::with_implicit_options_read_only_collection);
sort_and_virtualize_and_bind_fixture!(sort_and_virtualize_and_bind_fixture_read_only_collection, super::read_only_collection);
sort_and_virtualize_and_bind_fixture!(sort_and_virtualize_and_bind_with_implicit_options_fixture, super::with_implicit_options);
sort_and_virtualize_and_bind_fixture!(sort_and_virtualize_and_bind_fixture, super::sort_and_bind);
