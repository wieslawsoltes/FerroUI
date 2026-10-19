//! `SortAndVirtualize`, `SortAndPage`, `Virtualise`, `Page` and `Top`
//! (`Cache/ObservableCacheEx.VirtualiseAndPage.cs`).
//!
//! `virtualise_and_page_methods!` holds the operators of every cache ([`ObservableCacheEx`]:
//! `sort_and_virtualize*`, `sort_and_page*`, `top`); `virtualise_and_page_sorted_methods!` holds
//! those of sorted change sets ([`SortedObservableCacheEx`]: `virtualise`, `page`, `top_sorted`),
//! which are obsolete upstream.
//!
//! The forms with an observable comparer are `*_dynamic*`, as for `sort_dynamic`; the forms with
//! the options are `*_with`.
//!
//! [`ObservableCacheEx`]: crate::cache::ObservableCacheEx
//! [`SortedObservableCacheEx`]: crate::cache::SortedObservableCacheEx

macro_rules! virtualise_and_page_methods {
    () => {
        /// The window of the items, sorted by `comparer`, that the latest request of
        /// `virtual_requests` asks for; the change sets hold the changes of the window, with the
        /// window, the comparer and the options as their context (`SortAndVirtualize(IComparer,
        /// IObservable<IVirtualRequest>)`).
        fn sort_and_virtualize(
            &self,
            comparer: std::rc::Rc<dyn Fn(&T, &T) -> std::cmp::Ordering>,
            virtual_requests: std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::VirtualRequest>>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSetWithContext<T, K, crate::cache::VirtualContext<T>>>> {
            self.sort_and_virtualize_with(comparer, virtual_requests, crate::cache::SortAndVirtualizeOptions::default())
        }

        /// [`sort_and_virtualize`](Self::sort_and_virtualize) sorted by the latest comparer of
        /// `comparer_changed` (`SortAndVirtualize(IObservable<IComparer>,
        /// IObservable<IVirtualRequest>)`).
        fn sort_and_virtualize_dynamic(
            &self,
            comparer_changed: std::rc::Rc<dyn ferroui_system::IObservable<std::rc::Rc<dyn Fn(&T, &T) -> std::cmp::Ordering>>>,
            virtual_requests: std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::VirtualRequest>>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSetWithContext<T, K, crate::cache::VirtualContext<T>>>> {
            self.sort_and_virtualize_dynamic_with(comparer_changed, virtual_requests, crate::cache::SortAndVirtualizeOptions::default())
        }

        /// [`sort_and_virtualize`](Self::sort_and_virtualize) with `options`
        /// (`SortAndVirtualize(IComparer, IObservable<IVirtualRequest>, SortAndVirtualizeOptions)`).
        fn sort_and_virtualize_with(
            &self,
            comparer: std::rc::Rc<dyn Fn(&T, &T) -> std::cmp::Ordering>,
            virtual_requests: std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::VirtualRequest>>,
            options: crate::cache::SortAndVirtualizeOptions,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSetWithContext<T, K, crate::cache::VirtualContext<T>>>> {
            crate::cache::internal::sort_and_virtualize(self.change_sets(), ferroui_reactive::Observable::return_(comparer), virtual_requests, options)
        }

        /// [`sort_and_virtualize_dynamic`](Self::sort_and_virtualize_dynamic) with `options`
        /// (`SortAndVirtualize(IObservable<IComparer>, IObservable<IVirtualRequest>,
        /// SortAndVirtualizeOptions)`).
        fn sort_and_virtualize_dynamic_with(
            &self,
            comparer_changed: std::rc::Rc<dyn ferroui_system::IObservable<std::rc::Rc<dyn Fn(&T, &T) -> std::cmp::Ordering>>>,
            virtual_requests: std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::VirtualRequest>>,
            options: crate::cache::SortAndVirtualizeOptions,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSetWithContext<T, K, crate::cache::VirtualContext<T>>>> {
            crate::cache::internal::sort_and_virtualize(self.change_sets(), comparer_changed, virtual_requests, options)
        }

        /// The first `size` items, sorted by `comparer` (`Top(IComparer<TObject>, int size)`).
        ///
        /// # Panics
        /// When `size` is 0 (`ArgumentOutOfRangeException`).
        fn top(
            &self,
            comparer: std::rc::Rc<dyn Fn(&T, &T) -> std::cmp::Ordering>,
            size: usize,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSetWithContext<T, K, crate::cache::VirtualContext<T>>>> {
            assert!(size > 0, "size must be greater than zero");
            self.sort_and_virtualize(comparer, ferroui_reactive::Observable::return_(crate::cache::VirtualRequest::new(0, size)))
        }

        /// The page of the items, sorted by `comparer`, that the latest request of
        /// `page_requests` asks for; the change sets hold the changes of the page, with the page,
        /// the comparer and the options as their context (`SortAndPage(IComparer,
        /// IObservable<IPageRequest>)`).
        fn sort_and_page(
            &self,
            comparer: std::rc::Rc<dyn Fn(&T, &T) -> std::cmp::Ordering>,
            page_requests: std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::PageRequest>>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSetWithContext<T, K, crate::cache::PageContext<T>>>> {
            self.sort_and_page_with(comparer, page_requests, crate::cache::SortAndPageOptions::default())
        }

        /// [`sort_and_page`](Self::sort_and_page) sorted by the latest comparer of
        /// `comparer_changed` (`SortAndPage(IObservable<IComparer>, IObservable<IPageRequest>)`).
        fn sort_and_page_dynamic(
            &self,
            comparer_changed: std::rc::Rc<dyn ferroui_system::IObservable<std::rc::Rc<dyn Fn(&T, &T) -> std::cmp::Ordering>>>,
            page_requests: std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::PageRequest>>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSetWithContext<T, K, crate::cache::PageContext<T>>>> {
            self.sort_and_page_dynamic_with(comparer_changed, page_requests, crate::cache::SortAndPageOptions::default())
        }

        /// [`sort_and_page`](Self::sort_and_page) with `options` (`SortAndPage(IComparer,
        /// IObservable<IPageRequest>, SortAndPageOptions)`).
        fn sort_and_page_with(
            &self,
            comparer: std::rc::Rc<dyn Fn(&T, &T) -> std::cmp::Ordering>,
            page_requests: std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::PageRequest>>,
            options: crate::cache::SortAndPageOptions,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSetWithContext<T, K, crate::cache::PageContext<T>>>> {
            crate::cache::internal::sort_and_page(self.change_sets(), ferroui_reactive::Observable::return_(comparer), page_requests, options)
        }

        /// [`sort_and_page_dynamic`](Self::sort_and_page_dynamic) with `options`
        /// (`SortAndPage(IObservable<IComparer>, IObservable<IPageRequest>, SortAndPageOptions)`).
        fn sort_and_page_dynamic_with(
            &self,
            comparer_changed: std::rc::Rc<dyn ferroui_system::IObservable<std::rc::Rc<dyn Fn(&T, &T) -> std::cmp::Ordering>>>,
            page_requests: std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::PageRequest>>,
            options: crate::cache::SortAndPageOptions,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSetWithContext<T, K, crate::cache::PageContext<T>>>> {
            crate::cache::internal::sort_and_page(self.change_sets(), comparer_changed, page_requests, options)
        }
    };
}
pub(crate) use virtualise_and_page_methods;

macro_rules! virtualise_and_page_sorted_methods {
    () => {
        /// The window of the sorted items that the latest request of `virtual_requests` asks
        /// for, with the indexes of its changes (`Virtualise(IObservable<IVirtualRequest>)`,
        /// obsolete upstream).
        fn virtualise(
            &self,
            virtual_requests: std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::VirtualRequest>>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::VirtualChangeSet<T, K>>> {
            crate::cache::internal::virtualise(self.sorted_change_sets(), virtual_requests)
        }

        /// The first `size` sorted items (`Top(int size)` of sorted change sets, obsolete
        /// upstream).
        ///
        /// Deviation (DEVIATIONS.md, DynamicData): named `top_sorted`, as `top` is the operator of
        /// every cache, which a sorted stream also has.
        ///
        /// # Panics
        /// When `size` is 0 (`ArgumentOutOfRangeException`).
        fn top_sorted(&self, size: usize) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::VirtualChangeSet<T, K>>> {
            assert!(size > 0, "size must be greater than zero");
            crate::cache::internal::virtualise(self.sorted_change_sets(), ferroui_reactive::Observable::return_(crate::cache::VirtualRequest::new(0, size)))
        }

        /// The page of the sorted items that the latest request of `page_requests` asks for, with
        /// the indexes of its changes (`Page(IObservable<IPageRequest>)`, obsolete upstream).
        fn page(
            &self,
            page_requests: std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::PageRequest>>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::PagedChangeSet<T, K>>> {
            crate::cache::internal::page(self.sorted_change_sets(), page_requests)
        }
    };
}
pub(crate) use virtualise_and_page_sorted_methods;
