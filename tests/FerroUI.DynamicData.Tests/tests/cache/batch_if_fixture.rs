//! `BatchIfFixture` (`DynamicData.Tests/Cache/BatchIfFixture.cs` and
//! `BatchIfFixture.Overloads.cs`): the change sets held back while a pause selector last gave
//! `true` (`BatchIf`), and the forms of its overloads.

use crate::domain::{Person, Randomizer};
use ferroui_dynamic_data::cache::testing::ChangeSetAggregator;
use ferroui_dynamic_data::cache::{ChangeSet, IConnectableCache, IObservableCache, ObservableCacheEx, SourceCache, SourceCacheEx};
use ferroui_reactive::subjects::Subject;
use ferroui_reactive::ObservableExt;
use ferroui_reactive_testing::{duration_to_ticks, TestScheduler};
use ferroui_system::{IDisposable, IObservable, IObserver};
use std::rc::Rc;
use std::time::Duration;

/// `OverloadSeed` (`BatchIfFixture.Overloads.cs`).
const OVERLOAD_SEED: i32 = 0x2409_1153;

/// The fixture (`BatchIfFixture` with its constructor and `Dispose`).
struct Fixture {
    pausing_subject: Rc<Subject<bool>>,
    results: ChangeSetAggregator<Rc<Person>, String>,
    scheduler: TestScheduler,
    source: SourceCache<Rc<Person>, String>,
    /// `_overloadRandomizer`.
    overload_randomizer: Randomizer,
}

impl Fixture {
    fn new() -> Self {
        let pausing_subject = Subject::<bool>::new();
        let scheduler = TestScheduler::new();
        let source = SourceCache::new(|p: &Rc<Person>| p.key());
        let results = source.connect().batch_if_on(pausing_subject.as_observable(), scheduler.scheduler()).as_aggregator();
        Self { pausing_subject, results, scheduler, source, overload_randomizer: Randomizer::new(OVERLOAD_SEED) }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.results.dispose();
        self.source.dispose();
    }
}

fn ticks(duration: Duration) -> i64 {
    duration_to_ticks(duration)
}

#[test]
fn can_toggle_suspend_resume() {
    let fixture = Fixture::new();

    fixture.pausing_subject.on_next(true);
    // advance otherwise nothing happens
    fixture.scheduler.advance_by(ticks(Duration::from_millis(10)));

    fixture.source.add_or_update(Person::new("A", 1));

    // go forward an arbitary amount of time
    fixture.scheduler.advance_by(ticks(Duration::from_secs(60)));
    assert_eq!(fixture.results.messages().len(), 0, "There should be no messages");

    fixture.pausing_subject.on_next(false);
    fixture.scheduler.advance_by(ticks(Duration::from_millis(10)));

    fixture.source.add_or_update(Person::new("B", 1));

    assert_eq!(fixture.results.messages().len(), 2, "There should be 2 messages");

    fixture.pausing_subject.on_next(true);
    // advance otherwise nothing happens
    fixture.scheduler.advance_by(ticks(Duration::from_millis(10)));

    fixture.source.add_or_update(Person::new("C", 1));

    // go forward an arbitary amount of time
    fixture.scheduler.advance_by(ticks(Duration::from_secs(60)));
    assert_eq!(fixture.results.messages().len(), 2, "There should be 2 messages");

    fixture.pausing_subject.on_next(false);
    fixture.scheduler.advance_by(ticks(Duration::from_millis(10)));

    assert_eq!(fixture.results.messages().len(), 3, "There should be 3 messages");
}

/// Test case to prove the issue and fix to DynamicData GitHub issue #98 - BatchIf race condition.
/// Upstream's "different thread" is a second `TestScheduler`, so the test runs on one thread.
#[test]
fn changes_not_lost_if_consumer_is_running_on_different_thread() {
    let fixture = Fixture::new();

    let producer_scheduler = TestScheduler::new();
    let consumer_scheduler = TestScheduler::new();

    // Note consumer is running on a different scheduler
    let (bound, target) = fixture
        .source
        .connect()
        .batch_if_on(fixture.pausing_subject.as_observable(), producer_scheduler.scheduler())
        .observe_on(consumer_scheduler.scheduler())
        .bind_read_only();
    let _aggregator = bound.as_aggregator();

    fixture.source.add_or_update(Person::new("A", 1));

    producer_scheduler.advance_by(1);
    consumer_scheduler.advance_by(1);

    assert_eq!(target.count(), 1, "There should be 1 message");

    fixture.pausing_subject.on_next(true);

    producer_scheduler.advance_by(1);
    consumer_scheduler.advance_by(1);

    fixture.source.add_or_update(Person::new("B", 2));

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
    fixture.scheduler.advance_by(ticks(Duration::from_millis(10)));

    fixture.source.add_or_update(Person::new("A", 1));

    // go forward an arbitary amount of time
    fixture.scheduler.advance_by(ticks(Duration::from_secs(60)));
    assert_eq!(fixture.results.messages().len(), 0, "There should be no messages");
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
fn pause_selector_only_starts_unpaused() {
    let fixture = Fixture::new();

    // The shortest form has to keep delegating with initialPauseState false, rather than binding
    // to an overload that starts paused.
    let pause = Subject::<bool>::new();
    let results = fixture.source.connect().batch_if(pause.as_observable()).as_aggregator();

    fixture.source.add_or_update(Person::new("A", 1));

    assert_eq!(results.data().count(), 1, "nothing has asked for buffering yet");

    results.dispose();
    pause.dispose();
}

/// Every supported call shape binds (`BatchIfFixture.Overloads.cs`). In the port each overload is a
/// method of its own name, so the shapes are the methods: upstream's `BatchIf(pause, null,
/// scheduler)` is `batch_if_with_time_out_on(pause, None, scheduler)`, its named-argument forms are
/// the methods with those arguments, and compiling is the assertion.
// Deviation (DEVIATIONS.md, DynamicData): the overloads are methods of their own names, so the
// call shapes are the method calls; the test checks that each one is available.
#[test]
fn supported_call_shapes_remain_unambiguous() {
    let fixture = Fixture::new();

    // Arrange
    let pause = Subject::<bool>::new();
    let timer = Subject::<()>::new();
    let changes = fixture.source.connect();
    let time_out = Duration::from_millis(fixture.overload_randomizer.int_range(10, 500) as u64);
    let p = || pause.as_observable();
    let t = || -> Option<Rc<dyn IObservable<()>>> { Some(timer.as_observable()) };
    let s = || fixture.scheduler.scheduler();

    // Act
    let preserved_shapes: Vec<Rc<dyn IObservable<ChangeSet<Rc<Person>, String>>>> = vec![
        changes.batch_if(p()),
        changes.batch_if_on(p(), s()),
        changes.batch_if_with_initial(p(), true),
        changes.batch_if_with_initial_on(p(), true, s()),
        changes.batch_if_with_time_out_on(p(), None, s()),
        changes.batch_if_with_time_out(p(), Some(time_out)),
        changes.batch_if_with_time_out_on(p(), Some(time_out), s()),
        changes.batch_if_with_initial_and_time_out(p(), true, Some(time_out)),
        changes.batch_if_with_initial_and_time_out_on(p(), true, Some(time_out), s()),
        changes.batch_if_with_initial_and_timer(p(), true, t()),
        changes.batch_if_with_initial_and_timer_on(p(), true, t(), s()),
        changes.batch_if_with_initial_and_timer_on(p(), true, t(), s()),
        changes.batch_if_with_initial_and_timer_on(p(), true, t(), s()),
    ];

    let restored_shapes: Vec<Rc<dyn IObservable<ChangeSet<Rc<Person>, String>>>> = vec![changes.batch_if_with_timer(p(), t()), changes.batch_if_with_timer_on(p(), t(), s())];

    // Assert
    assert_eq!(preserved_shapes.len(), 13, "every shape that compiled before this change must still bind");
    assert_eq!(restored_shapes.len(), 2, "the named timer shapes from issue 1175 must bind");

    pause.dispose();
    timer.dispose();
}
