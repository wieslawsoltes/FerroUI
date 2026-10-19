//! `AddKey` (`List/ObservableListEx.AddKey.cs`).

macro_rules! add_key_methods {
    () => {
        /// The changes of the list as changes of a cache, each item keyed by `key_selector`
        /// (`AddKey(Func<TObject, TKey>)`).
        fn add_key<K: Eq + std::hash::Hash + Clone + 'static>(
            &self,
            key_selector: impl Fn(&T) -> K + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>> {
            ferroui_reactive::ObservableExt::select(&self.change_sets(), move |changes: crate::list::ChangeSet<T>| {
                crate::list::internal::add_key(&changes, &key_selector)
            })
        }
    };
}
pub(crate) use add_key_methods;
