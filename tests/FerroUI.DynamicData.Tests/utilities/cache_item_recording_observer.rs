//! `CacheItemRecordingObserver<TObject, TKey>` (`Utilities/CacheItemRecordingObserver.cs`).

use super::RecordingObserverBase;
use ferroui_dynamic_data::cache::{ChangeReason, ChangeSet};
use ferroui_reactive::concurrency::IScheduler;
use ferroui_system::collections::generic::Dictionary;
use ferroui_system::{IObserver, ObservableError};
use std::cell::RefCell;
use std::hash::Hash;
use std::ops::Deref;
use std::rc::Rc;

/// Records the change sets of a cache and the items they make, by key and in the order of the
/// indexes of a sorted stream, until the sequence ends (`CacheItemRecordingObserver<TObject,
/// TKey>`). It dereferences to [`RecordingObserverBase`] for the error, the end and the
/// notifications.
pub struct CacheItemRecordingObserver<T, K> {
    base: RecordingObserverBase<ChangeSet<T, K>>,
    recorded_change_sets: RefCell<Vec<ChangeSet<T, K>>>,
    recorded_items_by_key: RefCell<Dictionary<K, T>>,
    recorded_items_sorted: RefCell<Vec<T>>,
}

impl<T: Clone + 'static, K: Eq + Hash + Clone + 'static> CacheItemRecordingObserver<T, K> {
    /// An observer that stamps the notifications with the time of `scheduler`.
    pub fn new(scheduler: Rc<dyn IScheduler>) -> Rc<Self> {
        Rc::new(Self {
            base: RecordingObserverBase::new(scheduler),
            recorded_change_sets: RefCell::new(Vec::new()),
            recorded_items_by_key: RefCell::new(Dictionary::new()),
            recorded_items_sorted: RefCell::new(Vec::new()),
        })
    }

    /// The change sets received before the end (`RecordedChangeSets`).
    pub fn recorded_change_sets(&self) -> Vec<ChangeSet<T, K>> {
        self.recorded_change_sets.borrow().clone()
    }

    /// The items the change sets make, by key (`RecordedItemsByKey`).
    pub fn recorded_items_by_key(&self) -> Dictionary<K, T> {
        self.recorded_items_by_key.borrow().clone()
    }

    /// The items in the order of the indexes of the changes; empty for a stream without indexes
    /// (`RecordedItemsSorted`).
    pub fn recorded_items_sorted(&self) -> Vec<T> {
        self.recorded_items_sorted.borrow().clone()
    }

    /// The items the change sets make, in the order of enumeration of the dictionary
    /// (`RecordedItemsByKey.Values`).
    pub fn recorded_items(&self) -> Vec<T> {
        self.recorded_items_by_key.borrow().values().cloned().collect()
    }
}

impl<T, K> Deref for CacheItemRecordingObserver<T, K> {
    type Target = RecordingObserverBase<ChangeSet<T, K>>;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl<T: Clone + 'static, K: Eq + Hash + Clone + 'static> IObserver<ChangeSet<T, K>> for CacheItemRecordingObserver<T, K> {
    fn on_next(&self, value: ChangeSet<T, K>) {
        self.base.record_next(&value);
        if self.base.has_finalized() {
            return;
        }
        self.recorded_change_sets.borrow_mut().push(value.clone());
        let mut by_key = self.recorded_items_by_key.borrow_mut();
        let mut sorted = self.recorded_items_sorted.borrow_mut();
        for change in &value {
            match change.reason() {
                ChangeReason::Add => {
                    by_key.add(change.key().clone(), change.current().clone());
                    if let Some(index) = change.current_index() {
                        sorted.insert(index, change.current().clone());
                    }
                }
                ChangeReason::Moved => {
                    sorted.remove(change.previous_index().expect("a move has a previous index"));
                    sorted.insert(change.current_index().expect("a move has a current index"), change.current().clone());
                }
                ChangeReason::Remove => {
                    by_key.remove(change.key());
                    if let Some(index) = change.current_index() {
                        sorted.remove(index);
                    }
                }
                ChangeReason::Update => {
                    by_key.insert(change.key().clone(), change.current().clone());
                    if let Some(index) = change.current_index() {
                        sorted.remove(change.previous_index().expect("an update with a current index has a previous index"));
                        sorted.insert(index, change.current().clone());
                    }
                }
                ChangeReason::Refresh => {}
            }
        }
    }

    fn on_error(&self, error: ObservableError) {
        self.base.record_error(error);
    }

    fn on_completed(&self) {
        self.base.record_completed();
    }
}
