//! The log of the system.
//!
//! The standard output and the standard error of an Android process go
//! nowhere, so what an application wants read (a panic, the lines of a
//! smoke run) is written to the log of the system, which `logcat` shows.
//! Not from the reference, whose runtime routes its traces there by itself.
//! An application routes the log of the framework here with
//! `AppBuilder::log_to_delegate`.

/// The tag the backend writes its own lines under.
pub const TAG: &str = "ferroui";

/// The priorities of the log of the system (`android_LogPriority`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(i32)]
pub enum LogPriority {
    Verbose = 2,
    Debug = 3,
    Info = 4,
    Warn = 5,
    Error = 6,
    Fatal = 7,
}

/// Writes a line to the log of the system under `tag`. On another system
/// than Android the line goes to the standard error.
pub fn write(priority: LogPriority, tag: &str, message: &str) {
    #[cfg(target_os = "android")]
    crate::interop::ndk::log_write(priority, tag, message);
    #[cfg(not(target_os = "android"))]
    eprintln!("{priority:?}/{tag}: {message}");
}

/// Writes an informational line under the tag of the backend.
pub fn info(message: &str) {
    write(LogPriority::Info, TAG, message);
}

/// Writes an error line under the tag of the backend.
pub fn error(message: &str) {
    write(LogPriority::Error, TAG, message);
}
