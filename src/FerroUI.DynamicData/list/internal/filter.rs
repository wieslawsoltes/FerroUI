//! `Filter.Static<T>` (`List/Internal/Filter.Static.cs`): the items of a list that a predicate
//! accepts, in their order, evaluated again on a refresh or a replacement.
//!
//! This is the filter of `ObservableListEx.Filter(Func<T, bool>)`; the older `FilterStatic<T>`
//! (`filter_static`), which follows neither refreshes nor moves, remains the filter of the
//! predicates of `SourceList.Connect`.

use crate::list::{ChangeAwareList, ChangeSet, IExtendedList, ListChangeReason};
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::IObservable;
use std::cell::RefCell;
use std::rc::Rc;

/// The items of the source that `predicate` accepts, in the order of the source
/// (`Filter.Static<T>.Create`): each indexed change is offset by the items excluded before its
/// index, a refresh or a replacement evaluates its item again, and moves are followed.
pub(crate) fn static_filter<T: Clone + PartialEq + 'static>(
    source: Rc<dyn IObservable<ChangeSet<T>>>,
    predicate: Rc<dyn Fn(&T) -> bool>,
    suppress_empty_change_sets: bool,
) -> Rc<dyn IObservable<ChangeSet<T>>> {
    Observable::defer(move || {
        let state = RefCell::new(StaticFilterState::<T> { upstream_items_states: Vec::new(), downstream_items: ChangeAwareList::new() });
        let predicate = predicate.clone();
        let downstream = source.select(move |upstream_changes: ChangeSet<T>| {
            let mut state = state.borrow_mut();
            state.process(&upstream_changes, &*predicate);
            state.downstream_items.capture_changes()
        });
        if suppress_empty_change_sets {
            downstream.where_(|changes: &ChangeSet<T>| changes.count() != 0)
        } else {
            downstream
        }
    })
}

struct StaticFilterState<T> {
    /// Each item of the source, and whether it is included.
    upstream_items_states: Vec<(T, bool)>,
    downstream_items: ChangeAwareList<T>,
}

impl<T: Clone + PartialEq> StaticFilterState<T> {
    /// This is how we implement order preservation, downstream: each time we do an indexed
    /// operation, we count how many excluded items there are before that index, and offset the
    /// index by that amount (`CountExcludedItemsBefore`).
    fn count_excluded_items_before(&self, index: usize) -> usize {
        self.upstream_items_states[..index].iter().filter(|(_, is_included)| !is_included).count()
    }

    fn process(&mut self, upstream_changes: &ChangeSet<T>, predicate: &dyn Fn(&T) -> bool) {
        for change in upstream_changes {
            match change.reason() {
                ListChangeReason::Add => {
                    let item = change.item();
                    let is_included = predicate(item.current());
                    match item.current_index() {
                        None => {
                            self.upstream_items_states.push((item.current().clone(), is_included));
                            if is_included {
                                self.downstream_items.add(item.current().clone());
                            }
                        }
                        Some(index) => {
                            self.upstream_items_states.insert(index, (item.current().clone(), is_included));
                            if is_included {
                                let downstream_index = index - self.count_excluded_items_before(index);
                                self.downstream_items.insert(downstream_index, item.current().clone());
                            }
                        }
                    }
                }
                ListChangeReason::AddRange => {
                    let range = change.range();
                    match range.index() {
                        Some(index) if index != self.upstream_items_states.len() => {
                            let mut item_states_buffer = Vec::with_capacity(range.count());
                            let mut items_buffer = Vec::with_capacity(range.count());
                            for item in range {
                                let is_included = predicate(item);
                                item_states_buffer.push((item.clone(), is_included));
                                if is_included {
                                    items_buffer.push(item.clone());
                                }
                            }
                            self.upstream_items_states.splice(index..index, item_states_buffer);
                            let downstream_index = index - self.count_excluded_items_before(index);
                            self.downstream_items.insert_range(items_buffer, downstream_index);
                        }
                        _ => {
                            for item in range {
                                let is_included = predicate(item);
                                self.upstream_items_states.push((item.clone(), is_included));
                                if is_included {
                                    self.downstream_items.add(item.clone());
                                }
                            }
                        }
                    }
                }
                ListChangeReason::Clear => {
                    self.upstream_items_states.clear();
                    self.downstream_items.clear();
                }
                ListChangeReason::Moved => {
                    let item = change.item();
                    let previous_index = item.previous_index().expect("a move has a previous index");
                    let current_index = item.current_index().expect("a move has a current index");
                    let item_state = self.upstream_items_states[previous_index].clone();
                    let downstream_previous_index = previous_index - self.count_excluded_items_before(previous_index);
                    self.upstream_items_states.remove(previous_index);
                    self.upstream_items_states.insert(current_index, item_state.clone());
                    if item_state.1 {
                        let downstream_current_index = current_index - self.count_excluded_items_before(current_index);
                        if downstream_previous_index != downstream_current_index {
                            self.downstream_items.move_index(downstream_previous_index, downstream_current_index);
                        }
                    }
                }
                ListChangeReason::Refresh => {
                    let item = change.item();
                    let index = item.current_index().expect("a refresh has an index");
                    let is_included = predicate(item.current());
                    let was_included = self.upstream_items_states[index].1;
                    self.upstream_items_states[index].1 = is_included;
                    let downstream_index = if is_included || was_included { Some(index - self.count_excluded_items_before(index)) } else { None };
                    match (was_included, is_included, downstream_index) {
                        (true, true, Some(downstream_index)) => self.downstream_items.refresh_with(item.current().clone(), downstream_index),
                        (false, true, Some(downstream_index)) => self.downstream_items.insert(downstream_index, item.current().clone()),
                        (true, false, Some(downstream_index)) => self.downstream_items.remove_at(downstream_index),
                        _ => {}
                    }
                }
                ListChangeReason::Remove => {
                    let index = change.item().current_index().expect("a removal has an index");
                    if self.upstream_items_states[index].1 {
                        let downstream_index = index - self.count_excluded_items_before(index);
                        self.downstream_items.remove_at(downstream_index);
                    }
                    self.upstream_items_states.remove(index);
                }
                ListChangeReason::RemoveRange => {
                    let range = change.range();
                    let index = range.index().expect("a removed range has an index");
                    let downstream_index = index - self.count_excluded_items_before(index);
                    let range_end = index + range.count();
                    let downstream_count = self.upstream_items_states[index..range_end].iter().filter(|(_, is_included)| *is_included).count();
                    if downstream_count != 0 {
                        self.downstream_items.remove_range(downstream_index, downstream_count);
                    }
                    self.upstream_items_states.drain(index..range_end);
                }
                ListChangeReason::Replace => {
                    let item = change.item();
                    let is_included = predicate(item.current());
                    // A Replace might have a negative CurrentIndex from a Refresh in
                    // RemoveKeyEnumerator.
                    let current_index = match item.current_index() {
                        Some(index) => index,
                        None => self
                            .upstream_items_states
                            .iter()
                            .position(|(state_item, _)| state_item == item.current())
                            .expect("Cannot find index of the replaced item. Expected to be in the list"),
                    };
                    let was_included = self.upstream_items_states[current_index].1;
                    self.upstream_items_states[current_index] = (item.current().clone(), is_included);
                    let downstream_index = if is_included || was_included { Some(current_index - self.count_excluded_items_before(current_index)) } else { None };
                    match (was_included, is_included, downstream_index) {
                        (true, true, Some(downstream_index)) => self.downstream_items.set(downstream_index, item.current().clone()),
                        (true, false, Some(downstream_index)) => self.downstream_items.remove_at(downstream_index),
                        (false, true, Some(downstream_index)) => self.downstream_items.insert(downstream_index, item.current().clone()),
                        _ => {}
                    }
                }
            }
        }
    }
}
