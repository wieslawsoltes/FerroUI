//! `Distinct` (`Linq/Observable/Distinct.cs`).

use crate::internal::{impl_sink_dispose, unary, OperatorSink, Sink};
use ferroui_system::{IObservable, IObserver, ObservableError};
use std::cell::RefCell;
use std::collections::HashSet;
use std::hash::Hash;
use std::rc::Rc;

pub(crate) type KeySelector<T, K> = Rc<dyn Fn(&T) -> Result<K, ObservableError>>;
pub(crate) type Comparer<K> = Rc<dyn Fn(&K, &K) -> Result<bool, ObservableError>>;
pub(crate) type KeySetFactory<K> = Rc<dyn Fn() -> Box<dyn KeySet<K>>>;

/// The set of the keys seen so far (`HashSet<TKey>` with its comparer).
pub(crate) trait KeySet<K> {
    /// Adds the key; false when an equal key was there already (`HashSet<TKey>.Add`). A comparer
    /// that fails ends the sequence with its error.
    fn add(&mut self, key: K) -> Result<bool, ObservableError>;
}

/// The keys in a hash set with the default comparer (`EqualityComparer<TKey>.Default`).
struct HashKeySet<K> {
    keys: HashSet<K>,
}

impl<K: Eq + Hash> KeySet<K> for HashKeySet<K> {
    fn add(&mut self, key: K) -> Result<bool, ObservableError> {
        Ok(self.keys.insert(key))
    }
}

/// The keys compared with a comparer.
///
/// Deviation (DEVIATIONS.md, Reactive): an `IEqualityComparer<TKey>` is a function of equality in
/// the port, with no hash code; the keys it compares are kept in a list that each new key is
/// compared against, in the order they were added.
struct ComparerKeySet<K> {
    keys: Vec<K>,
    comparer: Comparer<K>,
}

impl<K> KeySet<K> for ComparerKeySet<K> {
    fn add(&mut self, key: K) -> Result<bool, ObservableError> {
        for existing in &self.keys {
            if (self.comparer)(existing, &key)? {
                return Ok(false);
            }
        }
        self.keys.push(key);
        Ok(true)
    }
}

/// A set of keys for each subscription with the default comparer.
pub(crate) fn hash_key_set<K: Eq + Hash + 'static>() -> KeySetFactory<K> {
    Rc::new(|| Box::new(HashKeySet { keys: HashSet::new() }) as Box<dyn KeySet<K>>)
}

/// A set of keys for each subscription compared with `comparer`.
pub(crate) fn comparer_key_set<K: 'static>(comparer: Comparer<K>) -> KeySetFactory<K> {
    Rc::new(move || Box::new(ComparerKeySet { keys: Vec::new(), comparer: comparer.clone() }) as Box<dyn KeySet<K>>)
}

/// The sink of `Distinct` (`Distinct<TSource, TKey>._`): a key selector or comparer that fails
/// ends the sequence with its error.
struct DistinctSink<T, K> {
    sink: Sink<T>,
    key_selector: KeySelector<T, K>,
    keys: RefCell<Box<dyn KeySet<K>>>,
}

impl<T: 'static, K: 'static> OperatorSink<T, T> for DistinctSink<T, K> {
    fn sink(&self) -> &Sink<T> {
        &self.sink
    }
}

impl_sink_dispose!(DistinctSink<T, K>);

impl<T: 'static, K: 'static> IObserver<T> for DistinctSink<T, K> {
    fn on_next(&self, value: T) {
        let added = match (self.key_selector)(&value) {
            Ok(key) => self.keys.borrow_mut().add(key),
            Err(error) => Err(error),
        };
        match added {
            Ok(true) => self.sink.forward_on_next(value),
            Ok(false) => {}
            Err(error) => self.sink.forward_on_error(error),
        }
    }

    fn on_error(&self, error: ObservableError) {
        self.sink.forward_on_error(error);
    }

    fn on_completed(&self) {
        self.sink.forward_on_completed();
    }
}

pub(crate) fn distinct<T: 'static, K: 'static>(
    source: Rc<dyn IObservable<T>>,
    key_selector: KeySelector<T, K>,
    keys: KeySetFactory<K>,
) -> Rc<dyn IObservable<T>> {
    unary(source, move |observer| {
        Rc::new(DistinctSink { sink: Sink::new(observer), key_selector: key_selector.clone(), keys: RefCell::new(keys()) })
    })
}

/// The methods of `Distinct` (provided methods of [`ObservableExt`](super::ObservableExt)).
macro_rules! distinct_methods {
    () => {
        /// The values not seen before (`Distinct()`).
        fn distinct(&self) -> std::rc::Rc<dyn ferroui_system::IObservable<T>>
        where
            T: Clone + Eq + std::hash::Hash,
        {
            crate::linq::distinct::distinct(
                self.observable(),
                std::rc::Rc::new(|value: &T| Ok(value.clone())),
                crate::linq::distinct::hash_key_set(),
            )
        }

        /// The values not seen before, compared with `comparer` (`Distinct(IEqualityComparer<TSource>)`).
        fn distinct_with_comparer(&self, comparer: impl Fn(&T, &T) -> bool + 'static) -> std::rc::Rc<dyn ferroui_system::IObservable<T>>
        where
            T: Clone,
        {
            crate::linq::distinct::distinct(
                self.observable(),
                std::rc::Rc::new(|value: &T| Ok(value.clone())),
                crate::linq::distinct::comparer_key_set(std::rc::Rc::new(move |a: &T, b: &T| Ok(comparer(a, b)))),
            )
        }

        /// [`distinct_with_comparer`](Self::distinct_with_comparer) with a comparer that may fail;
        /// a failure ends the sequence with its error (`Distinct(IEqualityComparer<TSource>)` with a
        /// comparer that throws).
        fn try_distinct_with_comparer(
            &self,
            comparer: impl Fn(&T, &T) -> Result<bool, ferroui_system::ObservableError> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<T>>
        where
            T: Clone,
        {
            crate::linq::distinct::distinct(
                self.observable(),
                std::rc::Rc::new(|value: &T| Ok(value.clone())),
                crate::linq::distinct::comparer_key_set(std::rc::Rc::new(comparer)),
            )
        }

        /// The values whose key was not seen before (`Distinct(Func<TSource, TKey>)`).
        fn distinct_by_key<K: Eq + std::hash::Hash + 'static>(
            &self,
            key_selector: impl Fn(&T) -> K + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::distinct::distinct(
                self.observable(),
                std::rc::Rc::new(move |value: &T| Ok(key_selector(value))),
                crate::linq::distinct::hash_key_set(),
            )
        }

        /// [`distinct_by_key`](Self::distinct_by_key) with a key selector that may fail; a failure
        /// ends the sequence with its error (`Distinct(Func<TSource, TKey>)` with a selector that
        /// throws).
        fn try_distinct_by_key<K: Eq + std::hash::Hash + 'static>(
            &self,
            key_selector: impl Fn(&T) -> Result<K, ferroui_system::ObservableError> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::distinct::distinct(self.observable(), std::rc::Rc::new(key_selector), crate::linq::distinct::hash_key_set())
        }

        /// The values whose key was not seen before, compared with `comparer`
        /// (`Distinct(Func<TSource, TKey>, IEqualityComparer<TKey>)`).
        fn distinct_with<K: 'static>(
            &self,
            key_selector: impl Fn(&T) -> K + 'static,
            comparer: impl Fn(&K, &K) -> bool + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::distinct::distinct(
                self.observable(),
                std::rc::Rc::new(move |value: &T| Ok(key_selector(value))),
                crate::linq::distinct::comparer_key_set(std::rc::Rc::new(move |a: &K, b: &K| Ok(comparer(a, b)))),
            )
        }

        /// [`distinct_with`](Self::distinct_with) with a key selector and a comparer that may fail;
        /// a failure ends the sequence with its error.
        fn try_distinct_with<K: 'static>(
            &self,
            key_selector: impl Fn(&T) -> Result<K, ferroui_system::ObservableError> + 'static,
            comparer: impl Fn(&K, &K) -> Result<bool, ferroui_system::ObservableError> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::distinct::distinct(
                self.observable(),
                std::rc::Rc::new(key_selector),
                crate::linq::distinct::comparer_key_set(std::rc::Rc::new(comparer)),
            )
        }
    };
}
pub(crate) use distinct_methods;
