//! `ObserverBase<T>` (`ObserverBase.cs`).
//!
//! Upstream is an abstract class whose subclasses override `OnNextCore`, `OnErrorCore` and
//! `OnCompletedCore`. Rust has no inheritance: the port is a struct that wraps the three handlers
//! of the user, given as an [`ObserverCore`] or as functions, and adds what the base class does
//! (the grammar of the notifications, and `Dispose`).

use ferroui_system::{IDisposable, IObserver, ObservableError};
use std::cell::Cell;

/// The handlers of an [`ObserverBase`] (the abstract members `OnNextCore`, `OnErrorCore` and
/// `OnCompletedCore` of `ObserverBase<T>`).
pub trait ObserverCore<T> {
    /// Handles a value (`OnNextCore`).
    fn on_next_core(&self, value: T);

    /// Handles the error that ends the sequence (`OnErrorCore`).
    fn on_error_core(&self, error: ObservableError);

    /// Handles the end of the sequence (`OnCompletedCore`).
    fn on_completed_core(&self);
}

/// The core of an observer made of functions.
struct FnCore<T> {
    on_next: Box<dyn Fn(T)>,
    on_error: Box<dyn Fn(ObservableError)>,
    on_completed: Box<dyn Fn()>,
}

impl<T> ObserverCore<T> for FnCore<T> {
    fn on_next_core(&self, value: T) {
        (self.on_next)(value);
    }

    fn on_error_core(&self, error: ObservableError) {
        (self.on_error)(error);
    }

    fn on_completed_core(&self) {
        (self.on_completed)();
    }
}

/// An observer that enforces the grammar of the notifications on its core (`ObserverBase<T>`):
/// values reach the core until a terminal notification; the first terminal notification reaches
/// it, and every notification after it is ignored. Disposing the observer stops it the same way.
pub struct ObserverBase<T> {
    core: Box<dyn ObserverCore<T>>,
    is_stopped: Cell<bool>,
}

impl<T: 'static> ObserverBase<T> {
    /// An observer of the handlers of `core` (a subclass of `ObserverBase<T>` upstream).
    pub fn new(core: impl ObserverCore<T> + 'static) -> Self {
        Self { core: Box::new(core), is_stopped: Cell::new(false) }
    }

    /// An observer whose handlers are the three functions.
    pub fn from_fns(
        on_next_core: impl Fn(T) + 'static,
        on_error_core: impl Fn(ObservableError) + 'static,
        on_completed_core: impl Fn() + 'static,
    ) -> Self {
        Self::new(FnCore { on_next: Box::new(on_next_core), on_error: Box::new(on_error_core), on_completed: Box::new(on_completed_core) })
    }

    /// Stops the observer and hands `error` to the core, unless it is stopped already (`Fail`);
    /// true when the core received it.
    pub(crate) fn fail(&self, error: ObservableError) -> bool {
        if !self.is_stopped.replace(true) {
            self.core.on_error_core(error);
            return true;
        }
        false
    }
}

impl<T: 'static> IObserver<T> for ObserverBase<T> {
    /// Hands the value to the core unless the observer is stopped (`OnNext`).
    fn on_next(&self, value: T) {
        if !self.is_stopped.get() {
            self.core.on_next_core(value);
        }
    }

    /// Stops the observer and hands the error to the core, the first time (`OnError`).
    fn on_error(&self, error: ObservableError) {
        self.fail(error);
    }

    /// Stops the observer and tells the core, the first time (`OnCompleted`).
    fn on_completed(&self) {
        if !self.is_stopped.replace(true) {
            self.core.on_completed_core();
        }
    }
}

impl<T> IDisposable for ObserverBase<T> {
    /// Stops the observer: later notifications are ignored (`Dispose`).
    fn dispose(&self) {
        self.is_stopped.set(true);
    }
}
