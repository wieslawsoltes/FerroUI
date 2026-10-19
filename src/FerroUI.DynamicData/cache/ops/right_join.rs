//! `RightJoin` (`Cache/ObservableCacheEx.RightJoin.cs`).

macro_rules! right_join_methods {
    () => {
        /// Each right item with the left item of the key `right_key_selector` maps it to, if any
        /// (upstream's `Optional<TLeft>` is an `Option`), combined by `result_selector`, keyed by
        /// the right key; a result exists while the right item does (SQL `RIGHT JOIN`). The
        /// selector does not take the key (`RightJoin(IObservable<IChangeSet<TRight, TRightKey>>,
        /// Func<TRight, TLeftKey>, Func<Optional<TLeft>, TRight, TDestination>)`).
        fn right_join<TR: Clone + 'static, KR: Eq + std::hash::Hash + Clone + 'static, D: Clone + 'static>(
            &self,
            right: std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<TR, KR>>>,
            right_key_selector: impl Fn(&TR) -> K + 'static,
            result_selector: impl Fn(Option<&T>, &TR) -> D + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<D, KR>>> {
            self.right_join_with_key(right, right_key_selector, move |_: &KR, left: Option<&T>, right: &TR| result_selector(left, right))
        }

        /// [`right_join`](Self::right_join) whose selector also takes the right key
        /// (`RightJoin(IObservable<IChangeSet<TRight, TRightKey>>, Func<TRight, TLeftKey>,
        /// Func<TRightKey, Optional<TLeft>, TRight, TDestination>)`).
        ///
        /// The left items present at the subscription are joined as the right items arrive, in one
        /// change set.
        fn right_join_with_key<TR: Clone + 'static, KR: Eq + std::hash::Hash + Clone + 'static, D: Clone + 'static>(
            &self,
            right: std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<TR, KR>>>,
            right_key_selector: impl Fn(&TR) -> K + 'static,
            result_selector: impl Fn(&KR, Option<&T>, &TR) -> D + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<D, KR>>> {
            crate::cache::internal::right_join(self.change_sets(), right, std::rc::Rc::new(right_key_selector), std::rc::Rc::new(result_selector))
        }
    };
}
pub(crate) use right_join_methods;
