//! `SortError` (`List/SortException.cs`).

use ferroui_system::ObservableError;
use std::error::Error;
use std::fmt;
use std::rc::Rc;

/// The error of a sort that cannot find the position of an item (`SortException`): a comparer
/// that does not give a unique position to each item while a binary search is asked for, or an
/// item that is not in the sorted collection. The operators end their sequence with it, as an
/// exception thrown in an operator of upstream reaches `OnError`.
#[derive(Clone, Debug)]
pub struct SortError {
    message: String,
    inner: Option<ObservableError>,
}

/// The name of [`SortError`] before 0.5.
#[deprecated(since = "0.5.0", note = "use `SortError`")]
pub type SortException = SortError;

impl SortError {
    /// An error with `message` (`SortException(string)`).
    pub fn new(message: impl Into<String>) -> Self {
        Self { message: message.into(), inner: None }
    }

    /// An error with `message`, caused by `inner` (`SortException(string, Exception)`).
    pub fn with_inner(message: impl Into<String>, inner: ObservableError) -> Self {
        Self { message: message.into(), inner: Some(inner) }
    }

    /// The message (`Message`).
    pub fn message(&self) -> &str {
        &self.message
    }

    /// The error as the error of a sequence.
    pub fn into_error(self) -> ObservableError {
        Rc::new(self)
    }
}

impl Default for SortError {
    /// An error with the default message of an exception (`SortException()`).
    fn default() -> Self {
        Self::new("Exception of type 'DynamicData.SortException' was thrown.")
    }
}

impl fmt::Display for SortError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl Error for SortError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.inner.as_deref()
    }
}
