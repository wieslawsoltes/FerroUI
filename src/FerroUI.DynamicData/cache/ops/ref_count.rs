//! `RefCount` (`Cache/ObservableCacheEx.RefCount.cs`).

macro_rules! ref_count_methods {
    () => {
        /// One cache of the sequence, shared by the subscribers: made when the first subscribes,
        /// disposed when the last unsubscribes (`RefCount`).
        fn ref_count(&self) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>> {
            crate::cache::internal::ref_count(self.change_sets())
        }
    };
}
pub(crate) use ref_count_methods;
