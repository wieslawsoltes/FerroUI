//! `LimitSizeTo<T>` (`List/Internal/LimitSizeTo.cs`) with the operator that removes what it
//! expires (`ObservableListEx.LimitSizeTo`).

use super::ExpirableItem;
use crate::list::{remove_many, ChangeSet, IExtendedList, IList, ISourceList, ListChangeReason};
use ferroui_reactive::concurrency::IScheduler;
use ferroui_reactive::ObservableExt;
use ferroui_system::IObservable;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The `Transform` of the items into expirable items stamped with the time of the scheduler and
/// numbered in order, followed in the positions of the list (a refresh keeps the item).
fn transform<T: Clone + PartialEq>(transformed: &mut Vec<ExpirableItem<T>>, changes: &ChangeSet<T>, scheduler: &Rc<dyn IScheduler>, order_item_was_added: &Cell<u64>) {
    let make = |item: &T| {
        let index = order_item_was_added.get();
        order_item_was_added.set(index + 1);
        ExpirableItem { item: item.clone(), expire_at: scheduler.now(), index }
    };
    let find = |transformed: &Vec<ExpirableItem<T>>, item: &T| transformed.iter().position(|candidate| candidate.item == *item);
    for change in changes {
        match change.reason() {
            ListChangeReason::Add => {
                let item = change.item();
                let expirable = make(item.current());
                match item.current_index().filter(|index| *index < transformed.len()) {
                    Some(index) => transformed.insert(index, expirable),
                    None => transformed.push(expirable),
                }
            }
            ListChangeReason::AddRange => {
                let start_index = change.range().index().unwrap_or(transformed.len()).min(transformed.len());
                for (offset, item) in change.range().iter().enumerate() {
                    let expirable = make(item);
                    transformed.insert(start_index + offset, expirable);
                }
            }
            ListChangeReason::Refresh => {}
            ListChangeReason::Replace => {
                let item = change.item();
                let previous_index = item.previous_index().or_else(|| item.previous().and_then(|previous| find(transformed, previous))).expect("Cannot find index of the replaced item (UnspecifiedIndexException)");
                let current_index = item.current_index().unwrap_or(previous_index);
                let expirable = make(item.current());
                if current_index == previous_index {
                    transformed[current_index] = expirable;
                } else {
                    transformed.remove(previous_index);
                    transformed.insert(current_index.min(transformed.len()), expirable);
                }
            }
            ListChangeReason::Remove => {
                let item = change.item();
                if let Some(index) = item.current_index().or_else(|| find(transformed, item.current())) {
                    transformed.remove(index);
                }
            }
            ListChangeReason::RemoveRange => {
                let range = change.range();
                match range.index() {
                    Some(index) => {
                        transformed.drain(index..index + range.count());
                    }
                    None => {
                        for item in range.iter() {
                            if let Some(index) = find(transformed, item) {
                                transformed.remove(index);
                            }
                        }
                    }
                }
            }
            ListChangeReason::Clear => {
                let range = change.range();
                if transformed.len() == range.count() {
                    transformed.clear();
                } else {
                    for item in range.iter() {
                        if let Some(index) = find(transformed, item) {
                            transformed.remove(index);
                        }
                    }
                }
            }
            ListChangeReason::Moved => {
                let item = change.item();
                let (current, previous) = (item.current_index().expect("Cannot move as an index was not specified"), item.previous_index().expect("a move has its previous index"));
                let moved = transformed.remove(previous);
                transformed.insert(current, moved);
            }
        }
    }
}

/// `LimitSizeTo.Run`: the items over `size_limit`, the oldest first, each time there are some.
fn run<T: Clone + PartialEq + 'static, S: ISourceList<T> + ?Sized>(source_list: &S, size_limit: usize, scheduler: Rc<dyn IScheduler>) -> Rc<dyn IObservable<Vec<T>>> {
    let order_item_was_added = Rc::new(Cell::new(0u64));
    let transformed: Rc<RefCell<Vec<ExpirableItem<T>>>> = Rc::new(RefCell::new(Vec::new()));
    source_list
        .connect()
        .observe_on(scheduler.clone())
        .select(move |changes: ChangeSet<T>| {
            let mut transformed = transformed.borrow_mut();
            transform(&mut transformed, &changes, &scheduler, &order_item_was_added);
            // `ToCollection`, then the items to expire.
            let number_to_expire = transformed.len().saturating_sub(size_limit);
            let mut list: Vec<&ExpirableItem<T>> = transformed.iter().collect();
            list.sort_by(|a, b| a.expire_at.cmp(&b.expire_at).then(a.index.cmp(&b.index)));
            list.into_iter().take(number_to_expire).map(|expirable| expirable.item.clone()).collect::<Vec<T>>()
        })
        .where_(|items: &Vec<T>| !items.is_empty())
}

/// `ObservableListEx.LimitSizeTo(ISourceList, int, IScheduler?)`: the expired items, removed from
/// the source (`RemoveMany`) as they pass.
///
/// # Panics
/// For a size of zero (`ArgumentException`).
pub(crate) fn limit_size_to<T: Clone + PartialEq + 'static, S: ISourceList<T> + Clone + 'static>(source: S, size_limit: usize, scheduler: Rc<dyn IScheduler>) -> Rc<dyn IObservable<Vec<T>>> {
    assert!(size_limit > 0, "sizeLimit cannot be zero");
    let limiter = run(&source, size_limit, scheduler);
    limiter.do_(move |items: &Vec<T>| {
        let items = items.clone();
        let mut items = Some(items);
        source.edit_dyn(&mut |list: &mut dyn IExtendedList<T>| {
            if let Some(items) = items.take() {
                let list: &mut dyn IList<T> = list;
                remove_many(list, &items);
            }
        });
    })
}
