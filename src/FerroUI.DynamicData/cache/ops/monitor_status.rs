//! `MonitorStatus` (`Cache/ObservableCacheEx.MonitorStatus.cs`).

macro_rules! monitor_status_methods {
    () => {
        /// The status of the sequence: pending until its first value, then loaded, and completed or
        /// errored at its end; the error is forwarded after the status (`MonitorStatus`).
        fn monitor_status(&self) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::kernel::ConnectionStatus>> {
            crate::cache::internal::status_monitor(self.observable_sequence())
        }
    };
}
pub(crate) use monitor_status_methods;
