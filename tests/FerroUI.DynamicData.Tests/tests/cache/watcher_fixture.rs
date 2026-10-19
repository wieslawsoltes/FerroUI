//! `WatcherFixture` (`DynamicData.Tests/Cache/WatcherFixture.cs`): the watcher of the items of a
//! cache by key (`AsWatcher`, `Watch`), on a test scheduler.

use crate::domain::{Person, SelfObservingPerson};
use ferroui_dynamic_data::cache::testing::ChangeSetAggregator;
use ferroui_dynamic_data::cache::{Change, ChangeReason, IConnectableCache, IObservableCache, ObservableCacheEx, SourceCache, SourceCacheEx};
use ferroui_dynamic_data::experimental::{ExperimentalEx, IWatcher};
use ferroui_reactive::ObservableExt;
use ferroui_reactive_testing::{duration_to_ticks, TestScheduler};
use ferroui_system::IDisposable;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

/// The fixture: a cache of people, a watcher of it, and the people that watch themselves through
/// the watcher.
struct Fixture {
    results: ChangeSetAggregator<Rc<SelfObservingPerson>, String>,
    scheduler: TestScheduler,
    source: SourceCache<Rc<Person>, String>,
    watcher: Rc<dyn IWatcher<Rc<Person>, String>>,
}

impl Fixture {
    fn new() -> Self {
        let scheduler = TestScheduler::new();
        let source = SourceCache::new(|p: &Rc<Person>| p.key());
        let watcher = source.connect().as_watcher_on(scheduler.scheduler());

        let results = {
            let watcher = watcher.clone();
            ChangeSetAggregator::new(
                source
                    .connect()
                    .transform(move |p: &Rc<Person>| Rc::new(SelfObservingPerson::new(watcher.watch(p.key()).select(|w: Change<Rc<Person>, String>| w.current().clone()))))
                    .dispose_many(),
            )
        };

        Self { results, scheduler, source, watcher }
    }

    fn advance_by(&self, milliseconds: u64) {
        self.scheduler.advance_by(duration_to_ticks(Duration::from_millis(milliseconds)));
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.results.dispose();
        self.source.dispose();
        self.watcher.dispose();
    }
}

#[test]
fn add_new() {
    let fixture = Fixture::new();
    let person = Person::new("Adult1", 50);
    fixture.source.add_or_update(person);

    assert_eq!(fixture.results.data().count(), 1, "Should be 1 item in the cache");

    fixture.advance_by(50);
    let result = fixture.results.data().items()[0].clone();
    assert_eq!(result.update_count(), 1, "Person should have received 1 update");
    assert!(!result.completed(), "Person should have received 1 update");
}

#[test]
fn remove() {
    let fixture = Fixture::new();
    let person = Person::new("Adult1", 50);
    fixture.source.add_or_update(person.clone());

    fixture.advance_by(10);
    fixture.source.remove_key(person.key());

    fixture.advance_by(11);
    let messages = fixture.results.messages();
    assert_eq!(messages.len(), 2, "Should be 1 updates");
    assert_eq!(fixture.results.data().count(), 0, "Should be 0 item in the cache");

    let second_result = messages[1].changes()[0].clone();
    assert_eq!(second_result.current().update_count(), 1, "Second Person should have received 1 update");
    assert!(second_result.current().completed(), "Second person  should have received 1 update");
}

#[test]
fn update() {
    let fixture = Fixture::new();
    let first = Person::new("Adult1", 50);
    let second = Person::new("Adult1", 51);
    fixture.source.add_or_update(first);

    fixture.advance_by(10);
    fixture.source.add_or_update(second);

    fixture.advance_by(10);
    let messages = fixture.results.messages();
    assert_eq!(messages.len(), 2, "Should be 1 updates");
    assert_eq!(fixture.results.data().count(), 1, "Should be 1 item in the cache");

    let second_result = messages[1].changes()[0].clone();
    let previous = second_result.previous().expect("an update has the previous item");
    assert_eq!(previous.update_count(), 1, "Second Person should have received 1 update");
    assert!(previous.completed(), "Second person  should have received 1 update");
}

#[test]
fn watch() {
    let fixture = Fixture::new();
    let person = Person::new("Adult1", 50);
    fixture.source.add_or_update(person.clone());

    let result: Rc<RefCell<Vec<Change<Rc<Person>, String>>>> = Rc::new(RefCell::new(Vec::with_capacity(3)));
    let watch = {
        let result = result.clone();
        fixture.watcher.watch("Adult1".to_owned()).subscribe_next(move |change| result.borrow_mut().push(change))
    };

    fixture.advance_by(50);
    assert_eq!(result.borrow().len(), 1, "Should be 1 updates");
    assert!(Rc::ptr_eq(result.borrow()[0].current(), &person), "Should be 1 item in the cache");

    fixture.source.edit(|updater| updater.remove_key(&"Adult1".to_owned()));
    fixture.advance_by(500);

    watch.dispose();
}

#[test]
fn watch_many() {
    let fixture = Fixture::new();
    fixture.source.add_or_update(Person::new("Adult1", 50));

    let result: Rc<RefCell<Vec<Change<Rc<Person>, String>>>> = Rc::new(RefCell::new(Vec::with_capacity(3)));
    let subscribe = || {
        let result = result.clone();
        fixture.watcher.watch("Adult1".to_owned()).subscribe_next(move |change| result.borrow_mut().push(change))
    };
    let watch1 = subscribe();
    let watch2 = subscribe();
    let watch3 = subscribe();
    fixture.advance_by(100);

    assert_eq!(result.borrow().len(), 3, "Should be 3 updates");
    for update in result.borrow().iter() {
        assert_eq!(update.reason(), ChangeReason::Add, "Change reason should be add");
    }

    result.borrow_mut().clear();

    fixture.source.add_or_update(Person::new("Adult1", 51));
    fixture.advance_by(500);
    assert_eq!(result.borrow().len(), 3, "Should be 3 updates");
    for update in result.borrow().iter() {
        assert_eq!(update.reason(), ChangeReason::Update, "Change reason should be add");
    }

    result.borrow_mut().clear();

    fixture.source.remove_key("Adult1".to_owned());
    fixture.advance_by(500);
    assert_eq!(result.borrow().len(), 3, "Should be 3 updates");
    for update in result.borrow().iter() {
        assert_eq!(update.reason(), ChangeReason::Remove, "Change reason should be add");
    }

    result.borrow_mut().clear();

    watch1.dispose();
    watch2.dispose();
    watch3.dispose();
}
