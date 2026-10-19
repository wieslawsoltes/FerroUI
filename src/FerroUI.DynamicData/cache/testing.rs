//! The aids for tests that DynamicData ships with the library (`Cache/Tests`, namespace
//! `DynamicData.Tests`): `ChangeSetAggregator<TObject, TKey>`.

use super::{ChangeSet, IObservableCache, ObservableCache, ObservableCacheEx};
use crate::diagnostics::ChangeSummary;
use ferroui_reactive::disposables::Disposable;
use ferroui_reactive::ObservableExt;
use ferroui_system::{IDisposable, IObservable, ObservableError};
use std::cell::{Cell, RefCell};
use std::hash::Hash;
use std::rc::Rc;

struct State<T, K> {
    messages: RefCell<Vec<ChangeSet<T, K>>>,
    error: RefCell<Option<ObservableError>>,
    is_completed: Cell<bool>,
    summary: RefCell<ChangeSummary>,
}

/// Records what a sequence of change sets of a cache produces (`ChangeSetAggregator<TObject,
/// TKey>`): the change sets, the items they make, their statistics, the error and the end.
pub struct ChangeSetAggregator<T, K> {
    state: Rc<State<T, K>>,
    data: ObservableCache<T, K>,
    disposer: Rc<dyn IDisposable>,
}

impl<T: Clone + 'static, K: Eq + Hash + Clone + 'static> ChangeSetAggregator<T, K> {
    /// Subscribes to `source` (`ChangeSetAggregator(IObservable<IChangeSet>)`).
    pub fn new(source: Rc<dyn IObservable<ChangeSet<T, K>>>) -> Self {
        let state = Rc::new(State { messages: RefCell::new(Vec::new()), error: RefCell::new(None), is_completed: Cell::new(false), summary: RefCell::new(ChangeSummary::empty()) });
        let published = source.publish();
        let published_source: Rc<dyn IObservable<ChangeSet<T, K>>> = published.clone();
        let data = published_source.as_observable_cache();
        let (on_next, on_error, on_completed) = (state.clone(), state.clone(), state.clone());
        let results = published_source.subscribe_all(
            move |changes| on_next.messages.borrow_mut().push(changes),
            move |error| *on_error.error.borrow_mut() = Some(error),
            move || on_completed.is_completed.set(true),
        );
        let summary_target = state.clone();
        let summariser = published_source.collect_update_stats().subscribe_all(move |summary| *summary_target.summary.borrow_mut() = summary, |_| {}, || {});
        let connected = published.connect();
        let cache = data.clone();
        let disposer = Disposable::create(move || {
            cache.dispose();
            connected.dispose();
            summariser.dispose();
            results.dispose();
        });
        Self { state, data, disposer }
    }

    /// The items the change sets make (`Data`).
    pub fn data(&self) -> &ObservableCache<T, K> {
        &self.data
    }

    /// The change sets so far (`Messages`).
    pub fn messages(&self) -> Vec<ChangeSet<T, K>> {
        self.state.messages.borrow().clone()
    }

    /// The error of the sequence (`Error`).
    pub fn error(&self) -> Option<ObservableError> {
        self.state.error.borrow().clone()
    }

    /// Whether the sequence has ended (`IsCompleted`).
    pub fn is_completed(&self) -> bool {
        self.state.is_completed.get()
    }

    /// The statistics of the change sets (`Summary`).
    pub fn summary(&self) -> ChangeSummary {
        self.state.summary.borrow().clone()
    }

    /// The number of items the change sets make.
    pub fn count(&self) -> usize {
        self.data.count()
    }
}

impl<T, K> IDisposable for ChangeSetAggregator<T, K> {
    fn dispose(&self) {
        self.disposer.dispose();
    }
}

impl<T, K> Drop for ChangeSetAggregator<T, K> {
    /// `using var results = ...AsAggregator()`: the subscriptions end with the aggregator.
    fn drop(&mut self) {
        self.disposer.dispose();
    }
}
