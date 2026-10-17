//! Tests of the value types and helpers the core is built on (not a file of the reference, whose
//! tests reach these through the sources).

use crate::accessor::{Accessor, PropertyPathObserver};
use crate::index_path;
use crate::models::NotifyingListBase;
use crate::selection::{IndexRange, IndexRanges};
use crate::utils::{sort_helper, stable_sort};
use crate::{GridLength, GridUnitType, IndexPath};
use ferroui_base::collections::NotifyCollectionChangedAction;
use ferroui_base::data::model::{Event, INotifyPropertyChanged, Model, ModelTypeBuilder};
use ferroui_base::BoxedValue;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[test]
fn index_paths_compare_by_index_and_then_by_length() {
    assert!(IndexPath::new(0) < IndexPath::new(1));
    assert!(IndexPath::new(1) < index_path![1, 0]);
    assert!(index_path![0, 5] < IndexPath::new(1));
    assert!(IndexPath::UNSELECTED < IndexPath::new(0));
    assert_eq!(IndexPath::new(3), index_path![3]);
    assert_eq!(IndexPath::UNSELECTED, index_path![]);
    assert_eq!(index_path![1, 2, 3], IndexPath::new(1).append(2).append(3));
    assert_eq!(index_path![1, 2], index_path![1, 2, 3].parent());
    assert_eq!(IndexPath::new(1), index_path![1, 2].parent());
    assert_eq!(3, index_path![1, 2, 3].last());
    assert!(index_path![1].is_ancestor_of(&index_path![1, 2, 3]));
    assert!(!index_path![1, 2, 3].is_ancestor_of(&index_path![1, 2, 3]));
    assert!(index_path![1, 2].is_parent_of(&index_path![1, 2, 3]));
    assert!(!index_path![1].is_parent_of(&index_path![1, 2, 3]));
    assert_eq!("(1.2.3)", index_path![1, 2, 3].to_string());
    assert_eq!("(4)", IndexPath::new(4).to_string());
    assert_eq!("()", IndexPath::UNSELECTED.to_string());
    let mut set = std::collections::HashSet::new();
    set.insert(index_path![7]);
    assert!(set.contains(&IndexPath::new(7)), "equal paths hash alike however they are stored");
}

#[test]
fn grid_lengths_parse_and_print_as_the_reference() {
    assert_eq!(GridLength::AUTO, GridLength::parse("auto"));
    assert_eq!(GridLength::new(2.0, GridUnitType::Star), GridLength::parse("2*"));
    assert_eq!(GridLength::STAR, GridLength::parse("*"));
    assert_eq!(GridLength::pixels(120.0), GridLength::parse(" 120 "));
    assert_eq!("Auto", GridLength::AUTO.to_string());
    assert_eq!("1.5*", GridLength::new(1.5, GridUnitType::Star).to_string());
    assert!(GridLength::try_parse("wide").is_none());
}

#[test]
fn the_introspective_sort_and_the_stable_map_order_as_a_comparison_says() {
    let mut values: Vec<i32> = (0..200).map(|i| (i * 7919) % 211).collect();
    let mut expected = values.clone();
    expected.sort();
    sort_helper::sort(&mut values, &mut |a, b| a.cmp(b));
    assert_eq!(expected, values);

    let keys = [3, 1, 3, 2, 1];
    let map = stable_sort::sorted_map(keys.len(), &mut |a, b| keys[*a as usize].cmp(&keys[*b as usize]).then(a.cmp(b)));
    assert_eq!(vec![1, 4, 3, 0, 2], map);

    let sorted = [1, 3, 5, 7];
    assert_eq!(2, sort_helper::binary_search(&sorted, &5, &mut |a, b| a.cmp(b)));
    assert_eq!(!2, sort_helper::binary_search(&sorted, &4, &mut |a, b| a.cmp(b)));
}

#[test]
fn index_ranges_hold_paths_per_parent() {
    let mut ranges = IndexRanges::new();
    ranges.add(&index_path![0, 1]);
    ranges.add(&index_path![0, 2]);
    ranges.add(&index_path![3]);
    ranges.add_range(&index_path![0], IndexRange::new(5, 6));
    assert_eq!(5, ranges.count());
    assert_eq!(
        vec![index_path![3], index_path![0, 1], index_path![0, 2], index_path![0, 5], index_path![0, 6]],
        ranges.to_vec()
    );
    assert!(ranges.contains(&index_path![0, 5]));
    assert!(ranges.contains_descendents(&IndexPath::new(0)));
    assert!(ranges.remove(&index_path![0, 5]));
    assert!(!ranges.remove(&index_path![0, 5]));
    assert_eq!(4, ranges.count());
    assert_eq!(index_path![0, 6], ranges.get(3));
}

#[test]
fn a_notifying_list_raises_one_notification_per_batch() {
    let list = NotifyingListBase::<i32>::new();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let recorded = changes.clone();
    list.add_collection_changed(Rc::new(move |e| {
        recorded.borrow_mut().push((e.action, e.new_items.to_vec(), e.old_items.to_vec(), e.new_starting_index, e.old_starting_index));
    }));
    list.add(1);
    list.insert_range(1, |insert| {
        insert(2);
        insert(3);
    });
    list.remove_range(0, 2);
    list.move_item(0, 0);
    list.reset(|l| l.clear());
    assert_eq!(
        vec![
            (NotifyCollectionChangedAction::Add, vec![1], vec![], 0, -1),
            (NotifyCollectionChangedAction::Add, vec![2, 3], vec![], 1, -1),
            (NotifyCollectionChangedAction::Remove, vec![], vec![1, 2], -1, 0),
            (NotifyCollectionChangedAction::Move, vec![3], vec![3], 0, 0),
            (NotifyCollectionChangedAction::Reset, vec![], vec![], -1, -1),
        ],
        *changes.borrow()
    );
}

#[derive(Default)]
struct Expansion {
    is_expanded: Cell<bool>,
    changed: Event<str>,
}

impl PartialEq for Expansion {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl INotifyPropertyChanged for Expansion {
    fn property_changed(&self) -> &Event<str> {
        &self.changed
    }
}

impl Model for Expansion {
    fn describe(builder: ModelTypeBuilder<Self>) -> ModelTypeBuilder<Self> {
        builder.notify_property_changed()
    }
}

struct Person {
    expansion: RefCell<Rc<Expansion>>,
    changed: Event<str>,
}

impl PartialEq for Person {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl INotifyPropertyChanged for Person {
    fn property_changed(&self) -> &Event<str> {
        &self.changed
    }
}

impl Model for Person {
    fn describe(builder: ModelTypeBuilder<Self>) -> ModelTypeBuilder<Self> {
        builder.notify_property_changed()
    }
}

#[test]
fn a_path_observer_follows_the_owners_of_a_nested_member() {
    Expansion::register();
    Person::register();
    let accessor = Accessor::<Person, bool>::member("Expansion", |p| p.expansion.borrow().is_expanded.get())
        .through(|p| Some(p.expansion.borrow().clone() as BoxedValue), "IsExpanded");
    let observer = PropertyPathObserver::create(&accessor).expect("a path below the model");
    assert!(PropertyPathObserver::create(&Accessor::<Person, bool>::member("Flag", |_| true)).is_none());

    let first = Rc::new(Expansion::default());
    let person = Rc::new(Person { expansion: RefCell::new(first.clone()), changed: Event::new() });
    let count = Rc::new(Cell::new(0));
    let counter = count.clone();
    let subscription = observer.subscribe(&person, Rc::new(move || counter.set(counter.get() + 1)));

    first.changed.raise("IsExpanded");
    assert_eq!(1, count.get());
    first.changed.raise("Other");
    assert_eq!(1, count.get(), "another member of the owner");

    // The model replaces the owner: the old owner is no longer observed, the new one is.
    let second = Rc::new(Expansion::default());
    *person.expansion.borrow_mut() = second.clone();
    person.changed.raise("Expansion");
    assert_eq!(2, count.get());
    first.changed.raise("IsExpanded");
    assert_eq!(2, count.get());
    second.changed.raise("IsExpanded");
    assert_eq!(3, count.get());

    drop(subscription);
    second.changed.raise("IsExpanded");
    assert_eq!(3, count.get());
    assert!(!second.changed.has_handlers() && !person.changed.has_handlers());
}
