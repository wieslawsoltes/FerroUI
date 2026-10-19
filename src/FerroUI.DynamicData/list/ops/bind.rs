//! `Bind` of lists (`List/ObservableListEx.Bind.cs`): the change sets applied to an observable
//! collection. The form of `BindingList<T>` (Windows Forms) is not ported.

macro_rules! bind_methods {
    () => {
        /// Applies the change sets to `target_collection` with the default options
        /// (`Bind(IObservableCollection<T>, int resetThreshold = 25)`).
        fn bind<C: crate::binding::IObservableCollection<T> + Clone + 'static>(
            &self,
            target_collection: &C,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<T>>> {
            self.bind_with_options(target_collection, crate::DynamicDataOptions::binding())
        }

        /// [`bind`](Self::bind) with a threshold of changes above which the collection is loaded
        /// again with a reset (`Bind(IObservableCollection<T>, int resetThreshold)`).
        fn bind_with_threshold<C: crate::binding::IObservableCollection<T> + Clone + 'static>(
            &self,
            target_collection: &C,
            reset_threshold: usize,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<T>>> {
            self.bind_with_options(target_collection, crate::list::internal::bind_options(reset_threshold))
        }

        /// [`bind`](Self::bind) with `options` (`Bind(IObservableCollection<T>, BindingOptions)`).
        fn bind_with_options<C: crate::binding::IObservableCollection<T> + Clone + 'static>(
            &self,
            target_collection: &C,
            options: crate::binding::BindingOptions,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<T>>> {
            let adaptor = crate::binding::ListObservableCollectionAdaptor::with_options(std::rc::Rc::new(target_collection.clone()), options);
            self.adapt(std::rc::Rc::new(adaptor))
        }

        /// Applies the change sets to a new collection, returned as a read-only view
        /// (`Bind(out ReadOnlyObservableCollection<T>, int resetThreshold = 25)`).
        fn bind_read_only(&self) -> (std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<T>>>, crate::binding::ReadOnlyObservableCollection<T>) {
            self.bind_read_only_with_options(crate::DynamicDataOptions::binding())
        }

        /// [`bind_read_only`](Self::bind_read_only) with a threshold
        /// (`Bind(out ReadOnlyObservableCollection<T>, int resetThreshold)`).
        fn bind_read_only_with_threshold(
            &self,
            reset_threshold: usize,
        ) -> (std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<T>>>, crate::binding::ReadOnlyObservableCollection<T>) {
            self.bind_read_only_with_options(crate::list::internal::bind_options(reset_threshold))
        }

        /// [`bind_read_only`](Self::bind_read_only) with `options`
        /// (`Bind(out ReadOnlyObservableCollection<T>, BindingOptions)`).
        fn bind_read_only_with_options(
            &self,
            options: crate::binding::BindingOptions,
        ) -> (std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<T>>>, crate::binding::ReadOnlyObservableCollection<T>) {
            let target = crate::binding::ObservableCollectionExtended::<T>::new();
            let result = crate::binding::ReadOnlyObservableCollection::from(&target);
            let adaptor = crate::binding::ListObservableCollectionAdaptor::with_options(std::rc::Rc::new(target), options);
            (self.adapt(std::rc::Rc::new(adaptor)), result)
        }
    };
}
pub(crate) use bind_methods;
