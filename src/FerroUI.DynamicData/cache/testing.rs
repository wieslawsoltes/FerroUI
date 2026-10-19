//! The aids for tests that DynamicData ships with the library (`Cache/Tests`, namespace
//! `DynamicData.Tests`): `ChangeSetAggregator<TObject, TKey>`, and the aggregators of the sorted,
//! paged, virtual and contextual change sets ([`TypedChangeSetAggregator`]).

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

struct TypedState<C> {
    messages: RefCell<Vec<C>>,
    error: RefCell<Option<ObservableError>>,
    is_completed: Cell<bool>,
    summary: RefCell<ChangeSummary>,
}

/// Records what a sequence of change sets of a cache of any form produces: the change sets as
/// they are, the items they make, their statistics, the error and the end.
///
/// It is upstream's `SortedChangeSetAggregator<TObject, TKey>`,
/// `PagedChangeSetAggregator<TObject, TKey>`, `VirtualChangeSetAggregator<TObject, TKey>` and
/// `ChangeSetAggregator<TObject, TKey, TContext>`, which differ only by the type of their change
/// sets (the aliases [`SortedChangeSetAggregator`], [`PagedChangeSetAggregator`],
/// [`VirtualChangeSetAggregator`] and [`ContextChangeSetAggregator`]).
pub struct TypedChangeSetAggregator<T, K, C> {
    state: Rc<TypedState<C>>,
    data: ObservableCache<T, K>,
    disposer: Rc<dyn IDisposable>,
}

/// The aggregator of sorted change sets (`SortedChangeSetAggregator<TObject, TKey>`).
pub type SortedChangeSetAggregator<T, K> = TypedChangeSetAggregator<T, K, super::SortedChangeSet<T, K>>;

/// The aggregator of paged change sets (`PagedChangeSetAggregator<TObject, TKey>`).
pub type PagedChangeSetAggregator<T, K> = TypedChangeSetAggregator<T, K, super::PagedChangeSet<T, K>>;

/// The aggregator of virtual change sets (`VirtualChangeSetAggregator<TObject, TKey>`).
pub type VirtualChangeSetAggregator<T, K> = TypedChangeSetAggregator<T, K, super::VirtualChangeSet<T, K>>;

/// The aggregator of change sets with a context (`ChangeSetAggregator<TObject, TKey, TContext>`).
pub type ContextChangeSetAggregator<T, K, X> = TypedChangeSetAggregator<T, K, super::ChangeSetWithContext<T, K, X>>;

impl<T: Clone + 'static, K: Eq + Hash + Clone + 'static, C: super::IChangeSet<T, K>> TypedChangeSetAggregator<T, K, C> {
    /// Subscribes to `source` (`SortedChangeSetAggregator(IObservable<ISortedChangeSet>)` and its
    /// siblings).
    pub fn new(source: Rc<dyn IObservable<C>>) -> Self {
        let state = Rc::new(TypedState { messages: RefCell::new(Vec::new()), error: RefCell::new(None), is_completed: Cell::new(false), summary: RefCell::new(ChangeSummary::empty()) });
        let published = source.publish();
        let published_source: Rc<dyn IObservable<C>> = published.clone();
        let data = ObservableCacheEx::<T, K>::as_observable_cache(&published_source);
        let (on_next, on_error, on_completed) = (state.clone(), state.clone(), state.clone());
        let results = published_source.subscribe_all(
            move |changes| on_next.messages.borrow_mut().push(changes),
            move |error| *on_error.error.borrow_mut() = Some(error),
            move || on_completed.is_completed.set(true),
        );
        let summary_target = state.clone();
        let summariser = ObservableCacheEx::<T, K>::collect_update_stats(&published_source).subscribe_all(move |summary| *summary_target.summary.borrow_mut() = summary, |_| {}, || {});
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
    pub fn messages(&self) -> Vec<C> {
        self.state.messages.borrow().clone()
    }

    /// The error of the sequence (`Error`).
    pub fn error(&self) -> Option<ObservableError> {
        self.state.error.borrow().clone()
    }

    /// Whether the sequence has ended (`IsCompleted` of the aggregator of contextual change sets).
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

impl<T, K, C> IDisposable for TypedChangeSetAggregator<T, K, C> {
    fn dispose(&self) {
        self.disposer.dispose();
    }
}

impl<T, K, C> Drop for TypedChangeSetAggregator<T, K, C> {
    /// `using var results = ...AsAggregator()`: the subscriptions end with the aggregator.
    fn drop(&mut self) {
        self.disposer.dispose();
    }
}

struct GroupAggregatorState<T, K, G> {
    messages: std::cell::RefCell<Vec<super::GroupChangeSet<T, K, G>>>,
    error: std::cell::RefCell<Option<ferroui_system::ObservableError>>,
    is_completed: std::cell::Cell<bool>,
    summary: std::cell::RefCell<crate::diagnostics::ChangeSummary>,
}

/// Records what a sequence of change sets of groups produces (`GroupChangeSetAggregator<TObject,
/// TKey, TGroupKey>`): the change sets, the groups they make, an aggregator of the changes of each
/// group, their statistics, the error and the end.
pub struct GroupChangeSetAggregator<T, K, G> {
    state: std::rc::Rc<GroupAggregatorState<T, K, G>>,
    data: super::ObservableCache<super::Group<T, K, G>, G>,
    groups: super::ObservableCache<std::rc::Rc<ChangeSetAggregator<T, K>>, G>,
    disposer: std::rc::Rc<dyn ferroui_system::IDisposable>,
}

impl<T: Clone + 'static, K: Eq + std::hash::Hash + Clone + 'static, G: Eq + std::hash::Hash + Clone + 'static> GroupChangeSetAggregator<T, K, G> {
    /// Subscribes to `source` (`GroupChangeSetAggregator(IObservable<IGroupChangeSet>)`).
    pub fn new(source: std::rc::Rc<dyn ferroui_system::IObservable<super::GroupChangeSet<T, K, G>>>) -> Self {
        let state = std::rc::Rc::new(GroupAggregatorState {
            messages: std::cell::RefCell::new(Vec::new()),
            error: std::cell::RefCell::new(None),
            is_completed: std::cell::Cell::new(false),
            summary: std::cell::RefCell::new(crate::diagnostics::ChangeSummary::empty()),
        });
        let published = ferroui_reactive::ObservableExt::publish(&source);
        let published_source: std::rc::Rc<dyn ferroui_system::IObservable<super::GroupChangeSet<T, K, G>>> = published.clone();

        let data = super::ObservableCache::from_source(published_source.clone());
        // `Transform(grp => grp.Cache.Connect().AsAggregator()).DisposeMany().AsObservableCache()`.
        let groups = super::ObservableCache::from_source(group_aggregators(published_source.clone()));
        let (on_next, on_error, on_completed) = (state.clone(), state.clone(), state.clone());
        let results = ferroui_reactive::ObservableExt::subscribe_all(
            &published_source,
            move |changes| on_next.messages.borrow_mut().push(changes),
            move |error| *on_error.error.borrow_mut() = Some(error),
            move || on_completed.is_completed.set(true),
        );
        let summary_target = state.clone();
        let summariser = ferroui_reactive::ObservableExt::subscribe_all(
            &super::ObservableCacheEx::collect_update_stats(&published_source),
            move |summary| *summary_target.summary.borrow_mut() = summary,
            |_| {},
            || {},
        );
        let connected = published.connect();

        let (data_cache, groups_cache) = (data.clone(), groups.clone());
        let disposer = ferroui_reactive::disposables::Disposable::create(move || {
            ferroui_system::IDisposable::dispose(&data_cache);
            ferroui_system::IDisposable::dispose(&groups_cache);
            results.dispose();
            summariser.dispose();
            connected.dispose();
        });
        Self { state, data, groups, disposer }
    }

    /// The groups the change sets make (`Data`).
    pub fn data(&self) -> &super::ObservableCache<super::Group<T, K, G>, G> {
        &self.data
    }

    /// An aggregator of the changes of the cache of each group, by group key (`Groups`).
    pub fn groups(&self) -> &super::ObservableCache<std::rc::Rc<ChangeSetAggregator<T, K>>, G> {
        &self.groups
    }

    /// The change sets so far (`Messages`).
    pub fn messages(&self) -> Vec<super::GroupChangeSet<T, K, G>> {
        self.state.messages.borrow().clone()
    }

    /// The error of the sequence (`Error`).
    pub fn error(&self) -> Option<ferroui_system::ObservableError> {
        self.state.error.borrow().clone()
    }

    /// Whether the sequence has ended (`IsCompleted`).
    pub fn is_completed(&self) -> bool {
        self.state.is_completed.get()
    }

    /// The statistics of the change sets (`Summary`).
    pub fn summary(&self) -> crate::diagnostics::ChangeSummary {
        self.state.summary.borrow().clone()
    }
}

impl<T, K, G> ferroui_system::IDisposable for GroupChangeSetAggregator<T, K, G> {
    fn dispose(&self) {
        self.disposer.dispose();
    }
}

impl<T, K, G> Drop for GroupChangeSetAggregator<T, K, G> {
    /// `using var results = ...AsAggregator()`: the subscriptions end with the aggregator.
    fn drop(&mut self) {
        self.disposer.dispose();
    }
}

/// An aggregator of the changes of the cache of each group, disposed when its group is removed or
/// replaced, and when the subscription ends (the `Groups` of `GroupChangeSetAggregator`).
fn group_aggregators<T: Clone + 'static, K: Eq + std::hash::Hash + Clone + 'static, G: Eq + std::hash::Hash + Clone + 'static>(
    source: std::rc::Rc<dyn ferroui_system::IObservable<super::GroupChangeSet<T, K, G>>>,
) -> std::rc::Rc<dyn ferroui_system::IObservable<super::ChangeSet<std::rc::Rc<ChangeSetAggregator<T, K>>, G>>> {
    ferroui_reactive::Observable::create(move |observer: std::rc::Rc<dyn ferroui_system::IObserver<super::ChangeSet<std::rc::Rc<ChangeSetAggregator<T, K>>, G>>>| {
        let aggregators = std::rc::Rc::new(std::cell::RefCell::new(super::ChangeAwareCache::<std::rc::Rc<ChangeSetAggregator<T, K>>, G>::new()));
        let (target, on_error, on_completed) = (aggregators.clone(), observer.clone(), observer.clone());
        let subscription = ferroui_reactive::ObservableExt::subscribe_all(
            &source,
            move |changes: super::GroupChangeSet<T, K, G>| {
                let mut replaced = Vec::new();
                let output = {
                    let mut cache = target.borrow_mut();
                    for change in &changes {
                        match change.reason() {
                            super::ChangeReason::Add | super::ChangeReason::Update => {
                                let connected = super::IConnectableCache::connect(change.current().observable_cache());
                                let aggregator = std::rc::Rc::new(ChangeSetAggregator::new(connected));
                                replaced.extend(super::IQuery::lookup(&*cache, change.key()));
                                super::ICache::add_or_update(&mut *cache, aggregator, change.key().clone());
                            }
                            super::ChangeReason::Remove => {
                                replaced.extend(super::IQuery::lookup(&*cache, change.key()));
                                super::ICache::remove(&mut *cache, change.key());
                            }
                            super::ChangeReason::Refresh => super::ICache::refresh(&mut *cache, change.key()),
                            super::ChangeReason::Moved => {}
                        }
                    }
                    cache.capture_changes()
                };
                observer.on_next(output);
                for aggregator in replaced {
                    ferroui_system::IDisposable::dispose(&*aggregator);
                }
            },
            move |error| on_error.on_error(error),
            move || on_completed.on_completed(),
        );
        ferroui_reactive::disposables::Disposable::create(move || {
            subscription.dispose();
            let remaining = super::IQuery::items(&*aggregators.borrow());
            for aggregator in remaining {
                ferroui_system::IDisposable::dispose(&*aggregator);
            }
        })
    })
}

/// `AsAggregator` of a sequence of change sets of groups (`TestEx.AsAggregator(IObservable<IGroupChangeSet>)`).
///
/// Deviation (DEVIATIONS.md, DynamicData): an overload of `AsAggregator` by the type of the change
/// set; here it is `as_group_aggregator`, of a trait of its own, as `as_aggregator` of
/// `ObservableCacheEx` applies to sequences of groups too.
pub trait GroupChangeSetAggregatorEx<T, K, G> {
    /// Records the change sets of groups, for tests.
    fn as_group_aggregator(&self) -> GroupChangeSetAggregator<T, K, G>;
}

impl<T: Clone + 'static, K: Eq + std::hash::Hash + Clone + 'static, G: Eq + std::hash::Hash + Clone + 'static> GroupChangeSetAggregatorEx<T, K, G>
    for std::rc::Rc<dyn ferroui_system::IObservable<super::GroupChangeSet<T, K, G>>>
{
    fn as_group_aggregator(&self) -> GroupChangeSetAggregator<T, K, G> {
        GroupChangeSetAggregator::new(self.clone())
    }
}
