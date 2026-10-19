//! `Convert` (`Cache/ObservableCacheEx.Convert.cs`; obsolete upstream in favour of `Transform`).

macro_rules! convert_methods {
    () => {
        /// The change sets with each item, and each previous item, converted by
        /// `conversion_factory` (`Convert(Func<TObject, TDestination>)`).
        fn convert<U: Clone + 'static>(&self, conversion_factory: impl Fn(&T) -> U + 'static) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<U, K>>> {
            crate::cache::internal::cast(self.change_sets(), std::rc::Rc::new(conversion_factory))
        }
    };
}
pub(crate) use convert_methods;
