//! `TransformFixture` (`DynamicData.Tests/Cache/TransformFixture.cs`): the transform of a cache,
//! and its forced transforms.

use crate::domain::{Person, PersonWithGender};
use crate::utilities::assert_equivalent;
use ferroui_dynamic_data::cache::testing::ChangeSetAggregator;
use ferroui_dynamic_data::cache::{IConnectableCache, IObservableCache, ObservableCacheEx, SourceCache};
use ferroui_reactive::subjects::Subject;
use ferroui_system::{IDisposable, IObservable, IObserver};
use std::rc::Rc;

type PersonPredicate = Rc<dyn Fn(&Rc<Person>) -> bool>;

/// `TransformStub.TransformFactory`.
fn transform_factory(p: &Rc<Person>) -> Rc<PersonWithGender> {
    PersonWithGender::from_person(p, if p.age() % 2 == 0 { "M" } else { "F" })
}

/// `TransformStub`.
struct TransformStub {
    results: ChangeSetAggregator<Rc<PersonWithGender>, String>,
    source: SourceCache<Rc<Person>, String>,
}

impl TransformStub {
    fn new() -> Self {
        let source = SourceCache::new(|p: &Rc<Person>| p.name());
        let results = source.connect().transform(transform_factory).as_aggregator();
        Self { results, source }
    }

    fn with_retransform_all(retransformer: Rc<dyn IObservable<()>>) -> Self {
        let source = SourceCache::new(|p: &Rc<Person>| p.name());
        let results = source.connect().transform_forced_all(transform_factory, retransformer).as_aggregator();
        Self { results, source }
    }

    fn with_retransform(retransformer: Rc<dyn IObservable<PersonPredicate>>) -> Self {
        let source = SourceCache::new(|p: &Rc<Person>| p.name());
        let results = source.connect().transform_forced(transform_factory, retransformer).as_aggregator();
        Self { results, source }
    }

    fn add_or_update(&self, people: &[Rc<Person>]) {
        self.source.edit(|updater| updater.add_or_update_items(&mut people.iter().cloned()));
    }
}

impl Drop for TransformStub {
    fn drop(&mut self) {
        self.source.dispose();
        self.results.dispose();
    }
}

fn people(range: std::ops::RangeInclusive<i32>) -> Vec<Rc<Person>> {
    range.map(|i| Person::new(format!("Name{i}"), i)).collect()
}

#[test]
fn add() {
    let stub = TransformStub::new();
    let person = Person::new("Adult1", 50);
    stub.add_or_update(&[person.clone()]);

    assert_eq!(stub.results.messages().len(), 1, "Should be 1 updates");
    assert_eq!(stub.results.data().count(), 1, "Should be 1 item in the cache");
    assert_eq!(stub.results.data().items()[0], transform_factory(&person), "Should be same person");
}

#[test]
fn batch_of_unique_updates() {
    let people = people(1..=100);
    let stub = TransformStub::new();
    stub.add_or_update(&people);

    assert_eq!(stub.results.messages().len(), 1, "Should be 1 updates");
    assert_eq!(stub.results.messages()[0].adds(), 100, "Should return 100 adds");

    let mut transformed: Vec<Rc<PersonWithGender>> = people.iter().map(transform_factory).collect();
    transformed.sort_by_key(|p| p.age());
    let mut items = stub.results.data().items();
    items.sort_by_key(|p| p.age());
    assert_equivalent(&items, &transformed);
}

#[test]
fn clear() {
    let stub = TransformStub::new();
    let people = people(1..=100);

    stub.add_or_update(&people);
    stub.source.edit(|updater| updater.clear());

    assert_eq!(stub.results.messages().len(), 2, "Should be 2 updates");
    assert_eq!(stub.results.messages()[0].adds(), 100, "Should be 80 addes");
    assert_eq!(stub.results.messages()[1].removes(), 100, "Should be 80 removes");
    assert_eq!(stub.results.data().count(), 0, "Should be nothing cached");
}

#[test]
fn remove() {
    const KEY: &str = "Adult1";
    let person = Person::new(KEY, 50);

    let stub = TransformStub::new();
    stub.add_or_update(&[person]);
    stub.source.edit(|updater| updater.remove_key(&KEY.to_owned()));

    assert_eq!(stub.results.messages().len(), 2, "Should be 2 updates");
    assert_eq!(stub.results.messages()[0].adds(), 1, "Should be 80 addes");
    assert_eq!(stub.results.messages()[1].removes(), 1, "Should be 80 removes");
    assert_eq!(stub.results.data().count(), 0, "Should be nothing cached");
}

#[test]
fn re_transform_all() {
    let people = people(1..=10);
    let force_transform = Subject::<()>::new();

    let stub = TransformStub::with_retransform_all(force_transform.as_observable());
    stub.add_or_update(&people);
    force_transform.on_next(());

    let messages = stub.results.messages();
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[1].updates(), 10);

    for i in 1..=10 {
        let original = messages[0][i - 1].current();
        let updated = messages[1][i - 1].current();

        assert_eq!(updated, original);
        assert!(!Rc::ptr_eq(original, updated));
    }
}

#[test]
fn re_transform_selected() {
    let people = people(1..=10);
    let force_transform = Subject::<PersonPredicate>::new();

    let stub = TransformStub::with_retransform(force_transform.as_observable());
    stub.add_or_update(&people);
    force_transform.on_next(Rc::new(|person: &Rc<Person>| person.age() <= 5));

    let messages = stub.results.messages();
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[1].updates(), 5);

    for i in 1..=5 {
        let original = messages[0][i - 1].current();
        let updated = messages[1][i - 1].current();
        assert_eq!(updated, original);
        assert!(!Rc::ptr_eq(original, updated));
    }
}

#[test]
fn same_key_changes() {
    let stub = TransformStub::new();
    let people: Vec<Rc<Person>> = (1..=10).map(|i| Person::new("Name", i)).collect();

    stub.add_or_update(&people);

    assert_eq!(stub.results.messages().len(), 1, "Should be 1 updates");
    assert_eq!(stub.results.messages()[0].adds(), 1, "Should return 1 adds");
    assert_eq!(stub.results.messages()[0].updates(), 9, "Should return 9 adds");
    assert_eq!(stub.results.data().count(), 1, "Should result in 1 record");

    let last_transformed = transform_factory(people.last().unwrap());
    let only_item_in_cache = stub.results.data().items()[0].clone();

    assert_eq!(only_item_in_cache, last_transformed, "Incorrect transform result");
}

#[test]
fn update() {
    const KEY: &str = "Adult1";
    let newperson = Person::new(KEY, 50);
    let updated = Person::new(KEY, 51);

    let stub = TransformStub::new();
    stub.add_or_update(&[newperson]);
    stub.add_or_update(&[updated]);

    assert_eq!(stub.results.messages().len(), 2, "Should be 2 updates");
    assert_eq!(stub.results.messages()[0].adds(), 1, "Should be 1 adds");
    assert_eq!(stub.results.messages()[1].updates(), 1, "Should be 1 update");
}
