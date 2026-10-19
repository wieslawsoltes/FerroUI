//! `Reverse` of lists (`List/ObservableListEx.Reverse.cs`).

macro_rules! reverse_methods {
    () => {
        /// The changes of the list in reverse order (`Reverse`).
        fn reverse(&self) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<T>>> {
            let reverser = std::cell::RefCell::new(crate::list::internal::Reverser::default());
            ferroui_reactive::ObservableExt::select(&self.change_sets(), move |changes: crate::list::ChangeSet<T>| {
                crate::list::ChangeSet::from(reverser.borrow_mut().reverse(&changes))
            })
        }
    };
}
pub(crate) use reverse_methods;
