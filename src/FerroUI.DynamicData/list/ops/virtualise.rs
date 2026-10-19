//! `Virtualise` of lists (`List/ObservableListEx.Virtualise.cs`).

macro_rules! virtualise_methods {
    () => {
        /// The window of the items that the latest of `requests` asks for, with a description of
        /// the window (`Virtualise(IObservable<IVirtualRequest>)`).
        fn virtualise(
            &self,
            requests: std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::VirtualRequest>>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::list::VirtualChangeSet<T>>> {
            crate::list::internal::virtualiser(self.change_sets(), requests)
        }
    };
}
pub(crate) use virtualise_methods;
