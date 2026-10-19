//! `DistinctCalculator<TObject, TKey, TValue>` (`Cache/Internal/DistinctCalculator.cs`).

use crate::cache::{Change, ChangeReason, ChangeSet, DistinctChangeSet};
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::{IObservable, IObserver};
use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::Hash;
use std::rc::Rc;

struct Calculator<K, V> {
    item_cache: HashMap<K, V>,
    key_counters: HashMap<K, usize>,
    value_counters: HashMap<V, usize>,
}

impl<K: Eq + Hash + Clone, V: Eq + Hash + Clone> Calculator<K, V> {
    fn add_key_action(&mut self, key: &K, value: &V) {
        match self.key_counters.get_mut(key) {
            Some(count) => *count += 1,
            None => {
                self.key_counters.insert(key.clone(), 1);
                // Added to the cache.
                self.item_cache.insert(key.clone(), value.clone());
            }
        }
    }

    fn add_value_action(&mut self, value: &V, result: &mut Vec<Change<V, V>>) {
        match self.value_counters.get_mut(value) {
            Some(count) => *count += 1,
            None => {
                self.value_counters.insert(value.clone(), 1);
                result.push(Change::new(ChangeReason::Add, value.clone(), value.clone()));
            }
        }
    }

    fn remove_key_action(&mut self, key: &K) {
        let Some(counter) = self.key_counters.get_mut(key) else { return };
        *counter -= 1;
        if *counter != 0 {
            return;
        }
        self.key_counters.remove(key);
        self.item_cache.remove(key);
    }

    fn remove_value_action(&mut self, value: &V, result: &mut Vec<Change<V, V>>) {
        let Some(counter) = self.value_counters.get_mut(value) else { return };
        *counter -= 1;
        if *counter != 0 {
            return;
        }
        self.value_counters.remove(value);
        result.push(Change::new(ChangeReason::Remove, value.clone(), value.clone()));
    }

    /// `Calculate`.
    fn calculate<T>(&mut self, changes: &ChangeSet<T, K>, value_selector: &dyn Fn(&T) -> V) -> DistinctChangeSet<V> {
        let mut result = Vec::new();
        for change in changes {
            let key = change.key();
            match change.reason() {
                ChangeReason::Add => {
                    let value = value_selector(change.current());
                    self.add_key_action(key, &value);
                    self.add_value_action(&value, &mut result);
                }
                ChangeReason::Refresh | ChangeReason::Update => {
                    let value = value_selector(change.current());
                    let previous = self.item_cache.get(key).cloned().expect("the item of the key is in the cache (KeyNotFoundException)");
                    if value == previous {
                        continue;
                    }
                    self.remove_value_action(&previous, &mut result);
                    self.add_value_action(&value, &mut result);
                    self.item_cache.insert(key.clone(), value);
                }
                ChangeReason::Remove => {
                    let previous = self.item_cache.get(key).cloned().expect("the item of the key is in the cache (KeyNotFoundException)");
                    self.remove_key_action(key);
                    self.remove_value_action(&previous, &mut result);
                }
                ChangeReason::Moved => {}
            }
        }
        DistinctChangeSet::new(result)
    }
}

/// `DistinctCalculator.Run`: the distinct values of the items, as adds and removes; empty change
/// sets are left out.
pub(crate) fn distinct_calculator<T: 'static, K: Eq + Hash + Clone + 'static, V: Eq + Hash + Clone + 'static>(
    source: Rc<dyn IObservable<ChangeSet<T, K>>>,
    value_selector: Rc<dyn Fn(&T) -> V>,
) -> Rc<dyn IObservable<DistinctChangeSet<V>>> {
    Observable::create(move |observer: Rc<dyn IObserver<DistinctChangeSet<V>>>| {
        let calculator = RefCell::new(Calculator { item_cache: HashMap::new(), key_counters: HashMap::new(), value_counters: HashMap::new() });
        let value_selector = value_selector.clone();
        source
            .select(move |changes: ChangeSet<T, K>| calculator.borrow_mut().calculate(&changes, &*value_selector))
            .where_(|updates: &DistinctChangeSet<V>| updates.count() != 0)
            .subscribe_safe(observer)
    })
}
