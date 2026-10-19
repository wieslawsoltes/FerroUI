//! `InvokeEvaluate` (`Cache/ObservableCacheEx.InvokeEvaluate.cs`).

macro_rules! invoke_evaluate_methods {
    () => {
        /// Calls [`evaluate`](crate::binding::IEvaluateAware::evaluate) on the item of each refresh
        /// before the change set is forwarded (`InvokeEvaluate`).
        fn invoke_evaluate(&self) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>>
        where
            T: crate::binding::IEvaluateAware,
        {
            ferroui_reactive::ObservableExt::do_(&self.change_sets(), |changes: &crate::cache::ChangeSet<T, K>| {
                for change in changes.iter().filter(|change| change.reason() == crate::cache::ChangeReason::Refresh) {
                    crate::binding::IEvaluateAware::evaluate(change.current());
                }
            })
        }
    };
}
pub(crate) use invoke_evaluate_methods;
