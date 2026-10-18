//! `MaxBy` (`Linq/Observable/MaxBy.cs`); its sink is the one of `MinBy`
//! (`super::min_by::ExtremumBySink`).

use super::min_by::{extremum_by, KeyComparer, KeySelector};
use ferroui_system::IObservable;
use std::cmp::Ordering;
use std::rc::Rc;

pub(crate) fn max_by<T: 'static, K: 'static>(
    source: Rc<dyn IObservable<T>>,
    key_selector: KeySelector<T, K>,
    comparer: KeyComparer<K>,
) -> Rc<dyn IObservable<Vec<T>>> {
    extremum_by(source, key_selector, comparer, Ordering::Greater)
}

/// The methods of `MaxBy` (provided methods of [`ObservableExt`](super::ObservableExt)).
macro_rules! max_by_methods {
    () => {
        /// The values with the maximum key, in the order of the sequence (empty for a sequence
        /// without a value) (`MaxBy(Func<TSource, TKey>)`).
        fn max_by<K: Ord + 'static>(&self, key_selector: impl Fn(&T) -> K + 'static) -> std::rc::Rc<dyn ferroui_system::IObservable<std::vec::Vec<T>>> {
            crate::linq::max_by::max_by(
                self.observable(),
                std::rc::Rc::new(move |value: &T| Ok(key_selector(value))),
                std::rc::Rc::new(|x: &K, y: &K| Ok(x.cmp(y))),
            )
        }

        /// The values with the maximum key by `comparer` (`MaxBy(Func<TSource, TKey>, IComparer<TKey>)`).
        fn max_by_with<K: 'static>(
            &self,
            key_selector: impl Fn(&T) -> K + 'static,
            comparer: impl Fn(&K, &K) -> std::cmp::Ordering + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<std::vec::Vec<T>>> {
            crate::linq::max_by::max_by(
                self.observable(),
                std::rc::Rc::new(move |value: &T| Ok(key_selector(value))),
                std::rc::Rc::new(move |x: &K, y: &K| Ok(comparer(x, y))),
            )
        }

        /// [`max_by`](Self::max_by) with a key selector that may fail; a failure ends the sequence
        /// with its error (`MaxBy(Func<TSource, TKey>)` with a key selector that throws).
        fn try_max_by<K: Ord + 'static>(
            &self,
            key_selector: impl Fn(&T) -> Result<K, ferroui_system::ObservableError> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<std::vec::Vec<T>>> {
            crate::linq::max_by::max_by(self.observable(), std::rc::Rc::new(key_selector), std::rc::Rc::new(|x: &K, y: &K| Ok(x.cmp(y))))
        }

        /// [`max_by_with`](Self::max_by_with) with a key selector and a comparer that may fail; a
        /// failure ends the sequence with its error (`MaxBy(Func<TSource, TKey>, IComparer<TKey>)`
        /// with a key selector or a comparer that throws).
        fn try_max_by_with<K: 'static>(
            &self,
            key_selector: impl Fn(&T) -> Result<K, ferroui_system::ObservableError> + 'static,
            comparer: impl Fn(&K, &K) -> Result<std::cmp::Ordering, ferroui_system::ObservableError> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<std::vec::Vec<T>>> {
            crate::linq::max_by::max_by(self.observable(), std::rc::Rc::new(key_selector), std::rc::Rc::new(comparer))
        }
    };
}
pub(crate) use max_by_methods;
