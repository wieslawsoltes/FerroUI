use std::any::Any;
use std::fmt::Display;
use std::rc::Rc;
use std::sync::Arc;

use super::{ILogSink, LogEventLevel};

/// The sink a [`ParametrizedLogger`] writes to.
#[derive(Clone)]
pub(crate) enum SinkHandle {
    Global(Arc<dyn ILogSink + Send + Sync>),
    Thread(Rc<dyn ILogSink>),
}

impl SinkHandle {
    #[inline]
    pub(crate) fn get(&self) -> &dyn ILogSink {
        match self {
            SinkHandle::Global(sink) => &**sink,
            SinkHandle::Thread(sink) => &**sink,
        }
    }
}

/// Logger sink parametrized for given logging level.
#[derive(Clone)]
pub struct ParametrizedLogger {
    sink: SinkHandle,
    level: LogEventLevel,
    area: &'static str,
}

impl ParametrizedLogger {
    /// Creates a logger that writes to `sink` with the given level and area.
    pub fn new(sink: Rc<dyn ILogSink>, level: LogEventLevel, area: &'static str) -> Self {
        Self { sink: SinkHandle::Thread(sink), level, area }
    }

    pub(crate) fn from_handle(sink: SinkHandle, level: LogEventLevel, area: &'static str) -> Self {
        Self { sink, level, area }
    }

    /// Checks if this logger can be used.
    pub fn is_valid(&self) -> bool {
        true
    }

    /// Logs an event.
    ///
    /// `source` is the object from which the event originates.
    #[inline]
    pub fn log(&self, source: Option<&dyn Any>, message_template: &str) {
        self.sink.get().log(self.level, self.area, source, message_template);
    }

    /// Logs an event with message property values, given in the order of
    /// the `{Placeholders}` of the template.
    #[inline]
    pub fn log_with_values(&self, source: Option<&dyn Any>, message_template: &str, property_values: &[&dyn Display]) {
        self.sink.get().log_with_values(self.level, self.area, source, message_template, property_values);
    }
}
