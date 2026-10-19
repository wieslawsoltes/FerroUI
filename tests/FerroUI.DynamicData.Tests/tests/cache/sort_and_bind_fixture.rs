//! `SortAndBindFixture` and the fixtures of its targets (`DynamicData.Tests/Cache/SortAndBindFixture.cs`).
//!
//! The abstract fixture of upstream runs its tests for each target; here each target is a module of
//! the same tests (`sort_and_bind_fixture!`).
//!
//! Not ported: SortAndBindToBindingList - Windows Forms.

use crate::domain::{Person, RandomPersonGenerator};
use ferroui_dynamic_data::binding::{ObservableCollectionEx, SortAndBindOptions, SortExpressionComparer};
use ferroui_dynamic_data::cache::testing::ChangeSetAggregator;
use ferroui_dynamic_data::{IConnectableCache, IObservableCache, ObservableCacheEx, SourceCache};
use ferroui_reactive::ObservableExt;
use ferroui_system::collections::object_model::ObservableCollection;
use ferroui_system::collections::specialized::{NotifyCollectionChangedAction, SharedNotifyCollectionChangedEventArgs};
use ferroui_system::IDisposable;
use std::cell::RefCell;
use std::cmp::Ordering;
use std::rc::Rc;

type Comparer = Rc<dyn Fn(&Rc<Person>, &Rc<Person>) -> Ordering>;

/// The items of the bound list, read when a test looks (`IList<Person>`).
type BoundList = Rc<dyn Fn() -> Vec<Rc<Person>>>;

/// `SetUpTests`: the aggregator of the bound sequence and the bound list.
type SetUp = fn(&SourceCache<Rc<Person>, String>, &Comparer) -> (ChangeSetAggregator<Rc<Person>, String>, BoundList);

struct Fixture {
    generator: RandomPersonGenerator,
    results: ChangeSetAggregator<Rc<Person>, String>,
    bound_list: BoundList,
    comparer: Comparer,
    source: SourceCache<Rc<Person>, String>,
}

impl Fixture {
    fn new(set_up: SetUp) -> Self {
        let comparer = Person::default_comparer();
        let source = SourceCache::new(|p: &Rc<Person>| p.key());
        // Bind and sort in one hit.
        let (results, bound_list) = set_up(&source, &comparer);
        Self { generator: RandomPersonGenerator::new(), results, bound_list, comparer, source }
    }

    fn bound(&self) -> Vec<Rc<Person>> {
        (self.bound_list)()
    }

    fn sorted(&self, items: &[Rc<Person>]) -> Vec<Rc<Person>> {
        let mut items = items.to_vec();
        items.sort_by(|a, b| (self.comparer)(a, b));
        items
    }

    fn add_or_update(&self, people: &[Rc<Person>]) {
        self.source.edit(|updater| updater.add_or_update_items(&mut people.iter().cloned()));
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.source.dispose();
        self.results.dispose();
    }
}

fn shared_list() -> (Rc<RefCell<Vec<Rc<Person>>>>, BoundList) {
    let list = Rc::new(RefCell::new(Vec::with_capacity(100)));
    let read = list.clone();
    (list, Rc::new(move || read.borrow().clone()))
}

// The targets (the subclasses of upstream's fixture).

/// `SortByAndBindToList`.
fn sort_by_and_bind_to_list(source: &SourceCache<Rc<Person>, String>, comparer: &Comparer) -> (ChangeSetAggregator<Rc<Person>, String>, BoundList) {
    let (list, bound) = shared_list();
    (source.connect().sort_and_bind(&list, comparer.clone()).as_aggregator(), bound)
}

/// `SortAndBindToList`.
fn sort_and_bind_to_list(source: &SourceCache<Rc<Person>, String>, comparer: &Comparer) -> (ChangeSetAggregator<Rc<Person>, String>, BoundList) {
    let (list, bound) = shared_list();
    (source.connect().sort_and_bind(&list, comparer.clone()).as_aggregator(), bound)
}

/// `SortAndBindToListDefaultComparer`.
fn sort_and_bind_to_list_default_comparer(source: &SourceCache<Rc<Person>, String>, _comparer: &Comparer) -> (ChangeSetAggregator<Rc<Person>, String>, BoundList) {
    let (list, bound) = shared_list();
    (source.connect().sort_and_bind_by_default(&list).as_aggregator(), bound)
}

/// `SortAndBindToObservableCollection`.
fn sort_and_bind_to_observable_collection(source: &SourceCache<Rc<Person>, String>, comparer: &Comparer) -> (ChangeSetAggregator<Rc<Person>, String>, BoundList) {
    let list = ObservableCollection::from_items(Vec::with_capacity(100));
    let read = list.clone();
    (source.connect().sort_and_bind(&list, comparer.clone()).as_aggregator(), Rc::new(move || read.to_vec()))
}

/// `SortAndBindToReadOnlyObservableCollection`.
fn sort_and_bind_to_read_only_observable_collection(source: &SourceCache<Rc<Person>, String>, comparer: &Comparer) -> (ChangeSetAggregator<Rc<Person>, String>, BoundList) {
    let (bound, list) = source.connect().sort_and_bind_read_only(comparer.clone());
    (bound.as_aggregator(), Rc::new(move || list.to_vec()))
}

/// `SortAndBindWithBinarySearch1`.
fn sort_and_bind_with_binary_search1(source: &SourceCache<Rc<Person>, String>, comparer: &Comparer) -> (ChangeSetAggregator<Rc<Person>, String>, BoundList) {
    let options = SortAndBindOptions { use_binary_search: true, use_replace_for_updates: false, ..SortAndBindOptions::default() };
    let (bound, list) = source.connect().sort_and_bind_read_only_with(comparer.clone(), options);
    (bound.as_aggregator(), Rc::new(move || list.to_vec()))
}

/// `SortAndBindWithBinarySearch2`.
fn sort_and_bind_with_binary_search2(source: &SourceCache<Rc<Person>, String>, comparer: &Comparer) -> (ChangeSetAggregator<Rc<Person>, String>, BoundList) {
    let options = SortAndBindOptions { use_binary_search: true, use_replace_for_updates: true, ..SortAndBindOptions::default() };
    let (bound, list) = source.connect().sort_and_bind_read_only_with(comparer.clone(), options);
    (bound.as_aggregator(), Rc::new(move || list.to_vec()))
}

/// `SortAndBindToReadOnlyObservableCollectionDefaultComparer`.
fn sort_and_bind_to_read_only_observable_collection_default_comparer(
    source: &SourceCache<Rc<Person>, String>,
    _comparer: &Comparer,
) -> (ChangeSetAggregator<Rc<Person>, String>, BoundList) {
    let (bound, list) = source.connect().sort_and_bind_read_only_by_default();
    (bound.as_aggregator(), Rc::new(move || list.to_vec()))
}

// The tests of the abstract fixture.

fn insert_at_beginning(fixture: &Fixture) {
    let people = fixture.generator.take(100);
    fixture.add_or_update(&people);

    // Check the initial data set is sorted.
    assert_eq!(fixture.bound().len(), 100);
    assert_eq!(fixture.bound(), fixture.sorted(&people));

    // Create age 0 to ensure it is inserted first.
    // Deviation (DEVIATIONS.md, DynamicData): `!Aaron` for upstream's `_Aaron`, which sorts first
    // among the names by culture upstream; names compare by their characters here.
    let insert = Person::new("!Aaron", 0);
    fixture.add_or_update(std::slice::from_ref(&insert));

    assert_eq!(fixture.bound().len(), 101);

    let first_item = fixture.bound()[0].clone();
    assert_eq!(insert, first_item);

    assert_eq!(fixture.bound(), fixture.sorted(&fixture.source.items()));
}

fn insert_at_end(fixture: &Fixture) {
    let people = fixture.generator.take(100);
    fixture.add_or_update(&people);

    let to_insert = Person::new("zzzzz", 1000);
    fixture.add_or_update(std::slice::from_ref(&to_insert));

    assert_eq!(fixture.bound().len(), 101);

    let last = fixture.bound().last().cloned().expect("a last item");
    assert_eq!(last, to_insert);

    assert_eq!(fixture.bound(), fixture.sorted(&fixture.source.items()));
}

fn insert_in_middle(fixture: &Fixture) {
    let people: Vec<Rc<Person>> = (0..100).map(|i| Person::new(format!("P{i}"), i)).collect();
    fixture.add_or_update(&people);

    let insert = Person::new("Marvin", 50);
    fixture.add_or_update(std::slice::from_ref(&insert));

    assert_eq!(fixture.bound().len(), 101);

    let index = fixture.bound().iter().position(|p| p == &insert);
    assert_eq!(index, Some(50));

    assert_eq!(fixture.bound(), fixture.sorted(&fixture.source.items()));
}

fn insert_same_location(fixture: &Fixture) {
    let people: Vec<Rc<Person>> = (1..=10).map(|i| Person::new(format!("P{i}"), i * 10)).collect();
    fixture.add_or_update(&people);

    let update_and_asset_position = |person: Rc<Person>, expected_index: usize| {
        fixture.add_or_update(std::slice::from_ref(&person));

        // Check the item has been replaced.
        assert_eq!(fixture.bound().iter().filter(|p| p.key() == person.key()).count(), 1);
        assert!(Rc::ptr_eq(&fixture.bound()[expected_index], &person));
    };

    // Each of these changes should result in index 1.
    update_and_asset_position(Person::new("P2", 15), 1);
    update_and_asset_position(Person::new("P2", 20), 1);
    update_and_asset_position(Person::new("P2", 25), 1);

    assert_eq!(fixture.bound().len(), 10);
    assert_eq!(fixture.bound(), fixture.sorted(&fixture.source.items()));
}

fn refresh(fixture: &Fixture) {
    let people: Vec<Rc<Person>> = (1..=10).map(|i| Person::new(format!("P{i}"), i * 10)).collect();
    fixture.add_or_update(&people);

    let to_refresh = fixture.bound()[1].clone();

    let refresh_at_and_asset_position = |person: &Rc<Person>, action: &dyn Fn(&Rc<Person>), expected_index: usize| {
        action(person);
        fixture.source.edit(|inner_cache| inner_cache.refresh_key(&person.key()));

        // Check the item has been replaced.
        assert_eq!(fixture.bound().iter().filter(|p| p.key() == person.key()).count(), 1);
        assert!(Rc::ptr_eq(&fixture.bound()[expected_index], person));
    };

    // These all result in the same position.
    refresh_at_and_asset_position(&to_refresh, &|p| p.set_age(15), 1);
    refresh_at_and_asset_position(&to_refresh, &|p| p.set_age(20), 1);
    refresh_at_and_asset_position(&to_refresh, &|p| p.set_age(25), 1);

    // Move after.
    refresh_at_and_asset_position(&to_refresh, &|p| p.set_age(45), 3);

    assert_eq!(fixture.bound().len(), 10);
    assert_eq!(fixture.bound(), fixture.sorted(&fixture.source.items()));
}

fn batch_of_various_changes(fixture: &Fixture) {
    let make = || -> Vec<Rc<Person>> { (0..100).map(|i| Person::new(format!("P{i}"), i)).collect() };
    let people = make();
    fixture.add_or_update(&people);

    let to_update = people[3].clone();

    // A mixture of adds, updates and removes.
    fixture.source.edit(|inner_cache| {
        inner_cache.remove_key(&people[0].key());
        inner_cache.add_or_update(Person::from_names_with("Mr", "Z", 50, "M", ""));
        inner_cache.remove_key(&people[1].key());
        inner_cache.add_or_update(Person::new(to_update.name(), to_update.age() - 24));
        inner_cache.remove_key(&people[7].key());
    });

    // Mirror the actions to the expected items.
    let mut expected = make();
    let index_from_key = |expected: &[Rc<Person>], key: &str| expected.iter().position(|p| p.key() == key).expect("the key is there");
    let index = index_from_key(&expected, &people[0].key());
    expected.remove(index);
    let index = index_from_key(&expected, &people[1].key());
    expected.remove(index);
    expected.push(Person::from_names_with("Mr", "Z", 50, "M", ""));
    let index = index_from_key(&expected, &to_update.key());
    expected[index] = Person::new(to_update.name(), to_update.age() - 24);
    let index = index_from_key(&expected, &people[7].key());
    expected.remove(index);

    let expected_in_order = fixture.sorted(&expected);
    assert_eq!(expected_in_order, fixture.bound());
}

fn batch_of_various_ending_in_clear(fixture: &Fixture) {
    let people = fixture.generator.take(10);
    fixture.add_or_update(&people);

    let more = fixture.generator.take(10);
    fixture.source.edit(|updater| {
        updater.clear();
        updater.add_or_update_items(&mut more.iter().cloned());
        updater.clear();
    });

    assert_eq!(fixture.bound().len(), 0);
}

fn large_batch_change(fixture: &Fixture) {
    // This should produce what are effectively 2 resets for the bound collection.
    fixture.add_or_update(&(0..100).map(|i| Person::new(format!("P{i}"), i)).collect::<Vec<_>>());
    fixture.add_or_update(&(100..200).map(|i| Person::new(format!("P{i}"), i)).collect::<Vec<_>>());

    assert_eq!(fixture.bound().len(), 200);
    assert_eq!(fixture.bound(), fixture.sorted(&fixture.source.items()));
}

fn batch_update_shifting_indicies(fixture: &Fixture) {
    let test_data = [Person::new("A", 3), Person::new("B", 5), Person::new("C", 7), Person::new("D", 8), Person::new("E", 10), Person::new("F", 12), Person::new("G", 14)];
    fixture.add_or_update(&test_data);

    fixture.source.edit(|updater| {
        updater.add_or_update(Person::new(test_data[0].name(), 6));
        updater.add_or_update(Person::new(test_data[3].name(), 2));
    });

    let expected = vec![Person::new("D", 2), Person::new("B", 5), Person::new("A", 6), Person::new("C", 7), Person::new("E", 10), Person::new("F", 12), Person::new("G", 14)];

    assert_eq!(fixture.bound(), expected);
}

fn remove_first(fixture: &Fixture) {
    let mut people = fixture.generator.take(100);
    fixture.add_or_update(&people);

    assert_eq!(fixture.bound().len(), 100);

    fixture.source.edit(|updater| updater.remove_key(&people[0].key()));
    assert_eq!(fixture.bound().len(), 99);

    people.remove(0);

    assert_eq!(fixture.sorted(&people), fixture.bound());
}

fn remove_from_end(fixture: &Fixture) {
    let mut people = fixture.generator.take(100);
    fixture.add_or_update(&people);

    assert_eq!(fixture.bound().len(), 100);

    fixture.source.edit(|updater| updater.remove_key(&people[99].key()));
    assert_eq!(fixture.bound().len(), 99);

    people.remove(99);
    assert_eq!(fixture.sorted(&people), fixture.bound());
}

fn remove_from_middle(fixture: &Fixture) {
    let mut people = fixture.generator.take(100);
    fixture.add_or_update(&people);

    assert_eq!(fixture.bound().len(), 100);

    fixture.source.edit(|updater| updater.remove_key(&people[50].key()));
    assert_eq!(fixture.bound().len(), 99);

    let key = people[50].key();
    let index = people.iter().position(|p| p.key() == key).expect("the key is there");
    people.remove(index);

    assert_eq!(fixture.sorted(&people), fixture.bound());
}

fn sort_initial_batch(fixture: &Fixture) {
    let people = fixture.generator.take(100);
    fixture.add_or_update(&people);

    assert_eq!(fixture.bound().len(), 100);
    assert_eq!(fixture.sorted(&people), fixture.bound());
}

fn update_at(fixture: &Fixture, position: impl Fn(&[Rc<Person>]) -> Rc<Person>) {
    let mut people = fixture.generator.take(100);
    fixture.add_or_update(&people);

    let to_update = position(&fixture.bound());

    fixture.add_or_update(&[Person::new(to_update.name(), to_update.age() + 5)]);

    let index = people.iter().position(|p| p.key() == to_update.key()).expect("the key is there");
    people[index] = Person::new(to_update.name(), to_update.age() + 5);

    assert_eq!(fixture.sorted(&people), fixture.bound());
}

fn update_first(fixture: &Fixture) {
    update_at(fixture, |bound| bound[0].clone());
}

fn update_last(fixture: &Fixture) {
    update_at(fixture, |bound| bound[bound.len() - 1].clone());
}

fn update_middle(fixture: &Fixture) {
    update_at(fixture, |bound| bound[50].clone());
}

/// The tests of the abstract fixture for one target.
macro_rules! sort_and_bind_fixture {
    ($module:ident, $set_up:expr) => {
        mod $module {
            use super::Fixture;

            fn fixture() -> Fixture {
                Fixture::new($set_up)
            }

            #[test]
            fn insert_at_beginning() {
                super::insert_at_beginning(&fixture());
            }

            #[test]
            fn insert_at_end() {
                super::insert_at_end(&fixture());
            }

            #[test]
            fn insert_in_middle() {
                super::insert_in_middle(&fixture());
            }

            #[test]
            fn insert_same_location() {
                super::insert_same_location(&fixture());
            }

            #[test]
            fn refresh() {
                super::refresh(&fixture());
            }

            #[test]
            fn batch_of_various_changes() {
                super::batch_of_various_changes(&fixture());
            }

            #[test]
            fn batch_of_various_ending_in_clear() {
                super::batch_of_various_ending_in_clear(&fixture());
            }

            #[test]
            fn large_batch_change() {
                super::large_batch_change(&fixture());
            }

            #[test]
            fn batch_update_shifting_indicies() {
                super::batch_update_shifting_indicies(&fixture());
            }

            #[test]
            fn remove_first() {
                super::remove_first(&fixture());
            }

            #[test]
            fn remove_from_end() {
                super::remove_from_end(&fixture());
            }

            #[test]
            fn remove_from_middle() {
                super::remove_from_middle(&fixture());
            }

            #[test]
            fn sort_initial_batch() {
                super::sort_initial_batch(&fixture());
            }

            #[test]
            fn update_first() {
                super::update_first(&fixture());
            }

            #[test]
            fn update_last() {
                super::update_last(&fixture());
            }

            #[test]
            fn update_middle() {
                super::update_middle(&fixture());
            }
        }
    };
}

sort_and_bind_fixture!(sort_by_and_bind_to_list_fixture, super::sort_by_and_bind_to_list);
sort_and_bind_fixture!(sort_and_bind_to_list_fixture, super::sort_and_bind_to_list);
sort_and_bind_fixture!(sort_and_bind_to_list_default_comparer_fixture, super::sort_and_bind_to_list_default_comparer);
sort_and_bind_fixture!(sort_and_bind_to_observable_collection_fixture, super::sort_and_bind_to_observable_collection);
sort_and_bind_fixture!(sort_and_bind_to_read_only_observable_collection_fixture, super::sort_and_bind_to_read_only_observable_collection);
sort_and_bind_fixture!(sort_and_bind_with_binary_search1_fixture, super::sort_and_bind_with_binary_search1);
sort_and_bind_fixture!(sort_and_bind_with_binary_search2_fixture, super::sort_and_bind_with_binary_search2);
sort_and_bind_fixture!(sort_and_bind_to_read_only_observable_collection_default_comparer_fixture, super::sort_and_bind_to_read_only_observable_collection_default_comparer);

// `SortAndBindBinarySearch_ForSameKeyAndObjectValues`.

fn update_any_where_should_not_break(use_replace_for_updates: bool) {
    let target = Rc::new(RefCell::new(Vec::<i32>::new()));
    let strings = SourceCache::new(|i: &i32| *i);

    let options = SortAndBindOptions { use_binary_search: true, use_replace_for_updates, ..SortAndBindOptions::default() };

    let subscription = strings.connect().sort_and_bind_with(&target, SortExpressionComparer::<i32>::ascending(|i| *i).comparer(), options).subscribe_ignore();

    let items: Vec<i32> = (1..=10).collect();

    strings.edit(|updater| updater.add_or_update_items(&mut items.iter().copied()));
    strings.edit(|updater| updater.add_or_update(1));
    strings.edit(|updater| updater.add_or_update(5));
    strings.edit(|updater| updater.add_or_update(10));

    assert_eq!(*target.borrow(), items);
    subscription.dispose();
    strings.dispose();
}

#[test]
fn sort_and_bind_binary_search_for_same_key_and_object_values_update_any_where_should_not_break_case1() {
    update_any_where_should_not_break(false);
}

#[test]
fn sort_and_bind_binary_search_for_same_key_and_object_values_update_any_where_should_not_break_case2() {
    update_any_where_should_not_break(true);
}

// `SortAndBindWithResetOptions`.

fn reset_options_comparer() -> Comparer {
    SortExpressionComparer::<Rc<Person>>::ascending(|p| p.age()).then_by_ascending(|p| p.name()).comparer()
}

fn people(range: std::ops::Range<i32>) -> Vec<Rc<Person>> {
    range.map(|i| Person::new(format!("P{i}"), i)).collect()
}

fn reset_options_test(options: SortAndBindOptions, expectations: &[(std::ops::Range<i32>, usize, NotifyCollectionChangedAction)], final_count: usize) {
    let source = SourceCache::new(|p: &Rc<Person>| p.key());
    let collection_changed_event_args: Rc<RefCell<Vec<SharedNotifyCollectionChangedEventArgs<Rc<Person>>>>> = Rc::new(RefCell::new(Vec::new()));

    let (sorted, list) = source.connect().sort_and_bind_read_only_with(reset_options_comparer(), options);
    let sorted = sorted.subscribe_ignore();
    let target = collection_changed_event_args.clone();
    let collection_changed_events = list.observe_collection_changes().select(|e| e.event_args).subscribe_next(move |args| target.borrow_mut().push(args));

    for (range, count, action) in expectations {
        source.edit(|updater| updater.add_or_update_items(&mut people(range.clone()).into_iter()));
        let events = collection_changed_event_args.borrow().clone();
        assert_eq!(events.len(), *count);
        assert!(events.iter().all(|a| a.action() == *action));
        collection_changed_event_args.borrow_mut().clear();
    }

    assert_eq!(list.count(), final_count);

    collection_changed_events.dispose();
    sorted.dispose();
    source.dispose();
}

#[test]
fn sort_and_bind_with_reset_options_fires_reset_when_threshold_is_met() {
    // Check reset is fired when below threshold only. Historically first time load always fired
    // reset for first time load.
    let options = SortAndBindOptions { reset_threshold: 10, ..SortAndBindOptions::default() };
    reset_options_test(
        options,
        &[
            // Fire 5 changes, should always reset because it's below the threshold.
            (0..5, 5, NotifyCollectionChangedAction::Add),
            // Fire 15 changes, we should get a refresh event.
            (10..25, 1, NotifyCollectionChangedAction::Reset),
            // Fires further 5 changes, should result individual notifications.
            (-10..-5, 5, NotifyCollectionChangedAction::Add),
        ],
        25,
    );
}

#[test]
fn sort_and_bind_with_reset_options_never_fire_reset() {
    // Check reset is not fired.
    let options = SortAndBindOptions { reset_threshold: usize::MAX, ..SortAndBindOptions::default() };
    reset_options_test(
        options,
        &[
            // Fire 5 changes, should not reset because it's below the threshold.
            (0..5, 5, NotifyCollectionChangedAction::Add),
            // Fire 15 changes.
            (10..25, 15, NotifyCollectionChangedAction::Add),
        ],
        20,
    );
}

#[test]
fn sort_and_bind_with_reset_options_fire_reset_on_first_time_load() {
    // Check reset is fired on first time load. This checks historic first time load opt-in.
    let options = SortAndBindOptions { reset_threshold: 10, reset_on_first_time_load: true, ..SortAndBindOptions::default() };
    reset_options_test(
        options,
        &[
            // Fire 5 changes, should always reset even though it's below the threshold.
            (0..5, 1, NotifyCollectionChangedAction::Reset),
            // Fire 15 changes, we should get a refresh event.
            (10..25, 1, NotifyCollectionChangedAction::Reset),
            // Fires further 5 changes, should result individual notifications.
            (-10..-5, 5, NotifyCollectionChangedAction::Add),
        ],
        25,
    );
}
