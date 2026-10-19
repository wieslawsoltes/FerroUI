//! `FullJoinMany` (`Cache/ObservableCacheEx.FullJoinMany.cs`).
//!
//! Deviation (DEVIATIONS.md, DynamicData): the selectors take the group as an
//! [`ImmutableGroup`](crate::cache::ImmutableGroup), as those of
//! [`inner_join_many`](crate::cache::ObservableCacheEx::inner_join_many).

macro_rules! full_join_many_methods {
    () => {
        /// The left items and the groups of the right items, by the left key that
        /// `right_key_selector` maps them to, joined by that key, an empty group when there is
        /// none, combined by `result_selector`; a result exists while the left item or the group
        /// does (SQL `FULL OUTER JOIN` of the grouped right side). The selector does not take the
        /// key (`FullJoinMany(IObservable<IChangeSet<TRight, TRightKey>>, Func<TRight, TLeftKey>,
        /// Func<Optional<TLeft>, IGrouping<TRight, TRightKey, TLeftKey>, TDestination>)`).
        fn full_join_many<TR: Clone + 'static, KR: Eq + std::hash::Hash + Clone + 'static, D: Clone + 'static>(
            &self,
            right: std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<TR, KR>>>,
            right_key_selector: impl Fn(&TR) -> K + 'static,
            result_selector: impl Fn(Option<&T>, &crate::cache::ImmutableGroup<TR, KR, K>) -> D + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<D, K>>> {
            self.full_join_many_with_key(right, right_key_selector, move |_: &K, left: Option<&T>, right: &crate::cache::ImmutableGroup<TR, KR, K>| result_selector(left, right))
        }

        /// [`full_join_many`](Self::full_join_many) whose selector also takes the key
        /// (`FullJoinMany(IObservable<IChangeSet<TRight, TRightKey>>, Func<TRight, TLeftKey>,
        /// Func<TLeftKey, Optional<TLeft>, IGrouping<TRight, TRightKey, TLeftKey>, TDestination>)`).
        fn full_join_many_with_key<TR: Clone + 'static, KR: Eq + std::hash::Hash + Clone + 'static, D: Clone + 'static>(
            &self,
            right: std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<TR, KR>>>,
            right_key_selector: impl Fn(&TR) -> K + 'static,
            result_selector: impl Fn(&K, Option<&T>, &crate::cache::ImmutableGroup<TR, KR, K>) -> D + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<D, K>>> {
            crate::cache::internal::full_join_many(self.change_sets(), right, std::rc::Rc::new(right_key_selector), std::rc::Rc::new(result_selector))
        }
    };
}
pub(crate) use full_join_many_methods;
