//! Bounded diagnostic capture which forwards to the application's previous sink.
use ferroui_base::logging::{ILogSink, LogEventLevel, Logger};
use serde_json::{json, Value};
use std::{
    any::Any,
    collections::VecDeque,
    fmt::Display,
    sync::{Arc, Mutex},
};

#[derive(Clone, Default)]
pub struct LogCapture(Arc<Mutex<(u64, VecDeque<Value>)>>);
impl LogCapture {
    pub fn snapshot(&self, after: u64) -> Value {
        let state = self.0.lock().unwrap_or_else(|e| e.into_inner());
        json!({"cursor":state.0,"entries":state.1.iter().filter(|v|v["sequence"].as_u64().unwrap_or(0)>after).collect::<Vec<_>>()})
    }
    pub fn install(&self) -> LogGuard {
        let previous = Logger::sink();
        let sink: Arc<dyn ILogSink + Send + Sync> = Arc::new(CaptureSink {
            capture: self.clone(),
            previous: previous.clone(),
        });
        Logger::set_sink(Some(sink.clone()));
        LogGuard { sink, previous }
    }
}
pub struct LogGuard {
    sink: Arc<dyn ILogSink + Send + Sync>,
    previous: Option<Arc<dyn ILogSink + Send + Sync>>,
}
impl Drop for LogGuard {
    fn drop(&mut self) {
        if Logger::sink().is_some_and(|s| Arc::ptr_eq(&s, &self.sink)) {
            Logger::set_sink(self.previous.clone());
        }
    }
}
struct CaptureSink {
    capture: LogCapture,
    previous: Option<Arc<dyn ILogSink + Send + Sync>>,
}
impl CaptureSink {
    fn record(&self, level: LogEventLevel, area: &str, message: &str, values: &[&dyn Display]) {
        let mut state = self.capture.0.lock().unwrap_or_else(|e| e.into_inner());
        state.0 += 1;
        let sequence = state.0;
        // Keep placeholders and their values separately to avoid guessing the logging formatter.
        let message: String = message.chars().take(8192).collect();
        let values: Vec<String> = values
            .iter()
            .take(32)
            .map(|v| v.to_string().chars().take(8192).collect())
            .collect();
        state.1.push_back(json!({"sequence":sequence,"level":format!("{level:?}"),"area":area,"message":message,"values":values}));
        while state.1.len() > 1024 {
            state.1.pop_front();
        }
    }
}
impl ILogSink for CaptureSink {
    fn is_enabled(&self, _: LogEventLevel, _: &str) -> bool {
        true
    }
    fn log(&self, level: LogEventLevel, area: &str, source: Option<&dyn Any>, message: &str) {
        self.record(level, area, message, &[]);
        if let Some(sink) = &self.previous {
            if sink.is_enabled(level, area) {
                sink.log(level, area, source, message);
            }
        }
    }
    fn log_with_values(
        &self,
        level: LogEventLevel,
        area: &str,
        source: Option<&dyn Any>,
        message: &str,
        values: &[&dyn Display],
    ) {
        self.record(level, area, message, values);
        if let Some(sink) = &self.previous {
            if sink.is_enabled(level, area) {
                sink.log_with_values(level, area, source, message, values);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ring_is_bounded_and_cursor_reads_only_new_entries() {
        let capture = LogCapture::default();
        let sink = CaptureSink {
            capture: capture.clone(),
            previous: None,
        };
        for i in 0..1100 {
            sink.log_with_values(
                LogEventLevel::Warning,
                "Binding",
                None,
                "Failure {Value}",
                &[&i],
            );
        }
        let snapshot = capture.snapshot(0);
        assert_eq!(snapshot["cursor"], 1100);
        assert_eq!(snapshot["entries"].as_array().unwrap().len(), 1024);
        assert_eq!(
            capture.snapshot(1099)["entries"].as_array().unwrap().len(),
            1
        );
        assert_eq!(capture.snapshot(1100)["entries"], json!([]));
    }
}
