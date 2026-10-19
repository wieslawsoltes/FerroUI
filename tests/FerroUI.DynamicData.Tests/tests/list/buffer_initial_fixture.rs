//! `BufferInitialFixture` (`DynamicData.Tests/List/BufferInitialFixture.cs`).

use crate::domain::Person;
use ferroui_dynamic_data::list::{IObservableList, ObservableListEx, SourceList, SourceListEditConvenienceEx};
use ferroui_reactive_testing::TestScheduler;
use ferroui_system::IDisposable;
use std::rc::Rc;
use std::time::Duration;

/// The people of the fixture (upstream's static `People`).
fn people() -> Vec<Rc<Person>> {
    (1..=10_000).map(|i| Person::new(i.to_string(), i)).collect()
}

#[test]
fn buffer_initial() {
    let scheduler = TestScheduler::new();

    let cache = SourceList::<Rc<Person>>::new();
    let aggregator = cache.connect().buffer_initial_on(Duration::from_secs(1), scheduler.scheduler()).as_aggregator();
    for item in people() {
        cache.add(item);
    }

    assert_eq!(aggregator.data().count(), 0);
    assert_eq!(aggregator.messages().len(), 0);

    // `TestScheduler.Start()`.
    scheduler.run();

    assert_eq!(aggregator.data().count(), 10_000);
    assert_eq!(aggregator.messages().len(), 1);

    cache.add(Person::new("_New", 1));

    assert_eq!(aggregator.data().count(), 10_001);
    assert_eq!(aggregator.messages().len(), 2);

    aggregator.dispose();
    cache.dispose();
}
