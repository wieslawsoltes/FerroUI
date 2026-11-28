//! The notes of the backend: what it does not draw yet is said once, in the log, and drawn as
//! nothing.

use ferroui_base::logging::{LogEventLevel, Logger};
use std::collections::HashSet;
use std::sync::Mutex;

static SAID: Mutex<Option<HashSet<String>>> = Mutex::new(None);

/// Whether `key` is said for the first time.
fn first(key: &str) -> bool {
    let mut said = SAID.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    said.get_or_insert_with(HashSet::new).insert(key.to_owned())
}

/// Logs `message` as a warning of the area `ProGPU`, once per `key`. Without a log sink the note
/// goes to the standard error stream, so a missing part is never silent.
pub(crate) fn note_once(key: &str, message: &str) {
    if !first(key) {
        return;
    }
    match Logger::try_get(LogEventLevel::Warning, "ProGPU") {
        Some(logger) => logger.log(None, message),
        None => eprintln!("[ProGPU] {message}"),
    }
}

/// Logs a failure of a frame (every time: a frame that fails is not a missing part).
pub(crate) fn failure(message: &str) {
    match Logger::try_get(LogEventLevel::Error, "ProGPU") {
        Some(logger) => logger.log(None, message),
        None => eprintln!("[ProGPU] {message}"),
    }
}

/// What the notes said so far (for the tests of the safe failures).
#[cfg(test)]
pub(crate) fn said(key: &str) -> bool {
    SAID.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).as_ref().is_some_and(|said| said.contains(key))
}
