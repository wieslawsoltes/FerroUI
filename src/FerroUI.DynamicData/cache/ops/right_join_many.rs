//! `RightJoinMany` (`Cache/ObservableCacheEx.RightJoinMany.cs`).
//!
//! Deviation (DEVIATIONS.md, DynamicData): the selectors take the group as an
//! [`ImmutableGroup`](crate::cache::ImmutableGroup), as those of
//! [`inner_join_many`](crate::cache::ObservableCacheEx::inner_join_many).

macro_rules! right_join_many_methods {
    () => {
        /// Each group of the right items, by the left key that `right_key_selector` maps them to,
        /// with the left item of that key, if any, combined by `result_selector`, keyed by the left
        /// key; a result exists while the group has items (SQL `RIGHT JOIN` of the grouped right
        /// side). The selector does not take the key
        /// (`RightJoinMany(IObservable<IChangeSet<TRight, TRightKey>>, Func<TRight, TLeftKey>,
        /// Func<Optional<TLeft>, IGrouping<TRight, TRightKey, TLeftKey>, TDestination>)`).
        fn right_join_many<TR: Clone + 'static, KR: Eq + std::hash::Hash + Clone + 'static, D: Clone + 'static>(
            &self,
            right: std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<TR, KR>>>,
            right_key_selector: impl Fn(&TR) -> K + 'static,
            result_selector: impl Fn(Option<&T>, &crate::cache::ImmutableGroup<TR, KR, K>) -> D + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<D, K>>> {
            self.right_join_many_with_key(right, right_key_selector, move |_: &K, left: Option<&T>, right: &crate::cache::ImmutableGroup<TR, KR, K>| result_selector(left, right))
        }

        /// [`right_join_many`](Self::right_join_many) whose selector also takes the key
        /// (`RightJoinMany(IObservable<IChangeSet<TRight, TRightKey>>, Func<TRight, TLeftKey>,
        /// Func<TLeftKey, Optional<TLeft>, IGrouping<TRight, TRightKey, TLeftKey>, TDestination>)`).
        fn right_join_many_with_key<TR: Clone + 'static, KR: Eq + std::hash::Hash + Clone + 'static, D: Clone + 'static>(
            &self,
            right: std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<TR, KR>>>,
            right_key_selector: impl Fn(&TR) -> K + 'static,
            result_selector: impl Fn(&K, Option<&T>, &crate::cache::ImmutableGroup<TR, KR, K>) -> D + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<D, K>>> {
            crate::cache::internal::right_join_many(self.change_sets(), right, std::rc::Rc::new(right_key_selector), std::rc::Rc::new(result_selector))
        }
    };
}
pub(crate) use right_join_many_methods;
