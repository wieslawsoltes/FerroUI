//! One bounded diagnostic capture per process, shared by every inspection server (MCP, CDP). The
//! first [`LogCapture::install`] tees the framework's log sink; the tee forwards to the sink it
//! replaced, and that sink is restored when the last [`LogGuard`] is dropped.
use ferroui_base::logging::{ILogSink, LogEventLevel, Logger};
use serde_json::{json, Value};
use std::{
    any::Any,
    collections::VecDeque,
    fmt::Display,
    sync::{
        atomic::{AtomicI32, Ordering},
        Arc, Mutex,
    },
    time::{SystemTime, UNIX_EPOCH},
};

const CAPACITY: usize = 1024;
const DEFAULT_MINIMUM: LogEventLevel = LogEventLevel::Information;

/// A captured log event. The template keeps its placeholders; the values are kept apart.
#[derive(Clone, Debug)]
pub struct LogEntry {
    pub sequence: u64,
    pub level: LogEventLevel,
    pub area: String,
    pub template: String,
    pub values: Vec<String>,
    /// Milliseconds since the Unix epoch.
    pub timestamp: f64,
}
impl LogEntry {
    /// The template with its placeholders replaced by the values, in order.
    pub fn message(&self) -> String {
        format_template(&self.template, &self.values)
    }
    pub fn to_json(&self) -> Value {
        json!({"sequence":self.sequence,"level":format!("{:?}",self.level),"area":self.area,"message":self.template,"values":self.values,"timestamp":self.timestamp})
    }
}

struct Ring {
    last: u64,
    entries: VecDeque<LogEntry>,
}
static RING: Mutex<Ring> = Mutex::new(Ring {
    last: 0,
    entries: VecDeque::new(),
});
/// The installed tee and the number of guards that keep it.
static INSTALLED: Mutex<Option<(Arc<CaptureSink>, usize)>> = Mutex::new(None);
/// The lowest level that is captured, as the discriminant of [`LogEventLevel`].
static MINIMUM: AtomicI32 = AtomicI32::new(DEFAULT_MINIMUM as i32);

/// The process-wide capture. Entries are recorded only while a guard of [`LogCapture::install`] is
/// alive; the ring keeps the last 1024.
pub struct LogCapture;
impl LogCapture {
    /// Installs the tee, or keeps the installed one for one more guard.
    pub fn install() -> LogGuard {
        let mut installed = INSTALLED.lock().unwrap_or_else(|e| e.into_inner());
        if let Some((sink, owners)) = installed.as_mut() {
            let ours: Arc<dyn ILogSink + Send + Sync> = sink.clone();
            if Logger::sink().is_some_and(|current| Arc::ptr_eq(&current, &ours)) {
                *owners += 1;
                return LogGuard(());
            }
        }
        let sink = Arc::new(CaptureSink {
            previous: Logger::sink(),
        });
        Logger::set_sink(Some(sink.clone()));
        *installed = Some((sink, 1));
        LogGuard(())
    }
    /// Captures events from `level` up while the tee is installed (Information by default).
    pub fn capture_from(level: LogEventLevel) {
        MINIMUM.fetch_min(level as i32, Ordering::AcqRel);
    }
    /// The entries after `cursor`; `cursor` moves to the last recorded entry.
    pub fn since(cursor: &mut u64) -> Vec<LogEntry> {
        let ring = RING.lock().unwrap_or_else(|e| e.into_inner());
        let entries = ring
            .entries
            .iter()
            .filter(|e| e.sequence > *cursor)
            .cloned()
            .collect();
        *cursor = ring.last;
        entries
    }
    /// `{"cursor", "entries"}` of the entries after `after`.
    pub fn snapshot(after: u64) -> Value {
        let mut cursor = after;
        let entries: Vec<_> = Self::since(&mut cursor)
            .iter()
            .map(LogEntry::to_json)
            .collect();
        json!({"cursor":cursor.max(after),"entries":entries})
    }
}

/// Keeps the tee installed. The previous sink is restored when the last guard is dropped, if the
/// tee is still the current sink.
pub struct LogGuard(());
impl Drop for LogGuard {
    fn drop(&mut self) {
        let mut installed = INSTALLED.lock().unwrap_or_else(|e| e.into_inner());
        let Some((sink, owners)) = installed.as_mut() else {
            return;
        };
        *owners -= 1;
        if *owners > 0 {
            return;
        }
        let ours: Arc<dyn ILogSink + Send + Sync> = sink.clone();
        if Logger::sink().is_some_and(|current| Arc::ptr_eq(&current, &ours)) {
            Logger::set_sink(sink.previous.clone());
        }
        *installed = None;
        MINIMUM.store(DEFAULT_MINIMUM as i32, Ordering::Release);
    }
}

struct CaptureSink {
    previous: Option<Arc<dyn ILogSink + Send + Sync>>,
}
fn captured(level: LogEventLevel) -> bool {
    level as i32 >= MINIMUM.load(Ordering::Acquire)
}
fn record(level: LogEventLevel, area: &str, template: &str, values: &[&dyn Display]) {
    if !captured(level) {
        return;
    }
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64()
        * 1000.0;
    let mut ring = RING.lock().unwrap_or_else(|e| e.into_inner());
    ring.last += 1;
    let entry = LogEntry {
        sequence: ring.last,
        level,
        area: area.chars().take(256).collect(),
        template: template.chars().take(8192).collect(),
        values: values
            .iter()
            .take(32)
            .map(|v| v.to_string().chars().take(8192).collect())
            .collect(),
        timestamp,
    };
    ring.entries.push_back(entry);
    while ring.entries.len() > CAPACITY {
        ring.entries.pop_front();
    }
}
impl ILogSink for CaptureSink {
    fn is_enabled(&self, level: LogEventLevel, area: &str) -> bool {
        captured(level)
            || self
                .previous
                .as_ref()
                .is_some_and(|p| p.is_enabled(level, area))
    }
    fn log(&self, level: LogEventLevel, area: &str, source: Option<&dyn Any>, message: &str) {
        record(level, area, message, &[]);
        if let Some(sink) = self.previous.as_ref().filter(|p| p.is_enabled(level, area)) {
            sink.log(level, area, source, message);
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
        record(level, area, message, values);
        if let Some(sink) = self.previous.as_ref().filter(|p| p.is_enabled(level, area)) {
            sink.log_with_values(level, area, source, message, values);
        }
    }
}

/// Replaces the placeholders of the template (`{Name}`) with the values in order. The text of a
/// value is not searched for placeholders again.
fn format_template(template: &str, values: &[String]) -> String {
    let mut result = String::with_capacity(template.len());
    let mut values = values.iter();
    let mut rest = template;
    while let Some(start) = rest.find('{') {
        let Some(length) = rest[start..].find('}') else {
            break;
        };
        result.push_str(&rest[..start]);
        match values.next() {
            Some(value) => result.push_str(value),
            None => result.push_str(&rest[start..start + length + 1]),
        }
        rest = &rest[start + length + 1..];
    }
    result.push_str(rest);
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    // The ring and the tee are process state, shared with the other tests of this module.
    static LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn ring_is_bounded_and_cursor_reads_only_new_entries() {
        let _lock = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let mut start = 0;
        LogCapture::since(&mut start);
        for i in 0..1100 {
            record(LogEventLevel::Warning, "Binding", "Failure {Value}", &[&i]);
        }
        let snapshot = LogCapture::snapshot(start);
        assert_eq!(snapshot["cursor"], start + 1100);
        assert_eq!(snapshot["entries"].as_array().unwrap().len(), 1024);
        assert_eq!(
            LogCapture::snapshot(start + 1099)["entries"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        assert_eq!(LogCapture::snapshot(start + 1100)["entries"], json!([]));
        record(LogEventLevel::Debug, "Binding", "below the minimum", &[]);
        assert_eq!(LogCapture::snapshot(start + 1100)["entries"], json!([]));
    }

    #[test]
    fn values_with_braces_do_not_shift_the_placeholders() {
        assert_eq!(
            format_template("{A} and {B}", &["{x}".into(), "y".into()]),
            "{x} and y"
        );
        assert_eq!(format_template("{A} {B}", &["1".into()]), "1 {B}");
        assert_eq!(format_template("no placeholder {", &[]), "no placeholder {");
    }

    #[test]
    fn guards_share_one_tee_and_restore_the_previous_sink() {
        let _lock = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let previous = Logger::sink();
        let first = LogCapture::install();
        let sink = Logger::sink().unwrap();
        let second = LogCapture::install();
        assert!(Arc::ptr_eq(&sink, &Logger::sink().unwrap()));
        assert!(!sink.is_enabled(LogEventLevel::Debug, "Any") || previous.is_some());
        LogCapture::capture_from(LogEventLevel::Debug);
        assert!(sink.is_enabled(LogEventLevel::Debug, "Any"));
        drop(first);
        assert!(Arc::ptr_eq(&sink, &Logger::sink().unwrap()));
        drop(second);
        assert_eq!(Logger::sink().is_some(), previous.is_some());
        assert!(!captured(LogEventLevel::Debug));
    }
}
