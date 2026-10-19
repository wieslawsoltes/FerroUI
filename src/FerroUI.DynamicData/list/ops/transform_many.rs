//! `TransformMany` (`List/ObservableListEx.TransformMany.cs`).
//!
//! The items of each item of the source flattened into one list. A replacement (or a refresh,
//! which takes the items again) removes the items the item no longer has and adds the new ones,
//! by the equality of the items or by `equality_comparer` (`_with_comparer`). The overloads for
//! `ObservableCollection<T>` and `ReadOnlyObservableCollection<T>` children are one method,
//! `transform_many_observable_collection`, generic over the collection (`ObservableCollectionEx`,
//! which `ObservableCollectionExtended<T>` has too).

macro_rules! transform_many_methods {
    () => {
        /// The items that `many_selector` gives for each item, as one list
        /// (`TransformMany(Func<TSource, IEnumerable<TDestination>>, IEqualityComparer<TDestination>? equalityComparer = null)`).
        fn transform_many<U: Clone + PartialEq + 'static>(
            &self,
            many_selector: impl Fn(&T) -> Vec<U> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<U>>> {
            self.transform_many_with_comparer(many_selector, |a: &U, b: &U| a == b)
        }

        /// [`transform_many`](Self::transform_many) whose items are compared by
        /// `equality_comparer` (`TransformMany(Func<TSource, IEnumerable<TDestination>>, IEqualityComparer<TDestination>)`).
        fn transform_many_with_comparer<U: Clone + PartialEq + 'static>(
            &self,
            many_selector: impl Fn(&T) -> Vec<U> + 'static,
            equality_comparer: impl Fn(&U, &U) -> bool + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<U>>> {
            crate::list::internal::transform_many(self.change_sets(), std::rc::Rc::new(many_selector), std::rc::Rc::new(equality_comparer))
        }

        /// The items of the list that `many_selector` gives for each item, as one list, followed
        /// as the lists change
        /// (`TransformMany(Func<TSource, IObservableList<TDestination>>, IEqualityComparer<TDestination>? equalityComparer = null)`).
        fn transform_many_observable_list<U: Clone + PartialEq + 'static, C: crate::list::IObservableList<U> + 'static>(
            &self,
            many_selector: impl Fn(&T) -> C + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<U>>> {
            self.transform_many_observable_list_with_comparer(many_selector, |a: &U, b: &U| a == b)
        }

        /// [`transform_many_observable_list`](Self::transform_many_observable_list) whose items are
        /// compared by `equality_comparer`
        /// (`TransformMany(Func<TSource, IObservableList<TDestination>>, IEqualityComparer<TDestination>)`).
        fn transform_many_observable_list_with_comparer<U: Clone + PartialEq + 'static, C: crate::list::IObservableList<U> + 'static>(
            &self,
            many_selector: impl Fn(&T) -> C + 'static,
            equality_comparer: impl Fn(&U, &U) -> bool + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<U>>> {
            let many_selector: std::rc::Rc<dyn Fn(&T) -> C> = std::rc::Rc::new(many_selector);
            let items_selector = many_selector.clone();
            // `ManySelectorFunc`: the items of the list when they are read.
            let items: std::rc::Rc<dyn Fn(&T) -> Vec<U>> = std::rc::Rc::new(move |item: &T| crate::list::IObservableList::items(&items_selector(item)));
            crate::list::internal::transform_many_with_child_changes(
                self.change_sets(),
                items,
                std::rc::Rc::new(equality_comparer),
                crate::list::internal::observable_list_child_changes(many_selector),
            )
        }

        /// The items of the observable collection that `many_selector` gives for each item, as one
        /// list, followed as the collections change
        /// (`TransformMany(Func<TSource, ObservableCollection<TDestination>>, IEqualityComparer<TDestination>? equalityComparer = null)`,
        /// `TransformMany(Func<TSource, ReadOnlyObservableCollection<TDestination>>, IEqualityComparer<TDestination>? equalityComparer = null)`).
        fn transform_many_observable_collection<U: Clone + PartialEq + 'static, C: crate::binding::ObservableCollectionEx<U> + 'static>(
            &self,
            many_selector: impl Fn(&T) -> C + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<U>>> {
            self.transform_many_observable_collection_with_comparer(many_selector, |a: &U, b: &U| a == b)
        }

        /// [`transform_many_observable_collection`](Self::transform_many_observable_collection)
        /// whose items are compared by `equality_comparer`
        /// (`TransformMany(Func<TSource, ObservableCollection<TDestination>>, IEqualityComparer<TDestination>)`,
        /// `TransformMany(Func<TSource, ReadOnlyObservableCollection<TDestination>>, IEqualityComparer<TDestination>)`).
        fn transform_many_observable_collection_with_comparer<U: Clone + PartialEq + 'static, C: crate::binding::ObservableCollectionEx<U> + 'static>(
            &self,
            many_selector: impl Fn(&T) -> C + 'static,
            equality_comparer: impl Fn(&U, &U) -> bool + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<U>>> {
            let many_selector: std::rc::Rc<dyn Fn(&T) -> C> = std::rc::Rc::new(many_selector);
            let items_selector = many_selector.clone();
            // The collection is the enumerable of its items, read when they are needed.
            let items: std::rc::Rc<dyn Fn(&T) -> Vec<U>> =
                std::rc::Rc::new(move |item: &T| crate::binding::ObservableCollectionEx::notifying_collection(&items_selector(item)).to_vec());
            crate::list::internal::transform_many_with_child_changes(
                self.change_sets(),
                items,
                std::rc::Rc::new(equality_comparer),
                crate::list::internal::observable_collection_child_changes(many_selector),
            )
        }
    };
}
pub(crate) use transform_many_methods;
