//! `FilterControllerFixtureWithClearAndReplace`
//! (`DynamicData.Tests/List/FilterControllerFixtureWithClearAndReplace.cs`): the dynamic filter of a
//! list whose new predicates clear the filtered items and add those that match again.

use crate::domain::Person;
use crate::utilities::assert_equivalent_because;
use ferroui_dynamic_data::list::testing::ChangeSetAggregator;
use ferroui_dynamic_data::list::{IObservableList, ListFilterPolicy, ObservableListEx, SourceList};
use ferroui_reactive::subjects::BehaviorSubject;
use ferroui_system::{IDisposable, IObserver};
use std::rc::Rc;

type PersonPredicate = Rc<dyn Fn(&Rc<Person>) -> bool>;

fn predicate(f: impl Fn(&Rc<Person>) -> bool + 'static) -> PersonPredicate {
    Rc::new(f)
}

/// The fixture: a source, a subject of predicates that starts with `p.Age > 20`, and the
/// aggregator of the filter with the policy `ClearAndReplace`.
struct Fixture {
    filter: Rc<BehaviorSubject<PersonPredicate>>,
    results: ChangeSetAggregator<Rc<Person>>,
    source: SourceList<Rc<Person>>,
}

impl Fixture {
    fn new() -> Self {
        let source = SourceList::<Rc<Person>>::new();
        let filter = BehaviorSubject::new(predicate(|p| p.age() > 20));
        let results = source.connect().filter_dynamic_with(filter.as_observable(), ListFilterPolicy::ClearAndReplace).as_aggregator();
        Self { filter, results, source }
    }

    fn add(&self, person: Rc<Person>) {
        self.source.edit(|list| list.add(person));
    }

    fn add_range(&self, people: &[Rc<Person>]) {
        let people = people.to_vec();
        self.source.edit(|list| list.add_range(people));
    }

    fn remove(&self, person: &Rc<Person>) {
        self.source.edit(|list| {
            list.remove(person);
        });
    }

    fn clear(&self) {
        self.source.edit(|list| list.clear());
    }

    /// `Replace(original, replaceWith)` (`ListEx.Replace`): the item at the index of `original`.
    fn replace(&self, original: &Rc<Person>, replace_with: Rc<Person>) {
        self.source.edit(|list| {
            let index = list.index_of(original).expect("Cannot find index of original item");
            list.set(index, replace_with);
        });
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.source.dispose();
        self.results.dispose();
    }
}

fn people(prefix: &str, count: i32) -> Vec<Rc<Person>> {
    (1..=count).map(|i| Person::new(format!("{prefix}{i}"), i)).collect()
}

fn sorted_by_age(people: impl IntoIterator<Item = Rc<Person>>) -> Vec<Rc<Person>> {
    let mut people: Vec<Rc<Person>> = people.into_iter().collect();
    people.sort_by_key(|p| p.age());
    people
}

// Should be the same as standard lambda filter

#[test]
fn add_matched() {
    let fixture = Fixture::new();
    let person = Person::new("Adult1", 50);
    fixture.add(person.clone());

    assert_eq!(fixture.results.messages().len(), 1, "Should be 1 updates");
    assert_eq!(fixture.results.data().count(), 1, "Should be 1 item in the cache");
    assert_eq!(fixture.results.data().items()[0], person, "Should be same person");
}

#[test]
fn add_not_matched() {
    let fixture = Fixture::new();
    let person = Person::new("Adult1", 10);
    fixture.add(person);

    assert_eq!(fixture.results.messages().len(), 0, "Should have no item updates");
    assert_eq!(fixture.results.data().count(), 0, "Cache should have no items");
}

#[test]
fn add_not_matched_and_update_matched() {
    let fixture = Fixture::new();
    let key = "Adult1";
    let notmatched = Person::new(key, 19);
    let matched = Person::new(key, 21);

    fixture.source.edit(|updater| {
        updater.add(notmatched.clone());
        updater.add(matched.clone());
    });

    assert_eq!(fixture.results.messages().len(), 1, "Should be 1 updates");
    assert_eq!(fixture.results.messages()[0].changes()[0].range().items()[0], matched, "Should be same person");
    assert_eq!(fixture.results.data().items()[0], matched, "Should be same person");
}

#[test]
fn attempted_removal_of_a_non_existent_key_will_be_ignored() {
    let fixture = Fixture::new();
    fixture.remove(&Person::new("A", 1));
    assert_eq!(fixture.results.messages().len(), 0, "Should be 0 updates");
}

#[test]
fn batch_of_unique_updates() {
    let fixture = Fixture::new();
    let people = people("Name", 100);

    fixture.add_range(&people);
    assert_eq!(fixture.results.messages().len(), 1, "Should be 1 updates");
    assert_eq!(fixture.results.messages()[0].adds(), 80, "Should return 80 adds");

    let filtered = sorted_by_age(people.iter().filter(|p| p.age() > 20).cloned());
    assert_equivalent_because(&sorted_by_age(fixture.results.data().items()), &filtered, "Incorrect Filter result");
}

#[test]
fn batch_removes() {
    let fixture = Fixture::new();
    let people = people("Name", 100);

    fixture.add_range(&people);
    fixture.clear();

    assert_eq!(fixture.results.messages().len(), 2, "Should be 2 updates");
    assert_eq!(fixture.results.messages()[0].adds(), 80, "Should be 80 addes");
    assert_eq!(fixture.results.messages()[1].removes(), 80, "Should be 80 removes");
    assert_eq!(fixture.results.data().count(), 0, "Should be nothing cached");
}

#[test]
fn batch_successive_updates() {
    let fixture = Fixture::new();
    let people = people("Name", 100);
    for person in &people {
        fixture.add(person.clone());
    }

    assert_eq!(fixture.results.messages().len(), 80, "Should be 80 messages");
    assert_eq!(fixture.results.data().count(), 80, "Should be 80 in the cache");
    let filtered = sorted_by_age(people.iter().filter(|p| p.age() > 20).cloned());
    assert_equivalent_because(&sorted_by_age(fixture.results.data().items()), &filtered, "Incorrect Filter result");
}

#[test]
fn change_filter() {
    let fixture = Fixture::new();
    let people = people("P", 100);

    fixture.add_range(&people);
    assert_eq!(fixture.results.data().count(), 80, "Should be 80 people in the cache");

    fixture.filter.on_next(predicate(|p| p.age() <= 50));
    assert_eq!(fixture.results.data().count(), 50, "Should be 50 people in the cache");
    assert_eq!(fixture.results.messages().len(), 2, "Should be 2 update messages");

    assert!(fixture.results.data().items().iter().all(|p| p.age() <= 50));
}

#[test]
fn clear() {
    let fixture = Fixture::new();
    let people = people("Name", 100);
    fixture.add_range(&people);
    fixture.clear();

    assert_eq!(fixture.results.messages().len(), 2, "Should be 2 updates");
    assert_eq!(fixture.results.messages()[0].adds(), 80, "Should be 80 addes");
    assert_eq!(fixture.results.messages()[1].removes(), 80, "Should be 80 removes");
    assert_eq!(fixture.results.data().count(), 0, "Should be nothing cached");
}

#[test]
fn reevaluate_filter() {
    let fixture = Fixture::new();
    // re-evaluate for inline changes
    let people = people("P", 100);

    fixture.add_range(&people);
    assert_eq!(fixture.results.data().count(), 80, "Should be 80 people in the cache");

    for person in &people {
        person.set_age(person.age() + 10);
    }

    fixture.filter.on_next(predicate(|p| p.age() > 20));

    assert_eq!(fixture.results.data().count(), 90);
    assert_eq!(fixture.results.messages().len(), 2);
    assert_eq!(fixture.results.messages()[1].removes(), 80);
    assert_eq!(fixture.results.messages()[1].adds(), 90);

    for person in &people {
        person.set_age(person.age() - 10);
    }

    fixture.filter.on_next(predicate(|p| p.age() > 20));

    assert_eq!(fixture.results.data().count(), 80, "Should be 80 people in the cache");
    assert_eq!(fixture.results.messages().len(), 3, "Should be 3 update messages");
}

#[test]
fn remove() {
    let fixture = Fixture::new();
    let key = "Adult1";
    let person = Person::new(key, 50);

    fixture.add(person.clone());
    fixture.remove(&person);

    assert_eq!(fixture.results.messages().len(), 2, "Should be 2 updates");
    assert_eq!(fixture.results.messages().len(), 2, "Should be 2 updates");
    assert_eq!(fixture.results.messages()[0].adds(), 1, "Should be 80 addes");
    assert_eq!(fixture.results.messages()[1].removes(), 1, "Should be 80 removes");
    assert_eq!(fixture.results.data().count(), 0, "Should be nothing cached");
}

#[test]
fn same_key_changes() {
    let fixture = Fixture::new();
    let key = "Adult1";

    fixture.source.edit(|updater| {
        updater.add(Person::new(key, 50));
        updater.add(Person::new(key, 52));
        updater.add(Person::new(key, 53));
        // updater.Remove(key);
    });

    assert_eq!(fixture.results.messages().len(), 1, "Should be 1 updates");
    assert_eq!(fixture.results.messages()[0].adds(), 3, "Should be 3 adds");
}

#[test]
fn update_matched() {
    let fixture = Fixture::new();
    let key = "Adult1";
    let newperson = Person::new(key, 50);
    let updated = Person::new(key, 51);

    fixture.add(newperson.clone());
    fixture.replace(&newperson, updated);

    assert_eq!(fixture.results.messages().len(), 2, "Should be 2 updates");
    assert_eq!(fixture.results.messages()[0].adds(), 1, "Should be 1 adds");
    assert_eq!(fixture.results.messages()[1].replaced(), 1, "Should be 1 update");
}

#[test]
fn update_not_matched() {
    let fixture = Fixture::new();
    let key = "Adult1";
    let newperson = Person::new(key, 10);
    let updated = Person::new(key, 11);

    fixture.add(newperson.clone());
    fixture.replace(&newperson, updated);

    assert_eq!(fixture.results.messages().len(), 0, "Should be no updates");
    assert_eq!(fixture.results.data().count(), 0, "Should nothing cached");
}

#[test]
fn very_large_data_set() {
    let initial: Rc<dyn Fn(&i32) -> bool> = Rc::new(|_: &i32| false);
    let filter = BehaviorSubject::new(initial);
    let source = SourceList::<i32>::new();

    let _result = source.connect().filter_dynamic_with(filter.as_observable(), ListFilterPolicy::ClearAndReplace).as_observable_list();
    source.edit(|list| list.add_range((1..=250_000).collect()));

    filter.on_next(Rc::new(|_: &i32| true));
    filter.on_next(Rc::new(|_: &i32| false));
}
