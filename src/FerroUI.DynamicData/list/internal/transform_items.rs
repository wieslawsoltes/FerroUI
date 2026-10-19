//! The `Transform` that the merging and subscribing operators of lists apply inside
//! (`List/Internal/Transformer.cs`, without `transformOnRefresh`): each item becomes the value of
//! `factory`, kept in the order of the source, so that a removal or a replacement reports the value
//! made for the item it removes.

use crate::list::{Change, ChangeAwareList, ChangeSet, ChangeType, IExtendedList, IList, ListChangeReason};
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::IObservable;
use std::cell::RefCell;
use std::rc::Rc;

/// An item and its value (`TransformedItemContainer`): equal by the item of the source.
#[derive(Clone)]
struct Container<T, U> {
    source: T,
    destination: U,
}

impl<T: PartialEq, U> PartialEq for Container<T, U> {
    fn eq(&self, other: &Self) -> bool {
        self.source == other.source
    }
}

/// The index of the container of `item` (`FindItemIndex`).
fn find_item_index<T: Clone + PartialEq, U: Clone>(transformed: &ChangeAwareList<Container<T, U>>, item: &T) -> Option<usize> {
    transformed.items().iter().position(|container| &container.source == item)
}

/// The change sets of `source` with each item as `factory` makes it.
pub(crate) fn transform_items<T: Clone + PartialEq + 'static, U: Clone + 'static>(
    source: Rc<dyn IObservable<ChangeSet<T>>>,
    factory: Rc<dyn Fn(&T) -> U>,
) -> Rc<dyn IObservable<ChangeSet<U>>> {
    Observable::defer(move || {
        let transformed = RefCell::new(ChangeAwareList::<Container<T, U>>::new());
        let factory = factory.clone();
        source.select(move |changes: ChangeSet<T>| {
            for change in &changes {
                transform(&transformed, change, &*factory);
            }
            let captured = transformed.borrow_mut().capture_changes();
            map_change_set(&captured, |container: &Container<T, U>| container.destination.clone())
        })
    })
}

/// `Transform`: applies one change of the source.
fn transform<T: Clone + PartialEq, U: Clone>(transformed: &RefCell<ChangeAwareList<Container<T, U>>>, change: &Change<T>, factory: &dyn Fn(&T) -> U) {
    let make = |item: &T| Container { source: item.clone(), destination: factory(item) };
    match change.reason() {
        ListChangeReason::Add => {
            let item = change.item();
            let container = make(item.current());
            let mut transformed = transformed.borrow_mut();
            match item.current_index() {
                Some(index) if index < transformed.count() => transformed.insert(index, container),
                _ => transformed.add(container),
            }
        }
        ListChangeReason::AddRange => {
            let containers: Vec<_> = change.range().iter().map(&make).collect();
            IList::add_or_insert_range(&mut *transformed.borrow_mut(), containers, change.range().index());
        }
        ListChangeReason::Refresh => {
            let item = change.item();
            let mut transformed = transformed.borrow_mut();
            let index = item.current_index().or_else(|| find_item_index(&transformed, item.current())).expect("Cannot find index of the refreshed item");
            transformed.refresh_at(index);
        }
        ListChangeReason::Replace => {
            let item = change.item();
            let previous = item.previous().expect("a replacement has a previous item");
            let container = make(item.current());
            let mut transformed = transformed.borrow_mut();
            let previous_index = item.previous_index().or_else(|| find_item_index(&transformed, previous)).expect("Cannot find index of the replaced item");
            let current_index = item.current_index().unwrap_or(previous_index);
            if current_index == previous_index {
                transformed.set(current_index, container);
            } else {
                transformed.remove_at(previous_index);
                transformed.insert(current_index, container);
            }
        }
        ListChangeReason::Remove => {
            let item = change.item();
            let mut transformed = transformed.borrow_mut();
            let index = item.current_index().or_else(|| find_item_index(&transformed, item.current()));
            if let Some(index) = index {
                transformed.remove_at(index);
            }
        }
        ListChangeReason::RemoveRange => {
            let range = change.range();
            let mut transformed = transformed.borrow_mut();
            match range.index() {
                Some(index) => transformed.remove_range(index, range.count()),
                None => {
                    let many_to_remove: Vec<_> = range.iter().filter_map(|item| find_item_index(&transformed, item).map(|index| transformed.get(index).clone())).collect();
                    crate::list::remove_many(&mut *transformed, &many_to_remove);
                }
            }
        }
        ListChangeReason::Clear => {
            let mut transformed = transformed.borrow_mut();
            // The values made for the items, to clear them.
            let to_clear = Change::range_change(ListChangeReason::Clear, transformed.items().to_vec(), None);
            crate::list::clear_or_remove_many(&mut *transformed, &to_clear);
        }
        ListChangeReason::Moved => {
            let item = change.item();
            let (current_index, previous_index) = (
                item.current_index().expect("Cannot move as an index was not specified"),
                item.previous_index().expect("Cannot move as an index was not specified"),
            );
            transformed.borrow_mut().move_index(previous_index, current_index);
        }
    }
}

/// A change set with each item as `selector` makes it (the `Transform` of a change set,
/// `ChangeSetEx.Transform`).
pub(crate) fn map_change_set<T, U>(changes: &ChangeSet<T>, selector: impl Fn(&T) -> U) -> ChangeSet<U> {
    changes
        .iter()
        .map(|change| {
            if change.change_type() == ChangeType::Range {
                return Change::range_change(change.reason(), change.range().iter().map(&selector).collect(), change.range().index());
            }
            let item = change.item();
            if change.reason() == ListChangeReason::Moved {
                return Change::moved(
                    selector(item.current()),
                    item.current_index().expect("a move has its indexes"),
                    item.previous_index().expect("a move has its indexes"),
                );
            }
            Change::with_previous(change.reason(), selector(item.current()), item.previous().map(&selector), item.current_index(), item.previous_index())
        })
        .collect()
}
