//! `UpdateIndex` (`Cache/ObservableCacheEx.UpdateIndex.cs`).

macro_rules! update_index_methods {
    () => {
        /// Sets the index of each sorted item to its position before the change set is forwarded
        /// (`UpdateIndex`).
        ///
        /// Deviation (DEVIATIONS.md, DynamicData): the change sets keep their type (sorted, paged
        /// or virtual), where upstream's are typed as sorted change sets.
        fn update_index(&self) -> std::rc::Rc<dyn ferroui_system::IObservable<C>>
        where
            T: crate::binding::IIndexAware,
        {
            ferroui_reactive::ObservableExt::do_(&self.sorted_source(), |changes: &C| {
                for (index, (_, value)) in crate::cache::ISortedChangeSet::sorted_items(changes).iter().enumerate() {
                    crate::binding::IIndexAware::set_index(value, index);
                }
            })
        }
    };
}
pub(crate) use update_index_methods;
