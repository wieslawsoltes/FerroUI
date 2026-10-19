//! `SortedObservableCollectionAdaptor<TObject, TKey>` (`Binding/SortedObservableCollectionAdaptor.cs`).

use super::{BindingOptions, IObservableCollection, ISortedObservableCollectionAdaptor};
use crate::cache::{ChangeReason, IChangeSet, SortReason, SortedChangeSet};
use crate::DynamicDataOptions;

/// Applies the sorted change sets of a cache to an observable collection at the positions they
/// give, loading it with a reset above a threshold of changes
/// (`SortedObservableCollectionAdaptor<TObject, TKey>`).
#[derive(Clone, Copy, Debug)]
pub struct SortedObservableCollectionAdaptor {
    refresh_threshold: usize,
    use_replace_for_updates: bool,
    reset_on_first_time_load: bool,
}

impl Default for SortedObservableCollectionAdaptor {
    /// The adaptor with the default options (`SortedObservableCollectionAdaptor()`,
    /// `DynamicDataOptions.Binding`).
    fn default() -> Self {
        Self::with_options(DynamicDataOptions::binding())
    }
}

impl SortedObservableCollectionAdaptor {
    /// The adaptor with every part (`SortedObservableCollectionAdaptor(int, bool useReplaceForUpdates
    /// = true, bool resetOnFirstTimeLoad = true)`).
    pub fn new(refresh_threshold: usize, use_replace_for_updates: bool, reset_on_first_time_load: bool) -> Self {
        Self { refresh_threshold, use_replace_for_updates, reset_on_first_time_load }
    }

    /// The adaptor with `options` (`SortedObservableCollectionAdaptor(BindingOptions)`).
    pub fn with_options(options: BindingOptions) -> Self {
        Self::new(options.reset_threshold, options.use_replace_for_updates, options.reset_on_first_time_load)
    }

    fn sorted_values<T: Clone, K>(changes: &SortedChangeSet<T, K>) -> Vec<T> {
        changes.sorted_items().values().cloned().collect()
    }

    fn load<T: Clone + PartialEq + 'static, K: 'static>(changes: &SortedChangeSet<T, K>, collection: &dyn IObservableCollection<T>) {
        let suspension = collection.suspend_notifications();
        collection.load(Self::sorted_values(changes));
        suspension.dispose();
    }

    fn update<T: Clone + PartialEq + 'static, K: 'static>(&self, changes: &SortedChangeSet<T, K>, collection: &dyn IObservableCollection<T>) {
        let suspension = collection.suspend_count();
        self.do_update(changes, collection);
        suspension.dispose();
    }

    fn do_update<T: Clone + PartialEq + 'static, K: 'static>(&self, updates: &SortedChangeSet<T, K>, list: &dyn IObservableCollection<T>) {
        for update in updates.change_set() {
            let current_index = || update.current_index().expect("a sorted change has its index");
            let previous_index = || update.previous_index().expect("a sorted change has its previous index");
            match update.reason() {
                ChangeReason::Add => list.insert(current_index(), update.current().clone()),
                ChangeReason::Remove => list.remove_at(current_index()),
                ChangeReason::Moved => list.move_item(previous_index(), current_index()),
                ChangeReason::Update => {
                    if !self.use_replace_for_updates || update.previous_index() != update.current_index() {
                        list.remove_at(previous_index());
                        list.insert(current_index(), update.current().clone());
                    } else {
                        list.set(current_index(), update.current().clone());
                    }
                }
                ChangeReason::Refresh => {}
            }
        }
    }
}

impl<T: Clone + PartialEq + 'static, K: 'static> ISortedObservableCollectionAdaptor<T, K> for SortedObservableCollectionAdaptor {
    fn adapt(&self, changes: &SortedChangeSet<T, K>, collection: &dyn IObservableCollection<T>) {
        let plain = changes.change_set();
        match changes.sorted_items().sort_reason() {
            SortReason::ComparerChanged | SortReason::Reset => {
                // Multiply items count by 2 as we need to clear existing items.
                if changes.sorted_items().count() * 2 > self.refresh_threshold {
                    Self::load(changes, collection);
                } else {
                    collection.load(Self::sorted_values(changes));
                }
            }
            SortReason::InitialLoad => {
                if self.reset_on_first_time_load || plain.count() - plain.refreshes() > self.refresh_threshold {
                    Self::load(changes, collection);
                } else {
                    self.update(changes, collection);
                }
            }
            SortReason::DataChanged => {
                if plain.count() - plain.refreshes() > self.refresh_threshold {
                    Self::load(changes, collection);
                } else {
                    self.update(changes, collection);
                }
            }
            // Updates will only be moves, so apply logic.
            SortReason::Reorder => self.update(changes, collection),
        }
    }
}
