//! `InnerJoin` (`Cache/ObservableCacheEx.InnerJoin.cs`).

macro_rules! inner_join_methods {
    () => {
        /// The pairs of a left item and a right item whose key `right_key_selector` maps to the key
        /// of the left item, combined by `result_selector`, keyed by both keys; a pair exists while
        /// both items do (SQL `INNER JOIN`). The selector does not take the keys
        /// (`InnerJoin(IObservable<IChangeSet<TRight, TRightKey>>, Func<TRight, TLeftKey>,
        /// Func<TLeft, TRight, TDestination>)`).
        fn inner_join<TR: Clone + 'static, KR: Eq + std::hash::Hash + Clone + 'static, D: Clone + 'static>(
            &self,
            right: std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<TR, KR>>>,
            right_key_selector: impl Fn(&TR) -> K + 'static,
            result_selector: impl Fn(&T, &TR) -> D + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<D, (K, KR)>>> {
            self.inner_join_with_key(right, right_key_selector, move |_: &(K, KR), left: &T, right: &TR| result_selector(left, right))
        }

        /// [`inner_join`](Self::inner_join) whose selector also takes the pair of keys, the key of
        /// the result (`InnerJoin(IObservable<IChangeSet<TRight, TRightKey>>, Func<TRight,
        /// TLeftKey>, Func<(TLeftKey leftKey, TRightKey rightKey), TLeft, TRight, TDestination>)`).
        ///
        /// A left change pairs the left item with every right item of its key; a right change pairs
        /// the right item with the left item of its key, and removes its former pair when its key
        /// changed. The left items present at the subscription are joined once the right items
        /// arrive, in one change set.
        fn inner_join_with_key<TR: Clone + 'static, KR: Eq + std::hash::Hash + Clone + 'static, D: Clone + 'static>(
            &self,
            right: std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<TR, KR>>>,
            right_key_selector: impl Fn(&TR) -> K + 'static,
            result_selector: impl Fn(&(K, KR), &T, &TR) -> D + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<D, (K, KR)>>> {
            crate::cache::internal::inner_join(self.change_sets(), right, std::rc::Rc::new(right_key_selector), std::rc::Rc::new(result_selector))
        }
    };
}
pub(crate) use inner_join_methods;
