//! `BatchFixture` (`DynamicData.Tests/Cache/BatchFixture.cs`): the change sets of a window of time
//! delivered as one (`Batch`).

use crate::domain::Person;
use ferroui_dynamic_data::cache::testing::ChangeSetAggregator;
use ferroui_dynamic_data::cache::{IConnectableCache, ObservableCacheEx, SourceCache, SourceCacheEx};
use ferroui_reactive_testing::{duration_to_ticks, TestScheduler};
use ferroui_system::IDisposable;
use std::rc::Rc;
use std::time::Duration;

/// The fixture (`BatchFixture` with its constructor and `Dispose`).
struct Fixture {
    results: ChangeSetAggregator<Rc<Person>, String>,
    scheduler: TestScheduler,
    source: SourceCache<Rc<Person>, String>,
}

impl Fixture {
    fn new() -> Self {
        let scheduler = TestScheduler::new();
        let source = SourceCache::new(|p: &Rc<Person>| p.key());
        let results = source.connect().batch_on(Duration::from_secs(60), scheduler.scheduler()).as_aggregator();
        Self { results, scheduler, source }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.results.dispose();
        self.source.dispose();
    }
}

#[test]
fn no_results_will_be_received_before_closing_buffer() {
    let fixture = Fixture::new();

    fixture.source.add_or_update(Person::new("A", 1));
    assert_eq!(fixture.results.messages().len(), 0, "There should be no messages");
}

#[test]
fn results_will_be_received_after_closing_buffer() {
    let fixture = Fixture::new();

    fixture.source.add_or_update(Person::new("A", 1));

    // go forward an arbitary amount of time
    fixture.scheduler.advance_by(duration_to_ticks(Duration::from_secs(61)));
    assert_eq!(fixture.results.messages().len(), 1, "Should be 1 update");
}
