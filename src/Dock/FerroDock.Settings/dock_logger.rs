// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use crate::DockSettings;
/// Original settings-gated diagnostics helpers.
pub struct DockLogger;
impl DockLogger {
    /// Logs the original formatted message when diagnostics are enabled.
    pub fn log(category: &str, message: &str) {
        Self::log_internal(category, message);
    }
    /// Logs when enabled in every configuration, as the source method does.
    pub fn log_debug(category: &str, message: &str) {
        Self::log_internal(category, message);
    }
    fn log_internal(category: &str, message: &str) {
        if !DockSettings::enable_diagnostics_logging() {
            return;
        }
        let formatted = format!("[Dock] {category}: {message}");
        match DockSettings::diagnostics_log_handler() {
            Some(handler) => handler(formatted),
            None => {
                // The managed default is Debug.WriteLine, whose calls are debug-only.
                // Native debugger-output transport remains a platform integration gap.
                #[cfg(debug_assertions)]
                eprintln!("{formatted}");
            }
        }
    }
}
