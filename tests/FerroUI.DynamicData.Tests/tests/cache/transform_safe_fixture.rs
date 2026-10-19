//! `TransformSafeFixture` (`DynamicData.Tests/Cache/TransformSafeFixture.cs`): the transform of a cache whose factory
//! fails for some items, which go to the error handler.

use crate::domain::{Person, PersonWithGender};
use ferroui_dynamic_data::cache::testing::ChangeSetAggregator;
use ferroui_dynamic_data::cache::{IConnectableCache, IObservableCache, ObservableCacheEx, SourceCache};
use ferroui_dynamic_data::kernel::Error;
use ferroui_reactive_testing::new_error;
use ferroui_system::{IDisposable, ObservableError};
use std::cell::RefCell;
use std::rc::Rc;

/// `_transformFactory`: fails for the ages that 3 divides.
fn transform_factory(p: &Rc<Person>) -> Result<Rc<PersonWithGender>, ObservableError> {
    if p.age() % 3 == 0 {
        return Err(new_error(&format!("Cannot transform {p}")));
    }

    let gender = if p.age() % 2 == 0 { "M" } else { "F" };
    Ok(PersonWithGender::from_person(p, gender))
}

struct Fixture {
    errors: Rc<RefCell<Vec<Error<Rc<Person>, String>>>>,
    results: ChangeSetAggregator<Rc<PersonWithGender>, String>,
    source: SourceCache<Rc<Person>, String>,
}

impl Fixture {
    fn new() -> Self {
        let source = SourceCache::new(|p: &Rc<Person>| p.key());
        let errors = Rc::new(RefCell::new(Vec::new()));

        let target = errors.clone();
        let safe_transform = source.connect().transform_safe(transform_factory, move |error| target.borrow_mut().push(error));
        let results = ChangeSetAggregator::new(safe_transform);
        Self { errors, results, source }
    }

    fn add_or_update(&self, people: &[Rc<Person>]) {
        self.source.edit(|updater| updater.add_or_update_items(&mut people.iter().cloned()));
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.source.dispose();
        self.results.dispose();
    }
}

#[test]
fn add_with_error() {
    let fixture = Fixture::new();
    let person = Person::new("Person", 3);
    fixture.add_or_update(&[person]);

    assert_eq!(fixture.errors.borrow().len(), 1, "Should be 1 error reported");
    assert_eq!(fixture.results.messages().len(), 1, "Should be 1 message");
}

#[test]
fn add_with_no_error() {
    let fixture = Fixture::new();
    let person = Person::new("Adult1", 50);
    fixture.add_or_update(&[person.clone()]);

    assert_eq!(fixture.results.messages().len(), 1, "Should be 1 updates");
    assert_eq!(fixture.results.data().count(), 1, "Should be 1 item in the cache");
    assert_eq!(fixture.results.data().items()[0], transform_factory(&person).unwrap(), "Should be same person");
}

#[test]
fn update_batch() {
    let fixture = Fixture::new();
    const KEY: &str = "Adult1";
    let update1 = Person::new(KEY, 1);
    let update2 = Person::new(KEY, 2);
    let update3 = Person::new(KEY, 3);

    fixture.source.edit(|inner_cache| {
        inner_cache.add_or_update(update1);
        inner_cache.add_or_update(update2.clone());
        inner_cache.add_or_update(update3);
    });

    assert_eq!(fixture.errors.borrow().len(), 1, "Should be 1 error reported");
    assert_eq!(fixture.results.messages().len(), 1, "Should be 1 messages");

    assert_eq!(fixture.results.data().count(), 1, "Should 1 item in the cache");
    assert_eq!(fixture.results.data().items()[0], transform_factory(&update2).unwrap(), "Change 2 shoud be the only item cached");
}

#[test]
fn update_batch_and_clear() {
    let fixture = Fixture::new();
    let people: Vec<Rc<Person>> = (1..=100).map(|l| Person::new(format!("Name{l}"), l)).collect();

    fixture.add_or_update(&people);
    fixture.source.edit(|updater| updater.clear());

    assert_eq!(fixture.results.messages().len(), 2, "Should be 2 updates");

    assert_eq!(fixture.errors.borrow().len(), 33, "Should be 33 errors");
    assert_eq!(fixture.results.messages()[0].adds(), 67, "Should be 67 add");
    assert_eq!(fixture.results.messages()[1].removes(), 67, "Should be 67 removes");
    assert_eq!(fixture.results.data().count(), 0, "Should be nothing cached");
}

#[test]
fn update_sucessively() {
    let fixture = Fixture::new();
    const KEY: &str = "Adult1";
    let update1 = Person::new(KEY, 1);
    let update2 = Person::new(KEY, 2);
    let update3 = Person::new(KEY, 3);

    fixture.add_or_update(&[update1]);
    fixture.add_or_update(&[update2.clone()]);
    fixture.add_or_update(&[update3]);

    assert_eq!(fixture.errors.borrow().len(), 1, "Should be 1 error reported");
    assert_eq!(fixture.results.messages().len(), 3, "Should be 3 messages");

    assert_eq!(fixture.results.data().count(), 1, "Should 1 item in the cache");
    assert_eq!(fixture.results.data().items()[0], transform_factory(&update2).unwrap(), "Change 2 shoud be the only item cached");
}
