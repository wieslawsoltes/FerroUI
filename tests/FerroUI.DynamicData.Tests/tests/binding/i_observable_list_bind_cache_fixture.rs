//! `IObservableListBindCacheFixture` (`DynamicData.Tests/Binding/IObservableListBindCacheFixture.cs`).

use crate::domain::{Person, RandomPersonGenerator};
use crate::utilities::assert_equivalent;
use ferroui_dynamic_data::cache::testing::ChangeSetAggregator as CacheChangeSetAggregator;
use ferroui_dynamic_data::list::testing::ChangeSetAggregator;
use ferroui_dynamic_data::list::{IObservableList, ListChangeReason, ObservableListEx};
use ferroui_dynamic_data::{IConnectableCache, ObservableCacheEx, SourceCache};
use ferroui_system::IDisposable;
use std::rc::Rc;

struct Fixture {
    generator: RandomPersonGenerator,
    list: Rc<dyn IObservableList<Rc<Person>>>,
    list_notifications: ChangeSetAggregator<Rc<Person>>,
    source: SourceCache<Rc<Person>, String>,
    source_cache_notifications: CacheChangeSetAggregator<Rc<Person>, String>,
}

impl Fixture {
    fn new() -> Self {
        let source = SourceCache::new(|p: &Rc<Person>| p.name());
        let (bound, list) = source.connect().auto_refresh_any().bind_to_observable_list();
        let source_cache_notifications = bound.as_aggregator();
        let list_notifications = list.connect().as_aggregator();
        Self { generator: RandomPersonGenerator::new(), list, list_notifications, source, source_cache_notifications }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.source_cache_notifications.dispose();
        self.list_notifications.dispose();
        self.source.dispose();
    }
}

fn add_or_update(source: &SourceCache<Rc<Person>, String>, people: &[Rc<Person>]) {
    source.edit(|updater| updater.add_or_update_items(&mut people.iter().cloned()));
}

#[test]
fn add_to_source_adds_to_destination() {
    let fixture = Fixture::new();
    let person = Person::new("Adult1", 50);
    add_or_update(&fixture.source, std::slice::from_ref(&person));

    assert_eq!(fixture.list.count(), 1, "Should be 1 item in the collection");
    assert_eq!(fixture.list.items()[0], person, "Should be same person");
}

#[test]
fn batch_add() {
    let fixture = Fixture::new();
    let people = fixture.generator.take(100);
    add_or_update(&fixture.source, &people);

    assert_eq!(fixture.list.count(), 100, "Should be 100 items in the collection");
    assert_equivalent(&fixture.list.items(), &fixture.list.items());
}

#[test]
fn batch_remove() {
    let fixture = Fixture::new();
    let people = fixture.generator.take(100);
    add_or_update(&fixture.source, &people);
    fixture.source.edit(|updater| updater.clear());
    assert_eq!(fixture.list.count(), 0, "Should be 100 items in the collection");
}

#[test]
fn list_recieves_refresh() {
    let fixture = Fixture::new();
    let person = Person::new("Adult1", 50);
    add_or_update(&fixture.source, std::slice::from_ref(&person));

    person.set_age(60);

    assert_eq!(fixture.list_notifications.messages().len(), 2);
    assert_eq!(fixture.list_notifications.messages().last().expect("a refresh")[0].reason(), ListChangeReason::Refresh);
}

#[test]
fn remove_source_removes_from_the_destination() {
    let fixture = Fixture::new();
    let person = Person::new("Adult1", 50);
    add_or_update(&fixture.source, std::slice::from_ref(&person));
    fixture.source.edit(|updater| updater.remove_item(&person));

    assert_eq!(fixture.list.count(), 0, "Should be 1 item in the collection");
}

#[test]
fn update_to_source_updates_the_destination() {
    let fixture = Fixture::new();
    let person = Person::new("Adult1", 50);
    let person_updated = Person::new("Adult1", 51);
    add_or_update(&fixture.source, std::slice::from_ref(&person));
    add_or_update(&fixture.source, std::slice::from_ref(&person_updated));

    assert_eq!(fixture.list.count(), 1, "Should be 1 item in the collection");
    assert!(Rc::ptr_eq(&fixture.list.items()[0], &person_updated), "Should be updated person");
}
