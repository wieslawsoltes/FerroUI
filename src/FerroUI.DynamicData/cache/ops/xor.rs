//! `Xor` (`Cache/ObservableCacheEx.Xor.cs`): a logical Xor between caches. The forms on
//! collections and observable lists of sources are
//! [`CacheSourcesCombineEx`](crate::cache::CacheSourcesCombineEx)
//! and [`CacheSourceListCombineEx`](crate::cache::CacheSourceListCombineEx).

macro_rules! xor_methods {
    () => {
        /// The sequence combined with `others`: the items that are in exactly one of them
        /// (`Xor(IObservable<IChangeSet>, params IObservable<IChangeSet>[])`).
        /// The sequences are subscribed in order when the result is subscribed. An item that a
        /// sequence gives again as the same object (`T: Deref`) is not updated.
        ///
        /// # Panics
        /// When `others` is empty
        /// (`ArgumentException`: "At least one additional stream must be given").
        fn xor(
            &self,
            others: Vec<std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>>>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>>
        where
            T: std::ops::Deref,
        {
            assert!(!others.is_empty(), "At least one additional stream must be given");
            crate::cache::ops::combine::combine_with_others(self.change_sets(), crate::cache::internal::CombineOperator::Xor, others)
        }
    };
}
pub(crate) use xor_methods;
