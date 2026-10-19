//! `FunctionalExtensions` (`Utilities/FunctionalExtensions.cs`).

/// Runs an action on a value and gives the value back (`With`).
pub trait FunctionalExtensions: Sized {
    /// Runs `action` with the value, then returns it (`With(this T item, Action<T> action)`).
    fn with(self, action: impl FnOnce(&Self)) -> Self {
        action(&self);
        self
    }
}

impl<T> FunctionalExtensions for T {}
