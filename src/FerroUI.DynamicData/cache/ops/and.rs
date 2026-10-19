//! `And` (`Cache/ObservableCacheEx.And.cs`): a logical And between caches. The forms on
//! collections and observable lists of sources are
//! [`CacheSourcesCombineEx`](crate::cache::CacheSourcesCombineEx)
//! and [`CacheSourceListCombineEx`](crate::cache::CacheSourceListCombineEx).
//!
//! Naming (DEVIATIONS.md, DynamicData): `and_cache`, as `and` would hide the join pattern
//! `ObservableExt::and`.

macro_rules! and_methods {
    () => {
        /// The sequence combined with `others`: the items that are in every one of them
        /// (`And(IObservable<IChangeSet>, params IObservable<IChangeSet>[])`).
        /// The sequences are subscribed in order when the result is subscribed. An item that a
        /// sequence gives again as the same object (`T: Deref`) is not updated.
        ///
        /// # Panics
        /// When `others` is empty
        /// (`ArgumentException`: "At least one additional stream must be supplied").
        fn and_cache(
            &self,
            others: Vec<std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>>>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>>
        where
            T: std::ops::Deref,
        {
            assert!(!others.is_empty(), "At least one additional stream must be supplied");
            crate::cache::ops::combine::combine_with_others(self.change_sets(), crate::cache::internal::CombineOperator::And, others)
        }
    };
}
pub(crate) use and_methods;
