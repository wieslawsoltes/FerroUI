//! `BatchIfWithTimeOutFixture` (`DynamicData.Tests/List/BatchIfWithTimeOutFixture.cs`): `BufferIf`
//! with a pausing subject and a time-out of one minute, on a test scheduler.

use crate::domain::Person;
use ferroui_dynamic_data::list::testing::ChangeSetAggregator;
use ferroui_dynamic_data::list::{IObservableList, ObservableListEx, SourceList, SourceListEditConvenienceEx};
use ferroui_reactive::subjects::Subject;
use ferroui_reactive_testing::{duration_to_ticks, TestScheduler};
use ferroui_system::{IDisposable, IObserver};
use std::rc::Rc;
use std::time::Duration;

struct Fixture {
    pausing_subject: Rc<Subject<bool>>,
    results: ChangeSetAggregator<Rc<Person>>,
    scheduler: TestScheduler,
    source: SourceList<Rc<Person>>,
}

impl Fixture {
    fn new() -> Self {
        let pausing_subject = Subject::<bool>::new();
        let scheduler = TestScheduler::new();
        let source = SourceList::new();
        let results = source
            .connect()
            .buffer_if_with_time_out_on(pausing_subject.as_observable(), Some(Duration::from_secs(60)), scheduler.scheduler())
            .as_aggregator();
        Self { pausing_subject, results, scheduler, source }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.results.dispose();
        self.source.dispose();
        self.pausing_subject.on_completed();
    }
}

#[test]
fn can_toggle_suspend_resume() {
    let fixture = Fixture::new();
    fixture.pausing_subject.on_next(true);
    // advance otherwise nothing happens
    fixture.scheduler.advance_by(duration_to_ticks(Duration::from_millis(10)));

    fixture.source.add(Person::new("A", 1));

    // go forward an arbitary amount of time
    assert_eq!(fixture.results.messages().len(), 0, "There should be no messages");

    fixture.pausing_subject.on_next(false);
    fixture.scheduler.advance_by(duration_to_ticks(Duration::from_millis(10)));

    fixture.source.add(Person::new("B", 1));

    assert_eq!(fixture.results.messages().len(), 2, "There should be no messages");
}

#[test]
fn no_results_will_be_received_if_paused() {
    let fixture = Fixture::new();
    fixture.pausing_subject.on_next(true);
    // advance otherwise nothing happens
    fixture.scheduler.advance_by(duration_to_ticks(Duration::from_millis(10)));

    fixture.source.add(Person::new("A", 1));

    assert_eq!(fixture.results.messages().len(), 0, "There should be no messages");
}

#[test]
fn results_will_be_received_if_not_paused() {
    let fixture = Fixture::new();
    fixture.source.add(Person::new("A", 1));

    // go forward an arbitary amount of time
    fixture.scheduler.advance_by(duration_to_ticks(Duration::from_secs(60)));
    assert_eq!(fixture.results.messages().len(), 1, "Should be 1 update");
}

#[test]
fn will_apply_timeout() {
    let fixture = Fixture::new();
    fixture.pausing_subject.on_next(true);

    // should timeout
    fixture.scheduler.advance_by(duration_to_ticks(Duration::from_secs(61)));

    fixture.source.add(Person::new("A", 1));

    // go forward an arbitary amount of time
    assert_eq!(fixture.results.messages().len(), 1, "There should be no messages");
}
