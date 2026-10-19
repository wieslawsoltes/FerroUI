//! `CollectUpdateStats` of lists (`Diagnostics/DiagnosticOperators.cs`): replacements count as
//! updates, refreshes are not counted.

macro_rules! collect_update_stats_methods {
    () => {
        /// The statistics of each change set and of all of them so far (`CollectUpdateStats`).
        fn collect_update_stats(&self) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::diagnostics::ChangeSummary>> {
            ferroui_reactive::ObservableExt::scan_with(&self.change_sets(), crate::diagnostics::ChangeSummary::empty(), |seed, next: crate::list::ChangeSet<T>| {
                seed.next(next.adds(), next.replaced(), next.removes(), 0, next.moves(), next.count())
            })
        }
    };
}
pub(crate) use collect_update_stats_methods;
