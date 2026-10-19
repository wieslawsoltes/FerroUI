//! `ReactivePropertyMixins` (`ReactiveUI.Shared/ReactiveProperty/ReactivePropertyMixins.cs`).
//!
//! Not ported: `AddValidation(Expression<Func<ReactiveProperty<T>?>>)`, which reads the
//! DataAnnotations attributes of the property that the expression names (reflection). A view model
//! writes the validators out with `add_validation_error`.

use super::{DataErrors, ReactiveProperty};
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::rc::Rc;

/// The extension methods of the validation of a reactive property (`ReactivePropertyMixins`).
pub trait ReactivePropertyMixins {
    /// The first error message of the property each time its errors change, or `None` when it
    /// has no message (`ObserveValidationErrors`).
    fn observe_validation_errors(&self) -> Rc<dyn IObservable<Option<String>>>;
}

impl<T: Clone + PartialEq + 'static> ReactivePropertyMixins for ReactiveProperty<T> {
    fn observe_validation_errors(&self) -> Rc<dyn IObservable<Option<String>>> {
        Rc::new(ValidationErrorObservable { source: self.observe_error_changed() })
    }
}

/// `ValidationErrorObservable`: the first message of each collection of errors.
struct ValidationErrorObservable {
    source: Rc<dyn IObservable<Option<DataErrors>>>,
}

impl IObservable<Option<String>> for ValidationErrorObservable {
    fn subscribe(&self, observer: Rc<dyn IObserver<Option<String>>>) -> Rc<dyn IDisposable> {
        self.source.subscribe(Rc::new(ValidationErrorSink { downstream: observer }))
    }
}

/// `ValidationErrorObservable.Sink`.
struct ValidationErrorSink {
    downstream: Rc<dyn IObserver<Option<String>>>,
}

impl IObserver<Option<DataErrors>> for ValidationErrorSink {
    fn on_next(&self, value: Option<DataErrors>) {
        let message = value.and_then(|errors| errors.iter().find_map(|error| error.downcast_ref::<String>().cloned()));
        self.downstream.on_next(message);
    }

    fn on_error(&self, error: ObservableError) {
        self.downstream.on_error(error);
    }

    fn on_completed(&self) {
        self.downstream.on_completed();
    }
}
