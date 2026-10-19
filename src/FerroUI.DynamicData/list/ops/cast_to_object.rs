//! `CastToObject` (`List/ObservableListEx.CastToObject.cs`).

macro_rules! cast_to_object_methods {
    () => {
        /// The change sets with each item as an object (`CastToObject`): a `Rc<dyn Any>` that holds
        /// the item, which [`cast_list_any`](crate::list::ObservableObjectListEx::cast_list_any)
        /// casts back.
        fn cast_to_object(&self) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<std::rc::Rc<dyn std::any::Any>>>> {
            ferroui_reactive::ObservableExt::select(&self.change_sets(), |changes: crate::list::ChangeSet<T>| {
                crate::list::transform_change_set(&changes, &|item: &T| std::rc::Rc::new(item.clone()) as std::rc::Rc<dyn std::any::Any>)
            })
        }
    };
}
pub(crate) use cast_to_object_methods;
