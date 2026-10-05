use std::any::Any;
use std::fmt::Display;

use super::LogEventLevel;

/// Defines a sink for log events.
pub trait ILogSink {
    /// Checks if given log level and area is enabled.
    fn is_enabled(&self, level: LogEventLevel, area: &str) -> bool;

    /// Logs an event.
    ///
    /// `source` is the object from which the event originates.
    fn log(&self, level: LogEventLevel, area: &str, source: Option<&dyn Any>, message_template: &str);

    /// Logs a new event.
    ///
    /// `property_values` are the message property values, in the order of
    /// the `{Placeholders}` of the template.
    fn log_with_values(
        &self,
        level: LogEventLevel,
        area: &str,
        source: Option<&dyn Any>,
        message_template: &str,
        property_values: &[&dyn Display],
    );
}
