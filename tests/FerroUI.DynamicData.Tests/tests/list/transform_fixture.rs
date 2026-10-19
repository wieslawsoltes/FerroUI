//! `TransformFixture` (`DynamicData.Tests/List/TransformFixture.cs`): the transform of a list.

use crate::domain::{Person, PersonWithGender};
use crate::utilities::{assert_equivalent, assert_equivalent_strict, ListObservableExtensions};
use ferroui_dynamic_data::list::testing::ChangeSetAggregator;
use ferroui_dynamic_data::list::{replace, Change, ChangeSet, ChangeType, IObservableList, ListChangeReason, ObservableListEx, SourceList};
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::{IDisposable, IObservable};
use std::rc::Rc;

/// `_transformFactory`.
fn transform_factory(p: &Rc<Person>) -> Rc<PersonWithGender> {
    let gender = if p.age() % 2 == 0 { "M" } else { "F" };
    PersonWithGender::from_person(p, gender)
}

/// `RemoveIndex` (`ObservableListEx.RemoveIndex`, `List/Linq/WithoutIndexEnumerator.cs`), which no
/// operator of wave 1 provides: the changes without their indexes, and without the moves.
fn remove_index<T: Clone + PartialEq + 'static>(source: Rc<dyn IObservable<ChangeSet<T>>>) -> Rc<dyn IObservable<ChangeSet<T>>> {
    source.select(|changes: ChangeSet<T>| {
        changes
            .iter()
            .filter(|change| change.reason() != ListChangeReason::Moved)
            .map(|change| {
                if change.change_type() == ChangeType::Item {
                    let item = change.item();
                    Change::with_previous(change.reason(), item.current().clone(), item.previous().cloned(), None, None)
                } else {
                    Change::range_change(change.reason(), change.range().items().to_vec(), None)
                }
            })
            .collect::<ChangeSet<T>>()
    })
}

struct Fixture {
    results: ChangeSetAggregator<Rc<PersonWithGender>>,
    source: SourceList<Rc<Person>>,
}

impl Fixture {
    fn new() -> Self {
        let source = SourceList::new();
        let results = ChangeSetAggregator::new(source.connect().transform(transform_factory));
        Self { results, source }
    }

    fn add(&self, person: &Rc<Person>) {
        self.source.edit(|list| list.add(person.clone()));
    }

    fn add_range(&self, people: &[Rc<Person>]) {
        self.source.edit(|list| list.add_range(people.to_vec()));
    }

    fn remove(&self, person: &Rc<Person>) {
        self.source.edit(|list| {
            list.remove(person);
        });
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.source.dispose();
        self.results.dispose();
    }
}

#[test]
fn add() {
    let fixture = Fixture::new();
    let person = Person::new("Adult1", 50);
    fixture.add(&person);

    assert_eq!(fixture.results.messages().len(), 1, "Should be 1 updates");
    assert_eq!(fixture.results.data().count(), 1, "Should be 1 item in the cache");
    assert_eq!(fixture.results.data().items()[0], transform_factory(&person), "Should be same person");
}

#[test]
fn batch_of_unique_updates() {
    let fixture = Fixture::new();
    let people: Vec<Rc<Person>> = (1..=100).map(|i| Person::new(format!("Name{i}"), i)).collect();

    fixture.add_range(&people);

    assert_eq!(fixture.results.messages().len(), 1, "Should be 1 updates");
    assert_eq!(fixture.results.messages()[0].adds(), 100, "Should return 100 adds");

    let mut transformed: Vec<Rc<PersonWithGender>> = people.iter().map(transform_factory).collect();
    transformed.sort_by_key(|p| p.age());
    let mut items = fixture.results.data().items();
    items.sort_by_key(|p| p.age());
    assert_equivalent(&items, &transformed);
}

#[test]
fn clear() {
    let fixture = Fixture::new();
    let people: Vec<Rc<Person>> = (1..=100).map(|l| Person::new(format!("Name{l}"), l)).collect();

    fixture.add_range(&people);
    fixture.source.edit(|list| list.clear());

    assert_eq!(fixture.results.messages().len(), 2, "Should be 2 updates");
    assert_eq!(fixture.results.messages()[0].adds(), 100, "Should be 80 addes");
    assert_eq!(fixture.results.messages()[1].removes(), 100, "Should be 80 removes");
    assert_eq!(fixture.results.data().count(), 0, "Should be nothing cached");
}

#[test]
fn remove() {
    let fixture = Fixture::new();
    const KEY: &str = "Adult1";
    let person = Person::new(KEY, 50);

    fixture.add(&person);
    fixture.remove(&person);

    assert_eq!(fixture.results.messages().len(), 2, "Should be 2 updates");
    assert_eq!(fixture.results.messages()[0].adds(), 1, "Should be 80 addes");
    assert_eq!(fixture.results.messages()[1].removes(), 1, "Should be 80 removes");
    assert_eq!(fixture.results.data().count(), 0, "Should be nothing cached");
}

#[test]
fn remove_object_without_index() {
    let fixture = Fixture::new();
    const KEY: &str = "Adult1";
    let person = Person::new(KEY, 50);

    let results = remove_index(fixture.source.connect()).transform(transform_factory).as_aggregator();

    fixture.add(&person);
    fixture.remove(&person);

    assert_eq!(results.messages().len(), 2, "Should be 2 updates");
    assert_eq!(results.messages()[0].adds(), 1, "Should be 1 addes");
    assert_eq!(results.messages()[1].removes(), 1, "Should be 1 removes");
    assert_eq!(results.data().count(), 0, "Should be nothing cached");
}

// Mirrors coverage for https://github.com/reactivemarbles/DynamicData/issues/1169
#[test]
fn remove_range_without_index() {
    let source = Observable::to_observable(vec![
        ChangeSet::from(vec![Change::range_change(ListChangeReason::AddRange, vec![1, 2, 3, 4, 5, 4, 3, 2, 1], None)]),
        ChangeSet::from(vec![Change::range_change(ListChangeReason::RemoveRange, vec![1, 3, 5], None)]),
    ]);

    let (subscription, results) = source.transform(|item: &i32| item.to_string()).validate_list_change_sets().record_list_items();

    assert!(results.error().is_none(), "no errors should have occurred");
    assert_eq!(results.recorded_change_sets().len(), 2, "2 source operations were performed");
    assert_equivalent_strict(&results.recorded_items(), &["2", "4", "4", "3", "2", "1"].map(String::from));
    assert!(results.has_completed(), "the source, and all asynchronous operations, have completed");
    subscription.dispose();
}

// Mirrors coverage for https://github.com/reactivemarbles/DynamicData/issues/1169
#[test]
fn remove_value_without_index() {
    let source = Observable::to_observable(vec![
        ChangeSet::from(vec![Change::range_change(ListChangeReason::AddRange, vec![1, 2, 3, 2, 1], None)]),
        ChangeSet::from(vec![Change::new(ListChangeReason::Remove, 2, None)]),
    ]);

    let (subscription, results) = source.transform(|item: &i32| item.to_string()).validate_list_change_sets().record_list_items();

    assert!(results.error().is_none(), "no errors should have occurred");
    assert_eq!(results.recorded_change_sets().len(), 2, "2 source operations were performed");
    assert_equivalent_strict(&results.recorded_items(), &["1", "3", "2", "1"].map(String::from));
    assert!(results.has_completed(), "the source, and all asynchronous operations, have completed");
    subscription.dispose();
}

#[test]
fn same_key_changes() {
    let fixture = Fixture::new();
    let people: Vec<Rc<Person>> = (1..=10).map(|i| Person::new("Name", i)).collect();

    fixture.add_range(&people);

    assert_eq!(fixture.results.messages().len(), 1, "Should be 1 updates");
    assert_eq!(fixture.results.messages()[0].adds(), 10, "Should return 10 adds");
    assert_eq!(fixture.results.data().count(), 10, "Should result in 10 records");
}

#[test]
fn update() {
    let fixture = Fixture::new();
    const KEY: &str = "Adult1";
    let newperson = Person::new(KEY, 50);
    let updated = Person::new(KEY, 51);

    fixture.add(&newperson);
    fixture.add(&updated);

    assert_eq!(fixture.results.messages().len(), 2, "Should be 2 updates");
    assert_eq!(fixture.results.messages()[0].adds(), 1, "Should be 1 adds");
    assert_eq!(fixture.results.messages()[0].replaced(), 0, "Should be 1 update");
}

#[test]
fn multiple_subscribers_should_not_share_state() {
    let fixture = Fixture::new();
    fixture.add_range(&[Person::new("Adult1", 50), Person::new("Adult2", 51)]);

    let transformed = fixture.source.connect().transform(|o: &Rc<Person>| o.clone());

    // Will fail if the state of the ChangeAwareList is shared.
    let first = transformed.transform(|o: &Rc<Person>| o.clone()).subscribe_ignore();
    let second = transformed.transform(|o: &Rc<Person>| o.clone()).subscribe_ignore();
    first.dispose();
    second.dispose();
}

#[test]
fn transform_on_refresh() {
    let items: Vec<Rc<Person>> = (1..=100).map(|i| Person::new(format!("Person{i}"), 1)).collect();

    // The result should only be true when all items are set to true.
    let list = SourceList::<Rc<Person>>::new();
    let results = list.connect().auto_refresh(ferroui_system::prop!(Person => age)).transform_with(|v: &Rc<Person>| v.clone(), true).as_aggregator();
    list.edit(|l| l.add_range(items.clone()));

    assert_eq!(results.data().count(), 100);
    assert_eq!(results.messages().len(), 1);

    items[0].set_age(10);
    assert_eq!(results.data().count(), 100);
    assert_eq!(results.messages().len(), 2);

    assert_eq!(results.messages()[1][0].reason(), ListChangeReason::Replace);

    // Remove an item and check no change is fired.
    let to_remove = items[1].clone();
    list.edit(|l| {
        l.remove(&to_remove);
    });
    assert_eq!(results.data().count(), 99);
    assert_eq!(results.messages().len(), 3);
    to_remove.set_age(100);
    assert_eq!(results.messages().len(), 3);

    // Add it back in and check it updates.
    list.edit(|l| l.add(to_remove.clone()));
    assert_eq!(results.messages().len(), 4);
    to_remove.set_age(101);
    assert_eq!(results.messages().len(), 5);

    assert_eq!(results.messages().last().unwrap()[0].reason(), ListChangeReason::Replace);

    list.dispose();
}

#[test]
fn transform_on_replace() {
    let fixture = Fixture::new();
    // Arrange
    let person1 = Person::new("Bob", 37);
    let person2 = Person::new("Bob", 38);

    fixture.add(&person1);

    // Act
    fixture.source.edit(|list| replace(list, &person1, person2.clone()));

    // Assert
    assert_eq!(fixture.results.messages().len(), 2, "Should be 2 messages");
    assert_eq!(fixture.results.data().count(), 1, "Should be 1 item in the cache");
    assert_eq!(fixture.results.data().items()[0], transform_factory(&person2), "Should be same person");
}

#[test]
fn transform_on_replace_without_index() {
    let fixture = Fixture::new();
    // Arrange
    let person1 = Person::new("Bob", 37);
    let person2 = Person::new("Bob", 38);
    let results = remove_index(fixture.source.connect()).transform(transform_factory).as_aggregator();
    fixture.add(&person1);

    // Act
    fixture.source.edit(|list| replace(list, &person1, person2.clone()));

    // Assert
    assert_eq!(results.messages().len(), 2, "Should be 2 messages");
    assert_eq!(results.data().count(), 1, "Should be 1 item in the cache");
    assert_eq!(results.data().items()[0], transform_factory(&person2), "Should be same person");
}
