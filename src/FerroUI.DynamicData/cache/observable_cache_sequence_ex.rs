//! The operators of `ObservableCacheEx` that apply to any sequence, not only to sequences of change
//! sets (`FinallySafe<T>`, `MonitorStatus<T>`).

use ferroui_system::IObservable;
use std::rc::Rc;

/// The operators of `ObservableCacheEx` on a sequence of any values (`ObservableCacheEx.FinallySafe`,
/// `ObservableCacheEx.MonitorStatus`).
///
/// Their names are not those of operators of `ferroui_reactive::ObservableExt`, nor of the
/// operators of change sets, so the traits can all be in scope.
pub trait ObservableCacheSequenceEx<T: 'static> {
    /// The sequence as the handle the operators take.
    fn observable_sequence(&self) -> Rc<dyn IObservable<T>>;

    super::ops::finally_safe::finally_safe_methods!();
    super::ops::monitor_status::monitor_status_methods!();
}

impl<T: 'static> ObservableCacheSequenceEx<T> for Rc<dyn IObservable<T>> {
    fn observable_sequence(&self) -> Rc<dyn IObservable<T>> {
        self.clone()
    }
}
