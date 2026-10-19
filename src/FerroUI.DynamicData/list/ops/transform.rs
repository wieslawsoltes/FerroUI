//! `Transform` (`List/ObservableListEx.Transform.cs`).
//!
//! Upstream's overloads differ by the factory (of the item; of the item and its index; of the item
//! and the value it had; of all three), each with `bool transformOnRefresh = false`. The factory of
//! all three takes the flag (`transform_with_previous_and_index`), the plain factory has a form
//! with it (`transform_with`), and the others use `false`. The `try_` forms take a factory that may
//! fail, whose error ends the sequence.

macro_rules! transform_methods {
    () => {
        /// Each item in the form `transform_factory` gives it, at the same position: an addition
        /// or a replacement transforms the item, a refresh is forwarded, and removals and moves
        /// apply to the transformed items
        /// (`Transform(Func<TSource, TDestination>, bool transformOnRefresh = false)`).
        fn transform<U: Clone + PartialEq + 'static>(
            &self,
            transform_factory: impl Fn(&T) -> U + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<U>>> {
            self.transform_with(transform_factory, false)
        }

        /// [`transform`](Self::transform) that, when `transform_on_refresh` is true, transforms a
        /// refreshed item again and emits a replacement
        /// (`Transform(Func<TSource, TDestination>, bool transformOnRefresh)`).
        fn transform_with<U: Clone + PartialEq + 'static>(
            &self,
            transform_factory: impl Fn(&T) -> U + 'static,
            transform_on_refresh: bool,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<U>>> {
            crate::list::internal::transformer(self.change_sets(), move |item: &T, _previous: Option<&U>, _index: usize| Ok(transform_factory(item)), transform_on_refresh)
        }

        /// [`transform`](Self::transform) with a factory of the item and its index
        /// (`Transform(Func<TSource, int, TDestination>, bool transformOnRefresh = false)`).
        fn transform_with_index<U: Clone + PartialEq + 'static>(
            &self,
            transform_factory: impl Fn(&T, usize) -> U + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<U>>> {
            crate::list::internal::transformer(self.change_sets(), move |item: &T, _previous: Option<&U>, index: usize| Ok(transform_factory(item, index)), false)
        }

        /// [`transform`](Self::transform) with a factory of the item and the value it had (for a
        /// replacement, or a refresh that transforms)
        /// (`Transform(Func<TSource, Optional<TDestination>, TDestination>, bool transformOnRefresh = false)`).
        fn transform_with_previous<U: Clone + PartialEq + 'static>(
            &self,
            transform_factory: impl Fn(&T, Option<&U>) -> U + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<U>>> {
            crate::list::internal::transformer(self.change_sets(), move |item: &T, previous: Option<&U>, _index: usize| Ok(transform_factory(item, previous)), false)
        }

        /// [`transform`](Self::transform) with a factory of the item, the value it had and its
        /// index, and the flag of [`transform_with`](Self::transform_with)
        /// (`Transform(Func<TSource, Optional<TDestination>, int, TDestination>, bool transformOnRefresh)`).
        fn transform_with_previous_and_index<U: Clone + PartialEq + 'static>(
            &self,
            transform_factory: impl Fn(&T, Option<&U>, usize) -> U + 'static,
            transform_on_refresh: bool,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<U>>> {
            crate::list::internal::transformer(
                self.change_sets(),
                move |item: &T, previous: Option<&U>, index: usize| Ok(transform_factory(item, previous, index)),
                transform_on_refresh,
            )
        }

        /// [`transform`](Self::transform) with a factory that may fail: its error ends the
        /// sequence (`Transform` with a factory that throws).
        fn try_transform<U: Clone + PartialEq + 'static>(
            &self,
            transform_factory: impl Fn(&T) -> Result<U, ferroui_system::ObservableError> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<U>>> {
            crate::list::internal::transformer(self.change_sets(), move |item: &T, _previous: Option<&U>, _index: usize| transform_factory(item), false)
        }

        /// [`transform_with_previous_and_index`](Self::transform_with_previous_and_index) with a
        /// factory that may fail.
        fn try_transform_with_previous_and_index<U: Clone + PartialEq + 'static>(
            &self,
            transform_factory: impl Fn(&T, Option<&U>, usize) -> Result<U, ferroui_system::ObservableError> + 'static,
            transform_on_refresh: bool,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<U>>> {
            crate::list::internal::transformer(self.change_sets(), transform_factory, transform_on_refresh)
        }
    };
}
pub(crate) use transform_methods;
