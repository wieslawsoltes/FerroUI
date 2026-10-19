//! `Bind` (`Cache/ObservableCacheEx.Bind.cs`): the change sets applied to an observable collection.
//!
//! The forms of upstream for `IObservable<IChangeSet<TObject, TKey>>` and for
//! `IObservable<ISortedChangeSet<TObject, TKey>>` are the same methods here: the forms without an
//! adaptor look at the source when they run, and bind a sorted source (`Sort`, `Page`,
//! `Virtualise`) at the positions of its change sets (`crate::cache::internal::bind`). The forms of
//! `BindingList<T>` (Windows Forms) are not ported.

macro_rules! bind_methods {
    () => {
        /// Applies the change sets to `destination` with the default options
        /// (`Bind(IObservableCollection<TObject>, int refreshThreshold = 25)`, and
        /// `Bind(IObservableCollection<TObject>)` of a sorted source).
        fn bind<C: crate::binding::IObservableCollection<T> + Clone + 'static>(
            &self,
            destination: &C,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>>
        where
            T: PartialEq,
            Self: Sized + 'static,
        {
            crate::cache::internal::bind::bind_with_options(self, self.change_sets(), std::rc::Rc::new(destination.clone()), crate::DynamicDataOptions::binding())
        }

        /// [`bind`](Self::bind) with a threshold of changes above which the collection is loaded
        /// again with a reset (`Bind(IObservableCollection<TObject>, int refreshThreshold)`); it
        /// binds as a plain source, as upstream's only form with a threshold does.
        fn bind_with_threshold<C: crate::binding::IObservableCollection<T> + Clone + 'static>(
            &self,
            destination: &C,
            refresh_threshold: usize,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>>
        where
            T: PartialEq,
        {
            let defaults = crate::DynamicDataOptions::binding();
            let options = if refresh_threshold == crate::binding::BindingOptions::DEFAULT_RESET_THRESHOLD { defaults } else { defaults.with_reset_threshold(refresh_threshold) };
            crate::cache::internal::bind::bind_with_adaptor(
                self.change_sets(),
                std::rc::Rc::new(destination.clone()),
                std::rc::Rc::new(crate::binding::ObservableCollectionAdaptor::<T, K>::with_options(options)),
            )
        }

        /// [`bind`](Self::bind) with `options` (`Bind(IObservableCollection<TObject>,
        /// BindingOptions)`).
        fn bind_with_options<C: crate::binding::IObservableCollection<T> + Clone + 'static>(
            &self,
            destination: &C,
            options: crate::binding::BindingOptions,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>>
        where
            T: PartialEq,
            Self: Sized + 'static,
        {
            crate::cache::internal::bind::bind_with_options(self, self.change_sets(), std::rc::Rc::new(destination.clone()), options)
        }

        /// Applies the change sets to `destination` through `updater`
        /// (`Bind(IObservableCollection<TObject>, IObservableCollectionAdaptor<TObject, TKey>)`).
        fn bind_with_adaptor<C: crate::binding::IObservableCollection<T> + Clone + 'static>(
            &self,
            destination: &C,
            updater: std::rc::Rc<dyn crate::binding::IObservableCollectionAdaptor<T, K>>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>>
        where
            T: PartialEq,
        {
            crate::cache::internal::bind::bind_with_adaptor(self.change_sets(), std::rc::Rc::new(destination.clone()), updater)
        }

        /// Applies the sorted change sets to `destination` through `updater`
        /// (`Bind(IObservableCollection<TObject>, ISortedObservableCollectionAdaptor<TObject,
        /// TKey>)`).
        ///
        /// # Panics
        /// When the source is not a sequence of sorted change sets.
        fn bind_with_sorted_adaptor<C: crate::binding::IObservableCollection<T> + Clone + 'static>(
            &self,
            destination: &C,
            updater: std::rc::Rc<dyn crate::binding::ISortedObservableCollectionAdaptor<T, K>>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>>
        where
            T: PartialEq,
            Self: Sized + 'static,
        {
            crate::cache::internal::bind::bind_sorted_with_adaptor(crate::cache::internal::bind::expect_sorted_stream(self), std::rc::Rc::new(destination.clone()), updater)
        }

        /// Applies the change sets to a new collection, returned as a read-only view
        /// (`Bind(out ReadOnlyObservableCollection<TObject>, int resetThreshold = 25, bool
        /// useReplaceForUpdates = true, adaptor = null)`, and its sorted form).
        ///
        /// The results of `SortAndPage` and `SortAndVirtualize` are bound sorted by the comparer of
        /// their context (`Bind(out ReadOnlyObservableCollection<TObject>)` of
        /// `IChangeSet<TObject, TKey, PageContext<TObject>>`, `BindPaged`).
        fn bind_read_only(
            &self,
        ) -> (std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>>, crate::binding::ReadOnlyObservableCollection<T>)
        where
            T: PartialEq,
            Self: Sized + 'static,
        {
            let (target, read_only) = crate::cache::internal::bind::new_read_only_target::<T>(None);
            if let Some(bound) = crate::cache::internal::bind::bind_context_stream(self, target, None) {
                return (bound, read_only);
            }
            self.bind_read_only_with(crate::binding::BindingOptions::DEFAULT_RESET_THRESHOLD, crate::binding::BindingOptions::DEFAULT_USE_REPLACE_FOR_UPDATES)
        }

        /// [`bind_read_only`](Self::bind_read_only) with a threshold and the choice of
        /// replacements (`Bind(out ReadOnlyObservableCollection<TObject>, int, bool)`).
        fn bind_read_only_with(
            &self,
            reset_threshold: usize,
            use_replace_for_updates: bool,
        ) -> (std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>>, crate::binding::ReadOnlyObservableCollection<T>)
        where
            T: PartialEq,
            Self: Sized + 'static,
        {
            self.bind_read_only_with_options(crate::cache::internal::bind::binding_options(reset_threshold, use_replace_for_updates))
        }

        /// [`bind_read_only`](Self::bind_read_only) with `options`
        /// (`Bind(out ReadOnlyObservableCollection<TObject>, BindingOptions)`).
        fn bind_read_only_with_options(
            &self,
            options: crate::binding::BindingOptions,
        ) -> (std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>>, crate::binding::ReadOnlyObservableCollection<T>)
        where
            T: PartialEq,
            Self: Sized + 'static,
        {
            let (target, read_only) = crate::cache::internal::bind::new_read_only_target::<T>(None);
            (crate::cache::internal::bind::bind_with_options(self, self.change_sets(), std::rc::Rc::new(target), options), read_only)
        }

        /// [`bind_read_only`](Self::bind_read_only) through `adaptor`
        /// (`Bind(out ReadOnlyObservableCollection<TObject>, .., IObservableCollectionAdaptor<TObject,
        /// TKey>)`).
        fn bind_read_only_with_adaptor(
            &self,
            adaptor: std::rc::Rc<dyn crate::binding::IObservableCollectionAdaptor<T, K>>,
        ) -> (std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>>, crate::binding::ReadOnlyObservableCollection<T>)
        where
            T: PartialEq,
        {
            let (target, read_only) = crate::cache::internal::bind::new_read_only_target::<T>(None);
            (crate::cache::internal::bind::bind_with_adaptor(self.change_sets(), std::rc::Rc::new(target), adaptor), read_only)
        }

        /// [`bind_read_only`](Self::bind_read_only) of a sorted source through `adaptor`
        /// (`Bind(out ReadOnlyObservableCollection<TObject>, .., ISortedObservableCollectionAdaptor<TObject,
        /// TKey>)`).
        ///
        /// # Panics
        /// When the source is not a sequence of sorted change sets.
        fn bind_read_only_with_sorted_adaptor(
            &self,
            adaptor: std::rc::Rc<dyn crate::binding::ISortedObservableCollectionAdaptor<T, K>>,
        ) -> (std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>>, crate::binding::ReadOnlyObservableCollection<T>)
        where
            T: PartialEq,
            Self: Sized + 'static,
        {
            let (target, read_only) = crate::cache::internal::bind::new_read_only_target::<T>(None);
            (crate::cache::internal::bind::bind_sorted_with_adaptor(crate::cache::internal::bind::expect_sorted_stream(self), std::rc::Rc::new(target), adaptor), read_only)
        }
    };
}
pub(crate) use bind_methods;
