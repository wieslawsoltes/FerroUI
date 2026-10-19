//! `Sort<T>` (`List/Internal/Sort.cs`).

use super::unified;
use crate::list::{binary_search_by, clone_changes, remove_many, ChangeAwareList, ChangeSet, IExtendedList, ListChangeReason, SortException, SortOptions};
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::{IObservable, IObserver, ObservableError};
use std::cell::RefCell;
use std::cmp::Ordering;
use std::rc::Rc;

/// An `IComparer<T>`.
pub(crate) type Comparer<T> = Rc<dyn Fn(&T, &T) -> Ordering>;

/// The state of one subscription: the items of the source and the sorted list.
struct SortState<T> {
    original: Vec<T>,
    target: ChangeAwareList<T>,
}

/// The parts of the sort that do not change between subscriptions.
struct Sorter<T> {
    /// `_comparer`. Upstream defaults to `Comparer<T>.Default` when only an observable of
    /// comparers is given; here the items compare equal until the first comparer arrives.
    comparer: RefCell<Option<Comparer<T>>>,
    sort_options: SortOptions,
    reset_threshold: usize,
}

impl<T: Clone + PartialEq + 'static> Sorter<T> {
    fn compare(&self, a: &T, b: &T) -> Ordering {
        match &*self.comparer.borrow() {
            Some(comparer) => comparer(a, b),
            None => Ordering::Equal,
        }
    }

    /// The items in the order of the comparer (`OrderBy(t => t, _comparer)`, a stable sort).
    fn ordered(&self, items: &[T]) -> Vec<T> {
        let mut sorted = items.to_vec();
        sorted.sort_by(|a, b| self.compare(a, b));
        sorted
    }

    fn change_comparer(&self, target: &mut ChangeAwareList<T>, comparer: Comparer<T>) -> ChangeSet<T> {
        *self.comparer.borrow_mut() = Some(comparer);
        if self.reset_threshold > 0 && target.count() <= self.reset_threshold {
            return self.reorder(target);
        }
        let sorted = self.ordered(target.items());
        target.clear();
        target.add_range(sorted);
        target.capture_changes()
    }

    fn get_current_position(&self, target: &ChangeAwareList<T>, item: &T) -> Result<usize, ObservableError> {
        let index = if self.sort_options == SortOptions::UseBinarySearch {
            binary_search_by(target, item, &|a: &T, b: &T| self.compare(a, b)).ok()
        } else {
            target.index_of(item)
        };
        index.ok_or_else(|| SortException::new(format!("Cannot find item: {}", std::any::type_name::<T>())).into_error())
    }

    fn get_insert_position(&self, target: &ChangeAwareList<T>, item: &T) -> Result<usize, ObservableError> {
        if self.sort_options == SortOptions::UseBinarySearch {
            self.get_insert_position_binary(target, item)
        } else {
            Ok(self.get_insert_position_linear(target, item))
        }
    }

    fn get_insert_position_binary(&self, target: &ChangeAwareList<T>, item: &T) -> Result<usize, ObservableError> {
        match binary_search_by(target, item, &|a: &T, b: &T| self.compare(a, b)) {
            Err(insert_index) => Ok(insert_index),
            // The sort does not tell the items apart.
            Ok(_) => Err(SortException::new("Binary search has been specified, yet the sort does not yield uniqueness").into_error()),
        }
    }

    fn get_insert_position_linear(&self, target: &ChangeAwareList<T>, item: &T) -> usize {
        target.items().iter().position(|existing| self.compare(item, existing) == Ordering::Less).unwrap_or_else(|| target.count())
    }

    fn insert(&self, target: &mut ChangeAwareList<T>, item: T) -> Result<(), ObservableError> {
        let index = self.get_insert_position(target, &item)?;
        target.insert(index, item);
        Ok(())
    }

    fn remove(&self, target: &mut ChangeAwareList<T>, item: &T) -> Result<(), ObservableError> {
        let index = self.get_current_position(target, item)?;
        target.remove_at(index);
        Ok(())
    }

    fn process(&self, target: &mut ChangeAwareList<T>, changes: &ChangeSet<T>) -> Result<ChangeSet<T>, ObservableError> {
        // If all removes and not Clear, then more efficient to try clear range.
        if changes.total_changes() == changes.removes() && changes.iter().all(|change| change.reason() != ListChangeReason::Clear) && changes.removes() > 1 {
            let removed: Vec<T> = unified(changes).into_iter().map(|change| change.current).collect();
            remove_many(target, &removed);
            return Ok(target.capture_changes());
        }
        self.process_impl(target, changes)
    }

    fn process_impl(&self, target: &mut ChangeAwareList<T>, changes: &ChangeSet<T>) -> Result<ChangeSet<T>, ObservableError> {
        let mut refreshes = Vec::with_capacity(changes.refreshes());
        for change in changes {
            match change.reason() {
                ListChangeReason::Add => self.insert(target, change.item().current().clone())?,
                ListChangeReason::AddRange => {
                    let ordered = self.ordered(change.range().items());
                    if target.count() == 0 {
                        target.add_range(ordered);
                    } else {
                        for item in ordered {
                            self.insert(target, item)?;
                        }
                    }
                }
                ListChangeReason::Remove => self.remove(target, change.item().current())?,
                ListChangeReason::Refresh => {
                    let current = change.item().current();
                    // Add to the refresh list so that the position can be calculated.
                    refreshes.push(current.clone());

                    // Add to the current list so that downstream operators receive a refresh
                    // notification, with the latest index.
                    let Some(index) = target.index_of(current) else {
                        return Err(SortException::new(format!("Cannot find index of {}. Expected to be in the list", std::any::type_name::<T>())).into_error());
                    };
                    let item = target.get(index).clone();
                    target.refresh_with(item, index);
                }
                ListChangeReason::Replace => {
                    // TODO upstream: check whether an item should stay in the same position, i.e.
                    // update and move.
                    self.remove(target, change.item().previous().expect("a replacement has a previous item"))?;
                    self.insert(target, change.item().current().clone())?;
                }
                ListChangeReason::RemoveRange => remove_many(target, change.range().items()),
                ListChangeReason::Clear => target.clear(),
                ListChangeReason::Moved => {}
            }
        }

        // Now deal with refreshes (can be expensive).
        for item in refreshes {
            let Some(old) = target.index_of(&item) else { continue };
            let mut new_position = self.get_insert_position_linear(target, &item);
            if old < new_position {
                new_position -= 1;
            }
            if old == new_position {
                continue;
            }
            IExtendedList::move_index(target, old, new_position);
        }

        Ok(target.capture_changes())
    }

    /// Moves the items to their sorted positions (`Reorder`).
    ///
    /// Deviation (DEVIATIONS.md, DynamicData): an item that is already in its place is found by
    /// equality, where upstream compares references (`ReferenceEquals`).
    fn reorder(&self, target: &mut ChangeAwareList<T>) -> ChangeSet<T> {
        for (index, item) in self.ordered(target.items()).into_iter().enumerate() {
            // If the item is in the same place.
            if target.get(index) == &item {
                continue;
            }
            // Cannot use a binary search, as a resort implies a mutable change.
            let old = target.index_of(&item).expect("the reordered item is in the list");
            IExtendedList::move_index(target, old, index);
        }
        target.capture_changes()
    }

    fn reset(&self, original: &[T], target: &mut ChangeAwareList<T>) -> ChangeSet<T> {
        let sorted = self.ordered(original);
        target.clear();
        target.add_range(sorted);
        target.capture_changes()
    }
}

/// `new Sort<T>(source, comparer, sortOptions, resort, comparerChanged, resetThreshold).Run()`.
pub(crate) fn sort<T: Clone + PartialEq + 'static>(
    source: Rc<dyn IObservable<ChangeSet<T>>>,
    comparer: Option<Comparer<T>>,
    sort_options: SortOptions,
    resort: Option<Rc<dyn IObservable<()>>>,
    comparer_changed: Option<Rc<dyn IObservable<Comparer<T>>>>,
    reset_threshold: usize,
) -> Rc<dyn IObservable<ChangeSet<T>>> {
    let comparer_observable = comparer_changed.unwrap_or_else(Observable::never);
    let resort = resort.unwrap_or_else(Observable::never);
    let sorter = Rc::new(Sorter { comparer: RefCell::new(comparer), sort_options, reset_threshold });

    Observable::create(move |observer: Rc<dyn IObserver<ChangeSet<T>>>| {
        let state = Rc::new(RefCell::new(SortState { original: Vec::new(), target: ChangeAwareList::new() }));

        let data_changed = {
            let (sorter, state) = (sorter.clone(), state.clone());
            source.try_select(move |changes: ChangeSet<T>| {
                let mut state = state.borrow_mut();
                let SortState { original, target } = &mut *state;
                if sorter.reset_threshold > 1 {
                    clone_changes(original, &changes);
                }
                if changes.total_changes() > sorter.reset_threshold {
                    Ok(sorter.reset(original, target))
                } else {
                    sorter.process(target, &changes)
                }
            })
        };

        let resort_sync = {
            let (sorter, state) = (sorter.clone(), state.clone());
            resort.select(move |_| sorter.reorder(&mut state.borrow_mut().target))
        };

        let change_comparer = {
            let (sorter, state) = (sorter.clone(), state.clone());
            comparer_observable.select(move |comparer: Comparer<T>| sorter.change_comparer(&mut state.borrow_mut().target, comparer))
        };

        change_comparer.merge(resort_sync).merge(data_changed).where_(|changes: &ChangeSet<T>| changes.count() != 0).subscribe(observer)
    })
}
