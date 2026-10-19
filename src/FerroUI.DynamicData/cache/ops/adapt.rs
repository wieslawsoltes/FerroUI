//! `Adapt` (`Cache/ObservableCacheEx.Adapt.cs`).

macro_rules! adapt_methods {
    () => {
        /// Hands each change set to `adaptor` as it passes (`Adapt(IChangeSetAdaptor<TObject,
        /// TKey>)`).
        fn adapt(&self, adaptor: std::rc::Rc<dyn crate::cache::IChangeSetAdaptor<T, K>>) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>> {
            ferroui_reactive::ObservableExt::do_(&self.change_sets(), move |changes: &crate::cache::ChangeSet<T, K>| adaptor.adapt(changes))
        }

        /// Hands each sorted change set to `adaptor` as it passes
        /// (`Adapt(ISortedChangeSetAdaptor<TObject, TKey>)` of a sorted source), as plain change
        /// sets.
        ///
        /// # Panics
        /// When the source is not a sequence of sorted change sets.
        fn adapt_sorted(&self, adaptor: std::rc::Rc<dyn crate::cache::ISortedChangeSetAdaptor<T, K>>) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>>
        where
            Self: Sized + 'static,
        {
            let sorted = ferroui_reactive::ObservableExt::do_(&crate::cache::internal::bind::expect_sorted_stream::<Self, T, K>(self), move |changes: &crate::cache::SortedChangeSet<T, K>| {
                adaptor.adapt(changes)
            });
            ferroui_reactive::ObservableExt::select(&sorted, |changes: crate::cache::SortedChangeSet<T, K>| crate::cache::IChangeSet::change_set(&changes).clone())
        }
    };
}
pub(crate) use adapt_methods;
