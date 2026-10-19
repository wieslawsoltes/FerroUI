//! `MergeManyChangeSetsFixture` (`DynamicData.Tests/List/MergeManyChangeSetsFixture.cs`): the change
//! sets of the lists of a list, merged with `MergeMany` after their indexes are removed.

use crate::utilities::assert_equivalent;
use ferroui_dynamic_data::list::{Change, ChangeSet, ChangeType, IObservableList, ListChangeReason, ObservableListEx, SourceList};
use ferroui_reactive::ObservableExt;
use ferroui_system::IObservable;
use std::rc::Rc;

/// A child list as an item of the parent list. Upstream's `SourceList<int>` is equal only to
/// itself; the list of the port has no equality of its own, so the item compares the reference.
#[derive(Clone)]
struct Child(Rc<SourceList<i32>>);

impl PartialEq for Child {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

/// `RemoveIndex()` (`ObservableListEx.RemoveIndex`): the change sets without their indexes, and
/// without the moves. The library's operator is not public yet; this is its logic, as the merging
/// operators of the library apply it inside.
fn remove_index(source: Rc<dyn IObservable<ChangeSet<i32>>>) -> Rc<dyn IObservable<ChangeSet<i32>>> {
    source.select(|changes: ChangeSet<i32>| -> ChangeSet<i32> {
        changes
            .changes()
            .iter()
            .filter(|change| change.reason() != ListChangeReason::Moved)
            .map(|change| {
                if change.change_type() == ChangeType::Item {
                    let item = change.item();
                    // A refresh keeps its index, as the library's `RemoveIndex` does.
                    let index = if change.reason() == ListChangeReason::Refresh { item.current_index() } else { None };
                    Change::with_previous(change.reason(), *item.current(), item.previous().copied(), index, None)
                } else {
                    Change::range_change(change.reason(), change.range().items().to_vec(), None)
                }
            })
            .collect()
    })
}

#[test]
fn merge_many_should_work() {
    let a = Rc::new(SourceList::<i32>::new());
    let b = Rc::new(SourceList::<i32>::new());
    let c = Rc::new(SourceList::<i32>::new());

    let parent: SourceList<Child> = SourceList::new();
    for child in [&a, &b, &c] {
        let child = Child(child.clone());
        parent.edit(move |list| list.add(child));
    }

    let d = parent.connect().merge_many(|e: &Child| remove_index(e.0.connect())).as_observable_list();

    assert_eq!(d.count(), 0);

    a.edit(|list| list.add(1));

    assert_eq!(d.count(), 1);
    a.edit(|list| list.add(2));
    assert_eq!(d.count(), 2);

    b.edit(|list| list.add(3));
    assert_eq!(d.count(), 3);
    b.edit(|list| list.add(5));
    assert_eq!(d.count(), 4);
    assert_equivalent(&d.items(), &[1, 2, 3, 5]);

    b.edit(|list| list.clear());

    // Fails below
    assert_eq!(d.count(), 2);
    assert_equivalent(&d.items(), &[1, 2]);

    a.edit(|list| list.set(0, 100));
    assert_equivalent(&d.items(), &[2, 100]);

    let f = Rc::new(SourceList::<i32>::new());
    f.edit(|list| list.add_range((10..15).collect()));
    let replacement = Child(f.clone());
    parent.edit(move |list| list.set(2, replacement));

    assert_equivalent(&d.items(), &[2, 100, 10, 11, 12, 13, 14]);
}
