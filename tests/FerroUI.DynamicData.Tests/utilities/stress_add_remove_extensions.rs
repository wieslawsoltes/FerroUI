//! `StressAddRemoveExtensions` (`Utilities/StressAddRemoveExtensions.cs`): sequences of items that
//! are added to a collection when they arrive and removed after a time of their own.
//!
//! Upstream's stress tests run these on the task pool; here they run on the scheduler a test
//! gives (a `TestScheduler`, or the default scheduler of the thread when `None`), on one thread.

use ferroui_dynamic_data::cache::{ISourceCache, ISourceUpdater};
use ferroui_dynamic_data::list::{self, IExtendedList, ISourceList, ListChangeReason};
use ferroui_reactive::concurrency::{IScheduler, SchedulerDefaults};
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::{IObservable, IObserver, ObservableError};
use std::hash::Hash;
use std::rc::Rc;
use std::time::Duration;

/// The add and remove stresses of a sequence of items.
pub trait StressAddRemoveExtensions<T: Clone + 'static> {
    /// The items, each passed to `on_add` when it arrives and, when `get_remove_timeout` gives a
    /// time, to `on_remove` that time later; an item is delivered when it is removed, or at once
    /// when it is not (`StressAddRemove<T, TState>(state, onAdd, onRemove, getRemoveTimeout,
    /// scheduler)`).
    fn stress_add_remove_with<S: Clone + 'static>(
        &self,
        state: S,
        on_add: impl Fn(&T, &S) + 'static,
        on_remove: impl Fn(&T, &S) + 'static,
        get_remove_timeout: impl Fn(&T) -> Option<Duration> + 'static,
        scheduler: Option<Rc<dyn IScheduler>>,
    ) -> Rc<dyn IObservable<T>>;

    /// [`stress_add_remove_with`](Self::stress_add_remove_with) that adds the items to `cache` and
    /// removes them from it (`StressAddRemove<T, TKey>(ISourceCache<T, TKey>, ..)`).
    fn stress_add_remove_cache<K: Eq + Hash + Clone + 'static, C: ISourceCache<T, K> + Clone + 'static>(
        &self,
        cache: C,
        get_remove_timeout: impl Fn(&T) -> Option<Duration> + 'static,
        scheduler: Rc<dyn IScheduler>,
    ) -> Rc<dyn IObservable<T>>;

    /// [`stress_add_remove_with`](Self::stress_add_remove_with) that adds the items to `list` and
    /// removes them from it (`StressAddRemove<T>(ISourceList<T>, ..)`).
    fn stress_add_remove_list<L: ISourceList<T> + Clone + 'static>(
        &self,
        list: L,
        get_remove_timeout: impl Fn(&T) -> Option<Duration> + 'static,
        scheduler: Rc<dyn IScheduler>,
    ) -> Rc<dyn IObservable<T>>
    where
        T: PartialEq;

    /// The change sets of a list that adds each item when it arrives and removes it after its time
    /// (`StressAddRemoveExplicit<T>(getRemoveTime, scheduler)`).
    fn stress_add_remove_explicit(&self, get_remove_time: impl Fn(&T) -> Option<Duration> + 'static, scheduler: Option<Rc<dyn IScheduler>>) -> Rc<dyn IObservable<list::ChangeSet<T>>>
    where
        T: PartialEq;

    /// `parallel` of [`stress_add_remove_explicit`](Self::stress_add_remove_explicit), merged
    /// (`StressAddRemoveExplicit<T>(int parallel, getRemoveTime, scheduler)`).
    fn stress_add_remove_explicit_parallel(
        &self,
        parallel: usize,
        get_remove_time: impl Fn(&T) -> Option<Duration> + 'static,
        scheduler: Option<Rc<dyn IScheduler>>,
    ) -> Rc<dyn IObservable<list::ChangeSet<T>>>
    where
        T: PartialEq;
}

impl<T: Clone + 'static> StressAddRemoveExtensions<T> for Rc<dyn IObservable<T>> {
    fn stress_add_remove_with<S: Clone + 'static>(
        &self,
        state: S,
        on_add: impl Fn(&T, &S) + 'static,
        on_remove: impl Fn(&T, &S) + 'static,
        get_remove_timeout: impl Fn(&T) -> Option<Duration> + 'static,
        scheduler: Option<Rc<dyn IScheduler>>,
    ) -> Rc<dyn IObservable<T>> {
        let on_remove: Rc<dyn Fn(&T, &S)> = Rc::new(on_remove);
        let add_state = state.clone();
        self.do_(move |item: &T| on_add(item, &add_state)).select_many(move |item: T| match get_remove_timeout(&item) {
            Some(timeout) => {
                let scheduler = scheduler.clone().unwrap_or_else(SchedulerDefaults::default_scheduler);
                let (on_remove, state, removed) = (on_remove.clone(), state.clone(), item.clone());
                Observable::timer_on(timeout, scheduler).do_(move |_| on_remove(&removed, &state)).select(move |_| item.clone())
            }
            None => Observable::return_(item),
        })
    }

    fn stress_add_remove_cache<K: Eq + Hash + Clone + 'static, C: ISourceCache<T, K> + Clone + 'static>(
        &self,
        cache: C,
        get_remove_timeout: impl Fn(&T) -> Option<Duration> + 'static,
        scheduler: Rc<dyn IScheduler>,
    ) -> Rc<dyn IObservable<T>> {
        self.stress_add_remove_with(
            cache,
            |item: &T, cache: &C| cache.edit(&mut |updater: &mut dyn ISourceUpdater<T, K>| updater.add_or_update(item.clone())),
            |item: &T, cache: &C| cache.edit(&mut |updater: &mut dyn ISourceUpdater<T, K>| updater.remove_item(item)),
            get_remove_timeout,
            Some(scheduler),
        )
    }

    fn stress_add_remove_list<L: ISourceList<T> + Clone + 'static>(
        &self,
        list: L,
        get_remove_timeout: impl Fn(&T) -> Option<Duration> + 'static,
        scheduler: Rc<dyn IScheduler>,
    ) -> Rc<dyn IObservable<T>>
    where
        T: PartialEq,
    {
        self.stress_add_remove_with(
            list,
            |item: &T, list: &L| list.edit_dyn(&mut |items: &mut dyn IExtendedList<T>| items.add(item.clone())),
            |item: &T, list: &L| {
                list.edit_dyn(&mut |items: &mut dyn IExtendedList<T>| {
                    items.remove(item);
                })
            },
            get_remove_timeout,
            Some(scheduler),
        )
    }

    fn stress_add_remove_explicit(&self, get_remove_time: impl Fn(&T) -> Option<Duration> + 'static, scheduler: Option<Rc<dyn IScheduler>>) -> Rc<dyn IObservable<list::ChangeSet<T>>>
    where
        T: PartialEq,
    {
        let source = self.clone();
        let get_remove_time: Rc<dyn Fn(&T) -> Option<Duration>> = Rc::new(get_remove_time);
        Observable::create(move |observer: Rc<dyn IObserver<list::ChangeSet<T>>>| {
            let get_remove_time = get_remove_time.clone();
            let (on_error, on_completed) = (observer.clone(), observer.clone());
            source
                .stress_add_remove_with(
                    observer,
                    |item: &T, observer: &Rc<dyn IObserver<list::ChangeSet<T>>>| {
                        observer.on_next(list::ChangeSet::from(vec![list::Change::new(ListChangeReason::Add, item.clone(), None)]));
                    },
                    |item: &T, observer: &Rc<dyn IObserver<list::ChangeSet<T>>>| {
                        observer.on_next(list::ChangeSet::from(vec![list::Change::new(ListChangeReason::Remove, item.clone(), None)]));
                    },
                    move |item: &T| get_remove_time(item),
                    scheduler.clone(),
                )
                .subscribe_all(|_| {}, move |error: ObservableError| on_error.on_error(error), move || on_completed.on_completed())
        })
    }

    fn stress_add_remove_explicit_parallel(
        &self,
        parallel: usize,
        get_remove_time: impl Fn(&T) -> Option<Duration> + 'static,
        scheduler: Option<Rc<dyn IScheduler>>,
    ) -> Rc<dyn IObservable<list::ChangeSet<T>>>
    where
        T: PartialEq,
    {
        let get_remove_time: Rc<dyn Fn(&T) -> Option<Duration>> = Rc::new(get_remove_time);
        let sources: Vec<Rc<dyn IObservable<list::ChangeSet<T>>>> = (0..parallel)
            .map(|_| {
                let get_remove_time = get_remove_time.clone();
                self.stress_add_remove_explicit(move |item: &T| get_remove_time(item), scheduler.clone())
            })
            .collect();
        Observable::merge_all(sources)
    }
}
