//! `StreamCompletionStrategy` (`Utilities/StreamCompletionStrategy.cs`).

/// When a source of a test completes (`StreamCompletionStrategy`): before the subscription, or
/// after it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StreamCompletionStrategy {
    /// The source completes before it is subscribed to (`Immediate`).
    Immediate,
    /// The source completes after it is subscribed to (`Asynchronous`).
    Asynchronous,
}
