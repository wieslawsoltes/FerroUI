//! `ChangeKey` (`Cache/ObservableCacheEx.ChangeKey.cs`).

macro_rules! change_key_methods {
    () => {
        /// The changes keyed by `key_selector` of the item (`ChangeKey(Func<TObject,
        /// TDestinationKey>)`); the indexes are dropped, as upstream's are.
        fn change_key<K2: Eq + std::hash::Hash + Clone + 'static>(
            &self,
            key_selector: impl Fn(&T) -> K2 + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K2>>> {
            ferroui_reactive::ObservableExt::select(&self.change_sets(), move |updates: crate::cache::ChangeSet<T, K>| {
                updates
                    .iter()
                    .map(|u| crate::cache::Change::with_previous(u.reason(), key_selector(u.current()), u.current().clone(), u.previous().cloned(), None, None))
                    .collect::<crate::cache::ChangeSet<T, K2>>()
            })
        }

        /// The changes keyed by `key_selector` of the key and the item (`ChangeKey(Func<TSourceKey,
        /// TObject, TDestinationKey>)`).
        fn change_key_with_key<K2: Eq + std::hash::Hash + Clone + 'static>(
            &self,
            key_selector: impl Fn(&K, &T) -> K2 + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K2>>> {
            ferroui_reactive::ObservableExt::select(&self.change_sets(), move |updates: crate::cache::ChangeSet<T, K>| {
                updates
                    .iter()
                    .map(|u| crate::cache::Change::with_previous(u.reason(), key_selector(u.key(), u.current()), u.current().clone(), u.previous().cloned(), None, None))
                    .collect::<crate::cache::ChangeSet<T, K2>>()
            })
        }
    };
}
pub(crate) use change_key_methods;
