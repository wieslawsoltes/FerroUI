//! `IHandleObservableErrors` (`ReactiveUI.Core/Interfaces/IHandleObservableErrors.cs`).

use ferroui_system::{IObservable, ObservableError};
use std::rc::Rc;

/// An object whose pipelines report their errors on an observable instead of ending
/// (`IHandleObservableErrors`): reactive objects, commands and the helpers of derived properties.
///
/// An error that reaches this observable while nothing subscribes to it goes to
/// [`RxState::default_exception_handler`](crate::RxState::default_exception_handler), which
/// panics with an [`UnhandledErrorException`](crate::UnhandledErrorException).
pub trait IHandleObservableErrors {
    /// The errors of the object's pipelines (`ThrownExceptions`).
    fn thrown_exceptions(&self) -> Rc<dyn IObservable<ObservableError>>;
}
