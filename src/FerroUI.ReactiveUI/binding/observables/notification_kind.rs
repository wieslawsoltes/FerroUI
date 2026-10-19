//! `NotificationKind`.

/// The three notifications an observer receives (`NotificationKind`), as the binding stages hold
/// them while they wait for the owning thread.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NotificationKind {
    /// A value (`Next`).
    Next = 0,
    /// A terminating error (`Error`).
    Error = 1,
    /// Successful completion (`Completed`).
    Completed = 2,
}
