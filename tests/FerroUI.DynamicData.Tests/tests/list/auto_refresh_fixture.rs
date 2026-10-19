//! `AutoRefreshFixture` (`DynamicData.Tests/List/AutoRefreshFixture.cs`): the refreshes of a list
//! when a property of an item changes, and how the operators downstream (`DistinctValues`,
//! `Filter`, `GroupOn`, `GroupWithImmutableState`, `Sort`, `Transform`) follow them.

use crate::domain::Person;
use crate::utilities::assert_equivalent;
use ferroui_dynamic_data::binding::{AbstractNotifyPropertyChanged, SortExpressionComparer};
use ferroui_dynamic_data::list::{ChangeSet, Group, IGrouping, IObservableList, ImmutableGroup, ListChangeReason, ObservableListEx, SourceList};
use ferroui_reactive::ObservableExt;
use ferroui_reactive_testing::{duration_to_ticks, TestScheduler};
use ferroui_system::component_model::INotifyPropertyChanged;
use ferroui_system::events::Event;
use ferroui_system::{prop, IDisposable};
use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::rc::Rc;
use std::time::{Duration, SystemTime};

fn add<T: Clone + PartialEq + 'static>(list: &SourceList<T>, item: T) {
    list.edit(|list| list.add(item));
}

fn add_range<T: Clone + PartialEq + 'static>(list: &SourceList<T>, items: &[T]) {
    let items = items.to_vec();
    list.edit(|list| list.add_range(items));
}

fn remove<T: Clone + PartialEq + 'static>(list: &SourceList<T>, item: &T) {
    list.edit(|list| {
        list.remove(item);
    });
}

fn remove_range<T: Clone + PartialEq + 'static>(list: &SourceList<T>, index: usize, count: usize) {
    list.edit(|list| list.remove_range(index, count));
}

fn sorted_by_name(people: impl IntoIterator<Item = Rc<Person>>) -> Vec<Rc<Person>> {
    let mut people: Vec<Rc<Person>> = people.into_iter().collect();
    people.sort_by_key(|p| p.name());
    people
}

/// `items.GroupBy(p => p.Age % 10)`.
fn group_by_age_mod_10(items: &[Rc<Person>]) -> BTreeMap<i32, Vec<Rc<Person>>> {
    let mut groupings: BTreeMap<i32, Vec<Rc<Person>>> = BTreeMap::new();
    for item in items {
        groupings.entry(item.age() % 10).or_default().push(item.clone());
    }
    groupings
}

#[test]
fn auto_refresh() {
    let items: Vec<Rc<Person>> = (1..=100).map(|i| Person::new(format!("Person{i}"), 1)).collect();

    // result should only be true when all items are set to true
    let list = SourceList::<Rc<Person>>::new();
    let results = list.connect().auto_refresh(prop!(Person => age)).as_aggregator();
    add_range(&list, &items);

    assert_eq!(results.data().count(), 100);
    assert_eq!(results.messages().len(), 1);

    items[0].set_age(10);
    assert_eq!(results.data().count(), 100);
    assert_eq!(results.messages().len(), 2);

    assert_eq!(results.messages()[1].changes()[0].reason(), ListChangeReason::Refresh);

    // remove an item and check no change is fired
    let to_remove = items[1].clone();
    remove(&list, &to_remove);
    assert_eq!(results.data().count(), 99);
    assert_eq!(results.messages().len(), 3);
    to_remove.set_age(100);
    assert_eq!(results.messages().len(), 3);

    // add it back in and check it updates
    add(&list, to_remove.clone());
    assert_eq!(results.messages().len(), 4);
    to_remove.set_age(101);
    assert_eq!(results.messages().len(), 5);

    assert_eq!(results.messages().last().unwrap().changes()[0].reason(), ListChangeReason::Refresh);

    results.dispose();
    list.dispose();
}

#[test]
fn auto_refresh_batched() {
    let scheduler = TestScheduler::new();

    let items: Vec<Rc<Person>> = (1..=100).map(|i| Person::new(format!("Person{i}"), 1)).collect();

    // result should only be true when all items are set to true
    let list = SourceList::<Rc<Person>>::new();
    let results = list.connect().auto_refresh_with(prop!(Person => age), Some(Duration::from_secs(1)), None, Some(scheduler.scheduler())).as_aggregator();
    add_range(&list, &items);

    assert_eq!(results.data().count(), 100);
    assert_eq!(results.messages().len(), 1);

    // update 50 records
    for p in items.iter().skip(50) {
        p.set_age(p.age() + 1);
    }

    scheduler.advance_by(duration_to_ticks(Duration::from_secs(1)));

    // should be another message with 50 refreshes
    assert_eq!(results.messages().len(), 2);
    assert_eq!(results.messages()[1].refreshes(), 50);

    results.dispose();
    list.dispose();
}

#[test]
fn auto_refresh_distinct() {
    let items: Vec<Rc<Person>> = (1..=100).map(|i| Person::new(format!("Person{i}"), i)).collect();

    // result should only be true when all items are set to true
    let list = SourceList::<Rc<Person>>::new();
    let results = list.connect().auto_refresh(prop!(Person => age)).distinct_values(|p: &Rc<Person>| p.age() / 10).as_aggregator();
    add_range(&list, &items);

    assert_eq!(results.data().count(), 11);
    assert_eq!(results.messages().len(), 1);

    // update an item which did not match the filter and does so after change
    items[50].set_age(500);
    assert_eq!(results.data().count(), 12);

    let last: ChangeSet<i32> = results.messages().last().unwrap().clone();
    assert_eq!(last.changes()[0].reason(), ListChangeReason::Add);
    assert_eq!(*last.changes()[0].item().current(), 50);

    results.dispose();
    list.dispose();
}

#[test]
fn auto_refresh_filter() {
    let items: Vec<Rc<Person>> = (1..=100).map(|i| Person::new(format!("Person{i}"), i)).collect();

    // result should only be true when all items are set to true
    let list = SourceList::<Rc<Person>>::new();
    let results = list.connect().auto_refresh(prop!(Person => age)).filter(|p: &Rc<Person>| p.age() > 50).as_aggregator();
    add_range(&list, &items);

    assert_eq!(results.data().count(), 50);
    assert_eq!(results.messages().len(), 1);

    // update an item which did not match the filter and does so after change
    items[0].set_age(60);
    assert_eq!(results.data().count(), 51);
    assert_eq!(results.messages().len(), 2);
    assert_eq!(results.messages()[1].changes()[0].reason(), ListChangeReason::Add);

    // check for removes
    items[0].set_age(21);
    assert_eq!(results.data().count(), 50);
    assert_eq!(results.messages().last().unwrap().changes()[0].reason(), ListChangeReason::Remove);
    items[0].set_age(60);

    // update an item which matched the filter and still does [refresh should have propagated]
    items[60].set_age(160);
    assert_eq!(results.data().count(), 51);
    assert_eq!(results.messages().len(), 5);
    assert_eq!(results.messages().last().unwrap().changes()[0].reason(), ListChangeReason::Refresh);

    // remove an item and check no change is fired
    let to_remove = items[65].clone();
    remove(&list, &to_remove);
    assert_eq!(results.data().count(), 50);
    assert_eq!(results.messages().len(), 6);
    to_remove.set_age(100);
    assert_eq!(results.messages().len(), 6);

    // add it back in and check it updates
    add(&list, to_remove.clone());
    assert_eq!(results.messages().len(), 7);
    to_remove.set_age(101);
    assert_eq!(results.messages().len(), 8);

    assert_eq!(results.messages().last().unwrap().changes()[0].reason(), ListChangeReason::Refresh);

    results.dispose();
    list.dispose();
}

#[test]
fn auto_refresh_group() {
    let items: Vec<Rc<Person>> = (1..=100).map(|i| Person::new(format!("Person{i}"), i)).collect();

    // result should only be true when all items are set to true
    let list = SourceList::<Rc<Person>>::new();
    let results = list.connect().auto_refresh(prop!(Person => age)).group_on(|p: &Rc<Person>| p.age() % 10).as_aggregator();
    let check_content = || {
        let groups: Vec<Group<Rc<Person>, i32>> = results.data().items();
        for (key, grouping) in group_by_age_mod_10(&items) {
            let matching: Vec<&Group<Rc<Person>, i32>> = groups.iter().filter(|g| *g.group_key() == key).collect();
            assert_eq!(matching.len(), 1, "a single group of the key {key}");
            let child_group = matching[0];
            let expected = sorted_by_name(grouping);
            let actual = sorted_by_name(child_group.list().items());
            assert_equivalent(&actual, &expected);
        }
    };

    add_range(&list, &items);
    assert_eq!(results.data().count(), 10);
    assert_eq!(results.messages().len(), 1);
    check_content();

    // move person from group 1 to 2
    items[0].set_age(items[0].age() + 1);
    check_content();

    // change the value and move to a grouping which does not yet exist
    items[1].set_age(-1);
    assert_eq!(results.data().count(), 11);
    let groups = results.data().items();
    assert_eq!(*groups.last().unwrap().group_key(), -1);
    assert_eq!(groups.last().unwrap().list().count(), 1);
    assert_eq!(groups[0].list().count(), 9);
    check_content();

    // put the value back where it was and check the group was removed
    items[1].set_age(1);
    assert_eq!(results.data().count(), 10);
    check_content();

    let group_of3 = results.data().items()[2].clone();

    let changes: Rc<RefCell<Option<ChangeSet<Rc<Person>>>>> = Rc::new(RefCell::new(None));
    let captured = changes.clone();
    let _subscription = group_of3.list().connect().subscribe_next(move |c| *captured.borrow_mut() = Some(c));

    // refresh an item which makes it belong to the same group - should then propagate a refresh
    items[2].set_age(13);
    {
        let changes = changes.borrow();
        assert!(changes.is_some());
        let changes = changes.as_ref().unwrap();
        assert_eq!(changes.count(), 1);
        assert_eq!(changes.changes()[0].reason(), ListChangeReason::Replace);
        assert!(Rc::ptr_eq(changes.changes()[0].item().current(), &items[2]));
    }

    results.dispose();
    list.dispose();
}

#[test]
fn auto_refresh_group_immutable() {
    let items: Vec<Rc<Person>> = (1..=100).map(|i| Person::new(format!("Person{i}"), i)).collect();

    // result should only be true when all items are set to true
    let list = SourceList::<Rc<Person>>::new();
    let results = list.connect().auto_refresh(prop!(Person => age)).group_with_immutable_state(|p: &Rc<Person>| p.age() % 10).as_aggregator();
    let check_content = || {
        let groups: Vec<ImmutableGroup<Rc<Person>, i32>> = results.data().items();
        for (key, grouping) in group_by_age_mod_10(&items) {
            let matching: Vec<&ImmutableGroup<Rc<Person>, i32>> = groups.iter().filter(|g| *g.key() == key).collect();
            assert_eq!(matching.len(), 1, "a single group of the key {key}");
            let child_group = matching[0];
            let expected = sorted_by_name(grouping);
            let actual = sorted_by_name(child_group.items());
            assert_equivalent(&actual, &expected);
        }
    };

    add_range(&list, &items);
    assert_eq!(results.data().count(), 10);
    assert_eq!(results.messages().len(), 1);
    check_content();

    // move person from group 1 to 2
    items[0].set_age(items[0].age() + 1);
    check_content();

    // change the value and move to a grouping which does not yet exist
    items[1].set_age(-1);
    assert_eq!(results.data().count(), 11);
    let groups = results.data().items();
    assert_eq!(*groups.last().unwrap().key(), -1);
    assert_eq!(IGrouping::count(groups.last().unwrap()), 1);
    assert_eq!(IGrouping::count(&groups[0]), 9);
    check_content();

    // put the value back where it was and check the group was removed
    items[1].set_age(1);
    assert_eq!(results.data().count(), 10);
    assert_eq!(results.messages().len(), 4);
    check_content();

    // refresh an item which makes it belong to the same group - should then propagate a refresh
    items[2].set_age(13);
    check_content();

    assert_eq!(results.messages().len(), 5);

    results.dispose();
    list.dispose();
}

#[test]
fn auto_refresh_selected() {
    // test added as v6 broke unit test in DynamicData.Snippets
    let initial_items: Vec<Rc<SelectableItem>> = (1..=10).map(SelectableItem::new).collect();

    // result should only be true when all items are set to true
    let source_list = SourceList::<Rc<SelectableItem>>::new();
    let sut = source_list.connect().auto_refresh_any().filter(|si: &Rc<SelectableItem>| si.is_selected()).as_observable_list();
    add_range(&source_list, &initial_items);
    assert_eq!(sut.count(), 0);

    initial_items[0].set_is_selected(true);
    assert_eq!(sut.count(), 1);

    initial_items[1].set_is_selected(true);
    assert_eq!(sut.count(), 2);

    // remove the selected items
    remove_range(&source_list, 0, 2);
    assert_eq!(sut.count(), 0);

    sut.dispose();
    source_list.dispose();
}

#[test]
fn auto_refresh_sort() {
    let mut items: Vec<Rc<Person>> = (1..=100).map(|i| Person::new(format!("Person{i}"), i)).collect();
    items.sort_by_key(|p| std::cmp::Reverse(p.age()));

    let comparer = SortExpressionComparer::ascending(|p: &Rc<Person>| p.age()).comparer();

    // result should only be true when all items are set to true
    let list = SourceList::<Rc<Person>>::new();
    let results = list.connect().auto_refresh(prop!(Person => age)).sort(SortExpressionComparer::ascending(|p: &Rc<Person>| p.age()).comparer()).as_aggregator();
    let check_order = || {
        let mut sorted = items.clone();
        sorted.sort_by(|x, y| comparer(x, y));
        assert_equivalent(&results.data().items(), &sorted);
    };

    add_range(&list, &items);

    assert_eq!(results.data().count(), 100);
    assert_eq!(results.messages().len(), 1);
    check_order();

    items[0].set_age(60);
    check_order();
    assert_eq!(results.messages().len(), 2);
    assert_eq!(results.messages().last().unwrap().refreshes(), 1);
    assert_eq!(results.messages().last().unwrap().moves(), 1);

    items[90].set_age(-1); // move to beginning
    check_order();
    assert_eq!(results.messages().len(), 3);
    assert_eq!(results.messages().last().unwrap().refreshes(), 1);
    assert_eq!(results.messages().last().unwrap().moves(), 1);

    items[50].set_age(49); // same position so no move
    check_order();
    assert_eq!(results.messages().len(), 4);
    assert_eq!(results.messages().last().unwrap().refreshes(), 1);
    assert_eq!(results.messages().last().unwrap().moves(), 0);

    items[50].set_age(51); // same position so no move
    check_order();
    assert_eq!(results.messages().len(), 5);
    assert_eq!(results.messages().last().unwrap().refreshes(), 1);
    assert_eq!(results.messages().last().unwrap().moves(), 1);

    results.dispose();
    list.dispose();
}

#[test]
fn auto_refresh_transform() {
    let items: Vec<Rc<Person>> = (1..=100).map(|i| Person::new(format!("Person{i}"), i)).collect();

    // result should only be true when all items are set to true
    let list = SourceList::<Rc<Person>>::new();
    let results = list.connect().auto_refresh(prop!(Person => age)).transform_with_index(|p: &Rc<Person>, idx: usize| TransformedPerson::new(p.clone(), idx)).as_aggregator();
    add_range(&list, &items);

    assert_eq!(results.data().count(), 100);
    assert_eq!(results.messages().len(), 1);

    // update an item which did not match the filter and does so after change
    items[0].set_age(60);
    assert_eq!(results.messages().len(), 2);
    assert_eq!(results.messages().last().unwrap().refreshes(), 1);
    assert_eq!(results.messages().last().unwrap().changes()[0].item().reason(), ListChangeReason::Refresh);
    assert_eq!(results.messages().last().unwrap().changes()[0].item().current().index, 0);

    items[60].set_age(160);
    assert_eq!(results.messages().len(), 3);
    assert_eq!(results.messages().last().unwrap().refreshes(), 1);
    assert_eq!(results.messages().last().unwrap().changes()[0].item().reason(), ListChangeReason::Refresh);
    assert_eq!(results.messages().last().unwrap().changes()[0].item().current().index, 60);

    results.dispose();
    list.dispose();
}

#[test]
fn refresh_transform_as_list() {
    let list = SourceList::<Rc<Example>>::new();
    let value_list = list.connect().auto_refresh(prop!(Example => value)).transform_with(|e: &Rc<Example>| e.value(), true).as_observable_list();

    let obj = Example::new(0);
    add(&list, obj.clone());
    obj.set_value(1);
    assert_eq!(value_list.items()[0], 1);
}

/// `Example`: an object whose value raises `PropertyChanged`. As upstream's class, it is equal only
/// to itself.
struct Example {
    notify: AbstractNotifyPropertyChanged,
    value: Cell<i32>,
}

impl Example {
    /// `new Example { Value = value }`.
    fn new(value: i32) -> Rc<Self> {
        Rc::new(Self { notify: AbstractNotifyPropertyChanged::new(), value: Cell::new(value) })
    }

    fn value(&self) -> i32 {
        self.value.get()
    }

    fn set_value(&self, value: i32) {
        self.notify.set_and_raise(&self.value, value, "Value");
    }
}

impl PartialEq for Example {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl INotifyPropertyChanged for Example {
    fn property_changed(&self) -> &Event<str> {
        self.notify.property_changed()
    }
}

/// `SelectableItem`: an item with an id, which is its identity, and a selection that raises
/// `PropertyChanged`.
struct SelectableItem {
    notify: AbstractNotifyPropertyChanged,
    id: i32,
    is_selected: Cell<bool>,
}

impl SelectableItem {
    fn new(id: i32) -> Rc<Self> {
        Rc::new(Self { notify: AbstractNotifyPropertyChanged::new(), id, is_selected: Cell::new(false) })
    }

    fn is_selected(&self) -> bool {
        self.is_selected.get()
    }

    fn set_is_selected(&self, value: bool) {
        self.notify.set_and_raise(&self.is_selected, value, "IsSelected");
    }
}

impl PartialEq for SelectableItem {
    /// Items of the same id are equal (`Equals`).
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other) || self.id == other.id
    }
}

impl INotifyPropertyChanged for SelectableItem {
    fn property_changed(&self) -> &Event<str> {
        self.notify.property_changed()
    }
}

/// `TransformedPerson`: a person with its index at the time of the transform. As upstream's class,
/// it is equal only to itself.
struct TransformedPerson {
    index: usize,
    #[allow(dead_code)]
    person: Rc<Person>,
    #[allow(dead_code)]
    time_stamp: SystemTime,
}

impl TransformedPerson {
    fn new(person: Rc<Person>, index: usize) -> Rc<Self> {
        Rc::new(Self { index, person, time_stamp: SystemTime::now() })
    }
}

impl PartialEq for TransformedPerson {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
