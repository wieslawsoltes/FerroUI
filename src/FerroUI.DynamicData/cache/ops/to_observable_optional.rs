//! `ToObservableOptional` (`Cache/ObservableCacheEx.ToObservableOptional.cs`).

macro_rules! to_observable_optional_methods {
    () => {
        /// The item of `key` as an option each time it changes, `None` when it is removed; equal
        /// successive items are left out (`ToObservableOptional(TKey, IEqualityComparer? = null)`).
        fn to_observable_optional(&self, key: K) -> std::rc::Rc<dyn ferroui_system::IObservable<Option<T>>>
        where
            T: PartialEq,
        {
            crate::cache::internal::to_observable_optional(self.change_sets(), key, std::rc::Rc::new(|a: &T, b: &T| a == b))
        }

        /// [`to_observable_optional`](Self::to_observable_optional) with the equality of
        /// `equality_comparer` (`ToObservableOptional(TKey, IEqualityComparer)`).
        fn to_observable_optional_with_comparer(&self, key: K, equality_comparer: impl Fn(&T, &T) -> bool + 'static) -> std::rc::Rc<dyn ferroui_system::IObservable<Option<T>>> {
            crate::cache::internal::to_observable_optional(self.change_sets(), key, std::rc::Rc::new(equality_comparer))
        }

        /// [`to_observable_optional`](Self::to_observable_optional) that starts with `None` when the
        /// item is not there at first and `initial_optional_when_missing` is true
        /// (`ToObservableOptional(TKey, bool, IEqualityComparer? = null)`).
        fn to_observable_optional_with_initial(&self, key: K, initial_optional_when_missing: bool) -> std::rc::Rc<dyn ferroui_system::IObservable<Option<T>>>
        where
            T: PartialEq,
        {
            self.to_observable_optional_with_initial_and_comparer(key, initial_optional_when_missing, |a: &T, b: &T| a == b)
        }

        /// [`to_observable_optional_with_initial`](Self::to_observable_optional_with_initial) with
        /// the equality of `equality_comparer`.
        fn to_observable_optional_with_initial_and_comparer(
            &self,
            key: K,
            initial_optional_when_missing: bool,
            equality_comparer: impl Fn(&T, &T) -> bool + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<Option<T>>> {
            if initial_optional_when_missing {
                crate::cache::internal::to_observable_optional_with_initial(self.change_sets(), key, std::rc::Rc::new(equality_comparer))
            } else {
                crate::cache::internal::to_observable_optional(self.change_sets(), key, std::rc::Rc::new(equality_comparer))
            }
        }
    };
}
pub(crate) use to_observable_optional_methods;
