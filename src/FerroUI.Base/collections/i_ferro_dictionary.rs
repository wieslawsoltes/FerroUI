use super::{FerroDictionary, IFerroReadOnlyDictionary};
use std::fmt::Display;
use std::hash::Hash;

/// A notifying dictionary.
///
/// The members of the dictionary contract it extends are part of the trait.
pub trait IFerroDictionary<K, V>: IFerroReadOnlyDictionary<K, V> {
    /// Sets the value for `key`, adding the entry if it is not present.
    fn set(&self, key: K, value: V);

    /// Adds an entry. Panics if the key is already present.
    fn add(&self, key: K, value: V);

    /// Removes the entry for `key`. Returns false if it is not present.
    fn remove(&self, key: &K) -> bool;

    /// Removes all entries.
    fn clear(&self);

    fn is_read_only(&self) -> bool;
}

impl<K: Eq + Hash + Clone + Display, V: Clone> IFerroDictionary<K, V> for FerroDictionary<K, V> {
    fn set(&self, key: K, value: V) {
        FerroDictionary::set(self, key, value)
    }

    fn add(&self, key: K, value: V) {
        FerroDictionary::add(self, key, value)
    }

    fn remove(&self, key: &K) -> bool {
        FerroDictionary::remove(self, key)
    }

    fn clear(&self) {
        FerroDictionary::clear(self)
    }

    fn is_read_only(&self) -> bool {
        FerroDictionary::is_read_only(self)
    }
}
