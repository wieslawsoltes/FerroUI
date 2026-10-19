//! `PopulateInto` (`List/ObservableListEx.PopulateInto.cs`).

macro_rules! populate_into_methods {
    () => {
        /// Applies each change set to `destination` (`PopulateInto(ISourceList<T>)`).
        fn populate_into<D: crate::list::ISourceList<T> + Clone + 'static>(&self, destination: &D) -> std::rc::Rc<dyn ferroui_system::IDisposable> {
            let destination = destination.clone();
            ferroui_reactive::ObservableExt::subscribe_next(&self.change_sets(), move |changes: crate::list::ChangeSet<T>| {
                destination.edit_dyn(&mut |updater: &mut dyn crate::list::IExtendedList<T>| {
                    let updater: &mut dyn crate::list::IList<T> = updater;
                    crate::list::clone_changes(updater, &changes);
                });
            })
        }
    };
}
pub(crate) use populate_into_methods;
