//! `GroupOn` (`List/ObservableListEx.GroupOn.cs`).

macro_rules! group_on_methods {
    () => {
        /// The items grouped by the key `group_selector` selects: a change set of groups, each with
        /// the list of its items (`GroupOn(Func<TObject, TGroup>, IObservable<Unit>? regrouper =
        /// null)`).
        fn group_on<G: Eq + std::hash::Hash + Clone + 'static>(
            &self,
            group_selector: impl Fn(&T) -> G + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<crate::list::Group<T, G>>>> {
            crate::list::internal::group_on(self.change_sets(), std::rc::Rc::new(group_selector), None)
        }

        /// [`group_on`](Self::group_on) that evaluates the group of every item again when
        /// `regrouper` produces a value (`GroupOn(Func<TObject, TGroup>, IObservable<Unit>)`).
        fn group_on_with<G: Eq + std::hash::Hash + Clone + 'static>(
            &self,
            group_selector: impl Fn(&T) -> G + 'static,
            regrouper: std::rc::Rc<dyn ferroui_system::IObservable<()>>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<crate::list::Group<T, G>>>> {
            crate::list::internal::group_on(self.change_sets(), std::rc::Rc::new(group_selector), Some(regrouper))
        }
    };
}
pub(crate) use group_on_methods;
