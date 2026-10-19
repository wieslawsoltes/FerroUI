//! `Xor` (`List/ObservableListEx.Xor.cs`): a logical Xor between lists. The forms on collections and observable
//! lists of sources are [`ListSourcesCombineEx`](crate::list::ListSourcesCombineEx) and
//! [`ListSourceListCombineEx`](crate::list::ListSourceListCombineEx).

macro_rules! xor_methods {
    () => {
        /// The sequence combined with `others`: the items that are in exactly one of them
        /// (`Xor(IObservable<IChangeSet<T>>, params IObservable<IChangeSet<T>>[])`).
        /// The sequences are subscribed in order when the result is subscribed; a sequence given
        /// twice is combined once (`Union`). An item is in a list as many times as it was added,
        /// and counts by equality and hash.
        ///
        /// # Panics
        /// When `others` is empty
        /// (`ArgumentException`: "Must be at least one item to combine with").
        fn xor(
            &self,
            others: Vec<std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<T>>>>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<T>>>
        where
            T: Eq + std::hash::Hash,
        {
            assert!(!others.is_empty(), "Must be at least one item to combine with");
            crate::list::ops::combine::combine_with_others(self.change_sets(), crate::cache::internal::CombineOperator::Xor, others)
        }
    };
}
pub(crate) use xor_methods;
