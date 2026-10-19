//! `Filter.Static<TObject, TKey>` and `Filter.Dynamic<TObject, TKey, TState>`
//! (`Cache/Internal/Filter.Static.cs`, `Cache/Internal/Filter.Dynamic.cs`).

use crate::cache::{Change, ChangeAwareCache, ChangeReason, ChangeSet, ICache, IQuery};
use ferroui_reactive::disposables::Disposable;
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::collections::generic::Dictionary;
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::cell::RefCell;
use std::hash::Hash;
use std::rc::Rc;

/// The items of the source that `filter` accepts (`Filter.Static.Create`): an update or a refresh
/// evaluates the item again, which may add or remove it downstream; moves are not supported.
pub(crate) fn filter_static<T: Clone + 'static, K: Eq + Hash + Clone + 'static>(
    source: Rc<dyn IObservable<ChangeSet<T, K>>>,
    filter: Rc<dyn Fn(&T) -> bool>,
    suppress_empty_change_sets: bool,
) -> Rc<dyn IObservable<ChangeSet<T, K>>> {
    Observable::create(move |downstream: Rc<dyn IObserver<ChangeSet<T, K>>>| {
        let downstream_items = RefCell::new(ChangeAwareCache::<T, K>::new());
        let filter = filter.clone();
        source
            .select(move |upstream_changes: ChangeSet<T, K>| {
                let mut items = downstream_items.borrow_mut();
                for change in &upstream_changes {
                    match change.reason() {
                        ChangeReason::Add => {
                            if filter(change.current()) {
                                items.add(change.current().clone(), change.key().clone());
                            }
                        }
                        // Moves are not supported upstream: tracking the indexes is too much work.
                        ChangeReason::Moved => {}
                        ChangeReason::Refresh => {
                            let is_included = filter(change.current());
                            let was_included = items.lookup(change.key()).is_some();
                            if is_included && !was_included {
                                items.add(change.current().clone(), change.key().clone());
                            } else if is_included && was_included {
                                items.refresh(change.key());
                            } else if !is_included && was_included {
                                items.remove(change.key());
                            }
                        }
                        ChangeReason::Remove => items.remove(change.key()),
                        ChangeReason::Update => {
                            if filter(change.current()) {
                                items.add_or_update(change.current().clone(), change.key().clone());
                            } else {
                                items.remove(change.key());
                            }
                        }
                    }
                }
                items.capture_changes()
            })
            .where_(move |changes: &ChangeSet<T, K>| !suppress_empty_change_sets || changes.count() != 0)
            .subscribe(downstream)
    })
}

/// The state of an item of the source (`Filter.Dynamic.ItemState`).
struct ItemState<T> {
    is_included: bool,
    item: T,
}

/// The mutable part of a subscription of [`filter_dynamic`].
struct DynamicFilterState<T, K, S> {
    downstream_changes_buffer: Vec<Change<T, K>>,
    item_states_by_key: Dictionary<K, ItemState<T>>,
    has_initialized: bool,
    has_predicate_state_completed: bool,
    has_reapply_filter_completed: bool,
    has_source_completed: bool,
    /// `_latestPredicateState`, with `_isLatestPredicateStateValid` as its presence.
    latest_predicate_state: Option<S>,
}

/// A subscription of [`filter_dynamic`] (`Filter.Dynamic.Subscription`).
struct DynamicFilter<T, K, S> {
    downstream_observer: Rc<dyn IObserver<ChangeSet<T, K>>>,
    predicate: Rc<dyn Fn(&S, &T) -> bool>,
    suppress_empty_change_sets: bool,
    state: RefCell<DynamicFilterState<T, K, S>>,
    predicate_state_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    reapply_filter_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    source_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
}

/// The items of the source that `predicate` accepts with the latest value of `predicate_state`
/// (`Filter.Dynamic<TObject, TKey, TState>.Create`): each value of the state evaluates every item
/// again, as does each value of `reapply_filter`; an update or a refresh evaluates its item again.
/// Until the state has a value, no item is included.
///
/// The locks of upstream (the gates of the upstreams and of the downstream) become nothing: a
/// notification is computed with the state borrowed, and delivered after the borrow ends.
pub(crate) fn filter_dynamic<T: Clone + 'static, K: Eq + Hash + Clone + 'static, S: 'static>(
    source: Rc<dyn IObservable<ChangeSet<T, K>>>,
    predicate_state: Rc<dyn IObservable<S>>,
    predicate: Rc<dyn Fn(&S, &T) -> bool>,
    reapply_filter: Rc<dyn IObservable<()>>,
    suppress_empty_change_sets: bool,
) -> Rc<dyn IObservable<ChangeSet<T, K>>> {
    Observable::create(move |downstream_observer: Rc<dyn IObserver<ChangeSet<T, K>>>| {
        let subscription = Rc::new(DynamicFilter {
            downstream_observer,
            predicate: predicate.clone(),
            suppress_empty_change_sets,
            state: RefCell::new(DynamicFilterState {
                downstream_changes_buffer: Vec::new(),
                item_states_by_key: Dictionary::new(),
                has_initialized: false,
                has_predicate_state_completed: false,
                has_reapply_filter_completed: false,
                has_source_completed: false,
                latest_predicate_state: None,
            }),
            predicate_state_subscription: RefCell::new(None),
            reapply_filter_subscription: RefCell::new(None),
            source_subscription: RefCell::new(None),
        });
        DynamicFilter::activate(&subscription, &predicate_state, &reapply_filter, &source);
        let disposed = subscription.clone();
        Disposable::create(move || disposed.dispose())
    })
}

impl<T: Clone + 'static, K: Eq + Hash + Clone + 'static, S: 'static> DynamicFilter<T, K, S> {
    /// The subscriptions of the constructor of upstream's subscription, in its order.
    fn activate(
        this: &Rc<Self>,
        predicate_state: &Rc<dyn IObservable<S>>,
        reapply_filter: &Rc<dyn IObservable<()>>,
        source: &Rc<dyn IObservable<ChangeSet<T, K>>>,
    ) {
        let (next, error, completed) = (this.clone(), this.clone(), this.clone());
        let subscription = predicate_state.subscribe_all(
            move |state| {
                next.on_predicate_state_next(state);
            },
            move |e| {
                error.on_error(e);
            },
            move || {
                completed.on_predicate_state_completed();
            },
        );
        *this.predicate_state_subscription.borrow_mut() = Some(subscription);

        let (next, error, completed) = (this.clone(), this.clone(), this.clone());
        let subscription = reapply_filter.subscribe_all(
            move |_| {
                next.on_reapply_filter_next();
            },
            move |e| {
                error.on_error(e);
            },
            move || {
                completed.on_reapply_filter_completed();
            },
        );
        *this.reapply_filter_subscription.borrow_mut() = Some(subscription);

        let (next, error, completed) = (this.clone(), this.clone(), this.clone());
        let subscription = source.subscribe_all(
            move |changes| {
                next.on_source_next(changes);
            },
            move |e| {
                error.on_error(e);
            },
            move || {
                completed.on_source_completed();
            },
        );
        *this.source_subscription.borrow_mut() = Some(subscription);

        let need_to_complete = {
            let mut state = this.state.borrow_mut();
            state.has_initialized = true;
            // We withhold completions triggered by the other upstreams until after the source
            // stream has a chance to publish an initial changeset. If that happens, we need to
            // publish the completion now.
            this.suppress_empty_change_sets && state.has_predicate_state_completed && state.latest_predicate_state.is_none()
        };
        if need_to_complete {
            this.downstream_observer.on_completed();
        }
    }

    fn dispose(&self) {
        for slot in [&self.predicate_state_subscription, &self.reapply_filter_subscription, &self.source_subscription] {
            let subscription = slot.borrow_mut().take();
            if let Some(subscription) = subscription {
                subscription.dispose();
            }
        }
    }

    fn assemble_downstream_changes(state: &mut DynamicFilterState<T, K, S>) -> ChangeSet<T, K> {
        if state.downstream_changes_buffer.is_empty() {
            return ChangeSet::empty();
        }
        ChangeSet::from(std::mem::take(&mut state.downstream_changes_buffer))
    }

    fn on_error(&self, error: ObservableError) {
        // Upstream disposes the subscriptions of the state and of the source, not that of the
        // reapplications.
        for slot in [&self.predicate_state_subscription, &self.source_subscription] {
            let subscription = slot.borrow_mut().take();
            if let Some(subscription) = subscription {
                subscription.dispose();
            }
        }
        self.downstream_observer.on_error(error);
    }

    fn on_predicate_state_completed(&self) {
        let complete = {
            let mut state = self.state.borrow_mut();
            state.has_predicate_state_completed = true;
            // If we didn't get at least one predicateState value, we can't ever emit any
            // (non-empty) downstream changesets, no matter how many items come through from
            // source, so just go ahead and complete now.
            state.has_initialized
                && ((state.has_reapply_filter_completed && state.has_source_completed)
                    || (state.latest_predicate_state.is_none() && self.suppress_empty_change_sets))
        };
        if complete {
            self.downstream_observer.on_completed();
        }
    }

    fn on_predicate_state_next(&self, predicate_state: S) {
        let downstream_changes = {
            let mut state = self.state.borrow_mut();
            state.latest_predicate_state = Some(predicate_state);
            self.re_filter(&mut state);
            let downstream_changes = Self::assemble_downstream_changes(&mut state);
            ((downstream_changes.count() != 0 || !self.suppress_empty_change_sets) && state.has_initialized).then_some(downstream_changes)
        };
        if let Some(downstream_changes) = downstream_changes {
            self.downstream_observer.on_next(downstream_changes);
        }
    }

    fn on_reapply_filter_completed(&self) {
        let complete = {
            let mut state = self.state.borrow_mut();
            state.has_reapply_filter_completed = true;
            // If the other two sources have also completed, there's no chance of us ever needing
            // to emit further changesets.
            state.has_predicate_state_completed && state.has_source_completed
        };
        if complete {
            self.downstream_observer.on_completed();
        }
    }

    fn on_reapply_filter_next(&self) {
        let downstream_changes = {
            let mut state = self.state.borrow_mut();
            if state.latest_predicate_state.is_some() {
                self.re_filter(&mut state);
            }
            let downstream_changes = Self::assemble_downstream_changes(&mut state);
            ((downstream_changes.count() != 0 || !self.suppress_empty_change_sets) && state.has_initialized).then_some(downstream_changes)
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
            (state.has_predicate_state_completed && state.has_reapply_filter_completed)
                || (self.suppress_empty_change_sets
                    && (state.item_states_by_key.is_empty() || (state.has_predicate_state_completed && state.latest_predicate_state.is_none())))
        };
        if complete {
            self.downstream_observer.on_completed();
        }
    }

    fn on_source_next(&self, upstream_changes: ChangeSet<T, K>) {
        let downstream_changes = {
            let mut guard = self.state.borrow_mut();
            let state = &mut *guard;
            for change in &upstream_changes {
                let key = change.key();
                match change.reason() {
                    ChangeReason::Add => {
                        let is_included = state.latest_predicate_state.as_ref().is_some_and(|latest| (self.predicate)(latest, change.current()));
                        state.item_states_by_key.add(key.clone(), ItemState { is_included, item: change.current().clone() });
                        if is_included {
                            state.downstream_changes_buffer.push(Change::new(ChangeReason::Add, key.clone(), change.current().clone()));
                        }
                    }
                    // Intentionally not supporting Moved changes, too much work to try and track
                    // indexes.
                    ChangeReason::Moved => {}
                    ChangeReason::Refresh => {
                        let item_state = state.item_states_by_key.get(key).expect("the refreshed item is in the source");
                        let item = item_state.item.clone();
                        let was_included = item_state.is_included;
                        let is_included = state.latest_predicate_state.as_ref().is_some_and(|latest| (self.predicate)(latest, &item));
                        if is_included && was_included {
                            state.downstream_changes_buffer.push(Change::new(ChangeReason::Refresh, key.clone(), item.clone()));
                        } else if is_included && !was_included {
                            state.downstream_changes_buffer.push(Change::new(ChangeReason::Add, key.clone(), item.clone()));
                        } else if !is_included && was_included {
                            state.downstream_changes_buffer.push(Change::new(ChangeReason::Remove, key.clone(), item.clone()));
                        }
                        state.item_states_by_key.insert(key.clone(), ItemState { is_included, item });
                    }
                    ChangeReason::Remove => {
                        let item_state = state.item_states_by_key.remove(key).expect("the removed item is in the source");
                        if item_state.is_included {
                            state.downstream_changes_buffer.push(Change::new(ChangeReason::Remove, key.clone(), item_state.item));
                        }
                    }
                    ChangeReason::Update => {
                        let item_state = state.item_states_by_key.get(key).expect("the updated item is in the source");
                        let was_included = item_state.is_included;
                        let previous = item_state.item.clone();
                        let is_included = state.latest_predicate_state.as_ref().is_some_and(|latest| (self.predicate)(latest, change.current()));
                        if is_included && was_included {
                            state.downstream_changes_buffer.push(Change::update(key.clone(), change.current().clone(), previous));
                        } else if is_included && !was_included {
                            state.downstream_changes_buffer.push(Change::new(ChangeReason::Add, key.clone(), change.current().clone()));
                        } else if !is_included && was_included {
                            state.downstream_changes_buffer.push(Change::new(ChangeReason::Remove, key.clone(), previous));
                        }
                        state.item_states_by_key.insert(key.clone(), ItemState { is_included, item: change.current().clone() });
                    }
                }
            }
            let downstream_changes = Self::assemble_downstream_changes(state);
            (downstream_changes.count() != 0 || !self.suppress_empty_change_sets).then_some(downstream_changes)
        };
        if let Some(downstream_changes) = downstream_changes {
            self.downstream_observer.on_next(downstream_changes);
        }
    }

    /// Evaluates every item with the latest state (`ReFilter`).
    fn re_filter(&self, state: &mut DynamicFilterState<T, K, S>) {
        let DynamicFilterState { downstream_changes_buffer, item_states_by_key, latest_predicate_state, .. } = state;
        let Some(latest) = latest_predicate_state.as_ref() else { return };
        for (key, item_state) in item_states_by_key.iter_mut() {
            let is_included = (self.predicate)(latest, &item_state.item);
            if is_included && !item_state.is_included {
                downstream_changes_buffer.push(Change::new(ChangeReason::Add, key.clone(), item_state.item.clone()));
            } else if !is_included && item_state.is_included {
                downstream_changes_buffer.push(Change::new(ChangeReason::Remove, key.clone(), item_state.item.clone()));
            }
            item_state.is_included = is_included;
        }
    }
}
