//! `QueryWhenChangedFixture` (`DynamicData.Tests/Cache/QueryWhenChangedFixture.cs`): a snapshot of
//! the items after each change set (`QueryWhenChanged`).

use crate::domain::Person;
use ferroui_dynamic_data::cache::testing::ChangeSetAggregator;
use ferroui_dynamic_data::cache::{IConnectableCache, IQuery, ObservableCacheEx, SourceCache, SourceCacheEx};
use ferroui_reactive::ObservableExt;
use ferroui_system::IDisposable;
use std::cell::Cell;
use std::rc::Rc;

/// The fixture (`QueryWhenChangedFixture` with its constructor and `Dispose`).
struct Fixture {
    results: ChangeSetAggregator<Rc<Person>, String>,
    source: SourceCache<Rc<Person>, String>,
}

impl Fixture {
    fn new() -> Self {
        let source = SourceCache::new(|p: &Rc<Person>| p.name());
        let results = ChangeSetAggregator::new(source.connect_filtered(|p: &Rc<Person>| p.age() > 20));
        Self { results, source }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.source.dispose();
        self.results.dispose();
    }
}

#[test]
fn change_invoked_on_next() {
    let fixture = Fixture::new();

    let invoked = Rc::new(Cell::new(false));

    let subscription = {
        let invoked = invoked.clone();
        fixture.source.connect().query_when_changed().subscribe_next(move |_x: Rc<dyn IQuery<Rc<Person>, String>>| invoked.set(true))
    };

    assert!(!invoked.get());

    fixture.source.add_or_update(Person::new("A", 1));
    assert!(invoked.get());

    subscription.dispose();
}

#[test]
fn change_invoked_on_next_with_selector() {
    let fixture = Fixture::new();

    let invoked = Rc::new(Cell::new(false));

    let subscription = {
        let invoked = invoked.clone();
        fixture
            .source
            .connect()
            .query_when_changed_with(|query: Rc<dyn IQuery<Rc<Person>, String>>| query.count())
            .subscribe_next(move |_x: usize| invoked.set(true))
    };

    assert!(!invoked.get());

    fixture.source.add_or_update(Person::new("A", 1));
    assert!(invoked.get());

    subscription.dispose();
}

#[test]
fn change_invoked_on_subscription_if_it_has_data() {
    let fixture = Fixture::new();

    let invoked = Rc::new(Cell::new(false));
    fixture.source.add_or_update(Person::new("A", 1));
    let subscription = {
        let invoked = invoked.clone();
        fixture.source.connect().query_when_changed().subscribe_next(move |_x: Rc<dyn IQuery<Rc<Person>, String>>| invoked.set(true))
    };
    assert!(invoked.get());
    subscription.dispose();
}

#[test]
fn change_invoked_on_subscription_if_it_has_data_with_selector() {
    let fixture = Fixture::new();

    let invoked = Rc::new(Cell::new(false));
    fixture.source.add_or_update(Person::new("A", 1));
    let subscription = {
        let invoked = invoked.clone();
        fixture
            .source
            .connect()
            .query_when_changed_with(|query: Rc<dyn IQuery<Rc<Person>, String>>| query.count())
            .subscribe_next(move |_x: usize| invoked.set(true))
    };
    assert!(invoked.get());
    subscription.dispose();
}
