//! `OfTypeFixture` (`DynamicData.Tests/Cache/OfTypeFixture.cs`): the items of a cache that are of a
//! type (`OfType`).
//!
//! Upstream's `CatPerson` derives from its `Person` and the cache holds `Person`s. Rust has no
//! inheritance: the cache holds `Rc<dyn Any>` items, each a [`Person`] or a [`CatPerson`], keyed by
//! the `Id` of whichever it is, and `of_type_cache::<CatPerson>()` keeps the cat people; "is a
//! `Person`" (every item) is the whole cache. The test spies (`TestSpy`) only write to the output
//! of the test and are left out.

use crate::domain::{Faker, FakerContext, Guid, Randomizer};
use crate::utilities::assert_equivalent;
use ferroui_dynamic_data::cache::testing::ChangeSetAggregator;
use ferroui_dynamic_data::cache::{IConnectableCache, IObservableCache, ObservableCacheEx, SourceCache, SourceCacheEx};
use ferroui_system::IDisposable;
use std::any::Any;
use std::rc::Rc;

const ADD_COUNT: usize = if cfg!(debug_assertions) { 7 } else { 101 };
const UPDATE_COUNT: usize = if cfg!(debug_assertions) { 5 } else { 57 };
const REMOVE_COUNT: usize = if cfg!(debug_assertions) { 3 } else { 53 };

// Deviation (DEVIATIONS.md, DynamicData): the cache holds `Rc<dyn Any>` items (no inheritance), and
// the test spies (`TestSpy`, output only) are left out.
/// An item of the cache: a [`Person`] or a [`CatPerson`].
type Item = Rc<dyn Any>;

/// `record Person(string Name, string Id)`.
#[derive(Clone, Debug, PartialEq)]
struct Person {
    name: String,
    id: String,
}

impl Person {
    /// `Person(string Name)`: a new identifier.
    fn new(name: String) -> Rc<Self> {
        Self::with_id(name, Guid::new_guid().to_string())
    }

    fn with_id(name: String, id: String) -> Rc<Self> {
        Rc::new(Self { name, id })
    }
}

/// `record CatPerson(string Name, string CatName, string Id) : Person(Name, Id), ICatPerson`.
#[derive(Clone, Debug, PartialEq)]
struct CatPerson {
    name: String,
    cat_name: String,
    id: String,
}

impl CatPerson {
    /// `CatPerson(string Name, string CatName)`: a new identifier.
    fn new(name: String, cat_name: String) -> Rc<Self> {
        Self::with_id(name, cat_name, Guid::new_guid().to_string())
    }

    fn with_id(name: String, cat_name: String, id: String) -> Rc<Self> {
        Rc::new(Self { name, cat_name, id })
    }
}

/// An item as it compares (records compare by their members).
#[derive(Debug, PartialEq)]
enum Described {
    Person(Person),
    CatPerson(CatPerson),
}

fn describe(item: &Item) -> Described {
    if let Some(cat_person) = (**item).downcast_ref::<CatPerson>() {
        Described::CatPerson(cat_person.clone())
    } else if let Some(person) = (**item).downcast_ref::<Person>() {
        Described::Person(person.clone())
    } else {
        panic!("the cache holds only people")
    }
}

/// `p.Id`.
fn id_of(item: &Item) -> String {
    match describe(item) {
        Described::Person(person) => person.id,
        Described::CatPerson(cat_person) => cat_person.id,
    }
}

/// `p is CatPerson`.
fn is_cat_person(item: &Item) -> bool {
    (**item).downcast_ref::<CatPerson>().is_some()
}

fn as_item<X: Any>(item: &Rc<X>) -> Item {
    item.clone()
}

/// The fixture (`OfTypeFixture` with its constructor and `Dispose`).
struct Fixture {
    randomizer: Randomizer,
    person_faker: Faker<Rc<Person>>,
    cat_person_faker: Faker<Rc<CatPerson>>,
    source_cache: SourceCache<Item, String>,
    person_results: ChangeSetAggregator<Item, String>,
    cat_person_results: ChangeSetAggregator<Rc<CatPerson>, String>,
}

impl Fixture {
    fn new() -> Self {
        let source_cache = SourceCache::new(id_of);
        let randomizer = Randomizer::new(0x3737_ddcc);
        let person_faker = Faker::new(|faker: &FakerContext| Person::new(faker.person().full_name())).with_seed(&randomizer);
        let cat_person_faker =
            Faker::new(|faker: &FakerContext| CatPerson::new(faker.person().full_name(), format!("{} the {}", faker.hacker().adjective(), faker.hacker().noun()))).with_seed(&randomizer);
        let person_results = source_cache.connect().as_aggregator();
        let cat_person_results = source_cache.connect().of_type_cache::<CatPerson>().as_aggregator();
        Self { randomizer, person_faker, cat_person_faker, source_cache, person_results, cat_person_results }
    }

    /// `GenerateUpdateRandom`.
    fn generate_update_random(&self, id: &str) -> Item {
        if self.randomizer.bool() {
            as_item(&self.generate_update_person(id))
        } else {
            as_item(&self.generate_update_cat_person(id))
        }
    }

    /// `GenerateUpdatePerson`.
    fn generate_update_person(&self, id: &str) -> Rc<Person> {
        Person::with_id(self.person_faker.generate_one().name.clone(), id.to_owned())
    }

    /// `GenerateUpdateCatPerson`.
    fn generate_update_cat_person(&self, id: &str) -> Rc<CatPerson> {
        let new_cp = self.cat_person_faker.generate_one();
        CatPerson::with_id(new_cp.name.clone(), new_cp.cat_name.clone(), id.to_owned())
    }

    /// `CheckResults`.
    fn check_results(&self) {
        let expected_people = self.source_cache.items();
        let expected_cat_people: Vec<Rc<CatPerson>> = expected_people.iter().filter_map(|p| p.clone().downcast::<CatPerson>().ok()).collect();

        let actual_people: Vec<Described> = self.person_results.data().items().iter().map(describe).collect();
        let expected_people: Vec<Described> = expected_people.iter().map(describe).collect();
        assert_equivalent(&actual_people, &expected_people);
        assert_equivalent(&self.cat_person_results.data().items(), &expected_cat_people);
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.source_cache.dispose();
        self.person_results.dispose();
        self.cat_person_results.dispose();
    }
}

#[test]
fn added_items_are_in_results() {
    let fixture = Fixture::new();

    // Arrange
    let people = fixture.person_faker.generate(ADD_COUNT);
    let cat_people = fixture.cat_person_faker.generate(ADD_COUNT);

    fixture.source_cache.add_or_update_items(people.iter().map(as_item));
    fixture.source_cache.add_or_update_items(cat_people.iter().map(as_item));

    assert_eq!(fixture.person_results.summary().overall.adds, ADD_COUNT * 2);
    assert_eq!(fixture.person_results.messages().len(), 2);
    assert_eq!(fixture.cat_person_results.summary().overall.adds, ADD_COUNT);
    assert_eq!(fixture.cat_person_results.messages().len(), 1);
    fixture.check_results();
}

#[test]
fn removed_items_are_not_results() {
    let fixture = Fixture::new();

    let people = fixture.person_faker.generate(ADD_COUNT);
    let cat_people = fixture.cat_person_faker.generate(ADD_COUNT);

    fixture.source_cache.add_or_update_items(people.iter().map(as_item));
    fixture.source_cache.add_or_update_items(cat_people.iter().map(as_item));
    fixture.source_cache.remove_items(fixture.randomizer.list_items(&people, REMOVE_COUNT).iter().map(as_item));
    fixture.source_cache.remove_items(fixture.randomizer.list_items(&cat_people, REMOVE_COUNT).iter().map(as_item));

    let person_summary = fixture.person_results.summary();
    assert_eq!(person_summary.overall.adds, ADD_COUNT * 2);
    assert_eq!(person_summary.overall.removes, REMOVE_COUNT * 2);
    assert_eq!(fixture.person_results.messages().len(), 4);
    let cat_person_summary = fixture.cat_person_results.summary();
    assert_eq!(cat_person_summary.overall.adds, ADD_COUNT);
    assert_eq!(cat_person_summary.overall.removes, REMOVE_COUNT);
    assert_eq!(fixture.cat_person_results.messages().len(), 2);
    fixture.check_results();
}

#[test]
fn update_results_are_correct() {
    let fixture = Fixture::new();

    // Arrange
    let people = fixture.person_faker.generate(ADD_COUNT);
    let cat_people = fixture.cat_person_faker.generate(ADD_COUNT);

    fixture.source_cache.add_or_update_items(people.iter().map(as_item));
    fixture.source_cache.add_or_update_items(cat_people.iter().map(as_item));

    let all: Vec<Item> = people.iter().map(as_item).chain(cat_people.iter().map(as_item)).collect();
    let updates = fixture.randomizer.list_items(&all, UPDATE_COUNT);
    let pre_update_cat_people: Vec<Item> = updates.iter().filter(|p| is_cat_person(p)).cloned().collect();
    let updated: Vec<Item> = updates.iter().map(|p| fixture.generate_update_random(&id_of(p))).collect();
    let post_update_cat_people: Vec<Item> = updated.iter().filter(|p| is_cat_person(p)).cloned().collect();
    let cat_to_cat_count = pre_update_cat_people.iter().filter(|p| post_update_cat_people.iter().any(|pu| id_of(pu) == id_of(p))).count();
    let cat_to_non_count = pre_update_cat_people.len() - cat_to_cat_count;
    let non_to_cat_count = post_update_cat_people.iter().filter(|p| !pre_update_cat_people.iter().any(|pu| id_of(pu) == id_of(p))).count();

    // Act
    fixture.source_cache.add_or_update_items(updated);

    // Assert
    let person_summary = fixture.person_results.summary();
    assert_eq!(person_summary.overall.adds, ADD_COUNT * 2);
    assert_eq!(person_summary.overall.updates, UPDATE_COUNT);
    assert_eq!(fixture.person_results.messages().len(), 3);
    let cat_person_summary = fixture.cat_person_results.summary();
    assert_eq!(cat_person_summary.overall.adds, ADD_COUNT + non_to_cat_count);
    assert_eq!(cat_person_summary.overall.removes, cat_to_non_count);
    assert_eq!(cat_person_summary.overall.updates, cat_to_cat_count);
    assert_eq!(fixture.cat_person_results.messages().len(), 2);
    fixture.check_results();
}
