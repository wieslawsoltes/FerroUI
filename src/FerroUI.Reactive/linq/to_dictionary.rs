//! `ToDictionary` (`Linq/Observable/ToDictionary.cs`).

use super::aggregate::SequenceError;
use crate::internal::{impl_sink_dispose, unary, OperatorSink, Sink};
use ferroui_system::{IObservable, IObserver, ObservableError};
use std::cell::RefCell;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::hash::Hash;
use std::rc::Rc;

type KeySelector<T, K> = Rc<dyn Fn(&T) -> Result<K, ObservableError>>;
type ElementSelector<T, E> = Rc<dyn Fn(T) -> Result<E, ObservableError>>;

/// The sink of `ToDictionary` (`ToDictionary<TSource, TKey, TElement>._`): adds the element of
/// each value under its key and gives the map at the end; a key that is already in the map ends the
/// sequence with [`SequenceError::DuplicateKey`].
struct ToDictionarySink<T, K, E> {
    sink: Sink<HashMap<K, E>>,
    key_selector: KeySelector<T, K>,
    element_selector: ElementSelector<T, E>,
    dictionary: RefCell<HashMap<K, E>>,
}

impl<T: 'static, K: Eq + Hash + 'static, E: 'static> OperatorSink<T, HashMap<K, E>> for ToDictionarySink<T, K, E> {
    fn sink(&self) -> &Sink<HashMap<K, E>> {
        &self.sink
    }
}

impl_sink_dispose!(ToDictionarySink<T, K, E> where K: Eq + Hash);

impl<T: 'static, K: Eq + Hash + 'static, E: 'static> ToDictionarySink<T, K, E> {
    /// Adds the element of `value` under its key (`Dictionary<TKey, TElement>.Add` of the selected
    /// key and element).
    fn add(&self, value: T) -> Result<(), ObservableError> {
        let key = (self.key_selector)(&value)?;
        let element = (self.element_selector)(value)?;
        match self.dictionary.borrow_mut().entry(key) {
            Entry::Occupied(_) => Err(SequenceError::DuplicateKey.into_error()),
            Entry::Vacant(entry) => {
                entry.insert(element);
                Ok(())
            }
        }
    }
}

impl<T: 'static, K: Eq + Hash + 'static, E: 'static> IObserver<T> for ToDictionarySink<T, K, E> {
    fn on_next(&self, value: T) {
        if let Err(error) = self.add(value) {
            drop(self.dictionary.take());
            self.sink.forward_on_error(error);
        }
    }

    fn on_error(&self, error: ObservableError) {
        drop(self.dictionary.take());
        self.sink.forward_on_error(error);
    }

    fn on_completed(&self) {
        let dictionary = self.dictionary.take();
        self.sink.forward_on_next(dictionary);
        self.sink.forward_on_completed();
    }
}

pub(crate) fn to_dictionary<T: 'static, K: Eq + Hash + 'static, E: 'static>(
    source: Rc<dyn IObservable<T>>,
    key_selector: KeySelector<T, K>,
    element_selector: ElementSelector<T, E>,
) -> Rc<dyn IObservable<HashMap<K, E>>> {
    unary(source, move |observer| {
        Rc::new(ToDictionarySink {
            sink: Sink::new(observer),
            key_selector: key_selector.clone(),
            element_selector: element_selector.clone(),
            dictionary: RefCell::new(HashMap::new()),
        })
    })
}

// Deviation (DEVIATIONS.md, Reactive): the map is a `HashMap`, whose keys compare with `Eq` and
// `Hash`; the overloads with an `IEqualityComparer<TKey>` are not ported (a comparer closure has no
// hash to go with it).

/// The methods of `ToDictionary` (provided methods of [`ObservableExt`](super::ObservableExt)).
macro_rules! to_dictionary_methods {
    () => {
        /// The values in a map under the keys `key_selector` gives them, given when the sequence
        /// ends; a key given twice ends the sequence with `SequenceError::DuplicateKey`
        /// (`ToDictionary(Func<TSource, TKey>)`).
        fn to_dictionary<K: Eq + std::hash::Hash + 'static>(
            &self,
            key_selector: impl Fn(&T) -> K + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<std::collections::HashMap<K, T>>> {
            crate::linq::to_dictionary::to_dictionary(
                self.observable(),
                std::rc::Rc::new(move |value: &T| Ok(key_selector(value))),
                std::rc::Rc::new(|value: T| Ok(value)),
            )
        }

        /// The elements `element_selector` gives the values in a map under the keys
        /// `key_selector` gives them (`ToDictionary(Func<TSource, TKey>, Func<TSource, TElement>)`).
        fn to_dictionary_with<K: Eq + std::hash::Hash + 'static, E: 'static>(
            &self,
            key_selector: impl Fn(&T) -> K + 'static,
            element_selector: impl Fn(T) -> E + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<std::collections::HashMap<K, E>>> {
            crate::linq::to_dictionary::to_dictionary(
                self.observable(),
                std::rc::Rc::new(move |value: &T| Ok(key_selector(value))),
                std::rc::Rc::new(move |value: T| Ok(element_selector(value))),
            )
        }

        /// [`to_dictionary`](Self::to_dictionary) with a key selector that may fail; a failure
        /// ends the sequence with its error (`ToDictionary` with a key selector that throws).
        fn try_to_dictionary<K: Eq + std::hash::Hash + 'static>(
            &self,
            key_selector: impl Fn(&T) -> Result<K, ferroui_system::ObservableError> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<std::collections::HashMap<K, T>>> {
            crate::linq::to_dictionary::to_dictionary(self.observable(), std::rc::Rc::new(key_selector), std::rc::Rc::new(|value: T| Ok(value)))
        }

        /// [`to_dictionary_with`](Self::to_dictionary_with) with selectors that may fail; a
        /// failure ends the sequence with its error (`ToDictionary` with a selector that throws).
        fn try_to_dictionary_with<K: Eq + std::hash::Hash + 'static, E: 'static>(
            &self,
            key_selector: impl Fn(&T) -> Result<K, ferroui_system::ObservableError> + 'static,
            element_selector: impl Fn(T) -> Result<E, ferroui_system::ObservableError> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<std::collections::HashMap<K, E>>> {
            crate::linq::to_dictionary::to_dictionary(self.observable(), std::rc::Rc::new(key_selector), std::rc::Rc::new(element_selector))
        }
    };
}
pub(crate) use to_dictionary_methods;
