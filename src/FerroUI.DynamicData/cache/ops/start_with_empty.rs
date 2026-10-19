//! `StartWithEmpty` (`Cache/ObservableCacheEx.StartWithEmpty.cs`).
//!
//! The forms of the sorted, virtual, paged and group change sets, and of read-only collections,
//! are the operator of their own sequences: `start_with(Default::default())`, the empty change set
//! of each form being its default value.

macro_rules! start_with_empty_methods {
    () => {
        /// An empty change set, then the change sets (`StartWithEmpty`).
        fn start_with_empty(&self) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>> {
            ferroui_reactive::ObservableExt::start_with(&self.change_sets(), [crate::cache::ChangeSet::empty()])
        }
    };
}
pub(crate) use start_with_empty_methods;
