//! `MinBy` (`Linq/Observable/MinBy.cs`), and the sink `MaxBy` shares with it.

use crate::internal::{impl_sink_dispose, unary, OperatorSink, Sink};
use ferroui_system::{IObservable, IObserver, ObservableError};
use std::cell::RefCell;
use std::cmp::Ordering;
use std::rc::Rc;

/// The key of a value; an error ends the sequence with it (a key selector that throws).
pub(crate) type KeySelector<T, K> = Rc<dyn Fn(&T) -> Result<K, ObservableError>>;

/// A comparer of keys (`IComparer<TKey>`) that may fail.
pub(crate) type KeyComparer<K> = Rc<dyn Fn(&K, &K) -> Result<Ordering, ObservableError>>;

/// The sink of `MinBy` and `MaxBy` (`MinBy<TSource, TKey>._` and `MaxBy<TSource, TKey>._`): keeps
/// the values whose key compares as `direction` (`Less` for `MinBy`, `Greater` for `MaxBy`) to the
/// others, or equal to the kept key, and gives them as a list at the end (empty for a sequence
/// without a value).
pub(crate) struct ExtremumBySink<T, K> {
    sink: Sink<Vec<T>>,
    key_selector: KeySelector<T, K>,
    comparer: KeyComparer<K>,
    direction: Ordering,
    last_key: RefCell<Option<K>>,
    list: RefCell<Vec<T>>,
}

impl<T: 'static, K: 'static> ExtremumBySink<T, K> {
    /// Drops the kept values and key (`Cleanup`).
    fn cleanup(&self) {
        drop(self.list.take());
        drop(self.last_key.take());
    }
}

impl<T: 'static, K: 'static> OperatorSink<T, Vec<T>> for ExtremumBySink<T, K> {
    fn sink(&self) -> &Sink<Vec<T>> {
        &self.sink
    }
}

impl_sink_dispose!(ExtremumBySink<T, K>);

impl<T: 'static, K: 'static> IObserver<T> for ExtremumBySink<T, K> {
    fn on_next(&self, value: T) {
        let key = match (self.key_selector)(&value) {
            Ok(key) => key,
            Err(error) => {
                self.cleanup();
                self.sink.forward_on_error(error);
                return;
            }
        };
        let compared = self.last_key.borrow().as_ref().map(|last_key| (self.comparer)(&key, last_key));
        let comparison = match compared {
            None => {
                *self.last_key.borrow_mut() = Some(key);
                Ordering::Equal
            }
            Some(Ok(comparison)) => {
                if comparison == self.direction {
                    *self.last_key.borrow_mut() = Some(key);
                    self.list.borrow_mut().clear();
                }
                comparison
            }
            Some(Err(error)) => {
                self.cleanup();
                self.sink.forward_on_error(error);
                return;
            }
        };
        if comparison == self.direction || comparison == Ordering::Equal {
            self.list.borrow_mut().push(value);
        }
    }

    fn on_error(&self, error: ObservableError) {
        self.cleanup();
        self.sink.forward_on_error(error);
    }

    fn on_completed(&self) {
        let list = self.list.take();
        self.cleanup();
        self.sink.forward_on_next(list);
        self.sink.forward_on_completed();
    }
}

/// The observable of `MinBy` (`direction` `Less`) or `MaxBy` (`direction` `Greater`).
pub(crate) fn extremum_by<T: 'static, K: 'static>(
    source: Rc<dyn IObservable<T>>,
    key_selector: KeySelector<T, K>,
    comparer: KeyComparer<K>,
    direction: Ordering,
) -> Rc<dyn IObservable<Vec<T>>> {
    unary(source, move |observer| {
        Rc::new(ExtremumBySink {
            sink: Sink::new(observer),
            key_selector: key_selector.clone(),
            comparer: comparer.clone(),
            direction,
            last_key: RefCell::new(None),
            list: RefCell::new(Vec::new()),
        })
    })
}

pub(crate) fn min_by<T: 'static, K: 'static>(
    source: Rc<dyn IObservable<T>>,
    key_selector: KeySelector<T, K>,
    comparer: KeyComparer<K>,
) -> Rc<dyn IObservable<Vec<T>>> {
    extremum_by(source, key_selector, comparer, Ordering::Less)
}

/// The methods of `MinBy` (provided methods of [`ObservableExt`](super::ObservableExt)).
macro_rules! min_by_methods {
    () => {
        /// The values with the minimum key, in the order of the sequence (empty for a sequence
        /// without a value) (`MinBy(Func<TSource, TKey>)`).
        fn min_by<K: Ord + 'static>(&self, key_selector: impl Fn(&T) -> K + 'static) -> std::rc::Rc<dyn ferroui_system::IObservable<std::vec::Vec<T>>> {
            crate::linq::min_by::min_by(
                self.observable(),
                std::rc::Rc::new(move |value: &T| Ok(key_selector(value))),
                std::rc::Rc::new(|x: &K, y: &K| Ok(x.cmp(y))),
            )
        }

        /// The values with the minimum key by `comparer` (`MinBy(Func<TSource, TKey>, IComparer<TKey>)`).
        fn min_by_with<K: 'static>(
            &self,
            key_selector: impl Fn(&T) -> K + 'static,
            comparer: impl Fn(&K, &K) -> std::cmp::Ordering + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<std::vec::Vec<T>>> {
            crate::linq::min_by::min_by(
                self.observable(),
                std::rc::Rc::new(move |value: &T| Ok(key_selector(value))),
                std::rc::Rc::new(move |x: &K, y: &K| Ok(comparer(x, y))),
            )
        }

        /// [`min_by`](Self::min_by) with a key selector that may fail; a failure ends the sequence
        /// with its error (`MinBy(Func<TSource, TKey>)` with a key selector that throws).
        fn try_min_by<K: Ord + 'static>(
            &self,
            key_selector: impl Fn(&T) -> Result<K, ferroui_system::ObservableError> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<std::vec::Vec<T>>> {
            crate::linq::min_by::min_by(self.observable(), std::rc::Rc::new(key_selector), std::rc::Rc::new(|x: &K, y: &K| Ok(x.cmp(y))))
        }

        /// [`min_by_with`](Self::min_by_with) with a key selector and a comparer that may fail; a
        /// failure ends the sequence with its error (`MinBy(Func<TSource, TKey>, IComparer<TKey>)`
        /// with a key selector or a comparer that throws).
        fn try_min_by_with<K: 'static>(
            &self,
            key_selector: impl Fn(&T) -> Result<K, ferroui_system::ObservableError> + 'static,
            comparer: impl Fn(&K, &K) -> Result<std::cmp::Ordering, ferroui_system::ObservableError> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<std::vec::Vec<T>>> {
            crate::linq::min_by::min_by(self.observable(), std::rc::Rc::new(key_selector), std::rc::Rc::new(comparer))
        }
    };
}
pub(crate) use min_by_methods;
