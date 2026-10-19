//! `SizeLimitFixture` (`DynamicData.Tests/Cache/SizeLimitFixture.cs`): the oldest items of a source
//! cache removed beyond a size (`LimitSizeTo` on `ISourceCache`).

use crate::domain::{Person, RandomPersonGenerator};
use ferroui_dynamic_data::cache::testing::ChangeSetAggregator;
use ferroui_dynamic_data::cache::{IConnectableCache, IObservableCache, ObservableCacheEx, SourceCache, SourceCacheEx};
use ferroui_reactive::ObservableExt;
use ferroui_reactive_testing::{assert_throws, duration_to_ticks, TestScheduler};
use ferroui_system::IDisposable;
use std::cell::Cell;
use std::collections::HashSet;
use std::rc::Rc;
use std::time::Duration;

/// The fixture (`SizeLimitFixture` with its constructor and `Dispose`).
struct Fixture {
    generator: RandomPersonGenerator,
    results: ChangeSetAggregator<Rc<Person>, String>,
    scheduler: TestScheduler,
    size_limiter: Rc<dyn IDisposable>,
    source: SourceCache<Rc<Person>, String>,
}

impl Fixture {
    fn new() -> Self {
        let scheduler = TestScheduler::new();
        let source = SourceCache::new(|p: &Rc<Person>| p.key());
        let size_limiter = source.limit_size_to_on(10, scheduler.scheduler()).subscribe_ignore();
        let results = source.connect().as_aggregator();
        Self { generator: RandomPersonGenerator::new(), results, scheduler, size_limiter, source }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.size_limiter.dispose();
        self.source.dispose();
        self.results.dispose();
    }
}

/// `_generator.Take(n).DistinctBy(p => p.Key).Take(count)`.
fn distinct_by_key(people: Vec<Rc<Person>>, count: usize) -> Vec<Rc<Person>> {
    let mut seen = HashSet::new();
    people.into_iter().filter(|p| seen.insert(p.key())).take(count).collect()
}

fn ticks(duration: Duration) -> i64 {
    duration_to_ticks(duration)
}

#[test]
fn add() {
    let fixture = Fixture::new();

    let person = fixture.generator.take(1).into_iter().next().expect("a person");
    fixture.source.add_or_update(person.clone());

    assert_eq!(fixture.results.messages().len(), 1, "Should be 1 updates");
    assert_eq!(fixture.results.data().count(), 1, "Should be 1 item in the cache");
    assert_eq!(fixture.results.data().items()[0], person, "Should be same person");
}

#[test]
fn add_less_than_limit() {
    let fixture = Fixture::new();

    let person = fixture.generator.take(1).into_iter().next().expect("a person");
    fixture.source.add_or_update(person.clone());

    fixture.scheduler.advance_by(ticks(Duration::from_millis(150)));

    assert_eq!(fixture.results.messages().len(), 1, "Should be 1 updates");
    assert_eq!(fixture.results.data().count(), 1, "Should be 1 item in the cache");
    assert_eq!(fixture.results.data().items()[0], person, "Should be same person");
}

#[test]
fn add_more_than_limit() {
    let fixture = Fixture::new();

    let mut people = fixture.generator.take(100);
    people.sort_by_key(|p| p.name());
    fixture.source.add_or_update_items(people.iter().cloned());
    fixture.scheduler.advance_by(ticks(Duration::from_millis(50)));

    fixture.source.dispose();
    let messages = fixture.results.messages();
    assert_eq!(messages.len(), 2, "Should be 2 updates");
    assert_eq!(messages[0].adds(), 100, "Should be 100 adds in the first update");
    assert_eq!(messages[1].removes(), 90, "Should be 90 removes in the second update");
}

#[test]
fn add_more_than_limit_in_batched() {
    let fixture = Fixture::new();

    // _generator.Take(N) draws random Person rows from a finite name pool; a second
    // Take(10) call can produce keys that collide with the first batch, turning an
    // Add into an Update and breaking the per-message Adds count. Draw a larger pool
    // up front, dedupe by Key, then split into two non-overlapping batches of 10.
    let people = distinct_by_key(fixture.generator.take(60), 20);
    fixture.source.add_or_update_items(people[..10].iter().cloned());
    fixture.source.add_or_update_items(people[10..20].iter().cloned());

    fixture.scheduler.run();

    let messages = fixture.results.messages();
    assert_eq!(messages.len(), 3, "Should be 3 updates");
    assert_eq!(messages[0].adds(), 10, "Should be 10 adds in the first update");
    assert_eq!(messages[1].adds(), 10, "Should be 10 adds in the second update");
    assert_eq!(messages[2].removes(), 10, "Should be 10 removes in the third update");
}

#[test]
fn invoke_limit_size_to_when_over_limit() {
    let fixture = Fixture::new();

    let removes_triggered = Rc::new(Cell::new(false));
    let subscriber = {
        let removes_triggered = removes_triggered.clone();
        fixture.source.limit_size_to_on(10, fixture.scheduler.scheduler()).subscribe_next(move |_removes: Vec<(String, Rc<Person>)>| removes_triggered.set(true))
    };

    // _generator.Take(N) draws random Person rows from a finite name pool; a second
    // Take(10) call can produce keys that collide with the first batch, turning an
    // Add into an Update and breaking the per-message Adds count. Draw a larger pool
    // up front, dedupe by Key, then split into two non-overlapping batches of 10.
    let people = distinct_by_key(fixture.generator.take(60), 20);
    fixture.source.add_or_update_items(people[..10].iter().cloned());
    fixture.scheduler.advance_by(ticks(Duration::from_millis(150)));

    assert!(!removes_triggered.get());

    fixture.source.add_or_update_items(people[10..20].iter().cloned());

    fixture.scheduler.advance_by(ticks(Duration::from_millis(150)));

    assert!(removes_triggered.get());

    let messages = fixture.results.messages();
    assert_eq!(messages.len(), 3, "Should be 3 updates");
    assert_eq!(messages[0].adds(), 10, "Should be 10 adds in the first update");
    assert_eq!(messages[1].adds(), 10, "Should be 10 adds in the second update");
    assert_eq!(messages[2].removes(), 10, "Should be 10 removes in the third update");

    subscriber.dispose();
}

/// Skipped upstream (`[Fact(Skip = "Need to re-examine and fix failure")]`).
#[test]
#[ignore = "Need to re-examine and fix failure"]
fn on_complete_is_invoked_when_source_is_disposed() {
    let fixture = Fixture::new();

    let completed = Rc::new(Cell::new(false));

    let subscriber = {
        let completed = completed.clone();
        fixture.source.limit_size_to(10).finally(move || completed.set(true)).subscribe_next(|_updates: Vec<(String, Rc<Person>)>| {})
    };

    fixture.source.dispose();

    assert!(completed.get());

    subscriber.dispose();
}

#[test]
fn throws_if_size_limit_is_zero() {
    // Initialise();
    assert_throws(|| {
        let _ = SourceCache::new(|p: &Rc<Person>| p.key()).limit_size_to(0);
    });
}
