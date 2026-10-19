//! `ObservableEx` (`Internal/ObservableEx.cs`): the subscriptions of functions through
//! `SubscribeSafe`, and `Synchronize` with a caller's lock.
#![allow(dead_code)]

use ferroui_reactive::Observer;
use ferroui_system::{IDisposable, IObservable, ObservableError};
use std::rc::Rc;

/// Subscribes the three functions (`SubscribeSafe(Action<T>, Action<Exception>, Action)`).
pub(crate) fn subscribe_safe<T: 'static>(
    observable: &Rc<dyn IObservable<T>>,
    on_next: impl Fn(T) + 'static,
    on_error: impl Fn(ObservableError) + 'static,
    on_completed: impl Fn() + 'static,
) -> Rc<dyn IDisposable> {
    observable.subscribe(Observer::create_all(on_next, on_error, on_completed))
}

/// Subscribes the functions of values and errors (`SubscribeSafe(Action<T>, Action<Exception>)`).
pub(crate) fn subscribe_safe_next_error<T: 'static>(
    observable: &Rc<dyn IObservable<T>>,
    on_next: impl Fn(T) + 'static,
    on_error: impl Fn(ObservableError) + 'static,
) -> Rc<dyn IDisposable> {
    observable.subscribe(Observer::create_with_error(on_next, on_error))
}

/// Subscribes the functions of errors and the end, and ignores the values
/// (`SubscribeSafe(Action<Exception>, Action)`).
pub(crate) fn subscribe_safe_error_completed<T: 'static>(
    observable: &Rc<dyn IObservable<T>>,
    on_error: impl Fn(ObservableError) + 'static,
    on_completed: impl Fn() + 'static,
) -> Rc<dyn IDisposable> {
    observable.subscribe(Observer::create_all(|_: T| {}, on_error, on_completed))
}

/// Subscribes the function of errors, and ignores the values (`SubscribeSafe(Action<Exception>)`).
pub(crate) fn subscribe_safe_error<T: 'static>(observable: &Rc<dyn IObservable<T>>, on_error: impl Fn(ObservableError) + 'static) -> Rc<dyn IDisposable> {
    observable.subscribe(Observer::create_with_error(|_: T| {}, on_error))
}

/// `Synchronize(IObservable<T>, Lock)`: each notification under the caller's lock.
///
/// Deviation (DEVIATIONS.md, DynamicData): on one thread the lock serializes nothing that is not
/// serialized already, so the sequence is the source (docs/porting/reactive.md, section 5.4).
pub(crate) fn synchronize<T: 'static>(source: Rc<dyn IObservable<T>>) -> Rc<dyn IObservable<T>> {
    source
}
