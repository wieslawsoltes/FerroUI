//! `SortFixture` and `SortChangedFixture` of lists (`DynamicData.Tests/List/SortFixture.cs`).

use crate::domain::{Person, RandomPersonGenerator};
use crate::utilities::assert_equivalent;
use ferroui_dynamic_data::binding::SortExpressionComparer;
use ferroui_dynamic_data::list::testing::ChangeSetAggregator;
use ferroui_dynamic_data::list::{remove_many, IObservableList, ObservableListEx, SourceList};
use ferroui_reactive::subjects::Subject;
use ferroui_reactive::ObservableExt;
use ferroui_system::{IDisposable, IObserver};
use std::cmp::Ordering;
use std::rc::Rc;

type Comparer<T> = Rc<dyn Fn(&T, &T) -> Ordering>;

fn sorted<T: Clone>(items: &[T], comparer: &Comparer<T>) -> Vec<T> {
    let mut items = items.to_vec();
    items.sort_by(|a, b| comparer(a, b));
    items
}

// `SortChangedFixture`.

/// `ListItem`: a number, comparable by `DefaultComparer`.
#[derive(Clone, Debug, PartialEq)]
struct ListItem {
    number: i32,
}

/// See https://github.com/reactivemarbles/DynamicData/issues/473.
#[test]
fn sort_changed_fixture_sorts_without_error() {
    let source = SourceList::new();
    let sorter = Subject::<Comparer<ListItem>>::new();
    source.edit(|list| list.add_range((1..=10).map(|number| ListItem { number }).collect()));

    let (bound_changes, bound) = source.connect().sort_on_observable(sorter.as_observable()).bind_read_only();
    let subscription = bound_changes.subscribe_ignore();

    let numbers: Vec<i32> = bound.to_vec().iter().map(|x| x.number).collect();
    assert!(numbers.windows(2).all(|pair| pair[0] <= pair[1]), "in ascending order");

    sorter.on_next(SortExpressionComparer::<ListItem>::descending(|x| x.number).comparer());

    let numbers: Vec<i32> = bound.to_vec().iter().map(|x| x.number).collect();
    assert!(numbers.windows(2).all(|pair| pair[0] >= pair[1]), "in descending order");
    subscription.dispose();
}

// `SortFixture`.

struct Fixture {
    comparer: Comparer<Rc<Person>>,
    generator: RandomPersonGenerator,
    results: ChangeSetAggregator<Rc<Person>>,
    source: SourceList<Rc<Person>>,
}

impl Fixture {
    fn new() -> Self {
        let comparer = SortExpressionComparer::<Rc<Person>>::ascending(|p| p.name()).then_by_ascending(|p| p.age()).comparer();
        let source = SourceList::new();
        let results = source.connect().sort(comparer.clone()).as_aggregator();
        Self { comparer, generator: RandomPersonGenerator::new(), results, source }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.results.dispose();
        self.source.dispose();
    }
}

#[test]
fn insert() {
    let fixture = Fixture::new();
    let people = fixture.generator.take(100);
    fixture.source.edit(|list| list.add_range(people));

    // Deviation (DEVIATIONS.md, DynamicData): `!A` for upstream's `__A`, which sorts first among
    // the names by culture upstream; names compare by their characters here.
    let should_be_first = Person::new("!A", 99);
    fixture.source.edit(|list| list.add(should_be_first.clone()));

    assert_eq!(fixture.results.data().count(), 101, "Should be 100 people in the cache");
    assert_eq!(fixture.results.data().items()[0], should_be_first);
}

#[test]
fn remove() {
    let fixture = Fixture::new();
    let mut people = fixture.generator.take(100);
    fixture.source.edit(|list| list.add_range(people.clone()));

    let to_remove = people[20].clone();
    people.remove(20);
    fixture.source.edit(|list| list.remove_at(20));

    assert_eq!(fixture.results.data().count(), 99, "Should be 99 people in the cache");
    assert_eq!(fixture.results.messages().len(), 2, "Should be 2 update messages");
    assert_eq!(fixture.results.messages()[1][0].item().current(), &to_remove, "Incorrect item removed");

    let expected_result = sorted(&people, &fixture.comparer);
    assert_equivalent(&fixture.results.data().items(), &expected_result);
}

#[test]
fn remove_many_odds() {
    let fixture = Fixture::new();
    let people = fixture.generator.take(100);
    fixture.source.edit(|list| list.add_range(people.clone()));

    let odd: Vec<Rc<Person>> = people.iter().enumerate().filter(|(index, _)| index % 2 == 1).map(|(_, p)| p.clone()).collect();
    fixture.source.edit(|list| remove_many(list, &odd));

    assert_eq!(fixture.results.data().count(), 50, "Should be 99 people in the cache");
    assert_eq!(fixture.results.messages().len(), 2, "Should be 2 update messages");

    let expected_result: Vec<Rc<Person>> = people.iter().filter(|p| !odd.contains(p)).cloned().collect();
    assert_equivalent(&fixture.results.data().items(), &expected_result);
}

#[test]
fn remove_many_ordered() {
    let fixture = Fixture::new();
    let people = fixture.generator.take(100);
    fixture.source.edit(|list| list.add_range(people.clone()));

    let to_remove: Vec<Rc<Person>> = sorted(&people, &fixture.comparer).into_iter().skip(10).take(90).collect();
    fixture.source.edit(|list| remove_many(list, &to_remove));

    assert_eq!(fixture.results.data().count(), 10, "Should be 10 people in the cache");
    assert_eq!(fixture.results.messages().len(), 2, "Should be 2 update messages");

    let expected_result: Vec<Rc<Person>> = sorted(&people, &fixture.comparer).into_iter().take(10).collect();
    assert_equivalent(&fixture.results.data().items(), &expected_result);
}

#[test]
fn remove_many_reverse_ordered() {
    let fixture = Fixture::new();
    let people = fixture.generator.take(100);
    fixture.source.edit(|list| list.add_range(people.clone()));

    let descending: Vec<Rc<Person>> = sorted(&people, &fixture.comparer).into_iter().rev().collect();
    let to_remove: Vec<Rc<Person>> = descending.iter().skip(10).take(90).cloned().collect();
    fixture.source.edit(|list| remove_many(list, &to_remove));

    assert_eq!(fixture.results.data().count(), 10, "Should be 99 people in the cache");
    assert_eq!(fixture.results.messages().len(), 2, "Should be 2 update messages");

    let expected_result: Vec<Rc<Person>> = descending.into_iter().take(10).collect();
    assert_equivalent(&fixture.results.data().items(), &expected_result);
}

#[test]
fn replace() {
    let fixture = Fixture::new();
    let people = fixture.generator.take(100);
    fixture.source.edit(|list| list.add_range(people));

    // Deviation (DEVIATIONS.md, DynamicData): `!A` for upstream's `__A` (see `insert`).
    let should_be_first = Person::new("!A", 99);
    fixture.source.edit(|list| list.set(10, should_be_first.clone()));

    assert_eq!(fixture.results.data().count(), 100, "Should be 100 people in the cache");
    assert_eq!(fixture.results.data().items()[0], should_be_first);
}

#[test]
fn sort_initial_batch() {
    let fixture = Fixture::new();
    let people = fixture.generator.take(100);
    fixture.source.edit(|list| list.add_range(people.clone()));

    assert_eq!(fixture.results.data().count(), 100, "Should be 100 people in the cache");

    let expected_result = sorted(&people, &fixture.comparer);
    assert_equivalent(&fixture.results.data().items(), &expected_result);
}
