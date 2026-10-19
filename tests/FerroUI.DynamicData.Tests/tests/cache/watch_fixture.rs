//! `WatchFixture` (`DynamicData.Tests/Cache/WatchFixture.cs`): despite its name, the disposal of
//! the items of a cache when they are removed or replaced (`DisposeMany`).

use ferroui_dynamic_data::cache::testing::ChangeSetAggregator;
use ferroui_dynamic_data::cache::{IConnectableCache, IObservableCache, ObservableCacheEx, SourceCache, SourceCacheEx};
use ferroui_system::IDisposable;
use std::cell::Cell;
use std::rc::Rc;

/// `DisposableObject`: an item that records whether it has been disposed.
struct DisposableObject {
    id: i32,
    is_disposed: Cell<bool>,
}

impl DisposableObject {
    fn new(id: i32) -> Rc<Self> {
        Rc::new(Self { id, is_disposed: Cell::new(false) })
    }

    fn id(&self) -> i32 {
        self.id
    }

    fn is_disposed(&self) -> bool {
        self.is_disposed.get()
    }
}

impl IDisposable for DisposableObject {
    fn dispose(&self) {
        self.is_disposed.set(true);
    }
}

/// The fixture (`WatchFixture` with its constructor and `Dispose`).
struct Fixture {
    results: ChangeSetAggregator<Rc<DisposableObject>, i32>,
    source: SourceCache<Rc<DisposableObject>, i32>,
}

impl Fixture {
    fn new() -> Self {
        let source = SourceCache::new(|p: &Rc<DisposableObject>| p.id());
        let results = ChangeSetAggregator::new(source.connect().dispose_many());
        Self { results, source }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.source.dispose();
        self.results.dispose();
    }
}

#[test]
fn add_will_not_call_dispose() {
    let fixture = Fixture::new();

    fixture.source.add_or_update(DisposableObject::new(1));

    assert_eq!(fixture.results.messages().len(), 1, "Should be 1 updates");
    assert_eq!(fixture.results.data().count(), 1, "Should be 1 item in the cache");
    assert!(!fixture.results.data().items()[0].is_disposed(), "Should not be disposed");
}

#[test]
fn everything_is_disposed_when_stream_is_disposed() {
    let fixture = Fixture::new();

    fixture.source.add_or_update_items((1..=10).map(DisposableObject::new));
    fixture.source.clear();

    let messages = fixture.results.messages();
    assert_eq!(messages.len(), 2, "Should be 2 updates");
    assert!(messages[1].iter().all(|d| d.current().is_disposed()));
}

#[test]
fn remove_will_call_dispose() {
    let fixture = Fixture::new();

    fixture.source.add_or_update(DisposableObject::new(1));
    fixture.source.edit(|updater| updater.remove_key(&1));

    let messages = fixture.results.messages();
    assert_eq!(messages.len(), 2, "Should be 2 updates");
    assert_eq!(fixture.results.data().count(), 0, "Should be 0 items in the cache");
    assert!(messages[1].first().expect("a change").current().is_disposed(), "Should be disposed");
}

#[test]
fn update_will_call_dispose() {
    let fixture = Fixture::new();

    fixture.source.add_or_update(DisposableObject::new(1));
    fixture.source.add_or_update(DisposableObject::new(1));

    let messages = fixture.results.messages();
    assert_eq!(messages.len(), 2, "Should be 2 updates");
    assert_eq!(fixture.results.data().count(), 1, "Should be 1 items in the cache");
    let change = messages[1].first().expect("a change");
    assert!(!change.current().is_disposed(), "Current should not be disposed");
    assert!(change.previous().expect("a previous item").is_disposed(), "Previous should be disposed");
}
