//! `BatchIfFixture` (`DynamicData.Tests/List/BatchIfFixture.cs`): `BufferIf` with a pausing
//! subject, its signals observed on a test scheduler.

use crate::domain::Person;
use ferroui_dynamic_data::list::testing::ChangeSetAggregator;
use ferroui_dynamic_data::list::{IObservableList, ObservableListEx, SourceList, SourceListEditConvenienceEx};
use ferroui_reactive::subjects::Subject;
use ferroui_reactive::ObservableExt;
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
        let results = source.connect().buffer_if_on(pausing_subject.as_observable(), scheduler.scheduler()).as_aggregator();
        Self { pausing_subject, results, scheduler, source }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.results.dispose();
        self.source.dispose();
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
    fixture.scheduler.advance_by(duration_to_ticks(Duration::from_secs(60)));
    assert_eq!(fixture.results.messages().len(), 0, "There should be no messages");

    fixture.pausing_subject.on_next(false);
    fixture.scheduler.advance_by(duration_to_ticks(Duration::from_millis(10)));
    fixture.source.add(Person::new("B", 1));

    assert_eq!(fixture.results.messages().len(), 2, "There should be no messages");
}

/// Test case to prove the issue and fix to DynamicData GitHub issue #275 - BufferIf race condition.
#[test]
fn changes_not_lost_if_consumer_is_running_on_different_thread() {
    let fixture = Fixture::new();
    let producer_scheduler = TestScheduler::new();
    let consumer_scheduler = TestScheduler::new();

    // Note consumer is running on a different scheduler
    let (bound, target) = fixture
        .source
        .connect()
        .buffer_if_on(fixture.pausing_subject.as_observable(), producer_scheduler.scheduler())
        .observe_on(consumer_scheduler.scheduler())
        .bind_read_only();
    let _aggregator = bound.as_aggregator();

    fixture.source.add(Person::new("A", 1));

    producer_scheduler.advance_by(1);
    consumer_scheduler.advance_by(1);

    assert_eq!(target.count(), 1, "There should be 1 message");

    fixture.pausing_subject.on_next(true);

    producer_scheduler.advance_by(1);
    consumer_scheduler.advance_by(1);

    fixture.source.add(Person::new("B", 2));

    producer_scheduler.advance_by(1);
    consumer_scheduler.advance_by(1);

    assert_eq!(target.count(), 1, "There should be 1 message");

    fixture.pausing_subject.on_next(false);

    producer_scheduler.advance_by(1);

    // Target doesnt get the messages until its scheduler runs, but the
    // messages shouldnt be lost
    assert_eq!(target.count(), 1, "There should be 1 message");

    consumer_scheduler.advance_by(1);

    assert_eq!(target.count(), 2, "There should be 2 message");
}

#[test]
fn no_results_will_be_received_if_paused() {
    let fixture = Fixture::new();
    fixture.pausing_subject.on_next(true);
    // advance otherwise nothing happens
    fixture.scheduler.advance_by(duration_to_ticks(Duration::from_millis(10)));

    fixture.source.add(Person::new("A", 1));

    // go forward an arbitary amount of time
    fixture.scheduler.advance_by(duration_to_ticks(Duration::from_secs(60)));
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
