use super::{IList, ListHandle};
use ferroui_system::collections::object_model::ObservableCollection;
use std::cell::{Cell, RefCell};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::{Rc, Weak};

#[test]
fn plain_lists_keep_identity_shared_mutation_and_the_complete_mutable_contract() {
    let list = ListHandle::from_plain(vec![1, 2, 3]);
    let alias = list.clone();
    assert!(list.ptr_eq(&alias) && !list.is_read_only());
    assert!(list.as_notify_collection_changed().is_none());
    assert!(list.as_observable_collection().is_none());
    assert!(!list.ptr_eq(&ListHandle::from_plain(vec![1, 2, 3])));
    list.set(1, 4);
    alias.insert(0, 5);
    list.add(6);
    assert_eq!(alias.to_vec(), [5, 1, 4, 3, 6]);
    assert_eq!(list.index_of(&4), Some(2));
    assert!(list.contains(&6));
    assert!(!list.remove(&7));
    assert!(list.remove(&4));
    list.remove_at(0);
    let mut destination = [-1; 5];
    list.copy_to(&mut destination, 1);
    assert_eq!(destination, [-1, 1, 3, 6, -1]);
    list.clear();
    assert!(alias.is_empty());
}

#[test]
fn separate_observable_adapters_preserve_original_collection_identity_and_notifications() {
    let original = ObservableCollection::from_items(vec![1, 2]);
    let first = ListHandle::from_observable(original.clone());
    let second = ListHandle::from_observable(original.clone());
    assert!(first.ptr_eq(&second));
    assert!(first.as_observable_collection().unwrap().ptr_eq(&original));
    assert!(!first.ptr_eq(&ListHandle::from_items(vec![1, 2])));
    let changes = Rc::new(RefCell::new(Vec::new()));
    let log = changes.clone();
    let event = first
        .as_notify_collection_changed()
        .unwrap()
        .collection_changed();
    let token = event.add(Rc::new(move |change| log.borrow_mut().push(*change)));
    original.add(3);
    first.set(0, 9);
    first.insert(1, 8);
    second.remove_at(2);
    second.clear();
    assert!(original.is_empty());
    assert_eq!(changes.borrow().len(), 5);
    assert_eq!(changes.borrow()[0].new_starting_index, 2);
    assert!(event.remove(token));
}

#[test]
fn observable_adapter_does_not_add_a_subscriber_that_changes_reentrancy_rules() {
    let original = ObservableCollection::new();
    let list = ListHandle::from_observable(original.clone());
    let once = Rc::new(Cell::new(false));
    let entered = once.clone();
    let target = original.clone();
    let token = original.add_collection_changed(Rc::new(move |_| {
        if !entered.replace(true) {
            target.add(2);
        }
    }));
    list.add(1);
    assert_eq!(list.to_vec(), [1, 2]);
    original.remove_collection_changed(token);
}

#[test]
fn every_plain_list_mutation_invalidates_existing_enumerators() {
    type Mutation = fn(&ListHandle<i32>);
    for mutate in [
        |list: &ListHandle<i32>| list.add(4),
        |list| list.insert(0, 4),
        |list| list.set(0, 1),
        |list| list.remove_at(0),
        |list| list.clear(),
    ] as [Mutation; 5]
    {
        let list = ListHandle::from_plain(vec![1, 2, 3]);
        let mut enumerator = list.iter();
        assert_eq!(enumerator.next(), Some(1));
        mutate(&list);
        assert!(catch_unwind(AssertUnwindSafe(|| enumerator.next())).is_err());
    }
}

#[test]
fn enumeration_is_independent_live_and_still_checks_version_after_completion() {
    let list = ListHandle::from_plain(vec![1, 2]);
    let mut first = list.iter();
    let mut second = list.iter();
    assert_eq!(first.next(), Some(1));
    assert_eq!(first.next(), Some(2));
    assert_eq!(second.next(), Some(1));
    assert_eq!(first.next(), None);
    list.clear();
    assert!(catch_unwind(AssertUnwindSafe(|| first.next())).is_err());
    let empty = ListHandle::<i32>::from_plain(Vec::new());
    let mut enumerator = empty.iter();
    empty.clear();
    assert!(catch_unwind(AssertUnwindSafe(|| enumerator.next())).is_err());
}

#[test]
fn copy_to_rejects_invalid_destination_ranges() {
    let list = ListHandle::from_plain(vec![1, 2]);
    assert!(catch_unwind(AssertUnwindSafe(|| list.copy_to(&mut [0; 2], 1))).is_err());
    assert!(catch_unwind(AssertUnwindSafe(|| list.copy_to(&mut [0; 2], 3))).is_err());
}

struct ReadOnlySnapshotList {
    items: Vec<i32>,
    enumerations: Cell<usize>,
}
impl IList<i32> for ReadOnlySnapshotList {
    fn count(&self) -> usize {
        self.items.len()
    }
    fn is_read_only(&self) -> bool {
        true
    }
    fn get(&self, index: usize) -> i32 {
        self.items[index]
    }
    fn set(&self, _: usize, _: i32) {
        panic!("read-only");
    }
    fn insert(&self, _: usize, _: i32) {
        panic!("read-only");
    }
    fn remove_at(&self, _: usize) {
        panic!("read-only");
    }
    fn clear(&self) {
        panic!("read-only");
    }
    fn index_of(&self, item: &i32) -> Option<usize> {
        self.items.iter().position(|candidate| candidate == item)
    }
    fn get_enumerator(self: Rc<Self>) -> Box<dyn Iterator<Item = i32>> {
        self.enumerations.set(self.enumerations.get() + 1);
        Box::new(self.items.clone().into_iter())
    }
}

#[test]
fn custom_lists_keep_their_own_read_only_and_enumeration_behavior() {
    let custom = Rc::new(ReadOnlySnapshotList {
        items: vec![3, 5],
        enumerations: Cell::new(0),
    });
    let list = ListHandle::from_list(custom.clone());
    assert!(Rc::ptr_eq(
        &list.as_list(),
        &(custom.clone() as Rc<dyn IList<i32>>)
    ));
    assert!(list.is_read_only());
    assert!(list.contains(&5));
    assert_eq!(list.get(0), 3);
    assert_eq!(list.iter().collect::<Vec<_>>(), [3, 5]);
    assert_eq!(custom.enumerations.get(), 1);
    assert_eq!(list.to_vec(), [3, 5]);
    assert_eq!(custom.enumerations.get(), 2);
    assert!(catch_unwind(AssertUnwindSafe(|| list.add(7))).is_err());
    assert!(catch_unwind(AssertUnwindSafe(|| list.set(0, 7))).is_err());
    assert!(catch_unwind(AssertUnwindSafe(|| list.remove(&3))).is_err());
    assert!(catch_unwind(AssertUnwindSafe(|| list.clear())).is_err());
}

#[derive(Clone)]
struct WithoutPartialEq(i32);
#[test]
fn explicit_comparers_support_element_types_without_partial_eq() {
    let list = ListHandle::from_plain_with_comparer(
        vec![WithoutPartialEq(1)],
        Rc::new(|left, right| left.0 == right.0),
    );
    assert!(list.contains(&WithoutPartialEq(1)));
    assert!(!list.contains(&WithoutPartialEq(2)));
    assert!(list.remove(&WithoutPartialEq(1)));
    assert!(list.is_empty());
}

#[test]
fn element_comparer_callbacks_can_mutate_the_list_without_an_interior_borrow() {
    let target: Rc<RefCell<Option<Weak<dyn IList<i32>>>>> = Rc::new(RefCell::new(None));
    let callback = target.clone();
    let list = ListHandle::from_plain_with_comparer(
        vec![1],
        Rc::new(move |left, right| {
            callback
                .borrow()
                .as_ref()
                .unwrap()
                .upgrade()
                .unwrap()
                .clear();
            left == right
        }),
    );
    *target.borrow_mut() = Some(Rc::downgrade(&list.as_list()));
    assert_eq!(list.index_of(&1), Some(0));
    assert!(list.is_empty());
}

struct DropProbe(Rc<dyn Fn()>);
impl Drop for DropProbe {
    fn drop(&mut self) {
        (self.0)();
    }
}
#[test]
fn removed_values_drop_after_collection_borrows_and_version_updates_are_released() {
    let list =
        ListHandle::from_plain_with_comparer(Vec::<Rc<DropProbe>>::new(), Rc::new(Rc::ptr_eq));
    let observed = Rc::new(RefCell::new(Vec::new()));
    let log = observed.clone();
    let weak = Rc::downgrade(&list.as_list());
    let on_drop: Rc<dyn Fn()> =
        Rc::new(move || log.borrow_mut().push(weak.upgrade().unwrap().count()));
    list.add(Rc::new(DropProbe(on_drop.clone())));
    list.set(0, Rc::new(DropProbe(on_drop.clone())));
    list.remove_at(0);
    list.add(Rc::new(DropProbe(on_drop)));
    list.clear();
    assert_eq!(*observed.borrow(), [1, 0, 0]);
}
