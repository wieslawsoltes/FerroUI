//! `TransformAsync` (`List/ObservableListEx.TransformAsync.cs`).
//!
//! Each item in the form that a future gives it, at the same position. The change sets are handled
//! one after another, and the change set of the transformed items is emitted when every future of
//! its change set has ended; an error of a future ends the sequence. The disposal of the
//! subscription cancels the token of the futures in flight.
//!
//! Deviation (DEVIATIONS.md, DynamicData): upstream's factory returns a `Task<TDestination>`; here
//! it returns a future of `Result<TDestination, ObservableError>`, which runs as a task of the
//! thread (docs/porting/reactive.md, section 5.8), and takes its arguments by value, so that the
//! future owns them. The overloads are named as those of `transform`: each takes
//! `bool transformOnRefresh = false`; the plain factory has a form with the flag
//! (`transform_async_with`), as do the factory of the item, the value it had and its index
//! (`transform_async_with_previous_and_index`) and the one that also takes the token
//! (`transform_async_with_token`); the others use `false`.

macro_rules! transform_async_methods {
    () => {
        /// Each item in the form the future of `transform_factory` gives it, at the same position:
        /// an addition or a replacement awaits the factory, a refresh is forwarded, and removals and
        /// moves apply to the transformed items
        /// (`TransformAsync(Func<TSource, Task<TDestination>>, bool transformOnRefresh = false)`).
        fn transform_async<U: Clone + PartialEq + 'static, Fut: std::future::Future<Output = Result<U, ferroui_system::ObservableError>> + 'static>(
            &self,
            transform_factory: impl Fn(T) -> Fut + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<U>>> {
            self.transform_async_with(transform_factory, false)
        }

        /// [`transform_async`](Self::transform_async) that, when `transform_on_refresh` is true,
        /// transforms a refreshed item again and emits a replacement
        /// (`TransformAsync(Func<TSource, Task<TDestination>>, bool transformOnRefresh)`).
        fn transform_async_with<U: Clone + PartialEq + 'static, Fut: std::future::Future<Output = Result<U, ferroui_system::ObservableError>> + 'static>(
            &self,
            transform_factory: impl Fn(T) -> Fut + 'static,
            transform_on_refresh: bool,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<U>>> {
            crate::list::internal::transform_async(
                self.change_sets(),
                crate::list::internal::list_async_transform_factory(move |item: T, _previous: Option<U>, _index: usize, _token: ferroui_system::threading::CancellationToken| transform_factory(item)),
                transform_on_refresh,
            )
        }

        /// [`transform_async`](Self::transform_async) with a factory of the item and its index
        /// (`TransformAsync(Func<TSource, int, Task<TDestination>>, bool transformOnRefresh = false)`).
        fn transform_async_with_index<U: Clone + PartialEq + 'static, Fut: std::future::Future<Output = Result<U, ferroui_system::ObservableError>> + 'static>(
            &self,
            transform_factory: impl Fn(T, usize) -> Fut + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<U>>> {
            crate::list::internal::transform_async(
                self.change_sets(),
                crate::list::internal::list_async_transform_factory(move |item: T, _previous: Option<U>, index: usize, _token: ferroui_system::threading::CancellationToken| transform_factory(item, index)),
                false,
            )
        }

        /// [`transform_async`](Self::transform_async) with a factory of the item and the value it
        /// had (for a replacement, or a refresh that transforms)
        /// (`TransformAsync(Func<TSource, Optional<TDestination>, Task<TDestination>>, bool transformOnRefresh = false)`).
        fn transform_async_with_previous<U: Clone + PartialEq + 'static, Fut: std::future::Future<Output = Result<U, ferroui_system::ObservableError>> + 'static>(
            &self,
            transform_factory: impl Fn(T, Option<U>) -> Fut + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<U>>> {
            crate::list::internal::transform_async(
                self.change_sets(),
                crate::list::internal::list_async_transform_factory(move |item: T, previous: Option<U>, _index: usize, _token: ferroui_system::threading::CancellationToken| transform_factory(item, previous)),
                false,
            )
        }

        /// [`transform_async`](Self::transform_async) with a factory of the item, the value it had
        /// and its index, and the flag of [`transform_async_with`](Self::transform_async_with)
        /// (`TransformAsync(Func<TSource, Optional<TDestination>, int, Task<TDestination>>, bool transformOnRefresh)`).
        fn transform_async_with_previous_and_index<U: Clone + PartialEq + 'static, Fut: std::future::Future<Output = Result<U, ferroui_system::ObservableError>> + 'static>(
            &self,
            transform_factory: impl Fn(T, Option<U>, usize) -> Fut + 'static,
            transform_on_refresh: bool,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<U>>> {
            crate::list::internal::transform_async(
                self.change_sets(),
                crate::list::internal::list_async_transform_factory(move |item: T, previous: Option<U>, index: usize, _token: ferroui_system::threading::CancellationToken| {
                    transform_factory(item, previous, index)
                }),
                transform_on_refresh,
            )
        }

        /// [`transform_async_with_previous_and_index`](Self::transform_async_with_previous_and_index)
        /// whose factory also takes the token that the disposal of the subscription cancels
        /// (`TransformAsync(Func<TSource, Optional<TDestination>, int, CancellationToken, Task<TDestination>>, bool transformOnRefresh)`).
        fn transform_async_with_token<U: Clone + PartialEq + 'static, Fut: std::future::Future<Output = Result<U, ferroui_system::ObservableError>> + 'static>(
            &self,
            transform_factory: impl Fn(T, Option<U>, usize, ferroui_system::threading::CancellationToken) -> Fut + 'static,
            transform_on_refresh: bool,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<U>>> {
            crate::list::internal::transform_async(self.change_sets(), crate::list::internal::list_async_transform_factory(transform_factory), transform_on_refresh)
        }
    };
}
pub(crate) use transform_async_methods;
