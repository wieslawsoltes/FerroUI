//! `WhenAnyPropertyChanged` (`Cache/ObservableCacheEx.WhenAnyPropertyChanged.cs`).

macro_rules! when_any_property_changed_methods {
    () => {
        /// The item, whenever one of `properties_to_monitor` changes on it, or any property when
        /// none is given, for every item, merged (`WhenAnyPropertyChanged(params string[])`).
        fn when_any_property_changed(&self, properties_to_monitor: &[&str]) -> std::rc::Rc<dyn ferroui_system::IObservable<T>>
        where
            T: crate::binding::NotifyPropertyChangedEx,
        {
            let properties: std::rc::Rc<[String]> = properties_to_monitor.iter().map(|name| name.to_string()).collect();
            crate::cache::internal::merge_item_observables(
                self.change_sets(),
                std::rc::Rc::new(move |item: &T, _: &K| {
                    let names: Vec<&str> = properties.iter().map(String::as_str).collect();
                    crate::binding::NotifyPropertyChangedEx::when_any_property_changed(item, &names)
                }),
            )
        }
    };
}
pub(crate) use when_any_property_changed_methods;
