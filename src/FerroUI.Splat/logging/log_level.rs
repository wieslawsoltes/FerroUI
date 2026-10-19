//! `LogLevel`.

use std::fmt;

/// The level of a message (`LogLevel`), from the most verbose to the most severe. A logger writes
/// the messages of its level and above.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LogLevel {
    /// Messages for debugging.
    Debug = 1,
    /// Information.
    Info,
    /// Warnings.
    Warn,
    /// Errors.
    Error,
    /// Errors the application cannot recover from.
    Fatal,
}

/// The name of the level, as `Enum.ToString` writes it (`Debug`, `Info`, ...).
impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Debug => "Debug",
            Self::Info => "Info",
            Self::Warn => "Warn",
            Self::Error => "Error",
            Self::Fatal => "Fatal",
        })
    }
}

impl Default for LogLevel {
    /// The level of a logger whose level was not set: upstream's `default(LogLevel)` is 0, below
    /// `Debug`, so such a logger writes everything; `Debug` does the same here.
    // Deviation (DEVIATIONS.md, Splat): the enum has no value 0; a logger that was not given a
    // level starts at `Debug`, which writes the same messages.
    fn default() -> Self {
        Self::Debug
    }
}
