//! `BatchFixture` (`DynamicData.Tests/List/BatchFixture.cs`): the change sets buffered by time with
//! Rx's `Buffer` and flattened by `FlattenBufferResult`.

use crate::domain::Person;
use ferroui_dynamic_data::list::testing::ChangeSetAggregator;
use ferroui_dynamic_data::list::{ChangeSet, FlattenBufferResultListEx, IObservableList, ObservableListEx, SourceList, SourceListEditConvenienceEx};
use ferroui_reactive::ObservableExt;
use ferroui_reactive_testing::{duration_to_ticks, TestScheduler};
use ferroui_system::{IDisposable, IObservable};
use std::rc::Rc;
use std::time::Duration;

struct Fixture {
    results: ChangeSetAggregator<Rc<Person>>,
    scheduler: TestScheduler,
    source: SourceList<Rc<Person>>,
}

impl Fixture {
    fn new() -> Self {
        let scheduler = TestScheduler::new();
        let source = SourceList::new();
        let buffered: Rc<dyn IObservable<Vec<ChangeSet<Rc<Person>>>>> = source.connect().buffer_time_on(Duration::from_secs(60), scheduler.scheduler());
        let flattened: Rc<dyn IObservable<ChangeSet<Rc<Person>>>> = buffered.flatten_buffer_result();
        let results = flattened.as_aggregator();
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
    fixture.source.add(Person::new("A", 1));
    assert_eq!(fixture.results.messages().len(), 0, "There should be no messages");
}

#[test]
fn results_will_be_received_after_closing_buffer() {
    let fixture = Fixture::new();
    fixture.source.add(Person::new("A", 1));

    // go forward an arbitary amount of time
    fixture.scheduler.advance_by(duration_to_ticks(Duration::from_secs(61)));
    assert_eq!(fixture.results.messages().len(), 1, "Should be 1 update");
}
