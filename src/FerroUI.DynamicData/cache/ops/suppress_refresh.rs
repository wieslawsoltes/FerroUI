//! `SuppressRefresh` (`Cache/ObservableCacheEx.SuppressRefresh.cs`).

macro_rules! suppress_refresh_methods {
    () => {
        /// The changes without the refreshes (`SuppressRefresh`, `WhereReasonsAreNot(Refresh)`).
        fn suppress_refresh(&self) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>> {
            self.where_reasons_are_not(&[crate::cache::ChangeReason::Refresh])
        }
    };
}
pub(crate) use suppress_refresh_methods;
