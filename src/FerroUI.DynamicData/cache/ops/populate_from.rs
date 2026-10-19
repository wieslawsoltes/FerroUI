//! `PopulateFrom` (`Cache/ObservableCacheEx.PopulateFrom.cs`).

/// The forms on source caches, provided methods of [`SourceCacheEx`](crate::cache::SourceCacheEx).
macro_rules! populate_from_source_methods {
    () => {
        /// Adds or updates each item of `observable` (`PopulateFrom(IObservable<TObject>)`).
        fn populate_from(&self, observable: std::rc::Rc<dyn ferroui_system::IObservable<T>>) -> std::rc::Rc<dyn ferroui_system::IDisposable>
        where
            Self: crate::cache::ISourceCache<T, K> + Clone + Sized + 'static,
        {
            let source = self.clone();
            ferroui_reactive::ObservableExt::subscribe_next(&observable, move |item: T| source.add_or_update(item))
        }

        /// Adds or updates the items of each collection of `observable`
        /// (`PopulateFrom(IObservable<IEnumerable<TObject>>)`).
        fn populate_from_many<I: IntoIterator<Item = T> + 'static>(&self, observable: std::rc::Rc<dyn ferroui_system::IObservable<I>>) -> std::rc::Rc<dyn ferroui_system::IDisposable>
        where
            Self: crate::cache::ISourceCache<T, K> + Clone + Sized + 'static,
        {
            let source = self.clone();
            ferroui_reactive::ObservableExt::subscribe_next(&observable, move |items: I| source.add_or_update_items(items))
        }
    };
}
pub(crate) use populate_from_source_methods;
