//! `ObservableCollectionAdaptor<T>` and `ObservableCollectionAdaptor<TObject, TKey>`
//! (`Binding/ObservableCollectionAdaptor.cs`).
//!
//! Deviation (DEVIATIONS.md, DynamicData): the two classes of upstream have the same name and
//! differ by their number of type parameters; the adaptor of lists is
//! [`ListObservableCollectionAdaptor`] here.

use super::{BindingOptions, IObservableCollection, IObservableCollectionAdaptor};
use crate::cache::internal::Cache;
use crate::cache::{ChangeReason, ChangeSet, ICache, IQuery};
use crate::list::IChangeSetAdaptor;
use crate::DynamicDataOptions;
use std::cell::{Cell, RefCell};
use std::hash::Hash;
use std::rc::Rc;

/// Applies the change sets of a list to an observable collection, loading it with a reset above
/// a threshold of changes (`ObservableCollectionAdaptor<T>`).
pub struct ListObservableCollectionAdaptor<T: Clone + PartialEq + 'static> {
    collection: Rc<dyn IObservableCollection<T>>,
    refresh_threshold: usize,
    /// Unread upstream (`allowReplace`, "TODO: pass in allowReplace").
    #[allow(dead_code)]
    allow_replace: bool,
    reset_on_first_time_load: bool,
    loaded: Cell<bool>,
}

impl<T: Clone + PartialEq + 'static> ListObservableCollectionAdaptor<T> {
    /// The adaptor with the default options (`ObservableCollectionAdaptor(IObservableCollection<T>)`,
    /// `DynamicDataOptions.Binding`).
    pub fn new(collection: Rc<dyn IObservableCollection<T>>) -> Self {
        Self::with_options(collection, DynamicDataOptions::binding())
    }

    /// The adaptor with `options` (`ObservableCollectionAdaptor(IObservableCollection<T>,
    /// BindingOptions)`).
    pub fn with_options(collection: Rc<dyn IObservableCollection<T>>, options: BindingOptions) -> Self {
        Self::with_threshold(collection, options.reset_threshold, options.use_replace_for_updates, options.reset_on_first_time_load)
    }

    /// The adaptor with every part (`ObservableCollectionAdaptor(IObservableCollection<T>, int,
    /// bool allowReplace = true, bool resetOnFirstTimeLoad = true)`).
    pub fn with_threshold(collection: Rc<dyn IObservableCollection<T>>, refresh_threshold: usize, allow_replace: bool, reset_on_first_time_load: bool) -> Self {
        Self { collection, refresh_threshold, allow_replace, reset_on_first_time_load, loaded: Cell::new(false) }
    }
}

impl<T: Clone + PartialEq + 'static> IChangeSetAdaptor<T> for ListObservableCollectionAdaptor<T> {
    fn adapt(&self, changes: &crate::list::ChangeSet<T>) {
        if changes.total_changes() - changes.refreshes() > self.refresh_threshold || (!self.loaded.get() && self.reset_on_first_time_load) {
            let suspension = self.collection.suspend_notifications();
            self.collection.clone_changes(changes);
            self.loaded.set(true);
            suspension.dispose();
        } else {
            self.collection.clone_changes(changes);
        }
    }
}

/// Applies the change sets of a cache to an observable collection, loading it with a reset above
/// a threshold of changes (`ObservableCollectionAdaptor<TObject, TKey>`).
pub struct ObservableCollectionAdaptor<T, K> {
    cache: RefCell<Cache<T, K>>,
    refresh_threshold: usize,
    use_replace_for_updates: bool,
    reset_on_first_time_load: bool,
    loaded: Cell<bool>,
}

impl<T: Clone + PartialEq + 'static, K: Eq + Hash + Clone + 'static> Default for ObservableCollectionAdaptor<T, K> {
    /// `ObservableCollectionAdaptor(int refreshThreshold = 25, bool useReplaceForUpdates = true,
    /// bool resetOnFirstTimeLoad = true)` with the defaults.
    fn default() -> Self {
        Self::new(BindingOptions::DEFAULT_RESET_THRESHOLD, BindingOptions::DEFAULT_USE_REPLACE_FOR_UPDATES, BindingOptions::DEFAULT_RESET_ON_FIRST_TIME_LOAD)
    }
}

impl<T: Clone + PartialEq + 'static, K: Eq + Hash + Clone + 'static> ObservableCollectionAdaptor<T, K> {
    /// The adaptor with every part (`ObservableCollectionAdaptor(int, bool, bool)`).
    pub fn new(refresh_threshold: usize, use_replace_for_updates: bool, reset_on_first_time_load: bool) -> Self {
        Self { cache: RefCell::new(Cache::new()), refresh_threshold, use_replace_for_updates, reset_on_first_time_load, loaded: Cell::new(false) }
    }

    /// The adaptor with `options` (`ObservableCollectionAdaptor(BindingOptions)`).
    pub fn with_options(options: BindingOptions) -> Self {
        Self::new(options.reset_threshold, options.use_replace_for_updates, options.reset_on_first_time_load)
    }

    /// The defaults with the choice of replacements (`ObservableCollectionAdaptor(useReplaceForUpdates: ..)`).
    pub fn with_use_replace_for_updates(use_replace_for_updates: bool) -> Self {
        Self::new(BindingOptions::DEFAULT_RESET_THRESHOLD, use_replace_for_updates, BindingOptions::DEFAULT_RESET_ON_FIRST_TIME_LOAD)
    }

    fn do_update(&self, changes: &ChangeSet<T, K>, list: &dyn IObservableCollection<T>) {
        for change in changes {
            match change.reason() {
                ChangeReason::Add => list.add(change.current().clone()),
                ChangeReason::Remove => {
                    list.remove(change.current());
                }
                ChangeReason::Update => {
                    let previous = change.previous().expect("an update has a previous item");
                    // Remove / Add is default as some platforms do not support list[index] = XXX
                    // notifications.
                    if self.use_replace_for_updates {
                        let index = list.index_of(previous).expect("Cannot find index of original item. Either it does not exist in the list or the hashcode has mutated");
                        list.set(index, change.current().clone());
                    } else {
                        list.remove(previous);
                        list.add(change.current().clone());
                    }
                }
                ChangeReason::Refresh | ChangeReason::Moved => {}
            }
        }
    }
}

impl<T: Clone + PartialEq + 'static, K: Eq + Hash + Clone + 'static> IObservableCollectionAdaptor<T, K> for ObservableCollectionAdaptor<T, K> {
    fn adapt(&self, changes: &ChangeSet<T, K>, collection: &dyn IObservableCollection<T>) {
        self.cache.borrow_mut().clone_changes(changes);

        if changes.count() - changes.refreshes() > self.refresh_threshold || (!self.loaded.get() && self.reset_on_first_time_load) {
            self.loaded.set(true);
            let items = self.cache.borrow().items();
            let suspension = collection.suspend_notifications();
            collection.load(items);
            suspension.dispose();
        } else {
            let suspension = collection.suspend_count();
            self.do_update(changes, collection);
            suspension.dispose();
        }
    }
}
