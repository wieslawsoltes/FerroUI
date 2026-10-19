//! `SkipFirstObserver<T>` (`ReactiveUI.Shared/Routing/SkipFirstObserver.cs`).

use ferroui_system::{IObserver, ObservableError};
use std::cell::Cell;
use std::rc::Rc;

/// Forwards every value but the first to `downstream` (`SkipFirstObserver<T>`): the message bus
/// drops the value its channel replays on subscription.
// Deviation (DEVIATIONS.md, ReactiveUI): internal upstream (its tests see it through
// `InternalsVisibleTo`); public here, for the tests in another crate.
pub struct SkipFirstObserver<T> {
    downstream: Rc<dyn IObserver<T>>,
    /// Whether the first value has been skipped (`_skipped`).
    skipped: Cell<bool>,
}

impl<T> SkipFirstObserver<T> {
    /// An observer that skips the first value and forwards the rest to `downstream`.
    pub fn new(downstream: Rc<dyn IObserver<T>>) -> Self {
        Self { downstream, skipped: Cell::new(false) }
    }
}

impl<T> IObserver<T> for SkipFirstObserver<T> {
    fn on_next(&self, value: T) {
        if !self.skipped.replace(true) {
            return;
        }
        self.downstream.on_next(value);
    }

    fn on_error(&self, error: ObservableError) {
        self.downstream.on_error(error);
    }

    fn on_completed(&self) {
        self.downstream.on_completed();
    }
}
