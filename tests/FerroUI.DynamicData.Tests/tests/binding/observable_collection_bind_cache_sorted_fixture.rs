//! `ObservableCollectionBindCacheSortedFixture`
//! (`DynamicData.Tests/Binding/ObservableCollectionBindCacheSortedFixture.cs`).

use crate::domain::{Person, RandomPersonGenerator};
use crate::utilities::assert_equivalent;
use ferroui_dynamic_data::binding::{BindingOptions, ObservableCollectionAdaptor, ObservableCollectionEx, ObservableCollectionExtended, SortExpressionComparer};
use ferroui_dynamic_data::cache::{SortOptimisations, SortedObservableCacheEx};
use ferroui_dynamic_data::{ChangeSet, IConnectableCache, IObservableCache, ObservableCacheEx, SourceCache};
use ferroui_reactive::ObservableExt;
use ferroui_system::collections::specialized::{NotifyCollectionChangedAction, NotifyCollectionChangedEventArgs};
use ferroui_system::IDisposable;
use std::cell::{Cell, RefCell};
use std::cmp::Ordering;
use std::rc::Rc;

type Comparer = Rc<dyn Fn(&Rc<Person>, &Rc<Person>) -> Ordering>;

struct Fixture {
    binder: Rc<dyn IDisposable>,
    collection: ObservableCollectionExtended<Rc<Person>>,
    comparer: Comparer,
    generator: RandomPersonGenerator,
    source: SourceCache<Rc<Person>, String>,
}

impl Fixture {
    fn new() -> Self {
        let collection = ObservableCollectionExtended::new();
        let comparer = SortExpressionComparer::<Rc<Person>>::ascending(|p| p.name()).comparer();
        let source = SourceCache::new(|p: &Rc<Person>| p.name());
        let binder = source.connect().sort_with(comparer.clone(), SortOptimisations::NONE, 25).bind(&collection).subscribe_ignore();
        Self { binder, collection, comparer, generator: RandomPersonGenerator::new(), source }
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

fn sorted(items: &[Rc<Person>], comparer: &Comparer) -> Vec<Rc<Person>> {
    let mut items = items.to_vec();
    items.sort_by(|a, b| comparer(a, b));
    items
}

#[test]
fn reset_thresholds_for_binding_observable_collection() {
    let fixture = Fixture::new();
    let people = fixture.generator.take(100);

    let test = |options: Option<BindingOptions>| -> NotifyCollectionChangedAction {
        fixture.source.edit(|updater| updater.clear());

        let result = Rc::new(Cell::new(None));

        let list = ObservableCollectionExtended::new();
        let target = result.clone();
        let list_events = list.observe_collection_changes().take(1).select(|e| e.event_args.action()).subscribe_next(move |events| target.set(Some(events)));

        let binder = match options {
            None => fixture.source.connect().sort(fixture.comparer.clone()).bind(&list).subscribe_ignore(),
            Some(options) => fixture.source.connect().sort(fixture.comparer.clone()).bind_with_options(&list, options).subscribe_ignore(),
        };

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
fn reset_thresholds_for_binding_readonly_observable_collection() {
    let fixture = Fixture::new();
    let people = fixture.generator.take(100);

    let test = |options: Option<BindingOptions>| -> NotifyCollectionChangedAction {
        fixture.source.edit(|updater| updater.clear());

        let result = Rc::new(Cell::new(None));

        let (bound, list) = match options {
            None => fixture.source.connect().sort(fixture.comparer.clone()).bind_read_only(),
            Some(options) => fixture.source.connect().sort(fixture.comparer.clone()).bind_read_only_with_options(options),
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
fn collection_is_in_sort_order() {
    let fixture = Fixture::new();
    add_or_update(&fixture.source, &fixture.generator.take(100));
    let sorted = sorted(&fixture.source.items(), &fixture.comparer);
    assert_equivalent(&sorted, &fixture.collection.to_vec());
}

#[test]
fn large_update_invokes_a_reset() {
    let fixture = Fixture::new();
    // Update once, as the initial load is always a reset.
    add_or_update(&fixture.source, &[Person::new("Me", 21)]);

    let invoked = Rc::new(Cell::new(false));
    let target = invoked.clone();
    fixture.collection.add_collection_changed(Rc::new(move |e: &NotifyCollectionChangedEventArgs<'_, Rc<Person>>| {
        target.set(true);
        assert_eq!(e.action, NotifyCollectionChangedAction::Reset);
    }));
    add_or_update(&fixture.source, &fixture.generator.take(100));

    assert!(invoked.get());
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
fn small_change_does_not_invoke_reset() {
    let fixture = Fixture::new();
    // Update once, as the initial load is always a reset.
    add_or_update(&fixture.source, &[Person::new("Me", 21)]);

    let invoked = Rc::new(Cell::new(false));
    let reset_invoked = Rc::new(Cell::new(false));
    let (target, reset_target) = (invoked.clone(), reset_invoked.clone());
    fixture.collection.add_collection_changed(Rc::new(move |e: &NotifyCollectionChangedEventArgs<'_, Rc<Person>>| {
        target.set(true);
        if e.action == NotifyCollectionChangedAction::Reset {
            reset_target.set(true);
        }
    }));
    add_or_update(&fixture.source, &fixture.generator.take(24));

    assert!(invoked.get());
    assert!(!reset_invoked.get());
}

#[test]
fn treat_moves_as_remove_add() {
    let cache = SourceCache::new(|p: &Rc<Person>| p.name());

    let people: Vec<Rc<Person>> = (0..10).map(|age| Person::new(format!("Person{age}"), age)).collect();
    let important_guy = people[0].clone();
    add_or_update(&cache, &people);

    let latest_set_without_moves: Rc<RefCell<Option<ChangeSet<Rc<Person>, String>>>> = Rc::new(RefCell::new(None));
    let latest_set_with_moves: Rc<RefCell<Option<ChangeSet<Rc<Person>, String>>>> = Rc::new(RefCell::new(None));

    let bound_list1 = ObservableCollectionExtended::new();
    let bound_list2 = ObservableCollectionExtended::new();

    let by_age = || SortExpressionComparer::<Rc<Person>>::ascending(|p| p.age()).comparer();

    let target = latest_set_without_moves.clone();
    let without_moves = cache
        .connect()
        .auto_refresh(ferroui_system::prop!(Person => age))
        .sort(by_age())
        .treat_moves_as_remove_add()
        .bind(&bound_list1)
        .subscribe_next(move |set| *target.borrow_mut() = Some(set));

    let target = latest_set_with_moves.clone();
    let with_moves = cache.connect().auto_refresh(ferroui_system::prop!(Person => age)).sort(by_age()).bind(&bound_list2).subscribe_next(move |set| *target.borrow_mut() = Some(set));

    important_guy.set_age(important_guy.age() + 200);

    let latest_set_without_moves = latest_set_without_moves.borrow().clone().expect("latestSetWithoutMoves");
    let latest_set_with_moves = latest_set_with_moves.borrow().clone().expect("latestSetWithMoves");

    assert_eq!(latest_set_without_moves.removes(), 1);
    assert_eq!(latest_set_without_moves.adds(), 1);
    assert_eq!(latest_set_without_moves.moves(), 0);
    assert_eq!(latest_set_without_moves.updates(), 0);

    assert_eq!(latest_set_with_moves.moves(), 1);
    assert_eq!(latest_set_with_moves.updates(), 0);
    assert_eq!(latest_set_with_moves.removes(), 0);
    assert_eq!(latest_set_with_moves.adds(), 0);

    without_moves.dispose();
    with_moves.dispose();
}

#[test]
fn update_to_source_sends_remove_and_add_if_sorting_is_affected() {
    let person1 = Person::new("Adult1", 10);
    let person2 = Person::new("Adult2", 11);
    let person2_updated = Person::new("Adult2", 1);

    let actions = Rc::new(RefCell::new(Vec::new()));
    let collection = ObservableCollectionExtended::new();

    let source = SourceCache::new(|person: &Rc<Person>| person.name());
    let binder = source.connect().sort(SortExpressionComparer::<Rc<Person>>::ascending(|person| person.age()).comparer()).bind(&collection).subscribe_ignore();

    add_or_update(&source, std::slice::from_ref(&person1));
    add_or_update(&source, std::slice::from_ref(&person2));

    let target = actions.clone();
    let subscription = collection.observe_collection_changes().select(|change| change.event_args.action()).subscribe_next(move |act| target.borrow_mut().push(act));
    add_or_update(&source, std::slice::from_ref(&person2_updated));
    subscription.dispose();
    binder.dispose();
    source.dispose();

    assert_eq!(*actions.borrow(), vec![NotifyCollectionChangedAction::Remove, NotifyCollectionChangedAction::Add]);
    assert_eq!(collection.to_vec(), vec![person2_updated, person1]);
}

#[test]
fn update_to_source_sends_replace_if_sorting_is_not_affected() {
    let fixture = Fixture::new();
    let run_test = |use_replace: bool| {
        let collection = ObservableCollectionExtended::new();

        let source = SourceCache::new(|p: &Rc<Person>| p.name());
        let binder = source
            .connect()
            .sort_with(fixture.comparer.clone(), SortOptimisations::NONE, 25)
            .bind_with_adaptor(&collection, Rc::new(ObservableCollectionAdaptor::<Rc<Person>, String>::with_use_replace_for_updates(use_replace)))
            .subscribe_ignore();

        let person1 = Person::new("Adult1", 10);
        let person2 = Person::new("Adult2", 11);

        let action = Rc::new(Cell::new(None));
        add_or_update(&source, std::slice::from_ref(&person1));
        add_or_update(&source, std::slice::from_ref(&person2));

        let person2_updated = Person::new("Adult2", 12);

        let target = action.clone();
        let subscription = collection.observe_collection_changes().select(|x| x.event_args.action()).subscribe_next(move |update_type| target.set(Some(update_type)));
        add_or_update(&source, std::slice::from_ref(&person2_updated));
        subscription.dispose();

        if use_replace {
            assert_eq!(action.get(), Some(NotifyCollectionChangedAction::Replace), "The notification type should be Replace");
        } else {
            assert_eq!(action.get(), Some(NotifyCollectionChangedAction::Add), "The notification type should be Add");
        }

        assert_eq!(collection.to_vec(), vec![person1, person2_updated]);
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
