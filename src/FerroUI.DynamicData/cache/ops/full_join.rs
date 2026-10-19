//! `FullJoin` (`Cache/ObservableCacheEx.FullJoin.cs`).

macro_rules! full_join_methods {
    () => {
        /// The left items and the right items, the right ones keyed by the left key that
        /// `right_key_selector` maps them to, joined by that key and combined by `result_selector`
        /// (upstream's `Optional`s are `Option`s); a result exists while either side has an item
        /// of its key (SQL `FULL OUTER JOIN`). The selector does not take the key
        /// (`FullJoin(IObservable<IChangeSet<TRight, TRightKey>>, Func<TRight, TLeftKey>,
        /// Func<Optional<TLeft>, Optional<TRight>, TDestination>)`).
        fn full_join<TR: Clone + 'static, KR: Eq + std::hash::Hash + Clone + 'static, D: Clone + 'static>(
            &self,
            right: std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<TR, KR>>>,
            right_key_selector: impl Fn(&TR) -> K + 'static,
            result_selector: impl Fn(Option<&T>, Option<&TR>) -> D + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<D, K>>> {
            self.full_join_with_key(right, right_key_selector, move |_: &K, left: Option<&T>, right: Option<&TR>| result_selector(left, right))
        }

        /// [`full_join`](Self::full_join) whose selector also takes the key
        /// (`FullJoin(IObservable<IChangeSet<TRight, TRightKey>>, Func<TRight, TLeftKey>,
        /// Func<TLeftKey, Optional<TLeft>, Optional<TRight>, TDestination>)`).
        ///
        /// When several right items map to one left key, the latest one is joined.
        fn full_join_with_key<TR: Clone + 'static, KR: Eq + std::hash::Hash + Clone + 'static, D: Clone + 'static>(
            &self,
            right: std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<TR, KR>>>,
            right_key_selector: impl Fn(&TR) -> K + 'static,
            result_selector: impl Fn(&K, Option<&T>, Option<&TR>) -> D + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<D, K>>> {
            crate::cache::internal::full_join(self.change_sets(), right, std::rc::Rc::new(right_key_selector), std::rc::Rc::new(result_selector))
        }
    };
}
pub(crate) use full_join_methods;
