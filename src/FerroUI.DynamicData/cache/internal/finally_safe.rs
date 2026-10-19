//! `FinallySafe<T>` (`Cache/Internal/FinallySafe.cs`): an action that runs once, after the
//! completion or the error of the sequence has been forwarded, or when the subscription ends.

use ferroui_reactive::disposables::{CompositeDisposable, Disposable};
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::{IDisposable, IObservable, IObserver};
use std::rc::Rc;

/// Runs the action when it is dropped, so that it runs after the terminal notification even when
/// the observer panics (`try { ... } finally { finallyOnce.Dispose(); }`).
struct RunOnce(Rc<dyn IDisposable>);

impl Drop for RunOnce {
    fn drop(&mut self) {
        self.0.dispose();
    }
}

/// `FinallySafe.Run`.
pub(crate) fn finally_safe<T: 'static>(source: Rc<dyn IObservable<T>>, finally_action: Rc<dyn Fn()>) -> Rc<dyn IObservable<T>> {
    Observable::create(move |observer: Rc<dyn IObserver<T>>| {
        let action = finally_action.clone();
        let finally_once = Disposable::create(move || action());
        let (next_observer, error_observer, completed_observer) = (observer.clone(), observer.clone(), observer);
        let (error_finally, completed_finally) = (finally_once.clone(), finally_once.clone());
        let subscription = source.subscribe_all(
            move |value| next_observer.on_next(value),
            move |error| {
                let _finally = RunOnce(error_finally.clone());
                error_observer.on_error(error);
            },
            move || {
                let _finally = RunOnce(completed_finally.clone());
                completed_observer.on_completed();
            },
        );
        let composite: Rc<dyn IDisposable> = CompositeDisposable::from_disposables([subscription, finally_once]);
        composite
    })
}
