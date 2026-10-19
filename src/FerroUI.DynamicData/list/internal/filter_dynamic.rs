//! `Filter.Dynamic<T>` (`List/Internal/Filter.Dynamic.cs`): the items of a list that the latest of
//! a sequence of predicates accepts.

use super::transformer::{transform_change_set, transformer};
use crate::list::{clone_changes, remove_many, ChangeAwareList, ChangeSet, ListChangeReason, ListFilterPolicy};
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::IObservable;
use std::cell::{Cell, RefCell};
use std::fmt;
use std::rc::Rc;

/// An item with whether it matches the latest predicate and whether it matched the one before
/// (`Filter.Dynamic.ItemWithMatch`): shared, as upstream's class is, so that a new predicate
/// updates the flags of the item that the transformation and the list of all items hold.
struct ItemWithMatchInner<T> {
    item: T,
    is_match: Cell<bool>,
    was_match: Cell<bool>,
}

struct ItemWithMatch<T>(Rc<ItemWithMatchInner<T>>);

impl<T> Clone for ItemWithMatch<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T: PartialEq> PartialEq for ItemWithMatch<T> {
    /// Two items with matches are equal when their items are (`Equals`).
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0) || self.0.item == other.0.item
    }
}

impl<T: fmt::Debug> fmt::Debug for ItemWithMatch<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}, (was {} is {}", self.0.item, self.0.is_match.get(), self.0.was_match.get())
    }
}

impl<T> ItemWithMatch<T> {
    fn new(item: T, is_match: bool, was_match: bool) -> Self {
        Self(Rc::new(ItemWithMatchInner { item, is_match: Cell::new(is_match), was_match: Cell::new(was_match) }))
    }

    fn item(&self) -> &T {
        &self.0.item
    }

    fn is_match(&self) -> bool {
        self.0.is_match.get()
    }

    fn was_match(&self) -> bool {
        self.0.was_match.get()
    }
}

/// The items of the source that the latest predicate of `predicates` accepts (`Filter.Dynamic<T>`
/// with `IObservable<Func<T, bool>>`): until the first predicate, none is. A new predicate is
/// applied by `policy`. A refresh of the source evaluates its item again; a refreshed item that
/// matches still is replaced downstream, as upstream's transformation of a refresh is a
/// replacement.
pub(crate) fn dynamic_filter<T: Clone + PartialEq + 'static>(
    source: Rc<dyn IObservable<ChangeSet<T>>>,
    predicates: Rc<dyn IObservable<Rc<dyn Fn(&T) -> bool>>>,
    policy: ListFilterPolicy,
) -> Rc<dyn IObservable<ChangeSet<T>>> {
    Observable::defer(move || {
        let predicate: Rc<RefCell<Rc<dyn Fn(&T) -> bool>>> = Rc::new(RefCell::new(Rc::new(|_: &T| false)));
        let all: Rc<RefCell<Vec<ItemWithMatch<T>>>> = Rc::new(RefCell::new(Vec::new()));
        let filtered: Rc<RefCell<ChangeAwareList<ItemWithMatch<T>>>> = Rc::new(RefCell::new(ChangeAwareList::new()));

        let predicate_changed = {
            let (predicate, all, filtered) = (predicate.clone(), all.clone(), filtered.clone());
            predicates.select(move |new_predicate: Rc<dyn Fn(&T) -> bool>| {
                *predicate.borrow_mut() = new_predicate.clone();
                requery(&*new_predicate, &mut all.borrow_mut(), &mut filtered.borrow_mut(), policy)
            })
        };

        // Apply the transform operator so 'IsMatch' state can be evaluated and captured one time
        // only. This is to eliminate the need to re-apply the predicate when determining whether
        // an item was previously matched, which is essential when we have mutable state.
        let transform_predicate = predicate.clone();
        let filtered_result = transformer(
            source.clone(),
            move |item: &T, previous: Option<&ItemWithMatch<T>>, _: usize| {
                let was_match = previous.is_some_and(|previous| previous.is_match());
                let predicate = transform_predicate.borrow().clone();
                let is_match = predicate(item);
                Ok(ItemWithMatch::new(item.clone(), is_match, was_match))
            },
            true,
        )
        .select(move |changes: ChangeSet<ItemWithMatch<T>>| {
            // keep track of all changes if filtering on an observable
            clone_changes(&mut *all.borrow_mut(), &changes);
            process(&mut filtered.borrow_mut(), &changes)
        });

        predicate_changed
            .merge(filtered_result)
            .where_(|changes: &ChangeSet<ItemWithMatch<T>>| changes.count() != 0)
            // use convert, not transform
            .select(|changes: ChangeSet<ItemWithMatch<T>>| transform_change_set(&changes, |item: &ItemWithMatch<T>| item.item().clone()))
    })
}

/// Maintains the filtered list from the transformed changes (`Process`).
fn process<T: Clone + PartialEq>(filtered: &mut ChangeAwareList<ItemWithMatch<T>>, changes: &ChangeSet<ItemWithMatch<T>>) -> ChangeSet<ItemWithMatch<T>> {
    for item in changes {
        match item.reason() {
            ListChangeReason::Add => {
                let change = item.item();
                if change.current().is_match() {
                    filtered.add(change.current().clone());
                }
            }
            ListChangeReason::AddRange => {
                let matches: Vec<ItemWithMatch<T>> = item.range().iter().filter(|candidate| candidate.is_match()).cloned().collect();
                filtered.add_range(matches);
            }
            ListChangeReason::Replace | ListChangeReason::Refresh => {
                let change = item.item();
                let is_match = change.current().is_match();
                let was_match = change.current().was_match();
                // The item to find: the previous one of a replacement, the item of a refresh.
                let previous = if item.reason() == ListChangeReason::Replace { change.previous().expect("a replacement has a previous item") } else { change.current() };
                if is_match {
                    if was_match {
                        // an update, so get the latest index and pass the index up the chain
                        let index = filtered
                            .items()
                            .iter()
                            .position(|candidate| candidate.item() == previous.item())
                            .expect("Cannot find index of the previous item. Expected to be in the list");
                        if item.reason() == ListChangeReason::Replace {
                            // replace inline
                            filtered.set(index, change.current().clone());
                        } else {
                            filtered.refresh_at(index);
                        }
                    } else {
                        filtered.add(change.current().clone());
                    }
                } else if was_match {
                    filtered.remove(previous);
                }
            }
            ListChangeReason::Remove => {
                filtered.remove(item.item().current());
            }
            ListChangeReason::RemoveRange => remove_many(filtered, item.range().items()),
            ListChangeReason::Clear => crate::list::clear_or_remove_many(filtered, item),
            ListChangeReason::Moved => {}
        }
    }
    filtered.capture_changes()
}

/// Applies a new predicate (`Requery`).
fn requery<T: Clone + PartialEq>(
    predicate: &dyn Fn(&T) -> bool,
    all: &mut Vec<ItemWithMatch<T>>,
    filtered: &mut ChangeAwareList<ItemWithMatch<T>>,
    policy: ListFilterPolicy,
) -> ChangeSet<ItemWithMatch<T>> {
    if all.is_empty() {
        return ChangeSet::empty();
    }
    if policy == ListFilterPolicy::ClearAndReplace {
        let items_with_match: Vec<ItemWithMatch<T>> = all.iter().map(|iwm| ItemWithMatch::new(iwm.item().clone(), predicate(iwm.item()), iwm.is_match())).collect();
        // mark items as matched?
        filtered.clear();
        filtered.add_range(items_with_match.iter().filter(|iwm| iwm.is_match()).cloned().collect());
        // reset state for all items
        *all = items_with_match;
        return filtered.capture_changes();
    }
    let mut to_add = Vec::with_capacity(all.len());
    let mut to_remove = Vec::with_capacity(all.len());
    for current in all.iter() {
        let new_item = ItemWithMatch::new(current.item().clone(), predicate(current.item()), current.is_match());
        current.0.is_match.set(new_item.is_match());
        current.0.was_match.set(new_item.was_match());
        if new_item.is_match() && !new_item.was_match() {
            to_add.push(new_item);
        } else if !new_item.is_match() && new_item.was_match() {
            to_remove.push(new_item);
        }
    }
    remove_many(filtered, &to_remove);
    filtered.add_range(to_add);
    filtered.capture_changes()
}
