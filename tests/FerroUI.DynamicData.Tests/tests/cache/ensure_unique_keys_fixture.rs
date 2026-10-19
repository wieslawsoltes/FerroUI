//! `EnsureUniqueKeysFixture` (`DynamicData.Tests/Cache/EnsureUniqueKeysFixture.cs`): one change
//! per key in each change set (`EnsureUniqueKeys`).

use crate::domain::Person;
use ferroui_dynamic_data::cache::testing::ChangeSetAggregator;
use ferroui_dynamic_data::cache::{ChangeReason, IConnectableCache, ObservableCacheEx, SourceCache, SourceCacheEx};
use ferroui_system::IDisposable;
use std::rc::Rc;

/// The fixture (`EnsureUniqueKeysFixture` with its constructor and `Dispose`).
struct Fixture {
    source: SourceCache<Rc<Person>, String>,
    results: ChangeSetAggregator<Rc<Person>, String>,
}

impl Fixture {
    fn new() -> Self {
        let source = SourceCache::new(|p: &Rc<Person>| p.name());
        let results = source.connect_with(None, false).ensure_unique_keys().as_aggregator();
        Self { source, results }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.source.dispose();
        self.results.dispose();
    }
}

fn me() -> String {
    "Me".to_owned()
}

#[test]
fn unique_for_adds() {
    let fixture = Fixture::new();

    fixture.source.edit(|inner_cache| {
        inner_cache.add_or_update(Person::new("Me", 20));
        inner_cache.add_or_update(Person::new("Me", 21));
        inner_cache.add_or_update(Person::new("Me", 22));
    });

    let message1 = fixture.results.messages()[0].clone();
    assert_eq!(message1.count(), 1);
    let first = message1.first().expect("a change");
    assert_eq!(first.current().age(), 22);
    assert_eq!(first.reason(), ChangeReason::Add);
}

#[test]
fn add_and_remove() {
    let fixture = Fixture::new();

    fixture.source.edit(|inner_cache| {
        inner_cache.add_or_update(Person::new("Me", 20));
        inner_cache.add_or_update(Person::new("Me", 21));
        inner_cache.remove_key(&me());
    });

    let message1 = fixture.results.messages()[0].clone();
    assert_eq!(message1.count(), 0);
}

#[test]
fn refresh() {
    let fixture = Fixture::new();

    fixture.source.add_or_update(Person::new("Me", 20));

    fixture.source.edit(|inner_cache| {
        inner_cache.refresh_key(&me());
    });

    let message1 = fixture.results.messages()[1].clone();
    assert_eq!(message1.count(), 1);
    let first = message1.first().expect("a change");
    assert_eq!(first.current().age(), 20);
    assert_eq!(first.reason(), ChangeReason::Refresh);
}

#[test]
fn compound_refresh1() {
    let fixture = Fixture::new();

    fixture.source.edit(|inner_cache| {
        // A nested edit of the same cache, as upstream does.
        fixture.source.add_or_update(Person::new("Me", 20));
        inner_cache.refresh_key(&me());
    });

    let message1 = fixture.results.messages()[0].clone();
    assert_eq!(message1.count(), 1);
    let first = message1.first().expect("a change");
    assert_eq!(first.current().age(), 20);
    assert_eq!(first.reason(), ChangeReason::Add);
}

#[test]
fn compound_refresh2() {
    let fixture = Fixture::new();

    fixture.source.edit(|inner_cache| {
        inner_cache.add_or_update(Person::new("Me", 20));
        inner_cache.add_or_update(Person::new("Me", 21));
        inner_cache.refresh_key(&me());
        inner_cache.refresh_key(&me());
    });

    let message1 = fixture.results.messages()[0].clone();
    assert_eq!(message1.count(), 1);
    let first = message1.first().expect("a change");
    assert_eq!(first.current().age(), 21);
    assert_eq!(first.reason(), ChangeReason::Add);
}

#[test]
fn compound_refresh3() {
    let fixture = Fixture::new();

    fixture.source.add_or_update(Person::new("Me", 20));

    fixture.source.edit(|inner_cache| {
        inner_cache.refresh_key(&me());
        inner_cache.refresh_key(&me());
        inner_cache.refresh_key(&me());
    });

    let message1 = fixture.results.messages()[1].clone();
    assert_eq!(message1.count(), 1);
    let first = message1.first().expect("a change");
    assert_eq!(first.current().age(), 20);
    assert_eq!(first.reason(), ChangeReason::Refresh);
}
