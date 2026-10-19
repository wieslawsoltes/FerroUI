//! `Adapt` of lists (`List/ObservableListEx.Adapt.cs`).

macro_rules! adapt_methods {
    () => {
        /// Hands each change set to `adaptor` as it passes (`Adapt(IChangeSetAdaptor<T>)`).
        fn adapt(&self, adaptor: std::rc::Rc<dyn crate::list::IChangeSetAdaptor<T>>) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<T>>> {
            ferroui_reactive::ObservableExt::select(&self.change_sets(), move |changes: crate::list::ChangeSet<T>| {
                adaptor.adapt(&changes);
                changes
            })
        }
    };
}
pub(crate) use adapt_methods;
