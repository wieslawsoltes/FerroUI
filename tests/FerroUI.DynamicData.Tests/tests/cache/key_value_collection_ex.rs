//! `KeyValueCollectionEx` (`DynamicData.Tests/Cache/KeyValueCollectionEx.cs`): an aid of the
//! tests of sorted change sets, not a fixture.

use ferroui_dynamic_data::cache::{IndexedItem, KeyValueCollection};
use std::collections::HashMap;
use std::hash::Hash;

/// The sorted items by their keys, with their indexes (`Indexed`).
pub(crate) trait KeyValueCollectionEx<T, K> {
    fn indexed(&self) -> HashMap<K, IndexedItem<T, K>>;
}

impl<T: Clone, K: Eq + Hash + Clone> KeyValueCollectionEx<T, K> for KeyValueCollection<T, K> {
    fn indexed(&self) -> HashMap<K, IndexedItem<T, K>> {
        self.iter().enumerate().map(|(index, (key, value))| (key.clone(), IndexedItem::new(value.clone(), key.clone(), index))).collect()
    }
}
