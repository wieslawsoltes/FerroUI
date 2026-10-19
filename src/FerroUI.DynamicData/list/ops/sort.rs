//! `Sort` of lists (`List/ObservableListEx.Sort.cs`).

macro_rules! sort_methods {
    () => {
        /// The items sorted by `comparer` (`Sort(IComparer<T>)`, with no options, no resort, and
        /// a reset threshold of 50).
        fn sort(&self, comparer: std::rc::Rc<dyn Fn(&T, &T) -> std::cmp::Ordering>) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<T>>> {
            crate::list::internal::sort(self.change_sets(), Some(comparer), crate::list::SortOptions::None, None, None, 50)
        }

        /// [`sort`](Self::sort) with every option (`Sort(IComparer<T>, SortOptions options =
        /// None, IObservable<Unit>? resort = null, IObservable<IComparer<T>>? comparerChanged =
        /// null, int resetThreshold = 50)`): `resort` sorts the items again (after they changed
        /// in place), `comparer_changed` replaces the comparer, and a change set of more than
        /// `reset_threshold` changes sorts every item again.
        fn sort_with(
            &self,
            comparer: std::rc::Rc<dyn Fn(&T, &T) -> std::cmp::Ordering>,
            options: crate::list::SortOptions,
            resort: Option<std::rc::Rc<dyn ferroui_system::IObservable<()>>>,
            comparer_changed: Option<std::rc::Rc<dyn ferroui_system::IObservable<std::rc::Rc<dyn Fn(&T, &T) -> std::cmp::Ordering>>>>,
            reset_threshold: usize,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<T>>> {
            crate::list::internal::sort(self.change_sets(), Some(comparer), options, resort, comparer_changed, reset_threshold)
        }

        /// The items sorted by the latest comparer of `comparer_changed`
        /// (`Sort(IObservable<IComparer<T>>)`, with no options, no resort, and a reset threshold
        /// of 50). Until the first comparer, the items keep the order in which they come.
        fn sort_on_observable(
            &self,
            comparer_changed: std::rc::Rc<dyn ferroui_system::IObservable<std::rc::Rc<dyn Fn(&T, &T) -> std::cmp::Ordering>>>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<T>>> {
            crate::list::internal::sort(self.change_sets(), None, crate::list::SortOptions::None, None, Some(comparer_changed), 50)
        }

        /// [`sort_on_observable`](Self::sort_on_observable) with every option
        /// (`Sort(IObservable<IComparer<T>>, SortOptions options = None, IObservable<Unit>? resort =
        /// null, int resetThreshold = 50)`).
        fn sort_on_observable_with(
            &self,
            comparer_changed: std::rc::Rc<dyn ferroui_system::IObservable<std::rc::Rc<dyn Fn(&T, &T) -> std::cmp::Ordering>>>,
            options: crate::list::SortOptions,
            resort: Option<std::rc::Rc<dyn ferroui_system::IObservable<()>>>,
            reset_threshold: usize,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<T>>> {
            crate::list::internal::sort(self.change_sets(), None, options, resort, Some(comparer_changed), reset_threshold)
        }
    };
}
pub(crate) use sort_methods;
