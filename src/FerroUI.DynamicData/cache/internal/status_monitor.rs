//! `StatusMonitor<T>` (`Cache/Internal/StatusMonitor.cs`): the status of the connection to a
//! sequence, the machinery of `MonitorStatus` and `DeferUntilLoaded`.

use crate::kernel::ConnectionStatus;
use ferroui_reactive::disposables::Disposable;
use ferroui_reactive::subjects::Subject;
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::{IObservable, IObserver};
use std::cell::Cell;
use std::rc::Rc;

/// `StatusMonitor.Run`: `Pending`, then `Loaded` with the first value, then `Completed` or
/// `Errored` (the error follows).
pub(crate) fn status_monitor<T: 'static>(source: Rc<dyn IObservable<T>>) -> Rc<dyn IObservable<ConnectionStatus>> {
    Observable::create(move |observer: Rc<dyn IObserver<ConnectionStatus>>| {
        let status_subject = Subject::<ConnectionStatus>::new();
        let status = Rc::new(Cell::new(ConnectionStatus::Pending));

        let monitor = {
            let (updated_status, updated_subject) = (status.clone(), status_subject.clone());
            let (error_status, error_subject, error_observer) = (status.clone(), status_subject.clone(), observer.clone());
            let (completed_status, completed_subject) = (status.clone(), status_subject.clone());
            source.subscribe_all(
                move |_| {
                    if updated_status.get() != ConnectionStatus::Pending {
                        return;
                    }
                    updated_status.set(ConnectionStatus::Loaded);
                    updated_subject.on_next(ConnectionStatus::Loaded);
                },
                move |error| {
                    error_status.set(ConnectionStatus::Errored);
                    error_subject.on_next(ConnectionStatus::Errored);
                    error_observer.on_error(error);
                },
                move || {
                    if completed_status.get() == ConnectionStatus::Errored {
                        return;
                    }
                    completed_status.set(ConnectionStatus::Completed);
                    completed_subject.on_next(ConnectionStatus::Completed);
                },
            )
        };

        let subscriber = status_subject.as_observable().start_with([status.get()]).distinct_until_changed().subscribe_safe(observer);

        Disposable::create(move || {
            status_subject.on_completed();
            monitor.dispose();
            subscriber.dispose();
        })
    })
}
