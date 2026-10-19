//! `Group` (`Cache/ObservableCacheEx.Group.cs`).

macro_rules! group_methods {
    () => {
        /// The items grouped by the key `group_selector_key` selects: a change set of groups, each
        /// with the cache of its items; an update or a refresh evaluates the group of the item again
        /// (`Group(Func<TObject, TGroupKey>)`).
        fn group<G: Eq + std::hash::Hash + Clone + 'static>(
            &self,
            group_selector_key: impl Fn(&T) -> G + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::GroupChangeSet<T, K, G>>> {
            crate::cache::internal::group_on(self.change_sets(), std::rc::Rc::new(group_selector_key), None)
        }

        /// [`group`](Self::group) that evaluates the group of every item again when `regrouper`
        /// produces a value (`Group(Func<TObject, TGroupKey>, IObservable<Unit>)`).
        fn group_with<G: Eq + std::hash::Hash + Clone + 'static>(
            &self,
            group_selector_key: impl Fn(&T) -> G + 'static,
            regrouper: std::rc::Rc<dyn ferroui_system::IObservable<()>>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::GroupChangeSet<T, K, G>>> {
            crate::cache::internal::group_on(self.change_sets(), std::rc::Rc::new(group_selector_key), Some(regrouper))
        }

        /// The groups of the keys of `result_group_source` (a change set whose items are their own
        /// keys, as `distinct_values` gives), each with the items whose key `group_selector`
        /// selects: a group exists while its key does, even without items
        /// (`Group(Func<TObject, TGroupKey>, IObservable<IDistinctChangeSet<TGroupKey>>)`).
        fn group_with_result_groups<G: Eq + std::hash::Hash + Clone + 'static, C: crate::cache::IChangeSet<G, G>>(
            &self,
            group_selector: impl Fn(&T) -> G + 'static,
            result_group_source: std::rc::Rc<dyn ferroui_system::IObservable<C>>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::GroupChangeSet<T, K, G>>> {
            crate::cache::internal::specified_grouper(self.change_sets(), std::rc::Rc::new(group_selector), result_group_source)
        }

        /// The items grouped by the latest selector of `group_selector_key_observable`: nothing is
        /// grouped until the first selector, and each new one evaluates the group of every item
        /// again (`Group(IObservable<Func<TObject, TGroupKey>>, IObservable<Unit>? regrouper =
        /// null)`).
        fn group_dynamic<G: Eq + std::hash::Hash + Clone + 'static>(
            &self,
            group_selector_key_observable: std::rc::Rc<dyn ferroui_system::IObservable<std::rc::Rc<dyn Fn(&T) -> G>>>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::GroupChangeSet<T, K, G>>> {
            crate::cache::internal::group_on_dynamic(self.change_sets(), crate::cache::internal::adapt_group_selector(group_selector_key_observable), None)
        }

        /// [`group_dynamic`](Self::group_dynamic) that evaluates the group of every item again when
        /// `regrouper` produces a value (`Group(IObservable<Func<TObject, TGroupKey>>,
        /// IObservable<Unit>)`).
        fn group_dynamic_with<G: Eq + std::hash::Hash + Clone + 'static>(
            &self,
            group_selector_key_observable: std::rc::Rc<dyn ferroui_system::IObservable<std::rc::Rc<dyn Fn(&T) -> G>>>,
            regrouper: std::rc::Rc<dyn ferroui_system::IObservable<()>>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::GroupChangeSet<T, K, G>>> {
            crate::cache::internal::group_on_dynamic(self.change_sets(), crate::cache::internal::adapt_group_selector(group_selector_key_observable), Some(regrouper))
        }

        /// [`group_dynamic`](Self::group_dynamic) with selectors of the item and its key
        /// (`Group(IObservable<Func<TObject, TKey, TGroupKey>>, IObservable<Unit>? regrouper =
        /// null)`).
        fn group_dynamic_with_key<G: Eq + std::hash::Hash + Clone + 'static>(
            &self,
            group_selector_key_observable: std::rc::Rc<dyn ferroui_system::IObservable<std::rc::Rc<dyn Fn(&T, &K) -> G>>>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::GroupChangeSet<T, K, G>>> {
            crate::cache::internal::group_on_dynamic(self.change_sets(), group_selector_key_observable, None)
        }

        /// [`group_dynamic_with_key`](Self::group_dynamic_with_key) that evaluates the group of
        /// every item again when `regrouper` produces a value
        /// (`Group(IObservable<Func<TObject, TKey, TGroupKey>>, IObservable<Unit>)`).
        fn group_dynamic_with_key_with<G: Eq + std::hash::Hash + Clone + 'static>(
            &self,
            group_selector_key_observable: std::rc::Rc<dyn ferroui_system::IObservable<std::rc::Rc<dyn Fn(&T, &K) -> G>>>,
            regrouper: std::rc::Rc<dyn ferroui_system::IObservable<()>>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::GroupChangeSet<T, K, G>>> {
            crate::cache::internal::group_on_dynamic(self.change_sets(), group_selector_key_observable, Some(regrouper))
        }
    };
}
pub(crate) use group_methods;
