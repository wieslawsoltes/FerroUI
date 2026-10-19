//! `IObservableListBindListFixture` (`DynamicData.Tests/Binding/IObservableListBindListFixture.cs`).

use crate::domain::{Person, RandomPersonGenerator};
use crate::utilities::assert_equivalent;
use ferroui_dynamic_data::binding::{BindingOptions, ObservableCollectionEx, ObservableCollectionExtended};
use ferroui_dynamic_data::list::testing::ChangeSetAggregator;
use ferroui_dynamic_data::list::{replace, IObservableList, ListChangeReason, ObservableListEx, SourceList};
use ferroui_reactive::ObservableExt;
use ferroui_system::collections::specialized::NotifyCollectionChangedAction;
use ferroui_system::IDisposable;
use std::cell::Cell;
use std::rc::Rc;

struct Fixture {
    generator: RandomPersonGenerator,
    list: Rc<dyn IObservableList<Rc<Person>>>,
    observable_list_notifications: ChangeSetAggregator<Rc<Person>>,
    source: SourceList<Rc<Person>>,
    source_list_notifications: ChangeSetAggregator<Rc<Person>>,
}

impl Fixture {
    fn new() -> Self {
        let source = SourceList::new();
        let (bound, list) = source.connect().auto_refresh_any().bind_to_observable_list();
        let source_list_notifications = bound.as_aggregator();
        let observable_list_notifications = list.connect().as_aggregator();
        Self { generator: RandomPersonGenerator::new(), list, observable_list_notifications, source, source_list_notifications }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.source_list_notifications.dispose();
        self.observable_list_notifications.dispose();
        self.source.dispose();
    }
}

#[test]
fn reset_thresholds_for_binding_observable_collection() {
    let fixture = Fixture::new();
    let people = fixture.generator.take(100);

    let test = |options: Option<BindingOptions>| -> NotifyCollectionChangedAction {
        fixture.source.edit(|list| list.clear());

        let result = Rc::new(Cell::new(None));

        let list = ObservableCollectionExtended::new();
        let target = result.clone();
        let list_events = list.observe_collection_changes().take(1).select(|e| e.event_args.action()).subscribe_next(move |events| target.set(Some(events)));

        let binder = match options {
            None => fixture.source.connect().bind(&list).subscribe_ignore(),
            Some(options) => fixture.source.connect().bind_with_options(&list, options).subscribe_ignore(),
        };

        let people = people.clone();
        fixture.source.edit(|list| list.add_range(people));
        binder.dispose();
        list_events.dispose();

        result.get().expect("a notification")
    };

    // Check whether reset is fired with different params.
    let test1 = test(None);
    let test2 = test(Some(BindingOptions::new(95)));
    let test3 = test(Some(BindingOptions::new(105).with_reset_on_first_time_load(false)));
    let test4 = test(Some(BindingOptions::never_fire_reset()));

    assert_eq!(test1, NotifyCollectionChangedAction::Reset);
    assert_eq!(test2, NotifyCollectionChangedAction::Reset);
    assert_eq!(test3, NotifyCollectionChangedAction::Add);
    assert_eq!(test4, NotifyCollectionChangedAction::Add);
}

#[test]
fn reset_thresholds_for_binding_readonly_observable_collection() {
    let fixture = Fixture::new();
    let people = fixture.generator.take(100);

    let test = |options: Option<BindingOptions>| -> NotifyCollectionChangedAction {
        fixture.source.edit(|list| list.clear());

        let result = Rc::new(Cell::new(None));

        let (bound, list) = match options {
            None => fixture.source.connect().bind_read_only(),
            Some(options) => fixture.source.connect().bind_read_only_with_options(options),
        };
        let binder = bound.subscribe_ignore();

        let target = result.clone();
        let list_events = list.observe_collection_changes().take(1).select(|e| e.event_args.action()).subscribe_next(move |events| target.set(Some(events)));

        let people = people.clone();
        fixture.source.edit(|list| list.add_range(people));
        binder.dispose();
        list_events.dispose();
        result.get().expect("a notification")
    };

    // Check whether reset is fired with different params.
    let test1 = test(None);
    let test2 = test(Some(BindingOptions::new(95)));
    let test3 = test(Some(BindingOptions::new(105).with_reset_on_first_time_load(false)));
    let test4 = test(Some(BindingOptions::never_fire_reset()));

    assert_eq!(test1, NotifyCollectionChangedAction::Reset);
    assert_eq!(test2, NotifyCollectionChangedAction::Reset);
    assert_eq!(test3, NotifyCollectionChangedAction::Add);
    assert_eq!(test4, NotifyCollectionChangedAction::Add);
}

#[test]
fn add_range() {
    let fixture = Fixture::new();
    let people = fixture.generator.take(100);
    fixture.source.edit(|list| list.add_range(people));

    assert_eq!(fixture.list.count(), 100, "Should be 100 items in the collection");
    assert_equivalent(&fixture.list.items(), &fixture.list.items());
}

#[test]
fn add_to_source_adds_to_destination() {
    let fixture = Fixture::new();
    let person = Person::new("Adult1", 50);
    fixture.source.edit(|list| list.add(person.clone()));

    assert_eq!(fixture.list.count(), 1, "Should be 1 item in the collection");
    assert_eq!(fixture.list.items()[0], person, "Should be same person");
}

#[test]
fn clear() {
    let fixture = Fixture::new();
    let people = fixture.generator.take(100);
    fixture.source.edit(|list| list.add_range(people));
    fixture.source.edit(|list| list.clear());

    assert_eq!(fixture.list.count(), 0, "Should be 100 items in the collection");
}

#[test]
fn list_recieves_refresh() {
    let fixture = Fixture::new();
    let person = Person::new("Adult1", 50);
    fixture.source.edit(|list| list.add(person.clone()));

    person.set_age(60);

    assert_eq!(fixture.observable_list_notifications.messages().len(), 2);
    assert_eq!(fixture.observable_list_notifications.messages().last().expect("a refresh")[0].reason(), ListChangeReason::Refresh);
}

#[test]
fn remove_source_removes_from_the_destination() {
    let fixture = Fixture::new();
    let person = Person::new("Adult1", 50);
    fixture.source.edit(|list| list.add(person.clone()));
    fixture.source.edit(|list| {
        list.remove(&person);
    });

    assert_eq!(fixture.list.count(), 0, "Should be 1 item in the collection");
}

#[test]
fn update_to_source_updates_the_destination() {
    let fixture = Fixture::new();
    let person = Person::new("Adult1", 50);
    let person_updated = Person::new("Adult1", 51);
    fixture.source.edit(|list| list.add(person.clone()));
    fixture.source.edit(|list| replace(list, &person, person_updated.clone()));

    assert_eq!(fixture.list.count(), 1, "Should be 1 item in the collection");
    assert!(Rc::ptr_eq(&fixture.list.items()[0], &person_updated), "Should be updated person");
}
