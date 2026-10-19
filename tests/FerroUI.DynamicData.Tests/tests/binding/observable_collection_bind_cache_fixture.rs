//! `ObservableCollectionBindCacheFixture` (`DynamicData.Tests/Binding/ObservableCollectionBindCacheFixture.cs`).

use crate::domain::{Person, RandomPersonGenerator};
use crate::utilities::assert_equivalent;
use ferroui_dynamic_data::binding::{BindingOptions, ObservableCollectionAdaptor, ObservableCollectionEx, ObservableCollectionExtended};
use ferroui_dynamic_data::{IConnectableCache, ObservableCacheEx, SourceCache};
use ferroui_reactive::ObservableExt;
use ferroui_system::collections::specialized::NotifyCollectionChangedAction;
use ferroui_system::IDisposable;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

struct Fixture {
    binder: Rc<dyn IDisposable>,
    collection: ObservableCollectionExtended<Rc<Person>>,
    generator: RandomPersonGenerator,
    source: SourceCache<Rc<Person>, String>,
}

impl Fixture {
    fn new() -> Self {
        let collection = ObservableCollectionExtended::new();
        let source = SourceCache::new(|p: &Rc<Person>| p.name());
        let binder = source.connect().bind(&collection).subscribe_ignore();
        Self { binder, collection, generator: RandomPersonGenerator::new(), source }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.binder.dispose();
        self.source.dispose();
    }
}

fn add_or_update(source: &SourceCache<Rc<Person>, String>, people: &[Rc<Person>]) {
    source.edit(|updater| updater.add_or_update_items(&mut people.iter().cloned()));
}

#[test]
fn reset_thresholds_for_binding_observable_collection() {
    let fixture = Fixture::new();
    let people = fixture.generator.take(100);

    let test = |options: Option<BindingOptions>| -> (NotifyCollectionChangedAction, ObservableCollectionExtended<Rc<Person>>) {
        fixture.source.edit(|updater| updater.clear());

        let result = Rc::new(Cell::new(None));

        let list = ObservableCollectionExtended::new();
        let target = result.clone();
        let list_events = list.observe_collection_changes().take(1).select(|e| e.event_args.action()).subscribe_next(move |events| target.set(Some(events)));

        let binder = match options {
            None => fixture.source.connect().bind(&list).subscribe_ignore(),
            Some(options) => fixture.source.connect().bind_with_options(&list, options).subscribe_ignore(),
        };

        add_or_update(&fixture.source, &people);
        binder.dispose();
        list_events.dispose();

        (result.get().expect("a notification"), list)
    };

    // Check whether reset is fired with different params.
    let test1 = test(None);
    let test2 = test(Some(BindingOptions::new(95)));
    let test3 = test(Some(BindingOptions::new(105).with_reset_on_first_time_load(false)));
    let test4 = test(Some(BindingOptions::never_fire_reset()));

    assert_eq!(test1.0, NotifyCollectionChangedAction::Reset);
    assert_eq!(test2.0, NotifyCollectionChangedAction::Reset);
    assert_eq!(test3.0, NotifyCollectionChangedAction::Add);
    assert_eq!(test4.0, NotifyCollectionChangedAction::Add);
}

#[test]
fn reset_thresholds_for_binding_readonly_observable_collection() {
    let fixture = Fixture::new();
    let people = fixture.generator.take(100);

    let test = |options: Option<BindingOptions>| -> NotifyCollectionChangedAction {
        fixture.source.edit(|updater| updater.clear());

        let result = Rc::new(Cell::new(None));

        let (bound, list) = match options {
            None => fixture.source.connect().bind_read_only(),
            Some(options) => fixture.source.connect().bind_read_only_with_options(options),
        };
        let binder = bound.subscribe_ignore();

        let target = result.clone();
        let list_events = list.observe_collection_changes().take(1).select(|e| e.event_args.action()).subscribe_next(move |events| target.set(Some(events)));

        add_or_update(&fixture.source, &people);
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
fn add_to_source_adds_to_destination() {
    let fixture = Fixture::new();
    let person = Person::new("Adult1", 50);
    add_or_update(&fixture.source, std::slice::from_ref(&person));

    assert_eq!(fixture.collection.count(), 1, "Should be 1 item in the collection");
    assert_eq!(fixture.collection.get(0), person, "Should be same person");
}

#[test]
fn batch_add() {
    let fixture = Fixture::new();
    let people = fixture.generator.take(100);
    add_or_update(&fixture.source, &people);

    assert_eq!(fixture.collection.count(), 100, "Should be 100 items in the collection");
    assert_equivalent(&fixture.collection.to_vec(), &fixture.collection.to_vec());
}

#[test]
fn batch_remove() {
    let fixture = Fixture::new();
    let people = fixture.generator.take(100);
    add_or_update(&fixture.source, &people);
    fixture.source.edit(|updater| updater.clear());
    assert_eq!(fixture.collection.count(), 0, "Should be 100 items in the collection");
}

#[test]
fn remove_source_removes_from_the_destination() {
    let fixture = Fixture::new();
    let person = Person::new("Adult1", 50);
    add_or_update(&fixture.source, std::slice::from_ref(&person));
    fixture.source.edit(|updater| updater.remove_item(&person));

    assert_eq!(fixture.collection.count(), 0, "Should be 1 item in the collection");
}

#[test]
fn update_to_source_sends_replace_on_destination() {
    let run_test = |use_replace: bool| {
        let collection = ObservableCollectionExtended::new();

        let source = SourceCache::new(|p: &Rc<Person>| p.name());
        let binder = source
            .connect()
            .bind_with_adaptor(&collection, Rc::new(ObservableCollectionAdaptor::<Rc<Person>, String>::with_use_replace_for_updates(use_replace)))
            .subscribe_ignore();

        let action = Rc::new(RefCell::new(None));
        add_or_update(&source, &[Person::new("Adult1", 50)]);

        let target = action.clone();
        let subscription = collection.observe_collection_changes().select(|x| x.event_args.action()).subscribe_next(move |update_type| *target.borrow_mut() = Some(update_type));
        add_or_update(&source, &[Person::new("Adult1", 51)]);
        subscription.dispose();

        let action = action.borrow().expect("a notification");
        if use_replace {
            assert_eq!(action, NotifyCollectionChangedAction::Replace);
        } else {
            assert_eq!(action, NotifyCollectionChangedAction::Add);
        }
        binder.dispose();
        source.dispose();
    };

    run_test(true);
    run_test(false);
}

#[test]
fn update_to_source_updates_the_destination() {
    let fixture = Fixture::new();
    let person = Person::new("Adult1", 50);
    let person_updated = Person::new("Adult1", 51);
    add_or_update(&fixture.source, std::slice::from_ref(&person));
    add_or_update(&fixture.source, std::slice::from_ref(&person_updated));

    assert_eq!(fixture.collection.count(), 1, "Should be 1 item in the collection");
    assert!(Rc::ptr_eq(&fixture.collection.get(0), &person_updated), "Should be updated person");
}

