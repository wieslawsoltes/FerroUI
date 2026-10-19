//! `SourceUpdaterFixture` (`DynamicData.Tests/Kernal/SourceUpdaterFixture.cs`): the updater of a
//! cache with a key selector.
//!
//! Upstream builds a `CacheUpdater` with the key selector `p => p.Name` over a `ChangeAwareCache`
//! and captures the changes of the cache. `CacheUpdater` is internal to the library here, so the
//! tests run the same updater through the public path that builds it over a `ChangeAwareCache` with
//! the key selector of the cache: an edit of an `ObservableCache::new(key_selector)`
//! (`UpdateFromSource`). The calls that upstream makes before one `CaptureChanges` run in one edit,
//! and the change set that the edit delivers to `connect()` is the captured one; an edit without
//! changes delivers none, which is an empty capture.

use crate::domain::Person;
use crate::utilities::assert_equivalent;
use ferroui_dynamic_data::cache::{Change, ChangeReason, ChangeSet, ICacheUpdater, IConnectableCache, IObservableCache, ISourceUpdater, ObservableCache};
use ferroui_reactive::ObservableExt;
use ferroui_system::IDisposable;
use std::cell::RefCell;
use std::rc::Rc;

/// The cache and the change sets that its edits delivered (upstream's `_cache` and `_updater`).
struct Fixture {
    cache: ObservableCache<Rc<Person>, String>,
    delivered: Rc<RefCell<Vec<ChangeSet<Rc<Person>, String>>>>,
    subscription: Rc<dyn IDisposable>,
}

impl Fixture {
    fn new() -> Self {
        let cache = ObservableCache::new(|p: &Rc<Person>| p.name());
        let delivered = Rc::new(RefCell::new(Vec::new()));
        let sink = delivered.clone();
        let subscription = cache.connect().subscribe_next(move |changes: ChangeSet<Rc<Person>, String>| sink.borrow_mut().push(changes));
        Self { cache, delivered, subscription }
    }

    /// Runs the calls of the updater as one edit.
    fn update(&self, update: impl FnOnce(&mut dyn ISourceUpdater<Rc<Person>, String>)) {
        self.cache.update_from_source(update);
    }

    /// The changes since the last capture (`_cache.CaptureChanges()`).
    fn capture_changes(&self) -> ChangeSet<Rc<Person>, String> {
        let delivered = std::mem::take(&mut *self.delivered.borrow_mut());
        delivered.iter().flat_map(|changes| changes.iter().cloned()).collect()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.subscription.dispose();
    }
}

fn count_of(updates: &ChangeSet<Rc<Person>, String>, reason: ChangeReason) -> usize {
    updates.iter().filter(|update| update.reason() == reason).count()
}

fn people() -> Vec<Rc<Person>> {
    (1..=100).map(|i| Person::new(format!("Name{i}"), i)).collect()
}

#[test]
fn add() {
    let fixture = Fixture::new();
    let person = Person::new("Adult1", 50);
    fixture.update(|updater| updater.add_or_update(person.clone()));
    let updates = fixture.capture_changes();

    assert_eq!(fixture.cache.lookup(&"Adult1".to_string()), Some(person.clone()));
    assert_eq!(fixture.cache.count(), 1);
    assert_eq!(updates.count(), 1);
    assert_eq!(updates[0], Change::new(ChangeReason::Add, person.name(), person.clone()), "Should be 1 updates");
}

#[test]
fn attempted_removal_of_a_non_existent_key_will_be_ignored() {
    let fixture = Fixture::new();
    let key = "Adult1".to_string();

    fixture.update(|updater| ICacheUpdater::remove_key(updater, &key));
    let updates = fixture.capture_changes();

    assert_eq!(fixture.cache.count(), 0);
    assert_eq!(updates.count(), 0, "Should be 0 updates");
}

#[test]
fn batch_of_unique_updates() {
    let fixture = Fixture::new();
    let people = people();
    fixture.update(|updater| updater.add_or_update_items(&mut people.iter().cloned()));
    let updates = fixture.capture_changes();

    assert_equivalent(&fixture.cache.items(), &people);
    assert_eq!(fixture.cache.count(), 100);
    assert_eq!(updates.adds(), 100);
    assert_eq!(updates.count(), 100);
}

#[test]
fn batch_removes() {
    let fixture = Fixture::new();
    let people = people();
    fixture.update(|updater| {
        updater.add_or_update_items(&mut people.iter().cloned());
        updater.remove_items(&mut people.iter().cloned());
    });
    let updates = fixture.capture_changes();

    assert_eq!(fixture.cache.count(), 0, "Everything should be removed");
    assert_eq!(count_of(&updates, ChangeReason::Add), 100);
    assert_eq!(count_of(&updates, ChangeReason::Remove), 100);
    assert_eq!(updates.count(), 200);
}

#[test]
fn batch_successive_updates() {
    let fixture = Fixture::new();
    let people: Vec<Rc<Person>> = (1..=100).map(|i| Person::new("Name1", i)).collect();
    fixture.update(|updater| updater.add_or_update_items(&mut people.iter().cloned()));

    let updates = fixture.capture_changes();

    assert_eq!(fixture.cache.lookup(&"Name1".to_string()).map(|person| person.age()), Some(100));
    assert_eq!(fixture.cache.count(), 1, "Successive updates should replace cache value");
    assert_eq!(count_of(&updates, ChangeReason::Update), 99);
    assert_eq!(count_of(&updates, ChangeReason::Add), 1);
    assert_eq!(updates.count(), 100);
}

#[test]
fn can_remove() {
    let fixture = Fixture::new();
    let key = "Adult1";

    let person = Person::new(key, 50);
    fixture.update(|updater| {
        updater.add_or_update(person.clone());
        updater.remove_item(&person);
    });
    let updates = fixture.capture_changes();

    assert_eq!(fixture.cache.count(), 0);
    assert_eq!(count_of(&updates, ChangeReason::Add), 1);
    assert_eq!(count_of(&updates, ChangeReason::Remove), 1);
    assert_eq!(updates.count(), 2);
}

#[test]
fn can_update() {
    let fixture = Fixture::new();
    let key = "Adult1".to_string();

    let newperson = Person::new(key.clone(), 50);
    let updated = Person::new(key.clone(), 51);
    fixture.update(|updater| {
        updater.add_or_update(newperson.clone());
        updater.add_or_update(updated.clone());
    });
    let updates = fixture.capture_changes();

    assert_eq!(fixture.cache.lookup(&key), Some(updated.clone()));
    assert_eq!(fixture.cache.lookup(&key).map(|person| person.age()), Some(51));
    assert_eq!(fixture.cache.count(), 1);
    assert_eq!(count_of(&updates, ChangeReason::Add), 1);
    assert_eq!(count_of(&updates, ChangeReason::Update), 1);
    assert_eq!(updates.count(), 2);
}

#[test]
fn clear() {
    let fixture = Fixture::new();
    let people = people();
    fixture.update(|updater| {
        updater.add_or_update_items(&mut people.iter().cloned());
        ICacheUpdater::clear(updater);
    });
    let updates = fixture.capture_changes();

    assert_eq!(fixture.cache.count(), 0, "Everything should be removed");
    assert_eq!(count_of(&updates, ChangeReason::Add), 100);
    assert_eq!(count_of(&updates, ChangeReason::Remove), 100);
    assert_eq!(updates.count(), 200);
}

// Not ported: NullSelectorWillThrow - argument validation (the body of the upstream test is commented out)
