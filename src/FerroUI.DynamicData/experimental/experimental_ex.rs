//! `ExperimentalEx` (`Experimental/ExperimentalEx.cs`).

use super::{IWatcher, Watcher};
use crate::cache::ObservableCacheEx;
use ferroui_reactive::concurrency::{IScheduler, SchedulerDefaults};
use ferroui_system::IObservable;
use std::hash::Hash;
use std::rc::Rc;

/// The experimental operators of caches (`ExperimentalEx`).
pub trait ExperimentalEx<T: Clone + 'static, K: Eq + Hash + Clone + 'static> {
    /// A watcher of the items of the cache by key, which notifies on the default scheduler
    /// (`AsWatcher(IScheduler? scheduler = null)`, `GlobalConfig.DefaultScheduler`).
    fn as_watcher(&self) -> Rc<dyn IWatcher<T, K>>;

    /// [`as_watcher`](Self::as_watcher) that notifies on `scheduler` (`AsWatcher(IScheduler)`).
    fn as_watcher_on(&self, scheduler: Rc<dyn IScheduler>) -> Rc<dyn IWatcher<T, K>>;
}

impl<T: Clone + 'static, K: Eq + Hash + Clone + 'static, C: crate::cache::IChangeSet<T, K>> ExperimentalEx<T, K> for Rc<dyn IObservable<C>> {
    fn as_watcher(&self) -> Rc<dyn IWatcher<T, K>> {
        ExperimentalEx::<T, K>::as_watcher_on(self, SchedulerDefaults::default_scheduler())
    }

    fn as_watcher_on(&self, scheduler: Rc<dyn IScheduler>) -> Rc<dyn IWatcher<T, K>> {
        Rc::new(Watcher::new(ObservableCacheEx::<T, K>::change_sets(self), scheduler))
    }
}
