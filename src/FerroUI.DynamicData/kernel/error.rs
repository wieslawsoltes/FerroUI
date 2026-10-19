//! `Error<TObject, TKey>` (`Kernel/Error.cs`).

use ferroui_system::ObservableError;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

/// An item that an operator could not handle, with its key and the error it raised
/// (`Error<TObject, TKey>`): what the error handler of `TransformSafe` receives.
pub struct Error<T, K> {
    exception: Option<ObservableError>,
    key: K,
    value: T,
}

impl<T, K> Error<T, K> {
    /// `Error(Exception?, TObject value, TKey key)`.
    pub fn new(exception: Option<ObservableError>, value: T, key: K) -> Self {
        Self { exception, key, value }
    }

    /// The error (`Exception`).
    pub fn exception(&self) -> Option<&ObservableError> {
        self.exception.as_ref()
    }

    /// The key of the item (`Key`).
    pub fn key(&self) -> &K {
        &self.key
    }

    /// The item (`Value`).
    pub fn value(&self) -> &T {
        &self.value
    }
}

impl<T: Clone, K: Clone> Clone for Error<T, K> {
    fn clone(&self) -> Self {
        Self { exception: self.exception.clone(), key: self.key.clone(), value: self.value.clone() }
    }
}

impl<T: PartialEq, K: PartialEq> PartialEq for Error<T, K> {
    /// `Equals(Error<TObject, TKey>)`: the same key, an equal item and the same error (the error
    /// compares by reference, as an `Exception` does).
    fn eq(&self, other: &Self) -> bool {
        let same_exception = match (&self.exception, &other.exception) {
            (None, None) => true,
            (Some(a), Some(b)) => Rc::ptr_eq(a, b),
            _ => false,
        };
        self.key == other.key && self.value == other.value && same_exception
    }
}

impl<T: Eq, K: Eq> Eq for Error<T, K> {}

impl<T: Hash, K: Hash> Hash for Error<T, K> {
    /// `GetHashCode`: the key, the item and the identity of the error.
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.key.hash(state);
        self.value.hash(state);
        self.exception.as_ref().map(|exception| Rc::as_ptr(exception) as *const () as usize).hash(state);
    }
}

impl<T: fmt::Debug, K: fmt::Debug> fmt::Display for Error<T, K> {
    /// `ToString`: `Key: {Key}, Value: {Value}, Exception: {Exception}`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.exception {
            Some(exception) => write!(f, "Key: {:?}, Value: {:?}, Exception: {}", self.key, self.value, exception),
            None => write!(f, "Key: {:?}, Value: {:?}, Exception: ", self.key, self.value),
        }
    }
}

impl<T: fmt::Debug, K: fmt::Debug> fmt::Debug for Error<T, K> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
