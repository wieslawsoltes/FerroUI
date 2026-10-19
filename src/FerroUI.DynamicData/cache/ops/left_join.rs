//! `LeftJoin` (`Cache/ObservableCacheEx.LeftJoin.cs`).

macro_rules! left_join_methods {
    () => {
        /// Each left item with the right item whose key `right_key_selector` maps to its key, if
        /// any (upstream's `Optional<TRight>` is an `Option`), combined by `result_selector`, keyed
        /// by the left key; a result exists while the left item does (SQL `LEFT JOIN`). The selector
        /// does not take the key (`LeftJoin(IObservable<IChangeSet<TRight, TRightKey>>,
        /// Func<TRight, TLeftKey>, Func<TLeft, Optional<TRight>, TDestination>)`).
        fn left_join<TR: Clone + 'static, KR: Eq + std::hash::Hash + Clone + 'static, D: Clone + 'static>(
            &self,
            right: std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<TR, KR>>>,
            right_key_selector: impl Fn(&TR) -> K + 'static,
            result_selector: impl Fn(&T, Option<&TR>) -> D + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<D, K>>> {
            self.left_join_with_key(right, right_key_selector, move |_: &K, left: &T, right: Option<&TR>| result_selector(left, right))
        }

        /// [`left_join`](Self::left_join) whose selector also takes the left key
        /// (`LeftJoin(IObservable<IChangeSet<TRight, TRightKey>>, Func<TRight, TLeftKey>,
        /// Func<TLeftKey, TLeft, Optional<TRight>, TDestination>)`).
        ///
        /// When several right items map to one left key, the latest one is joined. The right items
        /// present at the subscription are joined as the left items arrive, in one change set.
        fn left_join_with_key<TR: Clone + 'static, KR: Eq + std::hash::Hash + Clone + 'static, D: Clone + 'static>(
            &self,
            right: std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<TR, KR>>>,
            right_key_selector: impl Fn(&TR) -> K + 'static,
            result_selector: impl Fn(&K, &T, Option<&TR>) -> D + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<D, K>>> {
            crate::cache::internal::left_join(self.change_sets(), right, std::rc::Rc::new(right_key_selector), std::rc::Rc::new(result_selector))
        }
    };
}
pub(crate) use left_join_methods;
