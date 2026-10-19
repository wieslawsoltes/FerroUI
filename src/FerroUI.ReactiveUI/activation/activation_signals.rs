//! `ActivationSignals` (`Shared/ActivationSignals.cs`).

use crate::RxSchedulers;
use ferroui_reactive::disposables::Disposable;
use ferroui_reactive::subjects::{Subject, SubjectBase};
use ferroui_system::{IDisposable, IObserver};
use std::rc::Rc;

/// The handling of the activation signals of the views of a user-interface toolkit
/// (`ActivationSignals`).
///
/// The views of a toolkit derive from unrelated base types, so the handling of their
/// `Activated`/`Deactivated` subjects cannot be inherited and lives here. A view implements
/// [`ICanActivate`](crate::ICanActivate) with two subjects, raises them with
/// [`raise`](Self::raise), and disposes them with [`dispose_when`](Self::dispose_when).
// Deviation (DEVIATIONS.md, ReactiveUI): upstream links this internal class into each platform
// assembly; here it is public, for the integration with FerroUI in another crate.
pub struct ActivationSignals;

impl ActivationSignals {
    /// Notifies the subscribers that the view was activated or deactivated (`Raise`). A view can
    /// be called back by the platform after it was disposed: a disposed signal is left alone
    /// rather than panicking.
    pub fn raise(signal: &Rc<Subject<()>>) {
        if signal.is_disposed() {
            return;
        }
        signal.on_next(());
    }

    /// Disposes the activation signals of a view when it is disposed deterministically
    /// (`DisposeWhen(bool, Signal, Signal)`); nothing happens when `disposing` is false.
    pub fn dispose_when(disposing: bool, activated: &Rc<Subject<()>>, deactivated: &Rc<Subject<()>>) {
        if !disposing {
            return;
        }
        activated.dispose();
        deactivated.dispose();
    }

    /// [`dispose_when`](Self::dispose_when) with one more signal of the view, disposed last
    /// (`DisposeWhen<TResult>(bool, Signal, Signal, Signal<TResult>)`).
    pub fn dispose_when_with<T: Clone + 'static>(
        disposing: bool,
        activated: &Rc<Subject<()>>,
        deactivated: &Rc<Subject<()>>,
        additional: &Rc<Subject<T>>,
    ) {
        if !disposing {
            return;
        }
        activated.dispose();
        deactivated.dispose();
        additional.dispose();
    }

    /// Raises the activation or the deactivation on [`RxSchedulers::main_thread_scheduler`]
    /// (`ScheduleActivation`), for `ICanForceManualActivation::activate`. The view may be disposed
    /// before the work runs; the work then does nothing.
    pub fn schedule_activation(is_activating: bool, activated: &Rc<Subject<()>>, deactivated: &Rc<Subject<()>>) {
        let signal = if is_activating { activated.clone() } else { deactivated.clone() };
        let _ = RxSchedulers::main_thread_scheduler().schedule(Box::new(move |_| {
            Self::raise(&signal);
            Disposable::empty()
        }));
    }
}
