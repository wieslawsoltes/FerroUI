//! `Filter.WithPredicateState<T, TState>` (`List/Internal/Filter.WithPredicateState.cs`): the
//! items of a list that a predicate accepts with the latest value of a state.

use crate::list::{Change, ChangeSet, ItemChange, ListChangeReason, ListFilterPolicy, RangeChange};
use ferroui_reactive::disposables::Disposable;
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::cell::RefCell;
use std::rc::Rc;

/// The state of an item of the source: its index in the filtered list when it is included
/// (`SubscriptionBase.ItemState`).
#[derive(Clone)]
struct ItemState<T> {
    filtered_index: Option<usize>,
    item: T,
}

/// The mutable part of a subscription (the fields of `SubscriptionBase` and of its two kinds).
struct FilterState<T, S> {
    downstream_changes_buffer: Vec<Change<T>>,
    item_states: Vec<ItemState<T>>,
    has_predicate_state_completed: bool,
    has_source_completed: bool,
    /// `_latestPredicateState`, with `_isLatestPredicateStateValid` as its presence.
    latest_predicate_state: Option<S>,
    /// `ClearAndReplaceSubscription._filteredCount`.
    filtered_count: usize,
}

/// A subscription (`SubscriptionBase`, with `CalculateDiffSubscription` and
/// `ClearAndReplaceSubscription` as its policy).
struct Subscription<T, S> {
    downstream_observer: Rc<dyn IObserver<ChangeSet<T>>>,
    predicate: Rc<dyn Fn(&S, &T) -> bool>,
    policy: ListFilterPolicy,
    suppress_empty_change_sets: bool,
    state: RefCell<FilterState<T, S>>,
    predicate_state_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    source_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
}

/// The items of the source that `predicate` accepts with the latest value of `predicate_state`,
/// in their order (`Filter.WithPredicateState<T, TState>.Create`). Until the state has a value, no
/// item is included. `filter_policy` decides what a new value of the state emits: the differences
/// (`CalculateDiff`), or a clear and the matching items again (`ClearAndReplace`, which does not
/// propagate moves and appends the items that a change includes).
///
/// The locks of upstream become nothing: a notification is computed with the state borrowed and
/// delivered after the borrow ends.
pub(crate) fn filter_with_predicate_state<T: Clone + PartialEq + 'static, S: 'static>(
    source: Rc<dyn IObservable<ChangeSet<T>>>,
    predicate_state: Rc<dyn IObservable<S>>,
    predicate: Rc<dyn Fn(&S, &T) -> bool>,
    filter_policy: ListFilterPolicy,
    suppress_empty_change_sets: bool,
) -> Rc<dyn IObservable<ChangeSet<T>>> {
    Observable::create(move |downstream_observer: Rc<dyn IObserver<ChangeSet<T>>>| {
        let subscription = Rc::new(Subscription {
            downstream_observer,
            predicate: predicate.clone(),
            policy: filter_policy,
            suppress_empty_change_sets,
            state: RefCell::new(FilterState {
                downstream_changes_buffer: Vec::new(),
                item_states: Vec::new(),
                has_predicate_state_completed: false,
                has_source_completed: false,
                latest_predicate_state: None,
                filtered_count: 0,
            }),
            predicate_state_subscription: RefCell::new(None),
            source_subscription: RefCell::new(None),
        });
        Subscription::activate(&subscription, &predicate_state, &source);
        let disposed = subscription.clone();
        Disposable::create(move || disposed.dispose())
    })
}

impl<T: Clone + PartialEq + 'static, S: 'static> Subscription<T, S> {
    /// Keeping subscriptions out of the constructor prevents subscriptions that emit immediately
    /// from triggering virtual method calls within the constructor (`Activate`).
    fn activate(this: &Rc<Self>, predicate_state: &Rc<dyn IObservable<S>>, source: &Rc<dyn IObservable<ChangeSet<T>>>) {
        let (next, error, completed) = (this.clone(), this.clone(), this.clone());
        let subscription = predicate_state.subscribe_all(move |state| next.on_predicate_state_next(state), move |e| error.on_error(e), move || completed.on_predicate_state_completed());
        *this.predicate_state_subscription.borrow_mut() = Some(subscription);
        let (next, error, completed) = (this.clone(), this.clone(), this.clone());
        let subscription = source.subscribe_all(move |changes| next.on_source_next(changes), move |e| error.on_error(e), move || completed.on_source_completed());
        *this.source_subscription.borrow_mut() = Some(subscription);
    }

    fn dispose(&self) {
        for slot in [&self.predicate_state_subscription, &self.source_subscription] {
            let subscription = slot.borrow_mut().take();
            if let Some(subscription) = subscription {
                subscription.dispose();
            }
        }
    }

    fn assemble_downstream_changes(state: &mut FilterState<T, S>) -> ChangeSet<T> {
        if state.downstream_changes_buffer.is_empty() {
            return ChangeSet::empty();
        }
        ChangeSet::from(std::mem::take(&mut state.downstream_changes_buffer))
    }

    fn on_error(&self, error: ObservableError) {
        self.dispose();
        self.downstream_observer.on_error(error);
    }

    fn on_predicate_state_completed(&self) {
        let complete = {
            let mut state = self.state.borrow_mut();
            state.has_predicate_state_completed = true;
            // If we didn't get at least one predicateState value, we can't ever emit any
            // (non-empty) downstream changesets, no matter how many items come through from
            // source, so just go ahead and complete now.
            state.has_source_completed || (state.latest_predicate_state.is_none() && self.suppress_empty_change_sets)
        };
        if complete {
            self.downstream_observer.on_completed();
        }
    }

    fn on_predicate_state_next(&self, predicate_state: S) {
        let downstream_changes = {
            let mut state = self.state.borrow_mut();
            state.latest_predicate_state = Some(predicate_state);
            match self.policy {
                ListFilterPolicy::CalculateDiff => self.calculate_diff_re_filter(&mut state),
                ListFilterPolicy::ClearAndReplace => self.clear_and_replace_re_filter(&mut state),
            }
            let downstream_changes = Self::assemble_downstream_changes(&mut state);
            (downstream_changes.count() != 0 || !self.suppress_empty_change_sets).then_some(downstream_changes)
        };
        if let Some(downstream_changes) = downstream_changes {
            self.downstream_observer.on_next(downstream_changes);
        }
    }

    fn on_source_completed(&self) {
        let complete = {
            let mut state = self.state.borrow_mut();
            state.has_source_completed = true;
            // We can never emit any (non-empty) downstream changes in the future, if the
            // collection is empty and the source has reported that it'll never change, so go
            // ahead and complete now.
            state.has_predicate_state_completed || (state.item_states.is_empty() && self.suppress_empty_change_sets)
        };
        if complete {
            self.downstream_observer.on_completed();
        }
    }

    fn on_source_next(&self, upstream_changes: ChangeSet<T>) {
        let downstream_changes = {
            let mut state = self.state.borrow_mut();
            for change in &upstream_changes {
                match change.reason() {
                    ListChangeReason::Add => self.perform_add(&mut state, change.item()),
                    ListChangeReason::AddRange => self.perform_add_range(&mut state, change.range()),
                    ListChangeReason::Clear => {
                        if !state.item_states.is_empty() {
                            self.perform_clear(&mut state);
                        }
                    }
                    ListChangeReason::Moved => {
                        let item = change.item();
                        if item.current_index() != item.previous_index() {
                            self.perform_move(&mut state, item);
                        }
                    }
                    ListChangeReason::Refresh => self.perform_refresh(&mut state, change.item()),
                    ListChangeReason::Remove => self.perform_remove(&mut state, change.item()),
                    ListChangeReason::RemoveRange => self.perform_remove_range(&mut state, change.range()),
                    ListChangeReason::Replace => self.perform_replace(&mut state, change.item()),
                }
            }
            let downstream_changes = Self::assemble_downstream_changes(&mut state);
            (downstream_changes.count() != 0 || !self.suppress_empty_change_sets).then_some(downstream_changes)
        };
        if let Some(downstream_changes) = downstream_changes {
            self.downstream_observer.on_next(downstream_changes);
        }
    }

    /// Whether the latest state accepts `item`; nothing is accepted before the first state.
    fn is_included(&self, state: &FilterState<T, S>, item: &T) -> bool {
        state.latest_predicate_state.as_ref().is_some_and(|latest| (self.predicate)(latest, item))
    }

    /// The filtered index after the last included item before `index`.
    fn next_filtered_index_before(state: &FilterState<T, S>, index: usize) -> usize {
        state.item_states[..index].iter().rev().find_map(|item_state| item_state.filtered_index).map_or(0, |prior| prior + 1)
    }

    /// Adds `delta` to the filtered indexes of the items from `start`.
    fn shift_filtered_indexes_from(state: &mut FilterState<T, S>, start: usize, increase: bool, amount: usize) {
        for other in state.item_states[start..].iter_mut() {
            if let Some(index) = other.filtered_index.as_mut() {
                if increase {
                    *index += amount;
                } else {
                    *index -= amount;
                }
            }
        }
    }

    /// Decrements the filtered indexes greater than `filtered_index`, anywhere in the list.
    fn decrement_filtered_indexes_after(state: &mut FilterState<T, S>, filtered_index: usize) {
        for other in state.item_states.iter_mut() {
            if let Some(index) = other.filtered_index.as_mut() {
                if *index > filtered_index {
                    *index -= 1;
                }
            }
        }
    }

    fn perform_add(&self, state: &mut FilterState<T, S>, change: &ItemChange<T>) {
        let current_index = change.current_index().expect("an addition has an index");
        let is_included = self.is_included(state, change.current());
        let mut filtered_index = None;
        if is_included {
            let index = match self.policy {
                ListFilterPolicy::CalculateDiff => Self::next_filtered_index_before(state, current_index),
                ListFilterPolicy::ClearAndReplace => {
                    let index = state.filtered_count;
                    state.filtered_count += 1;
                    index
                }
            };
            filtered_index = Some(index);
            state.downstream_changes_buffer.push(Change::new(ListChangeReason::Add, change.current().clone(), Some(index)));
        }
        state.item_states.insert(current_index, ItemState { filtered_index, item: change.current().clone() });
        if self.policy == ListFilterPolicy::CalculateDiff {
            Self::shift_filtered_indexes_from(state, current_index + 1, true, 1);
        }
    }

    fn perform_add_range(&self, state: &mut FilterState<T, S>, change: &RangeChange<T>) {
        let start = change.index().unwrap_or(state.item_states.len());
        let (mut next_filtered_index, filtered_insert_index) = match self.policy {
            ListFilterPolicy::CalculateDiff => {
                let index = Self::next_filtered_index_before(state, start);
                (index, index)
            }
            ListFilterPolicy::ClearAndReplace => (state.filtered_count, state.filtered_count),
        };
        let mut item_states_buffer = Vec::with_capacity(change.count());
        let mut items_buffer = Vec::new();
        for item in change {
            let is_included = self.is_included(state, item);
            let mut filtered_index = None;
            if is_included {
                filtered_index = Some(next_filtered_index);
                next_filtered_index += 1;
                items_buffer.push(item.clone());
            }
            item_states_buffer.push(ItemState { filtered_index, item: item.clone() });
        }
        if self.policy == ListFilterPolicy::ClearAndReplace {
            state.filtered_count = next_filtered_index;
        }
        if item_states_buffer.is_empty() {
            return;
        }
        let count = item_states_buffer.len();
        state.item_states.splice(start..start, item_states_buffer);
        if self.policy == ListFilterPolicy::CalculateDiff {
            // As upstream, the indexes after the range move by one, whatever the number of the
            // items included.
            Self::shift_filtered_indexes_from(state, start + count, true, 1);
        }
        if !items_buffer.is_empty() {
            state.downstream_changes_buffer.push(Change::range_change(ListChangeReason::AddRange, items_buffer, Some(filtered_insert_index)));
        }
    }

    fn perform_clear(&self, state: &mut FilterState<T, S>) {
        let items_buffer: Vec<T> = match self.policy {
            ListFilterPolicy::CalculateDiff => state.item_states.iter().filter(|item_state| item_state.filtered_index.is_some()).map(|item_state| item_state.item.clone()).collect(),
            ListFilterPolicy::ClearAndReplace => {
                let mut items: Vec<Option<T>> = vec![None; state.filtered_count];
                for item_state in &state.item_states {
                    if let Some(filtered_index) = item_state.filtered_index {
                        items[filtered_index] = Some(item_state.item.clone());
                    }
                }
                state.filtered_count = 0;
                items.into_iter().map(|item| item.expect("each filtered index has an item")).collect()
            }
        };
        state.item_states.clear();
        if !items_buffer.is_empty() {
            state.downstream_changes_buffer.push(Change::range_change(ListChangeReason::Clear, items_buffer, None));
        }
    }

    fn perform_move(&self, state: &mut FilterState<T, S>, change: &ItemChange<T>) {
        let previous_index = change.previous_index().expect("a move has a previous index");
        let current_index = change.current_index().expect("a move has a current index");
        let mut item_state = state.item_states[previous_index].clone();
        if self.policy == ListFilterPolicy::ClearAndReplace {
            // We're not supporting propagation of move changes, but we do still need to process
            // them, to keep ItemStates correct.
            state.item_states.remove(previous_index);
            state.item_states.insert(current_index, item_state);
            return;
        }
        if let Some(previous_filtered_index) = item_state.filtered_index {
            // Determine the filtered index to use for the moved item, after the move, by searching
            // backwards from the target location.
            let current_filtered_index = if current_index > previous_index {
                // When moving forwards, only search back to the original position of the item, to
                // see if the index needs to change at all, and if it does, account for the fact
                // that the filtered index of the items in that range will need to be adjusted by
                // -1, to account for the move.
                (previous_index..=current_index).rev().find_map(|i| state.item_states[i].filtered_index).unwrap_or(previous_filtered_index)
            } else {
                // When moving backwards, search to the beginning of the list, where items'
                // filtered indexes will not be changing.
                Self::next_filtered_index_before(state, current_index)
            };
            if current_filtered_index != previous_filtered_index {
                state.downstream_changes_buffer.push(Change::moved(change.current().clone(), current_filtered_index, previous_filtered_index));
                item_state.filtered_index = Some(current_filtered_index);
            }
        }
        state.item_states.remove(previous_index);
        let is_included = item_state.filtered_index.is_some();
        state.item_states.insert(current_index, item_state);
        if is_included {
            if current_index < previous_index {
                for i in current_index + 1..=previous_index {
                    if let Some(index) = state.item_states[i].filtered_index.as_mut() {
                        *index += 1;
                    }
                }
            } else {
                for i in previous_index..current_index {
                    if let Some(index) = state.item_states[i].filtered_index.as_mut() {
                        *index -= 1;
                    }
                }
            }
        }
    }

    fn calculate_diff_re_filter(&self, state: &mut FilterState<T, S>) {
        let FilterState { downstream_changes_buffer, item_states, latest_predicate_state, .. } = state;
        let Some(latest) = latest_predicate_state.as_ref() else { return };
        let mut next_filtered_index = 0;
        for item_state in item_states.iter_mut() {
            let is_included = (self.predicate)(latest, &item_state.item);
            if item_state.filtered_index.is_some() {
                if is_included {
                    item_state.filtered_index = Some(next_filtered_index);
                    next_filtered_index += 1;
                } else {
                    item_state.filtered_index = None;
                    downstream_changes_buffer.push(Change::new(ListChangeReason::Remove, item_state.item.clone(), Some(next_filtered_index)));
                }
            } else if is_included {
                downstream_changes_buffer.push(Change::new(ListChangeReason::Add, item_state.item.clone(), Some(next_filtered_index)));
                item_state.filtered_index = Some(next_filtered_index);
                next_filtered_index += 1;
            }
        }
    }

    fn clear_and_replace_re_filter(&self, state: &mut FilterState<T, S>) {
        let FilterState { downstream_changes_buffer, item_states, latest_predicate_state, filtered_count, .. } = state;
        let Some(latest) = latest_predicate_state.as_ref() else { return };
        let mut next_filtered_index = 0;
        let mut cleared_items: Vec<Option<T>> = vec![None; *filtered_count];
        let mut items_buffer = Vec::new();
        for item_state in item_states.iter_mut() {
            let is_included = (self.predicate)(latest, &item_state.item);
            if let Some(filtered_index) = item_state.filtered_index {
                cleared_items[filtered_index] = Some(item_state.item.clone());
                if is_included {
                    items_buffer.push(item_state.item.clone());
                    item_state.filtered_index = Some(next_filtered_index);
                    next_filtered_index += 1;
                } else {
                    *filtered_count -= 1;
                    item_state.filtered_index = None;
                }
            } else if is_included {
                *filtered_count += 1;
                item_state.filtered_index = Some(next_filtered_index);
                items_buffer.push(item_state.item.clone());
                next_filtered_index += 1;
            }
        }
        if !cleared_items.is_empty() {
            let cleared_items: Vec<T> = cleared_items.into_iter().map(|item| item.expect("each filtered index has an item")).collect();
            downstream_changes_buffer.push(Change::range_change(ListChangeReason::Clear, cleared_items, None));
        }
        if !items_buffer.is_empty() {
            downstream_changes_buffer.push(Change::range_change(ListChangeReason::AddRange, items_buffer, Some(0)));
        }
    }

    fn perform_refresh(&self, state: &mut FilterState<T, S>, change: &ItemChange<T>) {
        let current_index = change.current_index().expect("a refresh has an index");
        let item_state_index = state.item_states[current_index].filtered_index;
        let is_included = self.is_included(state, change.current());
        match item_state_index {
            Some(filtered_index) => {
                if is_included {
                    state.downstream_changes_buffer.push(Change::new(ListChangeReason::Refresh, change.current().clone(), Some(filtered_index)));
                } else {
                    state.downstream_changes_buffer.push(Change::new(ListChangeReason::Remove, change.current().clone(), Some(filtered_index)));
                    state.item_states[current_index] = ItemState { filtered_index: None, item: change.current().clone() };
                    match self.policy {
                        ListFilterPolicy::CalculateDiff => Self::shift_filtered_indexes_from(state, current_index + 1, false, 1),
                        ListFilterPolicy::ClearAndReplace => {
                            state.filtered_count -= 1;
                            Self::decrement_filtered_indexes_after(state, filtered_index);
                        }
                    }
                }
            }
            None if is_included => {
                let filtered_index = match self.policy {
                    ListFilterPolicy::CalculateDiff => Self::next_filtered_index_before(state, current_index),
                    ListFilterPolicy::ClearAndReplace => {
                        let index = state.filtered_count;
                        state.filtered_count += 1;
                        index
                    }
                };
                state.downstream_changes_buffer.push(Change::new(ListChangeReason::Add, change.current().clone(), Some(filtered_index)));
                state.item_states[current_index] = ItemState { filtered_index: Some(filtered_index), item: change.current().clone() };
                if self.policy == ListFilterPolicy::CalculateDiff {
                    Self::shift_filtered_indexes_from(state, current_index + 1, true, 1);
                }
            }
            None => {}
        }
    }

    fn perform_remove(&self, state: &mut FilterState<T, S>, change: &ItemChange<T>) {
        let current_index = change.current_index().expect("a removal has an index");
        let item_state = state.item_states.remove(current_index);
        if let Some(filtered_index) = item_state.filtered_index {
            match self.policy {
                ListFilterPolicy::CalculateDiff => Self::shift_filtered_indexes_from(state, current_index, false, 1),
                ListFilterPolicy::ClearAndReplace => {
                    state.filtered_count -= 1;
                    Self::decrement_filtered_indexes_after(state, filtered_index);
                }
            }
            state.downstream_changes_buffer.push(Change::new(ListChangeReason::Remove, change.current().clone(), Some(filtered_index)));
        }
    }

    fn perform_remove_range(&self, state: &mut FilterState<T, S>, change: &RangeChange<T>) {
        let index = change.index().expect("a removed range has an index");
        let end = index + change.count();
        match self.policy {
            ListFilterPolicy::CalculateDiff => {
                let mut filtered_range_index = None;
                let mut items_buffer = Vec::new();
                for item_state in &state.item_states[index..end] {
                    if let Some(filtered_index) = item_state.filtered_index {
                        filtered_range_index.get_or_insert(filtered_index);
                        items_buffer.push(item_state.item.clone());
                    }
                }
                state.item_states.drain(index..end);
                if !items_buffer.is_empty() {
                    let removed = items_buffer.len();
                    state.downstream_changes_buffer.push(Change::range_change(ListChangeReason::RemoveRange, items_buffer, filtered_range_index));
                    Self::shift_filtered_indexes_from(state, index, false, removed);
                }
            }
            ListFilterPolicy::ClearAndReplace => {
                for i in index..end {
                    let item_state = state.item_states[i].clone();
                    if let Some(filtered_index) = item_state.filtered_index {
                        state.downstream_changes_buffer.push(Change::new(ListChangeReason::Remove, item_state.item.clone(), Some(filtered_index)));
                        Self::decrement_filtered_indexes_after(state, filtered_index);
                        state.filtered_count -= 1;
                    }
                }
                state.item_states.drain(index..end);
            }
        }
    }

    fn perform_replace(&self, state: &mut FilterState<T, S>, change: &ItemChange<T>) {
        let current_index = change.current_index().expect("a replacement has an index");
        let item_state = state.item_states[current_index].clone();
        let is_included = self.is_included(state, change.current());
        match item_state.filtered_index {
            Some(filtered_index) => {
                if is_included {
                    state.downstream_changes_buffer.push(Change::with_previous(
                        ListChangeReason::Replace,
                        change.current().clone(),
                        change.previous().cloned(),
                        Some(filtered_index),
                        Some(filtered_index),
                    ));
                    state.item_states[current_index].item = change.current().clone();
                } else {
                    let previous = change.previous().expect("a replacement has a previous item").clone();
                    state.downstream_changes_buffer.push(Change::new(ListChangeReason::Remove, previous, Some(filtered_index)));
                    state.item_states[current_index] = ItemState { filtered_index: None, item: change.current().clone() };
                    match self.policy {
                        ListFilterPolicy::CalculateDiff => Self::shift_filtered_indexes_from(state, current_index + 1, false, 1),
                        ListFilterPolicy::ClearAndReplace => {
                            state.filtered_count -= 1;
                            Self::decrement_filtered_indexes_after(state, filtered_index);
                        }
                    }
                }
            }
            None if is_included => {
                let filtered_index = match self.policy {
                    ListFilterPolicy::CalculateDiff => Self::next_filtered_index_before(state, current_index),
                    ListFilterPolicy::ClearAndReplace => {
                        let index = state.filtered_count;
                        state.filtered_count += 1;
                        index
                    }
                };
                state.downstream_changes_buffer.push(Change::new(ListChangeReason::Add, change.current().clone(), Some(filtered_index)));
                state.item_states[current_index] = ItemState { filtered_index: Some(filtered_index), item: change.current().clone() };
                if self.policy == ListFilterPolicy::CalculateDiff {
                    Self::shift_filtered_indexes_from(state, current_index + 1, true, 1);
                }
            }
            None => state.item_states[current_index].item = change.current().clone(),
        }
    }
}
