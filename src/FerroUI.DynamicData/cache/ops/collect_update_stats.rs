//! `CollectUpdateStats` of caches (`Diagnostics/DiagnosticOperators.cs`).

macro_rules! collect_update_stats_methods {
    () => {
        /// The statistics of each change set and of all of them so far (`CollectUpdateStats`).
        fn collect_update_stats(&self) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::diagnostics::ChangeSummary>> {
            ferroui_reactive::ObservableExt::scan_with(&self.change_sets(), crate::diagnostics::ChangeSummary::empty(), |seed, next: crate::cache::ChangeSet<T, K>| {
                seed.next(next.adds(), next.updates(), next.removes(), next.refreshes(), next.moves(), next.count())
            })
        }
    };
}
pub(crate) use collect_update_stats_methods;
