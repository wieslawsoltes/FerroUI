//! `QueryWhenChangedFixture` (`DynamicData.Tests/List/QueryWhenChangedFixture.cs`).

use crate::domain::Person;
use ferroui_dynamic_data::list::testing::ChangeSetAggregator;
use ferroui_dynamic_data::list::{IObservableList, ObservableListEx, SourceList, SourceListEditConvenienceEx};
use ferroui_reactive::ObservableExt;
use ferroui_system::IDisposable;
use std::cell::Cell;
use std::rc::Rc;

struct Fixture {
    results: ChangeSetAggregator<Rc<Person>>,
    source: SourceList<Rc<Person>>,
}

impl Fixture {
    fn new() -> Self {
        let source = SourceList::<Rc<Person>>::new();
        let predicate: Rc<dyn Fn(&Rc<Person>) -> bool> = Rc::new(|p: &Rc<Person>| p.age() > 20);
        let results = ChangeSetAggregator::new(source.connect_with(Some(predicate)));
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
fn can_handle_adds_and_updates() {
    let fixture = Fixture::new();
    let invoked = Rc::new(Cell::new(false));
    let subscription = {
        let invoked = invoked.clone();
        fixture.source.connect().query_when_changed_with(|q: Vec<Rc<Person>>| q.len()).subscribe_next(move |_| invoked.set(true))
    };

    let person = Person::new("A", 1);
    fixture.source.add(person.clone());
    fixture.source.remove(&person);

    assert!(invoked.get());
    subscription.dispose();
}

#[test]
fn change_invoked_on_next() {
    let fixture = Fixture::new();
    let invoked = Rc::new(Cell::new(false));

    let subscription = {
        let invoked = invoked.clone();
        fixture.source.connect().query_when_changed().subscribe_next(move |_| invoked.set(true))
    };

    assert!(!invoked.get());

    fixture.source.add(Person::new("A", 1));
    assert!(invoked.get());

    subscription.dispose();
}

#[test]
fn change_invoked_on_next_with_selector() {
    let fixture = Fixture::new();
    let invoked = Rc::new(Cell::new(false));

    let subscription = {
        let invoked = invoked.clone();
        fixture.source.connect().query_when_changed_with(|query: Vec<Rc<Person>>| query.len()).subscribe_next(move |_| invoked.set(true))
    };

    assert!(!invoked.get());

    fixture.source.add(Person::new("A", 1));
    assert!(invoked.get());

    subscription.dispose();
}

#[test]
fn change_invoked_on_subscription_if_it_has_data() {
    let fixture = Fixture::new();
    let invoked = Rc::new(Cell::new(false));
    fixture.source.add(Person::new("A", 1));
    let subscription = {
        let invoked = invoked.clone();
        fixture.source.connect().query_when_changed().subscribe_next(move |_| invoked.set(true))
    };
    assert!(invoked.get());
    subscription.dispose();
}

#[test]
fn change_invoked_on_subscription_if_it_has_data_with_selector() {
    let fixture = Fixture::new();
    let invoked = Rc::new(Cell::new(false));
    fixture.source.add(Person::new("A", 1));
    let subscription = {
        let invoked = invoked.clone();
        fixture.source.connect().query_when_changed_with(|query: Vec<Rc<Person>>| query.len()).subscribe_next(move |_| invoked.set(true))
    };
    assert!(invoked.get());
    subscription.dispose();
}
