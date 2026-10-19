//! `SortMutableFixture` (`DynamicData.Tests/List/SortMutableFixture.cs`).

use crate::domain::{Person, RandomPersonGenerator, Randomizer};
use crate::utilities::assert_equivalent;
use ferroui_dynamic_data::binding::SortExpressionComparer;
use ferroui_dynamic_data::list::testing::ChangeSetAggregator;
use ferroui_dynamic_data::list::{remove_many, IObservableList, ListChangeReason, ObservableListEx, SortOptions, SourceList};
use ferroui_reactive::subjects::{BehaviorSubject, Subject};
use ferroui_system::{IDisposable, IObserver};
use std::cmp::Ordering;
use std::rc::Rc;

type Comparer = Rc<dyn Fn(&Rc<Person>, &Rc<Person>) -> Ordering>;

fn sorted(items: &[Rc<Person>], comparer: &Comparer) -> Vec<Rc<Person>> {
    let mut items = items.to_vec();
    items.sort_by(|a, b| comparer(a, b));
    items
}

struct Fixture {
    change_comparer: Rc<BehaviorSubject<Comparer>>,
    comparer: Comparer,
    generator: RandomPersonGenerator,
    resort: Rc<Subject<()>>,
    results: ChangeSetAggregator<Rc<Person>>,
    source: SourceList<Rc<Person>>,
}

impl Fixture {
    fn new() -> Self {
        let comparer = SortExpressionComparer::<Rc<Person>>::ascending(|p| p.age()).then_by_ascending(|p| p.name()).comparer();
        let source = SourceList::new();
        let change_comparer = BehaviorSubject::new(comparer.clone());
        let resort = Subject::new();
        let results = source.connect().sort_on_observable_with(change_comparer.as_observable(), SortOptions::None, Some(resort.as_observable()), 25).as_aggregator();
        Self { change_comparer, comparer, generator: RandomPersonGenerator::new(), resort, results, source }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.results.dispose();
        self.source.dispose();
    }
}

#[test]
fn change_comparer() {
    let fixture = Fixture::new();
    let people = fixture.generator.take(100);
    fixture.source.edit(|list| list.add_range(people.clone()));

    let new_comparer = SortExpressionComparer::<Rc<Person>>::ascending(|p| p.name()).then_by_ascending(|p| p.age()).comparer();
    fixture.change_comparer.on_next(new_comparer.clone());

    assert_eq!(fixture.results.data().count(), 100, "Should be 100 people in the cache");

    let expected_result = sorted(&people, &new_comparer);
    assert_equivalent(&fixture.results.data().items(), &expected_result);
}

#[test]
fn insert() {
    let fixture = Fixture::new();
    let people = fixture.generator.take(100);
    fixture.source.edit(|list| list.add_range(people));

    let should_be_last = Person::new("__A", 10000);
    fixture.source.edit(|list| list.add(should_be_last.clone()));

    assert_eq!(fixture.results.data().count(), 101);
    assert_eq!(fixture.results.data().items().last(), Some(&should_be_last));
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

    assert_eq!(fixture.results.data().count(), 10, "Should be 99 people in the cache");
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

    let should_be_last = Person::new("__A", 999);
    fixture.source.edit(|list| list.set(10, should_be_last.clone()));

    assert_eq!(fixture.results.data().count(), 100, "Should be 100 people in the cache");
    assert_eq!(fixture.results.data().items().last(), Some(&should_be_last));
}

#[test]
fn resort() {
    let fixture = Fixture::new();
    let people = fixture.generator.take(100);
    fixture.source.edit(|list| list.add_range(people.clone()));

    for (index, person) in Randomizer::new_unseeded().shuffle(&people).iter().enumerate() {
        person.set_age(index as i32);
    }

    fixture.resort.on_next(());

    assert_eq!(fixture.results.data().count(), 100, "Should be 100 people in the cache");

    let expected_result = sorted(&people, &fixture.comparer);
    assert_equivalent(&fixture.results.data().items(), &expected_result);
}

#[test]
fn resort_on_inline_changes() {
    let fixture = Fixture::new();
    let people = fixture.generator.take(10);
    fixture.source.edit(|list| list.add_range(people.clone()));

    people[0].set_age(-1);
    people[1].set_age(-10);
    people[2].set_age(-12);
    people[3].set_age(-5);
    people[4].set_age(-7);
    people[5].set_age(-6);

    let comparer = SortExpressionComparer::<Rc<Person>>::descending(|p| p.age()).then_by_ascending(|p| p.name()).comparer();
    fixture.change_comparer.on_next(comparer.clone());

    let expected_result = sorted(&people, &comparer);
    let actual_result = fixture.results.data().items();
    assert_equivalent(&actual_result, &expected_result);
}

#[test]
fn sort_initial_batch() {
    let fixture = Fixture::new();
    let people = fixture.generator.take(100);
    fixture.source.edit(|list| list.add_range(people.clone()));

    assert_eq!(fixture.results.data().count(), 100);

    let expected_result = sorted(&people, &fixture.comparer);
    assert_equivalent(&fixture.results.data().items(), &expected_result);
}

#[test]
fn update_more_than_threshold() {
    let fixture = Fixture::new();
    let all_people = fixture.generator.take(1100);
    let people: Vec<Rc<Person>> = all_people.iter().take(100).cloned().collect();
    fixture.source.edit(|list| list.add_range(people.clone()));

    assert_eq!(fixture.results.data().count(), 100, "Should be 100 people in the cache");

    let more_people: Vec<Rc<Person>> = all_people.iter().skip(100).cloned().collect();
    fixture.source.edit(|list| list.add_range(more_people.clone()));

    assert_eq!(fixture.results.data().count(), 1100, "Should be 1100 people in the cache");

    let mut union = people.clone();
    for person in &more_people {
        if !union.contains(person) {
            union.push(person.clone());
        }
    }
    let expected_result = sorted(&union, &fixture.comparer);
    assert_equivalent(&fixture.results.data().items(), &expected_result);

    let messages = fixture.results.messages();
    assert_eq!(messages.len(), 2, "Should be 2 messages");

    let last_message = messages.last().expect("a last message");
    assert_eq!(last_message[0].range().count(), 100, "Should be 100 in the range");
    assert_eq!(last_message[0].reason(), ListChangeReason::Clear);
    assert_eq!(last_message[last_message.count() - 1].range().count(), 1100, "Should be 1100 in the range");
    assert_eq!(last_message[last_message.count() - 1].reason(), ListChangeReason::AddRange);
}
