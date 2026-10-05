use ferroui_base::logging::{ILogSink, LogEventLevel, Logger};
use ferroui_base::reactive::{Disposable, IDisposable};
use std::any::Any;
use std::fmt::Display;
use std::rc::Rc;

/// The callback of a [`TestLogSink`]: the level, the area, the source, the
/// message template and the property values of a log event.
pub type LogCallback = dyn Fn(LogEventLevel, &str, Option<&dyn Any>, &str, &[&dyn Display]);

/// A log sink that hands every event to a callback.
pub struct TestLogSink {
    callback: Box<LogCallback>,
}

impl TestLogSink {
    /// Creates a sink for a callback.
    pub fn new(
        callback: impl Fn(LogEventLevel, &str, Option<&dyn Any>, &str, &[&dyn Display]) + 'static,
    ) -> TestLogSink {
        TestLogSink { callback: Box::new(callback) }
    }

    /// Installs a sink for `callback` until the returned object is
    /// disposed.
    ///
    /// The sink is the one of the calling thread: tests run in parallel,
    /// each on its own thread, so a test only sees its own events. The
    /// sink that was installed before is restored afterwards.
    pub fn start(
        callback: impl Fn(LogEventLevel, &str, Option<&dyn Any>, &str, &[&dyn Display]) + 'static,
    ) -> Rc<dyn IDisposable> {
        let sink: Rc<dyn ILogSink> = Rc::new(TestLogSink::new(callback));
        let previous = Logger::set_thread_sink(Some(sink));
        Disposable::create(move || {
            Logger::set_thread_sink(previous);
        })
    }
}

impl ILogSink for TestLogSink {
    fn is_enabled(&self, _level: LogEventLevel, _area: &str) -> bool {
        true
    }

    fn log(&self, level: LogEventLevel, area: &str, source: Option<&dyn Any>, message_template: &str) {
        (self.callback)(level, area, source, message_template, &[]);
    }

    fn log_with_values(
        &self,
        level: LogEventLevel,
        area: &str,
        source: Option<&dyn Any>,
        message_template: &str,
        property_values: &[&dyn Display],
    ) {
        (self.callback)(level, area, source, message_template, property_values);
    }
}
