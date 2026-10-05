//! Logging: a process-wide sink that the framework writes diagnostic events
//! to, filtered by level and area.

mod i_log_sink;
mod log_area;
mod log_event_level;
mod logger;
mod parametrized_logger;
mod string_log_sink;

pub use i_log_sink::ILogSink;
pub use log_area::LogArea;
pub use log_event_level::LogEventLevel;
pub use logger::Logger;
pub use parametrized_logger::ParametrizedLogger;
pub use string_log_sink::StringLogSink;
