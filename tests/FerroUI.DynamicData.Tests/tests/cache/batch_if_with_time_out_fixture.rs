//! `BatchIfWithTimeOutFixture` (`DynamicData.Tests/Cache/BatchIfWithTimeOutFixture.cs`): `BatchIf`
//! with an initial pause state, a time-out and an interval timer. The upstream file holds two
//! classes: `BatchIfWithTimeoutFixture` (the tests `initial_pause` and `timeout`) and
//! `BatchIfWithTimeOutFixture` (the others, on [`Fixture`]).

use crate::domain::Person;
use ferroui_dynamic_data::cache::testing::ChangeSetAggregator;
use ferroui_dynamic_data::cache::{IConnectableCache, IObservableCache, ObservableCacheEx, SourceCache, SourceCacheEx};
use ferroui_reactive::subjects::Subject;
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_reactive_testing::{duration_to_ticks, TestScheduler};
use ferroui_system::{IDisposable, IObservable, IObserver};
use std::rc::Rc;
use std::time::Duration;

fn ticks(duration: Duration) -> i64 {
    duration_to_ticks(duration)
}

// --- BatchIfWithTimeoutFixture ---

/// The fixture of `BatchIfWithTimeoutFixture`.
struct TimeoutFixture {
    scheduler: TestScheduler,
    source: SourceCache<Rc<Person>, String>,
}

impl TimeoutFixture {
    fn new() -> Self {
        Self { scheduler: TestScheduler::new(), source: SourceCache::new(|p: &Rc<Person>| p.key()) }
    }
}

impl Drop for TimeoutFixture {
    fn drop(&mut self) {
        self.source.dispose();
    }
}

#[test]
fn initial_pause() {
    let fixture = TimeoutFixture::new();

    let pausing_subject = Subject::<bool>::new();
    let results = fixture.source.connect().batch_if_with_initial_on(pausing_subject.as_observable(), true, fixture.scheduler.scheduler()).as_aggregator();
    // no results because the initial pause state is pause
    fixture.source.add_or_update(Person::new("A", 1));
    assert_eq!(results.data().count(), 0);

    // resume and expect a result
    pausing_subject.on_next(false);
    assert_eq!(results.data().count(), 1);

    // add another in the window where there is no pause
    fixture.source.add_or_update(Person::new("B", 1));
    assert_eq!(results.data().count(), 2);

    // pause again
    pausing_subject.on_next(true);
    fixture.source.add_or_update(Person::new("C", 1));
    assert_eq!(results.data().count(), 2);

    // resume for the second time
    pausing_subject.on_next(false);
    assert_eq!(results.data().count(), 3);

    results.dispose();
}

#[test]
fn timeout() {
    let fixture = TimeoutFixture::new();

    let pausing_subject = Subject::<bool>::new();
    let results = fixture
        .source
        .connect()
        .batch_if_with_time_out_on(pausing_subject.as_observable(), Some(Duration::from_secs(1)), fixture.scheduler.scheduler())
        .as_aggregator();
    // no results because the initial pause state is pause
    fixture.source.add_or_update(Person::new("A", 1));
    assert_eq!(results.data().count(), 1);

    // pause and add
    pausing_subject.on_next(true);
    fixture.source.add_or_update(Person::new("B", 1));
    assert_eq!(results.data().count(), 1);

    // resume before timeout ends
    fixture.scheduler.advance_by(ticks(Duration::from_millis(500)));
    assert_eq!(results.data().count(), 1);

    pausing_subject.on_next(false);
    assert_eq!(results.data().count(), 2);

    // pause and advance past timeout window
    pausing_subject.on_next(true);
    fixture.source.add_or_update(Person::new("C", 1));
    fixture.scheduler.advance_by(ticks(Duration::from_millis(2100)));
    assert_eq!(results.data().count(), 3);

    fixture.source.add_or_update(Person::new("D", 1));
    assert_eq!(results.data().count(), 4);

    results.dispose();
}

// --- BatchIfWithTimeOutFixture ---

/// The fixture (`BatchIfWithTimeOutFixture` with its constructor and `Dispose`).
struct Fixture {
    pausing_subject: Rc<Subject<bool>>,
    results: ChangeSetAggregator<Rc<Person>, String>,
    scheduler: TestScheduler,
    source: SourceCache<Rc<Person>, String>,
}

impl Fixture {
    fn new() -> Self {
        let pausing_subject = Subject::<bool>::new();
        let scheduler = TestScheduler::new();
        let source = SourceCache::new(|p: &Rc<Person>| p.key());
        let results = source
            .connect()
            .batch_if_with_time_out_on(pausing_subject.as_observable(), Some(Duration::from_secs(60)), scheduler.scheduler())
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
    fixture.scheduler.advance_by(ticks(Duration::from_millis(10)));

    fixture.source.add_or_update(Person::new("A", 1));

    // go forward an arbitary amount of time
    assert_eq!(fixture.results.messages().len(), 0, "There should be no messages");

    fixture.pausing_subject.on_next(false);
    fixture.scheduler.advance_by(ticks(Duration::from_millis(10)));

    fixture.source.add_or_update(Person::new("B", 1));

    assert_eq!(fixture.results.messages().len(), 2, "There should be 2 messages");
}

#[test]
fn no_results_will_be_received_if_paused() {
    let fixture = Fixture::new();

    fixture.pausing_subject.on_next(true);
    // advance otherwise nothing happens
    fixture.scheduler.advance_by(ticks(Duration::from_millis(10)));

    fixture.source.add_or_update(Person::new("A", 1));

    assert_eq!(fixture.results.messages().len(), 0, "There should be no messages");
}

#[test]
fn publishes_on_interval_event() {
    let fixture = Fixture::new();

    let interval_timer: Rc<dyn IObservable<()>> = Observable::interval_on(Duration::from_millis(5), fixture.scheduler.scheduler()).select(|_: i64| ());
    let results = fixture
        .source
        .connect()
        .batch_if_with_initial_and_timer_on(fixture.pausing_subject.as_observable(), true, Some(interval_timer), fixture.scheduler.scheduler())
        .as_aggregator();

    // Buffering
    fixture.source.add_or_update(Person::new("A", 1));
    fixture.scheduler.advance_by(ticks(Duration::from_millis(1)));
    assert_eq!(results.messages().len(), 0, "There should be 0 messages");

    // Interval Fires and drains buffer
    fixture.scheduler.advance_by(ticks(Duration::from_millis(5)));
    assert_eq!(results.messages().len(), 1, "There should be 1 messages");

    // Buffering again
    fixture.source.add_or_update(Person::new("B", 2));
    fixture.scheduler.advance_by(ticks(Duration::from_millis(1)));
    assert_eq!(results.messages().len(), 1, "There should be 1 messages");

    // Interval Fires and drains buffer
    fixture.scheduler.advance_by(ticks(Duration::from_millis(5)));
    assert_eq!(results.messages().len(), 2, "There should be 2 messages");

    // Buffering again
    fixture.source.add_or_update(Person::new("C", 3));
    fixture.scheduler.advance_by(ticks(Duration::from_millis(1)));
    assert_eq!(results.messages().len(), 2, "There should be 2 messages");

    // Interval Fires and drains buffer
    fixture.scheduler.advance_by(ticks(Duration::from_millis(5)));
    assert_eq!(results.messages().len(), 3, "There should be 3 messages");
}

#[test]
fn publishes_on_timer_completion() {
    let fixture = Fixture::new();

    let interval_timer: Rc<dyn IObservable<()>> = Observable::timer_on(Duration::from_millis(5), fixture.scheduler.scheduler()).select(|_: i64| ());
    let results = fixture
        .source
        .connect()
        .batch_if_with_initial_and_timer_on(fixture.pausing_subject.as_observable(), true, Some(interval_timer), fixture.scheduler.scheduler())
        .as_aggregator();

    // Buffering
    fixture.source.add_or_update(Person::new("A", 1));
    fixture.scheduler.advance_by(ticks(Duration::from_millis(1)));
    assert_eq!(results.messages().len(), 0, "There should be 0 messages");

    // Timer should event, buffered items delivered
    fixture.scheduler.advance_by(ticks(Duration::from_millis(5)));
    assert_eq!(results.messages().len(), 1, "There should be 1 messages");

    // Unbuffered from here
    fixture.source.add_or_update(Person::new("B", 2));
    fixture.scheduler.advance_by(ticks(Duration::from_millis(1)));
    assert_eq!(results.messages().len(), 2, "There should be 2 messages");

    // Unbuffered from here
    fixture.source.add_or_update(Person::new("C", 3));
    fixture.scheduler.advance_by(ticks(Duration::from_millis(1)));
    assert_eq!(results.messages().len(), 3, "There should be 3 messages");
}

#[test]
fn results_will_be_received_if_not_paused() {
    let fixture = Fixture::new();

    fixture.source.add_or_update(Person::new("A", 1));

    // go forward an arbitary amount of time
    fixture.scheduler.advance_by(ticks(Duration::from_secs(60)));
    assert_eq!(fixture.results.messages().len(), 1, "Should be 1 update");
}

#[test]
fn will_apply_timeout() {
    let fixture = Fixture::new();

    fixture.pausing_subject.on_next(true);

    // should timeout
    fixture.scheduler.advance_by(ticks(Duration::from_secs(61)));

    fixture.source.add_or_update(Person::new("A", 1));

    assert_eq!(fixture.results.messages().len(), 1, "There should be 1 messages");
}
