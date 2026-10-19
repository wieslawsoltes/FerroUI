//! `FinallySafe` (`Cache/ObservableCacheEx.FinallySafe.cs`).

macro_rules! finally_safe_methods {
    () => {
        /// Runs `finally_action` once: after the completion or the error has been forwarded, or
        /// when the subscription ends (`FinallySafe(Action)`; obsolete upstream).
        fn finally_safe(&self, finally_action: impl Fn() + 'static) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::cache::internal::finally_safe(self.observable_sequence(), std::rc::Rc::new(finally_action))
        }
    };
}
pub(crate) use finally_safe_methods;
