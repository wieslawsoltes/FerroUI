//! Ported from `Splat.Logging.Tests/WrappingFullLoggers/ConsoleLoggerTests.cs` of Splat.
//! Adapted: the class inherits `FullLoggerTestBase` upstream, and so the tests of its whole chain
//! of bases; here the macro of each base of the chain (`logger_base!`,
//! `allocate_free_error_logger_test_base!`, `allocation_free_logger_base_test_base!`,
//! `full_logger_test_base!`) is invoked in a module of its own with `get_logger`, the port of
//! `GetLogger`. The logger is a `WrappingFullLogger` over a `WrappingLogLevelLogger` over a
//! `ConsoleLogger` at the level of the test, with the exception format `{0} {1}`. The
//! `ConsoleWriter` of upstream (a `TextWriter` that parses each line `Level: message`) is a
//! `std::io::Write` that collects the bytes, splits them into lines and parses each line; the
//! entries it records are shared with the `IMockLogTarget` that `get_logger` returns. A line
//! without a colon, or whose prefix is not the name of a level, is ignored, as upstream. The class
//! has no tests of its own. `wrapping_full_logger_test_base!` is not invoked: upstream does not
//! inherit it here, and its expectations of messages of a type do not hold for this logger (see
//! `wrapping_full_logger_test_base`).

use crate::tests::common::IMockLogTarget;
use ferroui_splat::{
    ConsoleLogger, IFullLogger, LogLevel, WrappingFullLogger, WrappingLogLevelLogger,
};
use std::cell::RefCell;
use std::rc::Rc;

/// The entries the writer recorded: the level and the message of each line.
type Logs = Rc<RefCell<Vec<(LogLevel, String)>>>;

/// A writer that captures the output of the console logger (`ConsoleWriter`).
struct ConsoleWriter {
    pending: Vec<u8>,
    logs: Logs,
}

impl ConsoleWriter {
    fn new(logs: Logs) -> Self {
        Self { pending: Vec::new(), logs }
    }

    /// Records the line `Level: message` (`WriteLine(string?)`).
    fn write_line(&self, value: &str) {
        if value.is_empty() {
            return;
        }
        let Some(colon_index) = value.find(':') else {
            // No colon: not a properly formatted log message.
            return;
        };
        let Some(level) = parse_level(&value[..colon_index]) else {
            // Not the name of a level: an improperly formatted line, ignored.
            return;
        };
        let message = value[colon_index + 1..].trim().to_owned();
        self.logs.borrow_mut().push((level, message));
    }
}

/// `Enum.Parse<LogLevel>` of the name of a level.
fn parse_level(name: &str) -> Option<LogLevel> {
    match name {
        "Debug" => Some(LogLevel::Debug),
        "Info" => Some(LogLevel::Info),
        "Warn" => Some(LogLevel::Warn),
        "Error" => Some(LogLevel::Error),
        "Fatal" => Some(LogLevel::Fatal),
        _ => None,
    }
}

impl std::io::Write for ConsoleWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.pending.extend_from_slice(buf);
        while let Some(index) = self.pending.iter().position(|&byte| byte == b'\n') {
            let line: Vec<u8> = self.pending.drain(..=index).collect();
            let line = String::from_utf8_lossy(&line);
            self.write_line(line.trim_end_matches(['\r', '\n']));
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// The `IMockLogTarget` side of the `ConsoleWriter`: the entries it recorded (`Logs`).
struct ConsoleTarget {
    logs: Logs,
}

impl IMockLogTarget for ConsoleTarget {
    fn logs(&self) -> Vec<(LogLevel, String)> {
        self.logs.borrow().clone()
    }
}

/// The logger to test and its target, for `minimum_log_level` (`GetLogger`).
fn get_logger(minimum_log_level: LogLevel) -> (Rc<dyn IFullLogger>, Rc<dyn IMockLogTarget>) {
    let logs: Logs = Rc::new(RefCell::new(Vec::new()));
    let console = ConsoleLogger::with_writer(Box::new(ConsoleWriter::new(logs.clone())));
    console.set_level(minimum_log_level);
    console.set_exception_message_format("{0} {1}");
    let logger = WrappingFullLogger::new(Rc::new(WrappingLogLevelLogger::new(Rc::new(console))));
    let target = ConsoleTarget { logs };
    (Rc::new(logger) as Rc<dyn IFullLogger>, Rc::new(target) as Rc<dyn IMockLogTarget>)
}

/// The tests of `LoggerBase<T>`.
mod logger_base_tests {
    use crate::tests::logging::base_tests::logger_base::logger_base;

    logger_base!(super::get_logger);
}

/// The tests of `AllocateFreeErrorLoggerTestBase<T>`.
mod allocate_free_error_logger_tests {
    use crate::tests::logging::base_tests::allocate_free_error_logger_test_base as base;

    base::allocate_free_error_logger_test_base!(super::get_logger);
}

/// The tests of `AllocationFreeLoggerBaseTestBase<T>`.
mod allocation_free_logger_base_tests {
    use crate::tests::logging::base_tests::allocation_free_logger_base_test_base as base;

    base::allocation_free_logger_base_test_base!(super::get_logger);
}

/// The tests of `FullLoggerTestBase`.
mod full_logger_tests {
    use crate::tests::logging::base_tests::full_logger_test_base::full_logger_test_base;

    full_logger_test_base!(super::get_logger);
}
