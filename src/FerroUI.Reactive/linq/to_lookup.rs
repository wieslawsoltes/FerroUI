//! `ToLookup` (`Linq/Observable/ToLookup.cs`), and [`Lookup`] (`System.Reactive.Lookup<K, E>`, the
//! `ILookup<TKey, TElement>` it gives).

use crate::internal::{impl_sink_dispose, unary, OperatorSink, Sink};
use ferroui_system::{IObservable, IObserver, ObservableError};
use std::borrow::Borrow;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::hash::Hash;
use std::rc::Rc;

/// The elements of one key of a [`Lookup`] (`IGrouping<TKey, TElement>`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Grouping<K, E> {
    key: K,
    elements: Vec<E>,
}

impl<K, E> Grouping<K, E> {
    /// The key of the group (`Key`).
    pub fn key(&self) -> &K {
        &self.key
    }

    /// The elements of the group, in the order they were added.
    pub fn elements(&self) -> &[E] {
        &self.elements
    }

    /// The elements of the group (`GetEnumerator`).
    pub fn iter(&self) -> std::slice::Iter<'_, E> {
        self.elements.iter()
    }
}

impl<'a, K, E> IntoIterator for &'a Grouping<K, E> {
    type Item = &'a E;
    type IntoIter = std::slice::Iter<'a, E>;

    fn into_iter(self) -> Self::IntoIter {
        self.elements.iter()
    }
}

/// Keys, each with the elements added under it (`ILookup<TKey, TElement>`, implemented by
/// `System.Reactive.Lookup<K, E>`): an ordered map of keys to their elements, iterated in the order
/// the keys were first added.
///
/// Deviation (DEVIATIONS.md, Reactive): upstream compares keys with an `IEqualityComparer<TKey>`
/// and enumerates the groups in the order of its dictionary (the order the keys were added, for a
/// dictionary that nothing was removed from); the port compares keys with `Eq` and `Hash` and keeps
/// that order explicitly.
#[derive(Clone)]
pub struct Lookup<K, E> {
    index: HashMap<K, usize>,
    groups: Vec<Grouping<K, E>>,
}

impl<K: Eq + Hash + Clone, E> Lookup<K, E> {
    /// An empty lookup.
    pub fn new() -> Self {
        Self { index: HashMap::new(), groups: Vec::new() }
    }

    /// Adds `element` under `key` (`Add`).
    pub fn add(&mut self, key: K, element: E) {
        match self.index.get(&key) {
            Some(&position) => self.groups[position].elements.push(element),
            None => {
                self.index.insert(key.clone(), self.groups.len());
                self.groups.push(Grouping { key, elements: vec![element] });
            }
        }
    }

    /// The elements under `key`, empty when there are none (the indexer `this[TKey]`).
    pub fn get<Q>(&self, key: &Q) -> &[E]
    where
        K: Borrow<Q>,
        Q: Eq + Hash + ?Sized,
    {
        match self.index.get(key) {
            Some(&position) => &self.groups[position].elements,
            None => &[],
        }
    }

    /// Whether there are elements under `key` (`Contains`).
    pub fn contains<Q>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Eq + Hash + ?Sized,
    {
        self.index.contains_key(key)
    }
}

impl<K, E> Lookup<K, E> {
    /// The number of keys (`Count`).
    pub fn count(&self) -> usize {
        self.groups.len()
    }

    /// Whether there are no keys.
    pub fn is_empty(&self) -> bool {
        self.groups.is_empty()
    }

    /// The groups, in the order their keys were first added (`GetEnumerator`).
    pub fn iter(&self) -> std::slice::Iter<'_, Grouping<K, E>> {
        self.groups.iter()
    }
}

impl<K: Eq + Hash + Clone, E> Default for Lookup<K, E> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, K, E> IntoIterator for &'a Lookup<K, E> {
    type Item = &'a Grouping<K, E>;
    type IntoIter = std::slice::Iter<'a, Grouping<K, E>>;

    fn into_iter(self) -> Self::IntoIter {
        self.groups.iter()
    }
}

/// Two lookups are equal when they have the same groups in the same order.
impl<K: PartialEq, E: PartialEq> PartialEq for Lookup<K, E> {
    fn eq(&self, other: &Self) -> bool {
        self.groups == other.groups
    }
}

impl<K: Eq, E: Eq> Eq for Lookup<K, E> {}

impl<K: fmt::Debug, E: fmt::Debug> fmt::Debug for Lookup<K, E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map().entries(self.groups.iter().map(|group| (&group.key, &group.elements))).finish()
    }
}

type KeySelector<T, K> = Rc<dyn Fn(&T) -> Result<K, ObservableError>>;
type ElementSelector<T, E> = Rc<dyn Fn(T) -> Result<E, ObservableError>>;

/// The sink of `ToLookup` (`ToLookup<TSource, TKey, TElement>._`): adds the element of each value
/// under its key and gives the lookup at the end.
struct ToLookupSink<T, K, E> {
    sink: Sink<Lookup<K, E>>,
    key_selector: KeySelector<T, K>,
    element_selector: ElementSelector<T, E>,
    lookup: RefCell<Lookup<K, E>>,
}

impl<T: 'static, K: Eq + Hash + Clone + 'static, E: 'static> OperatorSink<T, Lookup<K, E>> for ToLookupSink<T, K, E> {
    fn sink(&self) -> &Sink<Lookup<K, E>> {
        &self.sink
    }
}

impl_sink_dispose!(ToLookupSink<T, K, E> where K: Eq + Hash + Clone);

impl<T: 'static, K: Eq + Hash + Clone + 'static, E: 'static> ToLookupSink<T, K, E> {
    /// Adds the element of `value` under its key.
    fn add(&self, value: T) -> Result<(), ObservableError> {
        let key = (self.key_selector)(&value)?;
        let element = (self.element_selector)(value)?;
        self.lookup.borrow_mut().add(key, element);
        Ok(())
    }
}

impl<T: 'static, K: Eq + Hash + Clone + 'static, E: 'static> IObserver<T> for ToLookupSink<T, K, E> {
    fn on_next(&self, value: T) {
        if let Err(error) = self.add(value) {
            drop(self.lookup.take());
            self.sink.forward_on_error(error);
        }
    }

    fn on_error(&self, error: ObservableError) {
        drop(self.lookup.take());
        self.sink.forward_on_error(error);
    }

    fn on_completed(&self) {
        let lookup = self.lookup.take();
        self.sink.forward_on_next(lookup);
        self.sink.forward_on_completed();
    }
}

pub(crate) fn to_lookup<T: 'static, K: Eq + Hash + Clone + 'static, E: 'static>(
    source: Rc<dyn IObservable<T>>,
    key_selector: KeySelector<T, K>,
    element_selector: ElementSelector<T, E>,
) -> Rc<dyn IObservable<Lookup<K, E>>> {
    unary(source, move |observer| {
        Rc::new(ToLookupSink {
            sink: Sink::new(observer),
            key_selector: key_selector.clone(),
            element_selector: element_selector.clone(),
            lookup: RefCell::new(Lookup::new()),
        })
    })
}

// Deviation (DEVIATIONS.md, Reactive): the keys of the lookup compare with `Eq` and `Hash`; the
// overloads with an `IEqualityComparer<TKey>` are not ported (a comparer closure has no hash to go
// with it).

/// The methods of `ToLookup` (provided methods of [`ObservableExt`](super::ObservableExt)).
macro_rules! to_lookup_methods {
    () => {
        /// The values in a lookup under the keys `key_selector` gives them, given when the
        /// sequence ends (`ToLookup(Func<TSource, TKey>)`).
        fn to_lookup<K: Eq + std::hash::Hash + Clone + 'static>(
            &self,
            key_selector: impl Fn(&T) -> K + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::linq::to_lookup::Lookup<K, T>>> {
            crate::linq::to_lookup::to_lookup(
                self.observable(),
                std::rc::Rc::new(move |value: &T| Ok(key_selector(value))),
                std::rc::Rc::new(|value: T| Ok(value)),
            )
        }

        /// The elements `element_selector` gives the values in a lookup under the keys
        /// `key_selector` gives them (`ToLookup(Func<TSource, TKey>, Func<TSource, TElement>)`).
        fn to_lookup_with<K: Eq + std::hash::Hash + Clone + 'static, E: 'static>(
            &self,
            key_selector: impl Fn(&T) -> K + 'static,
            element_selector: impl Fn(T) -> E + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::linq::to_lookup::Lookup<K, E>>> {
            crate::linq::to_lookup::to_lookup(
                self.observable(),
                std::rc::Rc::new(move |value: &T| Ok(key_selector(value))),
                std::rc::Rc::new(move |value: T| Ok(element_selector(value))),
            )
        }

        /// [`to_lookup`](Self::to_lookup) with a key selector that may fail; a failure ends the
        /// sequence with its error (`ToLookup` with a key selector that throws).
        fn try_to_lookup<K: Eq + std::hash::Hash + Clone + 'static>(
            &self,
            key_selector: impl Fn(&T) -> Result<K, ferroui_system::ObservableError> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::linq::to_lookup::Lookup<K, T>>> {
            crate::linq::to_lookup::to_lookup(self.observable(), std::rc::Rc::new(key_selector), std::rc::Rc::new(|value: T| Ok(value)))
        }

        /// [`to_lookup_with`](Self::to_lookup_with) with selectors that may fail; a failure ends
        /// the sequence with its error (`ToLookup` with a selector that throws).
        fn try_to_lookup_with<K: Eq + std::hash::Hash + Clone + 'static, E: 'static>(
            &self,
            key_selector: impl Fn(&T) -> Result<K, ferroui_system::ObservableError> + 'static,
            element_selector: impl Fn(T) -> Result<E, ferroui_system::ObservableError> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::linq::to_lookup::Lookup<K, E>>> {
            crate::linq::to_lookup::to_lookup(self.observable(), std::rc::Rc::new(key_selector), std::rc::Rc::new(element_selector))
        }
    };
}
pub(crate) use to_lookup_methods;
