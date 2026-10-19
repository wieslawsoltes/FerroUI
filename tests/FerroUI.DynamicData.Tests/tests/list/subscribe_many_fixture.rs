//! `SubscribeManyFixture` (`DynamicData.Tests/List/SubscribeManyFixture.cs`): a subscription per
//! item of a list, made when the item is added and disposed when the item is removed.

use ferroui_dynamic_data::list::testing::ChangeSetAggregator;
use ferroui_dynamic_data::list::{IObservableList, ObservableListEx, SourceList};
use ferroui_reactive::disposables::Disposable;
use ferroui_system::IDisposable;
use std::cell::Cell;
use std::rc::Rc;

/// `SubscribeableObject`: an item that records whether it is subscribed. It has no `Equals`, so
/// it is equal only to itself.
struct SubscribeableObject {
    #[allow(dead_code)]
    id: i32,
    is_subscribed: Cell<bool>,
}

impl SubscribeableObject {
    fn new(id: i32) -> Rc<Self> {
        Rc::new(Self { id, is_subscribed: Cell::new(false) })
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

impl PartialEq for SubscribeableObject {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

type Item = Rc<SubscribeableObject>;

struct Fixture {
    results: ChangeSetAggregator<Item>,
    source: SourceList<Item>,
}

impl Fixture {
    fn new() -> Self {
        let source = SourceList::<Item>::new();
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
    fixture.source.edit(|list| list.add(SubscribeableObject::new(1)));

    assert_eq!(fixture.results.messages().len(), 1, "Should be 1 updates");
    assert_eq!(fixture.results.data().count(), 1, "Should be 1 item in the cache");
    assert!(fixture.results.data().items()[0].is_subscribed(), "Should be subscribed");
}

// Upstream's UpdateUnsubscribesPrevious is commented out (a list has no AddOrUpdate).

#[test]
fn everything_is_unsubscribed_when_stream_is_disposed() {
    let fixture = Fixture::new();
    let items: Vec<Item> = (1..=10).map(SubscribeableObject::new).collect();
    fixture.source.edit(move |list| list.add_range(items));
    fixture.source.edit(|list| list.clear());

    let messages = fixture.results.messages();
    assert_eq!(messages.len(), 2, "Should be 2 updates");

    let items: Vec<Item> = messages[0].iter().flat_map(|x| x.range().iter().cloned()).collect();

    assert!(items.iter().all(|d| !d.is_subscribed()));
}

#[test]
fn remove_is_unsubscribed() {
    let fixture = Fixture::new();
    fixture.source.edit(|list| list.add(SubscribeableObject::new(1)));
    fixture.source.edit(|list| list.remove_at(0));

    let messages = fixture.results.messages();
    assert_eq!(messages.len(), 2, "Should be 2 updates");
    assert_eq!(fixture.results.data().count(), 0, "Should be 0 items in the cache");
    assert!(!messages[1][0].item().current().is_subscribed(), "Should be be unsubscribed");
}
