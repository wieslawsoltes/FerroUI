//! `SizeLimitFixture` (`DynamicData.Tests/List/SizeLimitFixture.cs`): `LimitSizeTo` of source
//! lists, on a test scheduler.

use crate::domain::{Person, RandomPersonGenerator};
use ferroui_dynamic_data::cache::{SourceCache, SourceCacheEx};
use ferroui_dynamic_data::list::testing::ChangeSetAggregator;
use ferroui_dynamic_data::list::{IObservableList, ObservableListEx, SourceList, SourceListEditConvenienceEx, SourceListEx};
use ferroui_reactive::ObservableExt;
use ferroui_reactive_testing::{assert_throws, duration_to_ticks, TestScheduler};
use ferroui_system::IDisposable;
use std::rc::Rc;
use std::time::Duration;

struct Fixture {
    generator: RandomPersonGenerator,
    results: ChangeSetAggregator<Rc<Person>>,
    scheduler: TestScheduler,
    size_limiter: Rc<dyn IDisposable>,
    source: SourceList<Rc<Person>>,
}

impl Fixture {
    fn new() -> Self {
        let scheduler = TestScheduler::new();
        let source = SourceList::<Rc<Person>>::new();
        let size_limiter = SourceListEx::limit_size_to_on(&source, 10, scheduler.scheduler()).subscribe_ignore();
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

#[test]
fn add() {
    let fixture = Fixture::new();
    let person = fixture.generator.take(1)[0].clone();
    fixture.source.add(person.clone());

    assert_eq!(fixture.results.messages().len(), 1, "Should be 1 updates");
    assert_eq!(fixture.results.data().count(), 1, "Should be 1 item in the cache");
    assert_eq!(fixture.results.data().items()[0], person, "Should be same person");
}

#[test]
fn add_less_than_limit() {
    let fixture = Fixture::new();
    let person = fixture.generator.take(1)[0].clone();
    fixture.source.add(person.clone());

    fixture.scheduler.advance_by(duration_to_ticks(Duration::from_millis(150)));

    assert_eq!(fixture.results.messages().len(), 1, "Should be 1 updates");
    assert_eq!(fixture.results.data().count(), 1, "Should be 1 item in the cache");
    assert_eq!(fixture.results.data().items()[0], person, "Should be same person");
}

#[test]
fn add_more_than_limit() {
    let fixture = Fixture::new();
    let mut people = fixture.generator.take(100);
    people.sort_by_key(|p| p.name());
    fixture.source.add_range(people);
    fixture.scheduler.advance_by(duration_to_ticks(Duration::from_millis(50)));

    fixture.source.dispose();
    let messages = fixture.results.messages();
    assert_eq!(fixture.results.data().count(), 10, "Should be 10 items in the cache");
    assert_eq!(messages.len(), 2, "Should be 2 updates");
    assert_eq!(messages[0].adds(), 100, "Should be 100 adds in the first update");
    assert_eq!(messages[1].removes(), 90, "Should be 90 removes in the second update");
}

#[test]
fn add_more_than_limit_in_batched() {
    let fixture = Fixture::new();
    fixture.source.add_range(fixture.generator.take(10));
    fixture.source.add_range(fixture.generator.take(10));

    fixture.scheduler.advance_by(duration_to_ticks(Duration::from_millis(50)));
    let messages = fixture.results.messages();
    assert_eq!(fixture.results.data().count(), 10, "Should be 10 items in the cache");
    assert_eq!(messages.len(), 3, "Should be 3 updates");
    assert_eq!(messages[0].adds(), 10, "Should be 10 adds in the first update");
    assert_eq!(messages[1].adds(), 10, "Should be 10 adds in the second update");
    assert_eq!(messages[2].removes(), 10, "Should be 10 removes in the third update");
}

#[test]
fn force_error() {
    let fixture = Fixture::new();
    let _person = fixture.generator.take(1)[0].clone();
    // `ArgumentOutOfRangeException`: an index out of range panics.
    assert_throws(|| fixture.source.remove_at(1));
}

#[test]
fn throws_if_size_limit_is_zero() {
    // `ArgumentException`: a size of zero panics.
    assert_throws(|| {
        let cache = SourceCache::<Rc<Person>, String>::new(|p: &Rc<Person>| p.key());
        let _limiter = SourceCacheEx::limit_size_to(&cache, 0);
    });
}
