//! `SubscriptionErrors` (`SubscriptionErrors.cs`): handles the errors of observable subscriptions.

use ferroui_system::ObservableError;

/// Handles the errors of observable subscriptions (`SubscriptionErrors`).
pub struct SubscriptionErrors;

impl SubscriptionErrors {
    /// Rethrows the error of a subscription (`Throw`, `ExceptionDispatchInfo.Capture(error).Throw()`).
    ///
    /// Deviation (DEVIATIONS.md, ReactiveUI controls): an exception is a panic; the error is
    /// raised as a panic with its message.
    pub fn throw(error: ObservableError) -> ! {
        panic!("{error}")
    }
}
