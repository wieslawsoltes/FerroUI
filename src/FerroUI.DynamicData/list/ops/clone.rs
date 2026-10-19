//! `Clone` (`List/ObservableListEx.Clone.cs`).

macro_rules! clone_methods {
    () => {
        /// The change sets, applied to `target` as they pass (`Clone(IList<T>)`; named `clone_to`
        /// beside `Clone::clone`).
        fn clone_to<L: crate::list::IList<T> + 'static>(&self, target: std::rc::Rc<std::cell::RefCell<L>>) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<T>>> {
            ferroui_reactive::ObservableExt::do_(&self.change_sets(), move |changes: &crate::list::ChangeSet<T>| {
                crate::list::clone_changes(&mut *target.borrow_mut(), changes);
            })
        }
    };
}
pub(crate) use clone_methods;
