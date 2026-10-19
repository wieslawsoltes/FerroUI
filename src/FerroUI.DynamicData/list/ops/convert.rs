//! `Convert` (`List/ObservableListEx.Convert.cs`; obsolete upstream in favour of `Cast`).

macro_rules! convert_methods {
    () => {
        /// The change sets with each item converted by `conversion_factory`
        /// (`Convert(Func<TObject, TDestination>)`).
        fn convert<U: Clone + 'static>(&self, conversion_factory: impl Fn(&T) -> U + 'static) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<U>>> {
            ferroui_reactive::ObservableExt::select(&self.change_sets(), move |changes: crate::list::ChangeSet<T>| crate::list::transform_change_set(&changes, &conversion_factory))
        }
    };
}
pub(crate) use convert_methods;
