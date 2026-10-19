//! `SortAndBind` (`Cache/ObservableCacheEx.SortAndBind.cs`): the changes applied to a list at
//! their sorted positions, the sort being part of the binding.
//!
//! The target is any list of DynamicData ([`IList`](crate::list::IList)) that is a shared handle:
//! an [`ObservableCollectionExtended`](crate::binding::ObservableCollectionExtended) (whose
//! notifications are suspended around the changes, as upstream does), an
//! `ObservableCollection<T>`, or a shared `Rc<RefCell<Vec<T>>>` (a `List<T>`). The forms that
//! take an `IComparable<TObject>` item use [`Ord`].

macro_rules! sort_and_bind_methods {
    () => {
        /// Sorts the items by `comparer` into `target` (`SortAndBind(IList<TObject>,
        /// IComparer<TObject>)`), with the options of `DynamicDataOptions.SortAndBind`.
        fn sort_and_bind<L: crate::list::IList<T> + Clone + 'static>(
            &self,
            target: &L,
            comparer: std::rc::Rc<dyn Fn(&T, &T) -> std::cmp::Ordering>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>>
        where
            T: PartialEq,
        {
            crate::binding::sort_and_bind::sort_and_bind(self.change_sets(), comparer, crate::DynamicDataOptions::sort_and_bind(), target.clone())
        }

        /// [`sort_and_bind`](Self::sort_and_bind) with `options` (`SortAndBind(IList<TObject>,
        /// IComparer<TObject>, SortAndBindOptions)`).
        fn sort_and_bind_with(
            &self,
            target: &(impl crate::list::IList<T> + Clone + 'static),
            comparer: std::rc::Rc<dyn Fn(&T, &T) -> std::cmp::Ordering>,
            options: crate::binding::SortAndBindOptions,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>>
        where
            T: PartialEq,
        {
            crate::binding::sort_and_bind::sort_and_bind(self.change_sets(), comparer, options, target.clone())
        }

        /// Sorts the items in their natural order into `target` (`SortAndBind(IList<TObject>)`,
        /// `Comparer<TObject>.Default`).
        fn sort_and_bind_by_default(
            &self,
            target: &(impl crate::list::IList<T> + Clone + 'static),
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>>
        where
            T: Ord,
        {
            self.sort_and_bind_with(target, std::rc::Rc::new(|a: &T, b: &T| a.cmp(b)), crate::DynamicDataOptions::sort_and_bind())
        }

        /// [`sort_and_bind_by_default`](Self::sort_and_bind_by_default) with `options`
        /// (`SortAndBind(IList<TObject>, SortAndBindOptions)`).
        fn sort_and_bind_by_default_with(
            &self,
            target: &(impl crate::list::IList<T> + Clone + 'static),
            options: crate::binding::SortAndBindOptions,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>>
        where
            T: Ord,
        {
            self.sort_and_bind_with(target, std::rc::Rc::new(|a: &T, b: &T| a.cmp(b)), options)
        }

        /// Sorts the items into `target` by the latest comparer of `comparer_changed`, sorting
        /// again at each one (`SortAndBind(IList<TObject>, IObservable<IComparer<TObject>>)`).
        fn sort_and_bind_on_observable(
            &self,
            target: &(impl crate::list::IList<T> + Clone + 'static),
            comparer_changed: std::rc::Rc<dyn ferroui_system::IObservable<std::rc::Rc<dyn Fn(&T, &T) -> std::cmp::Ordering>>>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>>
        where
            T: PartialEq,
        {
            self.sort_and_bind_on_observable_with(target, comparer_changed, crate::DynamicDataOptions::sort_and_bind())
        }

        /// [`sort_and_bind_on_observable`](Self::sort_and_bind_on_observable) with `options`
        /// (`SortAndBind(IList<TObject>, IObservable<IComparer<TObject>>, SortAndBindOptions)`).
        fn sort_and_bind_on_observable_with(
            &self,
            target: &(impl crate::list::IList<T> + Clone + 'static),
            comparer_changed: std::rc::Rc<dyn ferroui_system::IObservable<std::rc::Rc<dyn Fn(&T, &T) -> std::cmp::Ordering>>>,
            options: crate::binding::SortAndBindOptions,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>>
        where
            T: PartialEq,
        {
            crate::binding::sort_and_bind::sort_and_bind_dynamic(self.change_sets(), comparer_changed, options, target.clone())
        }

        /// Sorts the items by `comparer` into a new collection, returned as a read-only view
        /// (`SortAndBind(out ReadOnlyObservableCollection<TObject>, IComparer<TObject>)`).
        fn sort_and_bind_read_only(
            &self,
            comparer: std::rc::Rc<dyn Fn(&T, &T) -> std::cmp::Ordering>,
        ) -> (std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>>, crate::binding::ReadOnlyObservableCollection<T>)
        where
            T: PartialEq,
        {
            self.sort_and_bind_read_only_with(comparer, crate::DynamicDataOptions::sort_and_bind())
        }

        /// [`sort_and_bind_read_only`](Self::sort_and_bind_read_only) with `options`, whose
        /// initial capacity sizes the collection (`SortAndBind(out
        /// ReadOnlyObservableCollection<TObject>, IComparer<TObject>, SortAndBindOptions)`).
        fn sort_and_bind_read_only_with(
            &self,
            comparer: std::rc::Rc<dyn Fn(&T, &T) -> std::cmp::Ordering>,
            options: crate::binding::SortAndBindOptions,
        ) -> (std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>>, crate::binding::ReadOnlyObservableCollection<T>)
        where
            T: PartialEq,
        {
            let (target, read_only) = crate::cache::internal::bind::new_read_only_target::<T>(options.initial_capacity);
            (crate::binding::sort_and_bind::sort_and_bind(self.change_sets(), comparer, options, target), read_only)
        }

        /// Sorts the items in their natural order into a new collection, returned as a read-only
        /// view (`SortAndBind(out ReadOnlyObservableCollection<TObject>)`).
        fn sort_and_bind_read_only_by_default(
            &self,
        ) -> (std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>>, crate::binding::ReadOnlyObservableCollection<T>)
        where
            T: Ord,
        {
            self.sort_and_bind_read_only_with(std::rc::Rc::new(|a: &T, b: &T| a.cmp(b)), crate::DynamicDataOptions::sort_and_bind())
        }

        /// [`sort_and_bind_read_only_by_default`](Self::sort_and_bind_read_only_by_default) with
        /// `options` (`SortAndBind(out ReadOnlyObservableCollection<TObject>, SortAndBindOptions)`).
        fn sort_and_bind_read_only_by_default_with(
            &self,
            options: crate::binding::SortAndBindOptions,
        ) -> (std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>>, crate::binding::ReadOnlyObservableCollection<T>)
        where
            T: Ord,
        {
            self.sort_and_bind_read_only_with(std::rc::Rc::new(|a: &T, b: &T| a.cmp(b)), options)
        }

        /// Sorts the items by the latest comparer of `comparer_changed` into a new collection,
        /// returned as a read-only view (`SortAndBind(out ReadOnlyObservableCollection<TObject>,
        /// IObservable<IComparer<TObject>>)`).
        fn sort_and_bind_read_only_on_observable(
            &self,
            comparer_changed: std::rc::Rc<dyn ferroui_system::IObservable<std::rc::Rc<dyn Fn(&T, &T) -> std::cmp::Ordering>>>,
        ) -> (std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>>, crate::binding::ReadOnlyObservableCollection<T>)
        where
            T: PartialEq,
        {
            self.sort_and_bind_read_only_on_observable_with(comparer_changed, crate::DynamicDataOptions::sort_and_bind())
        }

        /// [`sort_and_bind_read_only_on_observable`](Self::sort_and_bind_read_only_on_observable)
        /// with `options` (`SortAndBind(out ReadOnlyObservableCollection<TObject>,
        /// IObservable<IComparer<TObject>>, SortAndBindOptions)`).
        fn sort_and_bind_read_only_on_observable_with(
            &self,
            comparer_changed: std::rc::Rc<dyn ferroui_system::IObservable<std::rc::Rc<dyn Fn(&T, &T) -> std::cmp::Ordering>>>,
            options: crate::binding::SortAndBindOptions,
        ) -> (std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>>, crate::binding::ReadOnlyObservableCollection<T>)
        where
            T: PartialEq,
        {
            let (target, read_only) = crate::cache::internal::bind::new_read_only_target::<T>(options.initial_capacity);
            (crate::binding::sort_and_bind::sort_and_bind_dynamic(self.change_sets(), comparer_changed, options, target), read_only)
        }

        /// Binds the results of `SortAndPage` or `SortAndVirtualize` to `target_list`, sorted by
        /// the comparer of their context, with the options of their context
        /// (`Bind(IList<TObject>)` of `IChangeSet<TObject, TKey, PageContext<TObject>>` and of
        /// `IChangeSet<TObject, TKey, VirtualContext<TObject>>`).
        ///
        /// # Panics
        /// When the source is not a result of `SortAndPage` or `SortAndVirtualize`.
        fn bind_to_list(
            &self,
            target_list: &(impl crate::list::IList<T> + Clone + 'static),
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>>
        where
            T: PartialEq,
            Self: Sized + 'static,
        {
            crate::cache::internal::bind::expect_bind_context_stream(self, target_list.clone(), None)
        }

        /// [`bind_to_list`](Self::bind_to_list) with `options` (`Bind(IList<TObject>,
        /// SortAndBindOptions)` of the paged and virtualised change sets).
        ///
        /// # Panics
        /// When the source is not a result of `SortAndPage` or `SortAndVirtualize`.
        fn bind_to_list_with(
            &self,
            target_list: &(impl crate::list::IList<T> + Clone + 'static),
            options: crate::binding::SortAndBindOptions,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>>
        where
            T: PartialEq,
            Self: Sized + 'static,
        {
            crate::cache::internal::bind::expect_bind_context_stream(self, target_list.clone(), Some(options))
        }

        /// Binds the results of `SortAndPage` or `SortAndVirtualize` to a new collection, returned
        /// as a read-only view (`Bind(out ReadOnlyObservableCollection<TObject>,
        /// SortAndBindOptions)` of the paged and virtualised change sets). Their form without
        /// options is [`bind_read_only`](Self::bind_read_only).
        ///
        /// # Panics
        /// When the source is not a result of `SortAndPage` or `SortAndVirtualize`.
        fn bind_read_only_with_sort_and_bind_options(
            &self,
            options: crate::binding::SortAndBindOptions,
        ) -> (std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>>, crate::binding::ReadOnlyObservableCollection<T>)
        where
            T: PartialEq,
            Self: Sized + 'static,
        {
            let (target, read_only) = crate::cache::internal::bind::new_read_only_target::<T>(None);
            (crate::cache::internal::bind::expect_bind_context_stream(self, target, Some(options)), read_only)
        }
    };
}
pub(crate) use sort_and_bind_methods;
