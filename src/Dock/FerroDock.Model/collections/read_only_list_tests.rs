use super::{IReadOnlyList, ReadOnlyListHandle};
use ferroui_system::collections::object_model::ObservableCollection;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
#[test]
fn read_only_handles_preserve_array_and_observable_identity_live_reads_and_original_changes() {
    let a = ReadOnlyListHandle::from_items(vec![1, 2]);
    assert!(a == a.clone());
    assert!(a != ReadOnlyListHandle::from_items(vec![1, 2]));
    assert!(a.as_notify_collection_changed().is_none());
    assert_eq!(a.iter().collect::<Vec<_>>(), vec![1, 2]);
    assert_eq!(a.iter().collect::<Vec<_>>(), vec![1, 2]);
    let source = ObservableCollection::from_items(vec![1, 2]);
    let first = ReadOnlyListHandle::from_observable(source.clone());
    let second = ReadOnlyListHandle::from_observable(source.clone());
    assert!(first == second);
    assert!(std::ptr::eq(
        first
            .as_notify_collection_changed()
            .unwrap()
            .collection_changed(),
        second
            .as_notify_collection_changed()
            .unwrap()
            .collection_changed()
    ));
    let events = Rc::new(RefCell::new(Vec::new()));
    let output = events.clone();
    let token = first
        .add_collection_changed(Rc::new(move |e| {
            output.borrow_mut().push((
                e.action,
                e.new_items.to_vec(),
                e.old_items.to_vec(),
                e.new_starting_index,
                e.old_starting_index,
            ))
        }))
        .unwrap();
    source.add(3);
    source.set(0, 4);
    source.remove_at(1);
    source.move_item(0, 1);
    assert_eq!(first.count(), 2);
    assert_eq!(second.get(0), 3);
    assert_eq!(first.iter().collect::<Vec<_>>(), vec![3, 4]);
    assert_eq!(events.borrow().len(), 4);
    assert_eq!(events.borrow()[1].1, vec![4]);
    assert_eq!(events.borrow()[1].2, vec![1]);
    first.remove_collection_changed(token);
    source.clear();
    assert_eq!(events.borrow().len(), 4);
    assert_eq!(second.count(), 0);
}
struct Custom {
    reads: Cell<usize>,
    iterations: Cell<usize>,
    value: Cell<i32>,
}
impl IReadOnlyList<i32> for Custom {
    fn count(&self) -> usize {
        1
    }
    fn get(&self, index: usize) -> i32 {
        assert_eq!(index, 0);
        self.reads.set(self.reads.get() + 1);
        self.value.get()
    }
    fn get_enumerator(self: Rc<Self>) -> Box<dyn Iterator<Item = i32>> {
        self.iterations.set(self.iterations.get() + 1);
        Box::new(std::iter::once(self.value.get() + 100))
    }
    fn identity(&self) -> usize {
        self as *const Self as usize
    }
}
#[test]
fn custom_read_only_lists_keep_original_indexer_enumerator_identity_and_lifetime() {
    let custom = Rc::new(Custom {
        reads: Cell::new(0),
        iterations: Cell::new(0),
        value: Cell::new(5),
    });
    let weak = Rc::downgrade(&custom);
    let list = ReadOnlyListHandle::from_read_only(custom.clone());
    let other = ReadOnlyListHandle::from_read_only(custom.clone());
    assert!(list == other);
    assert_eq!(list.get(0), 5);
    custom.value.set(7);
    assert_eq!(list.get(0), 7);
    assert_eq!(list.iter().collect::<Vec<_>>(), vec![107]);
    assert_eq!(list.iter().collect::<Vec<_>>(), vec![107]);
    assert_eq!(custom.reads.get(), 2);
    assert_eq!(custom.iterations.get(), 2);
    drop(custom);
    assert!(weak.upgrade().is_some());
    drop(list);
    drop(other);
    assert!(weak.upgrade().is_none());
}
