//! `LeftJoinMany` (`Cache/ObservableCacheEx.LeftJoinMany.cs`).
//!
//! Deviation (DEVIATIONS.md, DynamicData): the selectors take the group as an
//! [`ImmutableGroup`](crate::cache::ImmutableGroup), as those of
//! [`inner_join_many`](crate::cache::ObservableCacheEx::inner_join_many).

macro_rules! left_join_many_methods {
    () => {
        /// Each left item with the group of the right items whose key `right_key_selector` maps to
        /// its key, an empty group when there is none, combined by `result_selector`, keyed by the
        /// left key; a result exists while the left item does (SQL `LEFT JOIN` of the grouped right
        /// side). The selector does not take the key
        /// (`LeftJoinMany(IObservable<IChangeSet<TRight, TRightKey>>, Func<TRight, TLeftKey>,
        /// Func<TLeft, IGrouping<TRight, TRightKey, TLeftKey>, TDestination>)`).
        fn left_join_many<TR: Clone + 'static, KR: Eq + std::hash::Hash + Clone + 'static, D: Clone + 'static>(
            &self,
            right: std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<TR, KR>>>,
            right_key_selector: impl Fn(&TR) -> K + 'static,
            result_selector: impl Fn(&T, &crate::cache::ImmutableGroup<TR, KR, K>) -> D + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<D, K>>> {
            self.left_join_many_with_key(right, right_key_selector, move |_: &K, left: &T, right: &crate::cache::ImmutableGroup<TR, KR, K>| result_selector(left, right))
        }

        /// [`left_join_many`](Self::left_join_many) whose selector also takes the key
        /// (`LeftJoinMany(IObservable<IChangeSet<TRight, TRightKey>>, Func<TRight, TLeftKey>,
        /// Func<TLeftKey, TLeft, IGrouping<TRight, TRightKey, TLeftKey>, TDestination>)`).
        fn left_join_many_with_key<TR: Clone + 'static, KR: Eq + std::hash::Hash + Clone + 'static, D: Clone + 'static>(
            &self,
            right: std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<TR, KR>>>,
            right_key_selector: impl Fn(&TR) -> K + 'static,
            result_selector: impl Fn(&K, &T, &crate::cache::ImmutableGroup<TR, KR, K>) -> D + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<D, K>>> {
            crate::cache::internal::left_join_many(self.change_sets(), right, std::rc::Rc::new(right_key_selector), std::rc::Rc::new(result_selector))
        }
    };
}
pub(crate) use left_join_many_methods;
