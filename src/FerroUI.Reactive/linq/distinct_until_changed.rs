//! `DistinctUntilChanged` (`Linq/Observable/DistinctUntilChanged.cs`).

use crate::internal::{impl_sink_dispose, unary, OperatorSink, Sink};
use ferroui_system::{IObservable, IObserver, ObservableError};
use std::cell::RefCell;
use std::rc::Rc;

type KeySelector<T, K> = Rc<dyn Fn(&T) -> Result<K, ObservableError>>;
type Comparer<K> = Rc<dyn Fn(&K, &K) -> Result<bool, ObservableError>>;

/// The sink of `DistinctUntilChanged` (`DistinctUntilChanged<TSource, TKey>._`): a key selector or
/// comparer that fails ends the sequence with its error.
struct DistinctUntilChangedSink<T, K> {
    sink: Sink<T>,
    key_selector: KeySelector<T, K>,
    comparer: Comparer<K>,
    current_key: RefCell<Option<K>>,
}

impl<T: 'static, K: 'static> OperatorSink<T, T> for DistinctUntilChangedSink<T, K> {
    fn sink(&self) -> &Sink<T> {
        &self.sink
    }
}

impl_sink_dispose!(DistinctUntilChangedSink<T, K>);

impl<T: 'static, K: 'static> IObserver<T> for DistinctUntilChangedSink<T, K> {
    fn on_next(&self, value: T) {
        let key = match (self.key_selector)(&value) {
            Ok(key) => key,
            Err(error) => {
                self.sink.forward_on_error(error);
                return;
            }
        };

        let compared = {
            let current = self.current_key.borrow();
            current.as_ref().map(|current| (self.comparer)(current, &key))
        };
        let (has_current_key, comparer_equals) = match compared {
            None => (false, false),
            Some(Ok(equals)) => (true, equals),
            Some(Err(error)) => {
                self.sink.forward_on_error(error);
                return;
            }
        };

        if !has_current_key || !comparer_equals {
            *self.current_key.borrow_mut() = Some(key);
            self.sink.forward_on_next(value);
        }
    }

    fn on_error(&self, error: ObservableError) {
        self.sink.forward_on_error(error);
    }

    fn on_completed(&self) {
        self.sink.forward_on_completed();
    }
}

pub(crate) fn distinct_until_changed<T: 'static, K: 'static>(
    source: Rc<dyn IObservable<T>>,
    key_selector: KeySelector<T, K>,
    comparer: Comparer<K>,
) -> Rc<dyn IObservable<T>> {
    unary(source, move |observer| {
        Rc::new(DistinctUntilChangedSink {
            sink: Sink::new(observer),
            key_selector: key_selector.clone(),
            comparer: comparer.clone(),
            current_key: RefCell::new(None),
        })
    })
}

/// The methods of `DistinctUntilChanged` (provided methods of [`ObservableExt`](super::ObservableExt)).
macro_rules! distinct_until_changed_methods {
    () => {
        /// The values that differ from the one before (`DistinctUntilChanged()`).
        fn distinct_until_changed(&self) -> std::rc::Rc<dyn ferroui_system::IObservable<T>>
        where
            T: Clone + PartialEq,
        {
            crate::linq::distinct_until_changed::distinct_until_changed(
                self.observable(),
                std::rc::Rc::new(|value: &T| Ok(value.clone())),
                std::rc::Rc::new(|a: &T, b: &T| Ok(a == b)),
            )
        }

        /// The values that differ from the one before, compared with `comparer`
        /// (`DistinctUntilChanged(IEqualityComparer<TSource>)`).
        fn distinct_until_changed_with_comparer(
            &self,
            comparer: impl Fn(&T, &T) -> bool + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<T>>
        where
            T: Clone,
        {
            crate::linq::distinct_until_changed::distinct_until_changed(
                self.observable(),
                std::rc::Rc::new(|value: &T| Ok(value.clone())),
                std::rc::Rc::new(move |a: &T, b: &T| Ok(comparer(a, b))),
            )
        }

        /// [`distinct_until_changed_with_comparer`](Self::distinct_until_changed_with_comparer) with
        /// a comparer that may fail; a failure ends the sequence with its error
        /// (`DistinctUntilChanged(IEqualityComparer<TSource>)` with a comparer that throws).
        fn try_distinct_until_changed_with_comparer(
            &self,
            comparer: impl Fn(&T, &T) -> Result<bool, ferroui_system::ObservableError> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<T>>
        where
            T: Clone,
        {
            crate::linq::distinct_until_changed::distinct_until_changed(
                self.observable(),
                std::rc::Rc::new(|value: &T| Ok(value.clone())),
                std::rc::Rc::new(comparer),
            )
        }

        /// The values whose key differs from the key of the one before
        /// (`DistinctUntilChanged(Func<TSource, TKey>)`).
        fn distinct_until_changed_by_key<K: PartialEq + 'static>(
            &self,
            key_selector: impl Fn(&T) -> K + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::distinct_until_changed::distinct_until_changed(
                self.observable(),
                std::rc::Rc::new(move |value: &T| Ok(key_selector(value))),
                std::rc::Rc::new(|a: &K, b: &K| Ok(a == b)),
            )
        }

        /// [`distinct_until_changed_by_key`](Self::distinct_until_changed_by_key) with a key
        /// selector that may fail; a failure ends the sequence with its error
        /// (`DistinctUntilChanged(Func<TSource, TKey>)` with a selector that throws).
        fn try_distinct_until_changed_by_key<K: PartialEq + 'static>(
            &self,
            key_selector: impl Fn(&T) -> Result<K, ferroui_system::ObservableError> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::distinct_until_changed::distinct_until_changed(
                self.observable(),
                std::rc::Rc::new(key_selector),
                std::rc::Rc::new(|a: &K, b: &K| Ok(a == b)),
            )
        }

        /// The values whose key differs from the key of the one before, compared with `comparer`
        /// (`DistinctUntilChanged(Func<TSource, TKey>, IEqualityComparer<TKey>)`).
        fn distinct_until_changed_with<K: 'static>(
            &self,
            key_selector: impl Fn(&T) -> K + 'static,
            comparer: impl Fn(&K, &K) -> bool + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::distinct_until_changed::distinct_until_changed(
                self.observable(),
                std::rc::Rc::new(move |value: &T| Ok(key_selector(value))),
                std::rc::Rc::new(move |a: &K, b: &K| Ok(comparer(a, b))),
            )
        }

        /// [`distinct_until_changed_with`](Self::distinct_until_changed_with) with a key selector
        /// and a comparer that may fail; a failure ends the sequence with its error.
        fn try_distinct_until_changed_with<K: 'static>(
            &self,
            key_selector: impl Fn(&T) -> Result<K, ferroui_system::ObservableError> + 'static,
            comparer: impl Fn(&K, &K) -> Result<bool, ferroui_system::ObservableError> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::distinct_until_changed::distinct_until_changed(
                self.observable(),
                std::rc::Rc::new(key_selector),
                std::rc::Rc::new(comparer),
            )
        }
    };
}
pub(crate) use distinct_until_changed_methods;
