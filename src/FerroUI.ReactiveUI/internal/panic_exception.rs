//! The exceptions of user code, which are panics in Rust: what catches them where upstream catches
//! an `Exception`, and the error that carries one on to an `OnError` or a `ThrownExceptions`.
//!
//! Not an upstream file: the C# code catches exceptions with `try`/`catch`. The port catches the
//! panic with `catch_unwind` at the same places.
//! Deviation (DEVIATIONS.md, ReactiveUI): an exception of a subscriber or of a command's code is a
//! panic; it is caught with `catch_unwind` where upstream catches it, and becomes a
//! [`PanicException`] that carries its message.

use crate::UnhandledErrorException;
use ferroui_system::ObservableError;
use std::any::Any;
use std::error::Error;
use std::fmt;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Rc;

/// The error of a panic of user code, caught where upstream catches an exception: the message
/// of the panic.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PanicException {
    /// The message of the panic (`Exception.Message`).
    pub message: String,
}

impl fmt::Display for PanicException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl Error for PanicException {}

impl PanicException {
    /// The error of a caught panic: an [`UnhandledErrorException`] stays one, any other payload
    /// becomes a `PanicException` with the text of the payload.
    pub fn from_payload(payload: &(dyn Any + Send)) -> ObservableError {
        if let Some(exception) = payload.downcast_ref::<UnhandledErrorException>() {
            return Rc::new(exception.clone());
        }
        let message = payload
            .downcast_ref::<&str>()
            .map(|text| (*text).to_owned())
            .or_else(|| payload.downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "the code panicked".to_owned());
        Rc::new(PanicException { message })
    }

    /// Runs `action` and catches its panic (`try { action() } catch (Exception ex)`): the result,
    /// or the error of the panic with its payload, which `std::panic::resume_unwind` rethrows
    /// (`throw;`).
    pub fn catch<R>(action: impl FnOnce() -> R) -> Result<R, (ObservableError, Box<dyn Any + Send>)> {
        catch_unwind(AssertUnwindSafe(action)).map_err(|payload| (Self::from_payload(&*payload), payload))
    }
}
