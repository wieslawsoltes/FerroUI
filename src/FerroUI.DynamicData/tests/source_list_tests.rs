use crate::list::{ChangeSet, IObservableList, ListChangeReason, SourceList};
use ferroui_reactive::ObservableExt;
use std::cell::RefCell;
use std::rc::Rc;

fn summary(changes: &ChangeSet<i32>) -> Vec<(ListChangeReason, Vec<i32>, Option<usize>)> {
    changes
        .iter()
        .map(|c| match c.try_item() {
            Some(item) => (c.reason(), vec![*item.current()], item.current_index()),
            None => (c.reason(), c.range().items().to_vec(), c.range().index()),
        })
        .collect()
}

#[test]
fn consecutive_adds_become_one_range_and_connect_starts_with_the_items() {
    let list = SourceList::<i32>::new();
    let log = Rc::new(RefCell::new(Vec::new()));
    let target = log.clone();
    let _subscription = list.connect().subscribe_next(move |changes: ChangeSet<i32>| target.borrow_mut().push(summary(&changes)));
    list.edit(|l| {
        l.add(1);
        l.add(2);
        l.add(3);
    });
    list.edit(|l| {
        l.remove_at(0);
        l.remove_at(0);
    });
    assert_eq!(
        *log.borrow(),
        vec![vec![(ListChangeReason::AddRange, vec![1, 2, 3], Some(0))], vec![(ListChangeReason::RemoveRange, vec![1, 2], Some(0))]]
    );

    let late = Rc::new(RefCell::new(Vec::new()));
    let target = late.clone();
    let _late = list.connect().subscribe_next(move |changes: ChangeSet<i32>| target.borrow_mut().push(summary(&changes)));
    assert_eq!(*late.borrow(), vec![vec![(ListChangeReason::AddRange, vec![3], Some(0))]]);
}

#[test]
fn removals_that_empty_the_list_are_a_clear_and_a_nested_edit_joins() {
    let list = SourceList::<i32>::new();
    list.edit(|l| l.add_range(vec![1, 2, 3]));
    let log = Rc::new(RefCell::new(Vec::new()));
    let target = log.clone();
    let _subscription = list.connect().subscribe_next(move |changes: ChangeSet<i32>| target.borrow_mut().push(summary(&changes)));
    let nested = list.clone();
    list.edit(move |l| {
        l.remove_at(2);
        nested.edit(|l| l.remove_at(0));
        l.remove_at(0);
    });
    assert_eq!(log.borrow()[1], vec![(ListChangeReason::Clear, vec![3, 1, 2], None)]);
    assert_eq!(list.count(), 0);
}
