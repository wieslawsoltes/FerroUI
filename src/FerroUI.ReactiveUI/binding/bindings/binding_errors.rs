//! `BindingErrors`, and the `TargetInvocationException` it raises.

use ferroui_splat::{AppLocator, ILogManager, LogHost};
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::error::Error;
use std::fmt;
use std::rc::Rc;

/// How a binding reacts when the sequence feeding its write faults (`BindingErrors`).
///
/// A faulted write has no caller stack to surface on: it arrives wherever the change notification
/// was raised. Every fault is logged against the bound expression. A fault that carries an inner
/// error (its [`source`](Error::source)), a setter that failed, is raised again as a
/// [`TargetInvocationException`] panic. A fault with none is the sequence itself ending in error,
/// and is only logged. Completion is ignored.
pub struct BindingErrors;

impl BindingErrors {
    /// Subscribes the write `on_next` of a binding to `source`, applying the fault contract;
    /// `binding_expression` names the bound expression in the log entry and the raised error
    /// (`Subscribe<T>(IObservable<T>, Action<T>, string)`).
    pub fn subscribe<T: 'static>(source: Rc<dyn IObservable<T>>, on_next: impl Fn(T) + 'static, binding_expression: impl Into<String>) -> Rc<dyn IDisposable> {
        source.subscribe(Rc::new(BindingObserver { on_next: Box::new(on_next), binding_expression: binding_expression.into() }))
    }
}

/// Applies the write, and the fault contract when the sequence errors.
struct BindingObserver<T> {
    on_next: Box<dyn Fn(T)>,
    binding_expression: String,
}

impl<T> IObserver<T> for BindingObserver<T> {
    fn on_next(&self, value: T) {
        (self.on_next)(value);
    }

    /// Logs the fault, and raises a [`TargetInvocationException`] when it carries an inner error.
    fn on_error(&self, error: ObservableError) {
        let message = format!("{} Binding received an Exception!", self.binding_expression);
        if let Some(log_manager) = AppLocator::get_service::<dyn ILogManager>() {
            log_manager.get_logger(std::any::type_name::<LogHost>()).error_exception(&*error, &message);
        }
        let Some(inner) = error.source() else {
            return;
        };
        std::panic::panic_any(TargetInvocationException::with_inner(message, inner));
    }

    fn on_completed(&self) {}
}

/// The error of a member that was invoked and failed (`System.Reflection.TargetInvocationException`):
/// what a binding raises, as a panic, when the sequence feeding its write faults with an inner
/// error.
///
/// Deviation (DEVIATIONS.md, ReactiveUI): the inner exception is kept as its message. The error is
/// the payload of a panic (`std::panic::panic_any`), which has to be `Send`, and the errors of the
/// reactive libraries (`Rc<dyn Error>`) are not.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TargetInvocationException {
    message: String,
    inner_exception: Option<String>,
}

impl TargetInvocationException {
    /// The exception with `message`, caused by `inner_exception`
    /// (`TargetInvocationException(string, Exception)`).
    pub fn with_inner(message: impl Into<String>, inner_exception: &dyn Error) -> Self {
        Self { message: message.into(), inner_exception: Some(inner_exception.to_string()) }
    }

    /// The message (`Message`).
    pub fn message(&self) -> &str {
        &self.message
    }

    /// The message of the error that caused this one (`InnerException.Message`).
    pub fn inner_exception(&self) -> Option<&str> {
        self.inner_exception.as_deref()
    }
}

impl fmt::Display for TargetInvocationException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.inner_exception {
            Some(inner) => write!(f, "{} ({inner})", self.message),
            None => f.write_str(&self.message),
        }
    }
}

impl Error for TargetInvocationException {}
