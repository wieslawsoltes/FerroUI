//! `Cast` (`Cache/ObservableCacheEx.Cast.cs`).

macro_rules! cast_methods {
    () => {
        /// The change sets with each item converted by `converter`
        /// (`Cast(Func<TSource, TDestination>)`; named `cast_cache` beside `ObservableExt::cast`).
        fn cast_cache<U: Clone + 'static>(&self, converter: impl Fn(&T) -> U + 'static) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<U, K>>> {
            crate::cache::internal::cast(self.change_sets(), std::rc::Rc::new(converter))
        }
    };
}
pub(crate) use cast_methods;
