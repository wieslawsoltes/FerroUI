use super::{CollectionChangedHandler, FerroDictionary};
use crate::data::model::{INotifyCollectionChanged, INotifyPropertyChanged};
use std::fmt::Display;
use std::hash::Hash;
use std::rc::Rc;

/// A read-only notifying dictionary.
///
/// The members of the read-only dictionary contract it extends are part of
/// the trait, together with the typed form of the collection changed event,
/// which carries the entries; [`INotifyCollectionChanged`] is its untyped
/// form.
pub trait IFerroReadOnlyDictionary<K, V>: INotifyCollectionChanged + INotifyPropertyChanged {
    /// The number of entries.
    fn count(&self) -> usize;

    /// The value for `key`. Panics if the key is not present.
    fn get(&self, key: &K) -> V;

    /// The value for `key`, if present.
    fn try_get_value(&self, key: &K) -> Option<V>;

    fn contains_key(&self, key: &K) -> bool;

    /// The keys, in enumeration order.
    fn keys(&self) -> Vec<K>;

    /// The values, in enumeration order.
    fn values(&self) -> Vec<V>;

    /// The entries, in enumeration order (enumeration).
    fn to_vec(&self) -> Vec<(K, V)>;

    /// Subscribes to changes of the collection. Returns a token for
    /// [`remove_collection_changed`](Self::remove_collection_changed).
    fn add_collection_changed(&self, handler: Rc<CollectionChangedHandler<(K, V)>>) -> u64;

    /// Unsubscribes a handler.
    fn remove_collection_changed(&self, token: u64) -> bool;
}

impl<K: Eq + Hash + Clone + Display, V: Clone> IFerroReadOnlyDictionary<K, V> for FerroDictionary<K, V> {
    fn count(&self) -> usize {
        FerroDictionary::count(self)
    }

    fn get(&self, key: &K) -> V {
        FerroDictionary::get(self, key)
    }

    fn try_get_value(&self, key: &K) -> Option<V> {
        FerroDictionary::try_get_value(self, key)
    }

    fn contains_key(&self, key: &K) -> bool {
        FerroDictionary::contains_key(self, key)
    }

    fn keys(&self) -> Vec<K> {
        FerroDictionary::keys(self)
    }

    fn values(&self) -> Vec<V> {
        FerroDictionary::values(self)
    }

    fn to_vec(&self) -> Vec<(K, V)> {
        FerroDictionary::to_vec(self)
    }

    fn add_collection_changed(&self, handler: Rc<CollectionChangedHandler<(K, V)>>) -> u64 {
        FerroDictionary::add_collection_changed(self, handler)
    }

    fn remove_collection_changed(&self, token: u64) -> bool {
        FerroDictionary::remove_collection_changed(self, token)
    }
}
