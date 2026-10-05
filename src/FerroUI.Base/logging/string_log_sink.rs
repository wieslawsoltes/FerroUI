use std::any::Any;
use std::fmt::{Display, Write as _};
use std::io::Write as _;

use super::{ILogSink, LogEventLevel};
use crate::FerroObject;

/// A sink that formats events as text lines and hands them to a callback.
pub struct StringLogSink {
    logger: Box<dyn Fn(&str) + Send + Sync>,
    level: LogEventLevel,
    areas: Option<Vec<String>>,
}

impl StringLogSink {
    /// Creates a sink that passes every formatted event with at least
    /// `minimum_level` to `logger`. When `areas` is not empty only events of
    /// those areas are logged; valid values are listed in
    /// [`LogArea`](super::LogArea).
    pub fn new(logger: impl Fn(&str) + Send + Sync + 'static, minimum_level: LogEventLevel, areas: &[&str]) -> Self {
        Self {
            logger: Box::new(logger),
            level: minimum_level,
            areas: if areas.is_empty() { None } else { Some(areas.iter().map(|area| area.to_string()).collect()) },
        }
    }

    /// Creates a sink that writes events to the diagnostic trace output of
    /// the process (standard error).
    pub fn trace(minimum_level: LogEventLevel, areas: &[&str]) -> Self {
        Self::new(
            |line| {
                // Tracing must never fail the caller.
                let _ = writeln!(std::io::stderr(), "{line}");
            },
            minimum_level,
            areas,
        )
    }

    fn format(area: &str, template: &str, source: Option<&dyn Any>, values: Option<&[&dyn Display]>) -> String {
        let mut result = String::with_capacity(template.len() + area.len() + 16);
        let mut chars = template.chars().peekable();
        let mut i = 0;

        result.push('[');
        result.push_str(area);
        // The plain overload separates the area from the message; the
        // overload with values does not.
        result.push_str(if values.is_none() { "] " } else { "]" });

        while let Some(c) = chars.next() {
            if c != '{' {
                result.push(c);
            } else if chars.peek() != Some(&'{') {
                result.push('\'');
                if let Some(value) = values.and_then(|values| values.get(i)) {
                    let _ = write!(result, "{value}");
                }
                i += 1;
                result.push('\'');
                // Skip the placeholder name and the closing brace.
                for c in chars.by_ref() {
                    if c == '}' {
                        break;
                    }
                }
            } else {
                result.push('{');
                chars.next();
            }
        }

        Self::format_source(source, &mut result);
        result
    }

    fn format_source(source: Option<&dyn Any>, result: &mut String) {
        let Some(source) = source else {
            return;
        };

        result.push_str(" (");
        if let Some(object) = source.downcast_ref::<FerroObject>() {
            result.push_str(object.get_type().name());
        } else {
            result.push_str("Object");
        }
        result.push_str(" #");
        let _ = write!(result, "{}", source as *const dyn Any as *const () as usize);
        result.push(')');
    }
}

impl ILogSink for StringLogSink {
    fn is_enabled(&self, level: LogEventLevel, area: &str) -> bool {
        level >= self.level && self.areas.as_ref().is_none_or(|areas| areas.iter().any(|a| a == area))
    }

    fn log(&self, level: LogEventLevel, area: &str, source: Option<&dyn Any>, message_template: &str) {
        if self.is_enabled(level, area) {
            (self.logger)(&Self::format(area, message_template, source, None));
        }
    }

    fn log_with_values(
        &self,
        level: LogEventLevel,
        area: &str,
        source: Option<&dyn Any>,
        message_template: &str,
        property_values: &[&dyn Display],
    ) {
        if self.is_enabled(level, area) {
            (self.logger)(&Self::format(area, message_template, source, Some(property_values)));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logging::{LogArea, Logger};
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::sync::{Arc, Mutex};

    fn collecting_sink(level: LogEventLevel, areas: &[&str]) -> (StringLogSink, Arc<Mutex<Vec<String>>>) {
        let lines = Arc::new(Mutex::new(Vec::new()));
        let l = lines.clone();
        (StringLogSink::new(move |line| l.lock().unwrap().push(line.to_string()), level, areas), lines)
    }

    #[test]
    fn is_enabled_filters_by_level_and_area() {
        let (sink, _) = collecting_sink(LogEventLevel::Warning, &[]);
        assert!(sink.is_enabled(LogEventLevel::Warning, LogArea::BINDING));
        assert!(sink.is_enabled(LogEventLevel::Fatal, LogArea::LAYOUT));
        assert!(!sink.is_enabled(LogEventLevel::Information, LogArea::BINDING));

        let (sink, _) = collecting_sink(LogEventLevel::Verbose, &[LogArea::LAYOUT]);
        assert!(sink.is_enabled(LogEventLevel::Verbose, LogArea::LAYOUT));
        assert!(!sink.is_enabled(LogEventLevel::Fatal, LogArea::BINDING));
    }

    #[test]
    fn formats_message_without_values() {
        let (sink, lines) = collecting_sink(LogEventLevel::Verbose, &[]);
        sink.log(LogEventLevel::Warning, LogArea::BINDING, None, "Something {Name} happened {{literal}}");
        assert_eq!(lines.lock().unwrap().as_slice(), ["[Binding] Something '' happened {literal}}"]);
    }

    #[test]
    fn formats_message_with_values() {
        let (sink, lines) = collecting_sink(LogEventLevel::Verbose, &[]);
        sink.log_with_values(
            LogEventLevel::Error,
            LogArea::LAYOUT,
            None,
            "Measure of {Control} took {Time}ms and {Missing}",
            &[&"Button", &12.5],
        );
        assert_eq!(lines.lock().unwrap().as_slice(), ["[Layout]Measure of 'Button' took '12.5'ms and ''"]);
    }

    #[test]
    fn formats_source() {
        let (sink, lines) = collecting_sink(LogEventLevel::Verbose, &[]);
        let source = 5i32;
        sink.log(LogEventLevel::Warning, LogArea::CONTROL, Some(&source), "Message");
        let lines = lines.lock().unwrap();
        assert!(lines[0].starts_with("[Control] Message (Object #"), "{}", lines[0]);
        assert!(lines[0].ends_with(')'));
    }

    #[test]
    fn disabled_events_are_not_logged() {
        let (sink, lines) = collecting_sink(LogEventLevel::Error, &[LogArea::LAYOUT]);
        sink.log(LogEventLevel::Warning, LogArea::LAYOUT, None, "low level");
        sink.log_with_values(LogEventLevel::Fatal, LogArea::BINDING, None, "other area {X}", &[&1]);
        assert!(lines.lock().unwrap().is_empty());
    }

    struct TestSink {
        events: RefCell<Vec<(LogEventLevel, String, String, Vec<String>)>>,
    }

    impl ILogSink for TestSink {
        fn is_enabled(&self, level: LogEventLevel, _area: &str) -> bool {
            level >= LogEventLevel::Warning
        }

        fn log(&self, level: LogEventLevel, area: &str, _source: Option<&dyn Any>, message_template: &str) {
            self.events.borrow_mut().push((level, area.to_string(), message_template.to_string(), Vec::new()));
        }

        fn log_with_values(
            &self,
            level: LogEventLevel,
            area: &str,
            _source: Option<&dyn Any>,
            message_template: &str,
            property_values: &[&dyn Display],
        ) {
            self.events.borrow_mut().push((
                level,
                area.to_string(),
                message_template.to_string(),
                property_values.iter().map(|v| v.to_string()).collect(),
            ));
        }
    }

    #[test]
    fn logger_uses_the_thread_sink() {
        let sink = Rc::new(TestSink { events: RefCell::new(Vec::new()) });
        let previous = Logger::set_thread_sink(Some(sink.clone()));

        assert!(Logger::is_enabled(LogEventLevel::Warning, LogArea::BINDING));
        assert!(!Logger::is_enabled(LogEventLevel::Debug, LogArea::BINDING));
        assert!(Logger::try_get(LogEventLevel::Debug, LogArea::BINDING).is_none());

        let logger = Logger::try_get(LogEventLevel::Error, LogArea::BINDING).unwrap();
        assert!(logger.is_valid());
        logger.log(None, "plain");
        logger.log_with_values(None, "value {A} {B}", &[&1, &"two"]);

        Logger::set_thread_sink(previous);
        assert!(Logger::thread_sink().is_none());

        let events = sink.events.borrow();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0], (LogEventLevel::Error, "Binding".to_string(), "plain".to_string(), vec![]));
        assert_eq!(events[1].3, vec!["1".to_string(), "two".to_string()]);
    }
}
