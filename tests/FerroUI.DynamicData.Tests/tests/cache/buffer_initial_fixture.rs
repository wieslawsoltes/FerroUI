//! `BufferInitialFixture` (`DynamicData.Tests/Cache/BufferInitialFixture.cs`): the change sets of
//! the first window delivered as one, then as they come (`BufferInitial`).

use crate::domain::Person;
use ferroui_dynamic_data::cache::{IConnectableCache, IObservableCache, ObservableCacheEx, SourceCache, SourceCacheEx};
use ferroui_reactive_testing::TestScheduler;
use ferroui_system::IDisposable;
use std::rc::Rc;
use std::time::Duration;

/// `People`: 10 000 people named by their number.
fn people() -> Vec<Rc<Person>> {
    (1..=10_000).map(|i: i32| Person::new(i.to_string(), i)).collect()
}

#[test]
fn buffer_initial() {
    let scheduler = TestScheduler::new();

    let cache = SourceCache::new(|i: &Rc<Person>| i.name());
    let aggregator = cache.connect().buffer_initial_on(Duration::from_secs(1), scheduler.scheduler()).as_aggregator();
    for item in people() {
        cache.add_or_update(item);
    }

    assert_eq!(aggregator.data().count(), 0);
    assert_eq!(aggregator.messages().len(), 0);

    scheduler.run();

    assert_eq!(aggregator.data().count(), 10_000);
    assert_eq!(aggregator.messages().len(), 1);

    cache.add_or_update(Person::new("_New", 1));

    assert_eq!(aggregator.data().count(), 10_001);
    assert_eq!(aggregator.messages().len(), 2);

    aggregator.dispose();
    cache.dispose();
}
