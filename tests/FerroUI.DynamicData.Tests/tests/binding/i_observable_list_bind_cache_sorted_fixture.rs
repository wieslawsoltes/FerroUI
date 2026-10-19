//! `IObservableListBindCacheSortedFixture`
//! (`DynamicData.Tests/Binding/IObservableListBindCacheSortedFixture.cs`).

use crate::domain::{Person, RandomPersonGenerator};
use ferroui_dynamic_data::binding::SortExpressionComparer;
use ferroui_dynamic_data::cache::testing::ChangeSetAggregator as CacheChangeSetAggregator;
use ferroui_dynamic_data::cache::{SortOptimisations, SortedObservableCacheEx};
use ferroui_dynamic_data::list::testing::ChangeSetAggregator;
use ferroui_dynamic_data::list::{IObservableList, ListChangeReason, ObservableListEx};
use ferroui_dynamic_data::{ChangeSet, IConnectableCache, IObservableCache, ObservableCacheEx, SourceCache};
use ferroui_reactive::subjects::BehaviorSubject;
use ferroui_reactive::ObservableExt;
use ferroui_system::{IDisposable, IObserver};
use std::cell::RefCell;
use std::cmp::Ordering;
use std::rc::Rc;

type Comparer = Rc<dyn Fn(&Rc<Person>, &Rc<Person>) -> Ordering>;

fn comparer_age_asc_then_name_asc() -> Comparer {
    SortExpressionComparer::<Rc<Person>>::ascending(|p| p.age()).then_by_ascending(|p| p.name()).comparer()
}

fn comparer_name_desc() -> Comparer {
    SortExpressionComparer::<Rc<Person>>::descending(|p| p.name()).comparer()
}

fn sorted(items: &[Rc<Person>], comparer: &Comparer) -> Vec<Rc<Person>> {
    let mut items = items.to_vec();
    items.sort_by(|a, b| comparer(a, b));
    items
}

struct Fixture {
    comparer: Rc<BehaviorSubject<Comparer>>,
    generator: RandomPersonGenerator,
    list: Rc<dyn IObservableList<Rc<Person>>>,
    list_notifications: ChangeSetAggregator<Rc<Person>>,
    source: SourceCache<Rc<Person>, String>,
    source_cache_notifications: CacheChangeSetAggregator<Rc<Person>, String>,
}

impl Fixture {
    fn new() -> Self {
        let comparer = BehaviorSubject::new(comparer_age_asc_then_name_asc());
        let source = SourceCache::new(|p: &Rc<Person>| p.name());
        let (bound, list) = source.connect().auto_refresh_any().sort_dynamic_with(comparer.as_observable(), SortOptimisations::NONE, 10).bind_to_observable_list();
        let source_cache_notifications = bound.as_aggregator();
        let list_notifications = list.connect().as_aggregator();
        Self { comparer, generator: RandomPersonGenerator::new(), list, list_notifications, source, source_cache_notifications }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.source_cache_notifications.dispose();
        self.list_notifications.dispose();
        self.source.dispose();
        self.comparer.dispose();
    }
}

fn add_or_update(source: &SourceCache<Rc<Person>, String>, people: &[Rc<Person>]) {
    source.edit(|updater| updater.add_or_update_items(&mut people.iter().cloned()));
}

#[test]
fn add_to_source_adds_to_destination() {
    let fixture = Fixture::new();
    let person = Person::new("Adult1", 50);
    add_or_update(&fixture.source, std::slice::from_ref(&person));

    assert_eq!(fixture.list.count(), 1, "Should be 1 item in the collection");
    assert_eq!(fixture.list.items()[0], person, "Should be same person");
}

#[test]
fn batch_add() {
    let fixture = Fixture::new();
    let people = fixture.generator.take(15);
    add_or_update(&fixture.source, &people);

    let sorted = sorted(&people, &comparer_age_asc_then_name_asc());

    assert_eq!(fixture.list.count(), 15, "Should be 15 items in the collection");
    assert_eq!(fixture.list.items(), sorted, "Collections should be equivalent");
}

#[test]
fn batch_remove() {
    let fixture = Fixture::new();
    let people = fixture.generator.take(100);
    add_or_update(&fixture.source, &people);
    fixture.source.edit(|updater| updater.clear());
    assert_eq!(fixture.list.count(), 0, "Should be 0 items in the collection");
}

#[test]
fn collection_is_in_sort_order() {
    let fixture = Fixture::new();
    add_or_update(&fixture.source, &fixture.generator.take(100));
    let sorted = sorted(&fixture.source.items(), &comparer_age_asc_then_name_asc());
    assert_eq!(sorted, fixture.list.items());
}

#[test]
fn initial_bind_with_existing_data() {
    let fixture = Fixture::new();
    let source = SourceCache::new(|p: &Rc<Person>| p.name());

    // Populate the source before binding.
    let person1 = Person::new("Adult1", 20);
    let person2 = Person::new("Adult2", 30);
    // Add out of order to assert the initial order.
    add_or_update(&source, std::slice::from_ref(&person2));
    add_or_update(&source, std::slice::from_ref(&person1));

    let (bound, list) = source.connect().auto_refresh_any().sort_dynamic_with(fixture.comparer.as_observable(), SortOptimisations::NONE, 10).bind_to_observable_list();
    let source_cache_notifications = bound.as_aggregator();
    let list_notifications = list.connect().as_aggregator();

    // Assert
    assert_eq!(list_notifications.messages().len(), 1);
    assert_eq!(list_notifications.messages()[0][0].reason(), ListChangeReason::AddRange);
    assert_eq!(list.items(), vec![person1, person2]);

    // Clean up
    source.dispose();
    source_cache_notifications.dispose();
    list_notifications.dispose();
    list.dispose();
}

#[test]
fn list_recieves_moves() {
    let fixture = Fixture::new();
    let person1 = Person::new("Person1", 10);
    let person2 = Person::new("Person2", 20);
    let person3 = Person::new("Person3", 30);

    add_or_update(&fixture.source, &[person1, person2, person3.clone()]);

    // Move person 3 to the front of the line.
    person3.set_age(1);

    // 1 change set with an AddRange, and 1 change set with a Refresh and a Move.
    let messages = fixture.list_notifications.messages();
    assert_eq!(messages.len(), 2);

    // Assert the AddRange.
    let add_change_set = &messages[0];
    assert_eq!(add_change_set[0].reason(), ListChangeReason::AddRange);

    // Assert the Refresh and the Move.
    let refresh_and_move_change_set = &messages[1];
    assert_eq!(refresh_and_move_change_set.count(), 2);

    let refresh_change = &refresh_and_move_change_set[0];
    assert_eq!(refresh_change.reason(), ListChangeReason::Refresh);
    assert_eq!(refresh_change.item().current(), &person3);

    let move_change = &refresh_and_move_change_set[1];
    assert_eq!(move_change.reason(), ListChangeReason::Moved);
    assert_eq!(move_change.item().current(), &person3);
    assert_eq!(move_change.item().previous_index(), Some(2));
    assert_eq!(move_change.item().current_index(), Some(0));
}

#[test]
fn list_recieves_refresh() {
    let fixture = Fixture::new();
    let person = Person::new("Adult1", 50);
    add_or_update(&fixture.source, std::slice::from_ref(&person));

    person.set_age(60);

    assert_eq!(fixture.list_notifications.messages().len(), 2);
    assert_eq!(fixture.list_notifications.messages().last().expect("a refresh")[0].reason(), ListChangeReason::Refresh);
}

#[test]
fn remove_source_removes_from_the_destination() {
    let fixture = Fixture::new();
    let person = Person::new("Adult1", 50);
    add_or_update(&fixture.source, std::slice::from_ref(&person));
    fixture.source.edit(|updater| updater.remove_item(&person));

    assert_eq!(fixture.list.count(), 0, "Should be 1 item in the collection");
}

#[test]
fn reset() {
    let fixture = Fixture::new();
    let people: Vec<Rc<Person>> = (1..=100).map(|i| Person::new(format!("P{i}"), i)).collect();
    add_or_update(&fixture.source, &people);
    fixture.comparer.on_next(comparer_name_desc());

    let sorted = sorted(&people, &comparer_name_desc());
    assert_eq!(fixture.list.items(), sorted);

    let messages = fixture.list_notifications.messages();
    // The initial load, and a reset as the change is over the reset threshold.
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0][0].reason(), ListChangeReason::AddRange);
    assert_eq!(messages[1].count(), 2);
    assert_eq!(messages[1][0].reason(), ListChangeReason::Clear);
    assert_eq!(messages[1][1].reason(), ListChangeReason::AddRange);
}

#[test]
fn treat_moves_as_remove_add() {
    let cache = SourceCache::new(|p: &Rc<Person>| p.name());

    let people: Vec<Rc<Person>> = (0..10).map(|age| Person::new(format!("Person{age}"), age)).collect();
    let important_guy = people[0].clone();
    add_or_update(&cache, &people);

    let latest_set_without_moves: Rc<RefCell<Option<ChangeSet<Rc<Person>, String>>>> = Rc::new(RefCell::new(None));
    let latest_set_with_moves: Rc<RefCell<Option<ChangeSet<Rc<Person>, String>>>> = Rc::new(RefCell::new(None));

    let by_age = || SortExpressionComparer::<Rc<Person>>::ascending(|p| p.age()).comparer();

    let (bound1, _bound_list1) = cache.connect().auto_refresh(ferroui_system::prop!(Person => age)).sort(by_age()).treat_moves_as_remove_add().bind_to_observable_list();
    let target = latest_set_without_moves.clone();
    let without_moves = bound1.subscribe_next(move |set| *target.borrow_mut() = Some(set));

    let (bound2, _bound_list2) = cache.connect().auto_refresh(ferroui_system::prop!(Person => age)).sort(by_age()).bind_to_observable_list();
    let target = latest_set_with_moves.clone();
    let with_moves = bound2.subscribe_next(move |set| *target.borrow_mut() = Some(set));

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
fn update_to_source_updates_the_destination() {
    let fixture = Fixture::new();
    let person1 = Person::new("Adult1", 20);
    let person2 = Person::new("Adult2", 30);
    let person_updated1 = Person::new("Adult1", 40);

    add_or_update(&fixture.source, std::slice::from_ref(&person1));
    add_or_update(&fixture.source, std::slice::from_ref(&person2));

    assert_eq!(fixture.list.items(), vec![person1, person2.clone()]);

    add_or_update(&fixture.source, std::slice::from_ref(&person_updated1));

    assert_eq!(fixture.list.items(), vec![person2, person_updated1]);
}
