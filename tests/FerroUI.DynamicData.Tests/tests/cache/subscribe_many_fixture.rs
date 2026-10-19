//! `SubscribeManyFixture` (`DynamicData.Tests/Cache/SubscribeManyFixture.cs`): a subscription per
//! item of a cache, made when the item is added and disposed when the item is removed or replaced.

use ferroui_dynamic_data::cache::testing::ChangeSetAggregator;
use ferroui_dynamic_data::cache::{IConnectableCache, IObservableCache, ObservableCacheEx, SourceCache};
use ferroui_reactive::disposables::Disposable;
use ferroui_system::IDisposable;
use std::cell::Cell;
use std::rc::Rc;

/// `SubscribeableObject`: an item that records whether it is subscribed.
struct SubscribeableObject {
    id: i32,
    is_subscribed: Cell<bool>,
}

impl SubscribeableObject {
    fn new(id: i32) -> Rc<Self> {
        Rc::new(Self { id, is_subscribed: Cell::new(false) })
    }

    fn id(&self) -> i32 {
        self.id
    }

    fn is_subscribed(&self) -> bool {
        self.is_subscribed.get()
    }

    fn subscribe(&self) {
        self.is_subscribed.set(true);
    }

    fn un_subscribe(&self) {
        self.is_subscribed.set(false);
    }
}

type Item = Rc<SubscribeableObject>;

struct Fixture {
    results: ChangeSetAggregator<Item, i32>,
    source: SourceCache<Item, i32>,
}

impl Fixture {
    fn new() -> Self {
        let source = SourceCache::new(|p: &Item| p.id());
        let results = source
            .connect()
            .subscribe_many(|subscribeable: &Item| {
                subscribeable.subscribe();
                let subscribeable = subscribeable.clone();
                Disposable::create(move || subscribeable.un_subscribe())
            })
            .as_aggregator();
        Self { results, source }
    }

    /// `_source.AddOrUpdate(item)`.
    fn add_or_update(&self, item: Item) {
        self.source.edit(move |updater| updater.add_or_update(item));
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.source.dispose();
        self.results.dispose();
    }
}

#[test]
fn added_item_willbe_subscribed() {
    let fixture = Fixture::new();
    fixture.add_or_update(SubscribeableObject::new(1));

    assert_eq!(fixture.results.messages().len(), 1, "Should be 1 updates");
    assert_eq!(fixture.results.data().count(), 1, "Should be 1 item in the cache");
    assert!(fixture.results.data().items()[0].is_subscribed(), "Should be subscribed");
}

#[test]
fn everything_is_unsubscribed_when_stream_is_disposed() {
    let fixture = Fixture::new();
    let items: Vec<Item> = (1..=10).map(SubscribeableObject::new).collect();
    fixture.source.edit(move |updater| updater.add_or_update_items(&mut items.into_iter()));
    fixture.source.edit(|updater| updater.clear());

    let messages = fixture.results.messages();
    assert_eq!(messages.len(), 2, "Should be 2 updates");
    assert!(messages[1].iter().all(|d| !d.current().is_subscribed()));
}

#[test]
fn remove_is_unsubscribed() {
    let fixture = Fixture::new();
    fixture.add_or_update(SubscribeableObject::new(1));
    fixture.source.edit(|updater| updater.remove_key(&1));

    let messages = fixture.results.messages();
    assert_eq!(messages.len(), 2, "Should be 2 updates");
    assert_eq!(fixture.results.data().count(), 0, "Should be 0 items in the cache");
    assert!(!messages[1][0].current().is_subscribed(), "Should be be unsubscribed");
}

#[test]
fn update_unsubscribes_previous() {
    let fixture = Fixture::new();
    fixture.add_or_update(SubscribeableObject::new(1));
    fixture.add_or_update(SubscribeableObject::new(1));

    let messages = fixture.results.messages();
    assert_eq!(messages.len(), 2, "Should be 2 updates");
    assert_eq!(fixture.results.data().count(), 1, "Should be 1 items in the cache");
    assert!(messages[1][0].current().is_subscribed(), "Current should be subscribed");
    let previous = messages[1][0].previous().expect("the update has a previous item");
    assert!(!previous.is_subscribed(), "Previous should not be subscribed");
}
