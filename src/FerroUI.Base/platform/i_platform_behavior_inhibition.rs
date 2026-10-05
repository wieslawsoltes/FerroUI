use crate::input::LocalBoxFuture;

/// Allows to inhibit platform specific behavior.
pub trait IPlatformBehaviorInhibition {
    /// Prevents (`inhibit_app_sleep` is `true`) or allows again the platform
    /// to put the application to sleep; `reason` is what the platform shows
    /// as the reason.
    ///
    /// The returned future completes when the platform has applied the
    /// request; it is polled on the dispatcher thread.
    fn set_inhibit_app_sleep(&self, inhibit_app_sleep: bool, reason: &str) -> LocalBoxFuture<()>;
}
