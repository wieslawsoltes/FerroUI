use ferroui_base::logging::{ILogSink, LogEventLevel, Logger};
use serde_json::{json, Value};
use std::any::Any;
use std::collections::VecDeque;
use std::fmt::Display;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, Weak};
static ACTIVE: Mutex<Weak<LogCapture>> = Mutex::new(Weak::new());
use std::time::{SystemTime, UNIX_EPOCH};

struct Buffer {
    next: u64,
    entries: VecDeque<(u64, Value)>,
}
/// Bounded, thread-safe log tee. The previous framework sink continues receiving messages.
pub(crate) struct LogCapture {
    owners: AtomicUsize,
    buffer: Mutex<Buffer>,
    previous: Option<Arc<dyn ILogSink + Send + Sync>>,
}
impl LogCapture {
    pub fn install() -> Arc<Self> {
        let mut active = ACTIVE.lock().unwrap();
        if let Some(capture) = active.upgrade() {
            let sink: Arc<dyn ILogSink + Send + Sync> = capture.clone();
            if Logger::sink().is_some_and(|current| Arc::ptr_eq(&current, &sink)) {
                capture.owners.fetch_add(1, Ordering::Relaxed);
                return capture;
            }
        }
        let this = Arc::new(Self {
            owners: AtomicUsize::new(1),
            buffer: Mutex::new(Buffer {
                next: 1,
                entries: VecDeque::new(),
            }),
            previous: Logger::sink(),
        });
        Logger::set_sink(Some(this.clone()));
        *active = Arc::downgrade(&this);
        this
    }
    pub fn uninstall(this: &Arc<Self>) {
        let _active = ACTIVE.lock().unwrap();
        if this.owners.fetch_sub(1, Ordering::Relaxed) != 1 {
            return;
        }
        let sink: Arc<dyn ILogSink + Send + Sync> = this.clone();
        if Logger::sink().is_some_and(|current| Arc::ptr_eq(&current, &sink)) {
            Logger::set_sink(this.previous.clone());
        }
    }
    pub fn since(&self, cursor: &mut u64) -> Vec<Value> {
        let buffer = self.buffer.lock().unwrap();
        let entries = buffer
            .entries
            .iter()
            .filter(|(id, _)| *id > *cursor)
            .map(|(_, v)| v.clone())
            .collect();
        *cursor = buffer.next - 1;
        entries
    }
    fn record(&self, level: LogEventLevel, area: &str, message: &str) {
        if level < LogEventLevel::Information {
            return;
        }
        let level = match level {
            LogEventLevel::Warning => "warning",
            LogEventLevel::Error | LogEventLevel::Fatal => "error",
            _ => "info",
        };
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs_f64()
            * 1000.0;
        let mut buffer = self.buffer.lock().unwrap();
        let id = buffer.next;
        buffer.next += 1;
        if buffer.entries.len() == 512 {
            buffer.entries.pop_front();
        }
        buffer.entries.push_back((id,json!({"source":"other","level":level,"text":format!("[{area}] {}", message.chars().take(8192).collect::<String>()),"timestamp":timestamp})));
    }
}
impl ILogSink for LogCapture {
    fn is_enabled(&self, level: LogEventLevel, area: &str) -> bool {
        level >= LogEventLevel::Information
            || self
                .previous
                .as_ref()
                .is_some_and(|p| p.is_enabled(level, area))
    }
    fn log(&self, level: LogEventLevel, area: &str, source: Option<&dyn Any>, message: &str) {
        self.record(level, area, message);
        if let Some(p) = self.previous.as_ref().filter(|p| p.is_enabled(level, area)) {
            p.log(level, area, source, message);
        }
    }
    fn log_with_values(
        &self,
        level: LogEventLevel,
        area: &str,
        source: Option<&dyn Any>,
        template: &str,
        values: &[&dyn Display],
    ) {
        let mut message = template.to_owned();
        for value in values {
            if let Some(start) = message.find('{') {
                if let Some(end) = message[start..].find('}') {
                    message.replace_range(start..start + end + 1, &value.to_string());
                }
            }
        }
        self.record(level, area, &message);
        if let Some(p) = self.previous.as_ref().filter(|p| p.is_enabled(level, area)) {
            p.log_with_values(level, area, source, template, values);
        }
    }
}
