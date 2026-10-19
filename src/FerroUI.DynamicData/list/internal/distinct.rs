//! `Distinct<T, TValue>` (`List/Internal/Distinct.cs`): the distinct values of the items of a list.
//!
//! Upstream transforms the items into `ItemWithMatch` (the item, its value and the value it had
//! before), with a transform on refresh, and processes the change sets of those; the port keeps
//! the items with their values in the positions of the list, as that transform does, and processes
//! each change as it comes.

use crate::list::{ChangeAwareList, ChangeSet, ListChangeReason};
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::{IObservable, IObserver};
use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::Hash;
use std::rc::Rc;

/// An item and its value (`ItemWithMatch` without its previous value, which is the value of the
/// item it replaces in the positions).
struct ItemWithMatch<T, V> {
    item: T,
    value: V,
}

struct State<T, V> {
    transformed: Vec<ItemWithMatch<T, V>>,
    values: HashMap<V, usize>,
    result: ChangeAwareList<V>,
}

impl<T: Clone + PartialEq, V: Eq + Hash + Clone> State<T, V> {
    fn add_action(&mut self, value: V) {
        match self.values.get_mut(&value) {
            Some(count) => *count += 1,
            None => {
                self.values.insert(value.clone(), 1);
                self.result.add(value);
            }
        }
    }

    fn remove_action(&mut self, value: &V) {
        let Some(counter) = self.values.get_mut(value) else { return };
        *counter -= 1;
        if *counter != 0 {
            return;
        }
        self.result.remove(value);
        self.values.remove(value);
    }

    fn find_item_index(&self, item: &T) -> Option<usize> {
        self.transformed.iter().position(|transformed| transformed.item == *item)
    }

    /// The transform of upstream's `Transformer` followed by `Process` for each change.
    fn process(&mut self, changes: &ChangeSet<T>, value_selector: &dyn Fn(&T) -> V) -> ChangeSet<V> {
        for change in changes {
            match change.reason() {
                ListChangeReason::Add => {
                    let item = change.item();
                    let value = value_selector(item.current());
                    let transformed = ItemWithMatch { item: item.current().clone(), value: value.clone() };
                    match item.current_index().filter(|index| *index < self.transformed.len()) {
                        Some(index) => self.transformed.insert(index, transformed),
                        None => self.transformed.push(transformed),
                    }
                    self.add_action(value);
                }
                ListChangeReason::AddRange => {
                    let start_index = change.range().index().unwrap_or(self.transformed.len()).min(self.transformed.len());
                    for (offset, item) in change.range().iter().enumerate() {
                        let value = value_selector(item);
                        self.transformed.insert(start_index + offset, ItemWithMatch { item: item.clone(), value: value.clone() });
                        self.add_action(value);
                    }
                }
                ListChangeReason::Refresh => {
                    let item = change.item();
                    let index = item.current_index().or_else(|| self.find_item_index(item.current())).expect("Cannot find index of the refreshed item (UnspecifiedIndexException)");
                    let value = value_selector(item.current());
                    let previous = std::mem::replace(&mut self.transformed[index], ItemWithMatch { item: item.current().clone(), value: value.clone() }).value;
                    if value == previous {
                        continue;
                    }
                    self.remove_action(&previous);
                    self.add_action(value);
                }
                ListChangeReason::Replace => {
                    let item = change.item();
                    let previous_item = item.previous().expect("a replacement has a previous item");
                    let previous_index = item.previous_index().or_else(|| self.find_item_index(previous_item)).expect("Cannot find index of the replaced item (UnspecifiedIndexException)");
                    let current_index = item.current_index().unwrap_or(previous_index);
                    let value = value_selector(item.current());
                    let transformed = ItemWithMatch { item: item.current().clone(), value: value.clone() };
                    if current_index == previous_index {
                        let previous = std::mem::replace(&mut self.transformed[current_index], transformed).value;
                        if value == previous {
                            continue;
                        }
                        self.remove_action(&previous);
                        self.add_action(value);
                    } else {
                        // A removal and an addition upstream.
                        let previous = self.transformed.remove(previous_index).value;
                        self.transformed.insert(current_index.min(self.transformed.len()), transformed);
                        self.remove_action(&previous);
                        self.add_action(value);
                    }
                }
                ListChangeReason::Remove => {
                    let item = change.item();
                    let Some(index) = item.current_index().or_else(|| self.find_item_index(item.current())) else { continue };
                    let previous = self.transformed.remove(index).value;
                    self.remove_action(&previous);
                }
                ListChangeReason::RemoveRange => {
                    let range = change.range();
                    let removed: Vec<V> = match range.index() {
                        Some(index) => self.transformed.drain(index..index + range.count()).map(|transformed| transformed.value).collect(),
                        None => range
                            .iter()
                            .filter_map(|item| self.find_item_index(item).map(|index| self.transformed.remove(index).value))
                            .collect(),
                    };
                    for value in &removed {
                        self.remove_action(value);
                    }
                }
                ListChangeReason::Clear => {
                    let range = change.range();
                    if self.transformed.len() == range.count() {
                        self.transformed.clear();
                        self.result.clear();
                        self.values.clear();
                    } else {
                        // `ClearOrRemoveMany` removes the items one by one.
                        let removed: Vec<V> = range.iter().filter_map(|item| self.find_item_index(item).map(|index| self.transformed.remove(index).value)).collect();
                        for value in &removed {
                            self.remove_action(value);
                        }
                    }
                }
                ListChangeReason::Moved => {
                    let item = change.item();
                    let (current, previous) = (item.current_index().expect("Cannot move as an index was not specified"), item.previous_index().expect("a move has its previous index"));
                    let moved = self.transformed.remove(previous);
                    self.transformed.insert(current, moved);
                }
            }
        }
        self.result.capture_changes()
    }
}

/// `Distinct.Run`.
pub(crate) fn distinct<T: Clone + PartialEq + 'static, V: Eq + Hash + Clone + 'static>(source: Rc<dyn IObservable<ChangeSet<T>>>, value_selector: Rc<dyn Fn(&T) -> V>) -> Rc<dyn IObservable<ChangeSet<V>>> {
    Observable::create(move |observer: Rc<dyn IObserver<ChangeSet<V>>>| {
        let state = RefCell::new(State { transformed: Vec::new(), values: HashMap::new(), result: ChangeAwareList::new() });
        let value_selector = value_selector.clone();
        source
            .select(move |changes: ChangeSet<T>| state.borrow_mut().process(&changes, &*value_selector))
            .where_(|changes: &ChangeSet<V>| changes.count() != 0)
            .subscribe_safe(observer)
    })
}
