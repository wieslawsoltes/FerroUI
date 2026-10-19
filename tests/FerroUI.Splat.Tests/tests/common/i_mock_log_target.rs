//! `IMockLogTarget`.

use ferroui_splat::LogLevel;

/// A target that records the messages written to it.
pub trait IMockLogTarget {
    /// The messages, in the order they were written (`Logs`).
    fn logs(&self) -> Vec<(LogLevel, String)>;
}
