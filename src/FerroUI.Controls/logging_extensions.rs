use crate::AppBuilder;
use ferroui_base::logging::{LogEventLevel, Logger, StringLogSink};
use std::io::Write;
use std::sync::{Arc, Mutex};

/// Logging configuration for the application builder.
impl AppBuilder {
    /// Logs framework events to the diagnostic trace output of the process.
    ///
    /// `level` is the minimum level to log (the reference default is
    /// warning) and `areas` the areas to log; when empty every area is
    /// logged. Valid values are listed in
    /// [`LogArea`](ferroui_base::logging::LogArea).
    pub fn log_to_trace(&self, level: LogEventLevel, areas: &[&str]) -> AppBuilder {
        Logger::set_sink(Some(Arc::new(StringLogSink::trace(level, areas))));
        self.clone()
    }

    /// Logs framework events to the diagnostic trace output of the process, with a minimum level
    /// per area. `verbosity` returns the minimum level to log for an area; well-known areas are
    /// listed in [`LogArea`](ferroui_base::logging::LogArea).
    pub fn log_to_trace_with_verbosity(
        &self,
        verbosity: impl Fn(&str) -> LogEventLevel + Send + Sync + 'static,
    ) -> AppBuilder {
        self.log_to_delegate_with_verbosity(
            |line| {
                // Tracing must never fail the caller.
                let _ = writeln!(std::io::stderr(), "{line}");
            },
            verbosity,
        )
    }

    /// Logs framework events to a custom callback, with a minimum level per area.
    pub fn log_to_delegate_with_verbosity(
        &self,
        log_callback: impl Fn(&str) + Send + Sync + 'static,
        verbosity: impl Fn(&str) -> LogEventLevel + Send + Sync + 'static,
    ) -> AppBuilder {
        Logger::set_sink(Some(Arc::new(StringLogSink::with_area_verbosity(log_callback, verbosity))));
        self.clone()
    }

    /// Logs framework events to a writer.
    ///
    /// `writer` is the writer that is used for log events, one line per
    /// event; `level` is the minimum level to log and `areas` the areas to
    /// log; when empty every area is logged.
    pub fn log_to_text_writer(
        &self,
        writer: impl Write + Send + 'static,
        level: LogEventLevel,
        areas: &[&str],
    ) -> AppBuilder {
        let writer = Mutex::new(writer);
        self.log_to_delegate(
            move |line| {
                // Logging must never fail the caller.
                if let Ok(mut writer) = writer.lock() {
                    let _ = writeln!(writer, "{line}");
                }
            },
            level,
            areas,
        )
    }

    /// Logs framework events to a custom callback.
    ///
    /// `log_callback` is the callback that is used for log events, `level`
    /// the minimum level to log and `areas` the areas to log; when empty
    /// every area is logged.
    pub fn log_to_delegate(
        &self,
        log_callback: impl Fn(&str) + Send + Sync + 'static,
        level: LogEventLevel,
        areas: &[&str],
    ) -> AppBuilder {
        Logger::set_sink(Some(Arc::new(StringLogSink::new(log_callback, level, areas))));
        self.clone()
    }
}
