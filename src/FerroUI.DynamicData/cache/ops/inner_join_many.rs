//! `InnerJoinMany` (`Cache/ObservableCacheEx.InnerJoinMany.cs`).
//!
//! Deviation (DEVIATIONS.md, DynamicData): upstream's selectors take the group of the right items
//! as the interface `IGrouping<TRight, TRightKey, TLeftKey>`; here they take the
//! [`ImmutableGroup`](crate::cache::ImmutableGroup) that implements
//! [`IGrouping`](crate::cache::IGrouping), by reference, which a result may clone and keep.

macro_rules! inner_join_many_methods {
    () => {
        /// The left items with the group of the right items whose key `right_key_selector` maps to
        /// their key, combined by `result_selector`, keyed by the left key; a result exists while
        /// the left item and at least one right item do (SQL `INNER JOIN` of the grouped right
        /// side). The selector does not take the key
        /// (`InnerJoinMany(IObservable<IChangeSet<TRight, TRightKey>>, Func<TRight, TLeftKey>,
        /// Func<TLeft, IGrouping<TRight, TRightKey, TLeftKey>, TDestination>)`).
        fn inner_join_many<TR: Clone + 'static, KR: Eq + std::hash::Hash + Clone + 'static, D: Clone + 'static>(
            &self,
            right: std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<TR, KR>>>,
            right_key_selector: impl Fn(&TR) -> K + 'static,
            result_selector: impl Fn(&T, &crate::cache::ImmutableGroup<TR, KR, K>) -> D + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<D, K>>> {
            self.inner_join_many_with_key(right, right_key_selector, move |_: &K, left: &T, right: &crate::cache::ImmutableGroup<TR, KR, K>| result_selector(left, right))
        }

        /// [`inner_join_many`](Self::inner_join_many) whose selector also takes the key
        /// (`InnerJoinMany(IObservable<IChangeSet<TRight, TRightKey>>, Func<TRight, TLeftKey>,
        /// Func<TLeftKey, TLeft, IGrouping<TRight, TRightKey, TLeftKey>, TDestination>)`).
        fn inner_join_many_with_key<TR: Clone + 'static, KR: Eq + std::hash::Hash + Clone + 'static, D: Clone + 'static>(
            &self,
            right: std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<TR, KR>>>,
            right_key_selector: impl Fn(&TR) -> K + 'static,
            result_selector: impl Fn(&K, &T, &crate::cache::ImmutableGroup<TR, KR, K>) -> D + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<D, K>>> {
            crate::cache::internal::inner_join_many(self.change_sets(), right, std::rc::Rc::new(right_key_selector), std::rc::Rc::new(result_selector))
        }
    };
}
pub(crate) use inner_join_many_methods;
