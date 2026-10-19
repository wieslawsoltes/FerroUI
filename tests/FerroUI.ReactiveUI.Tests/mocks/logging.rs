//! The logger of the tests (`ReactiveUI.Test.Utilities/Logging/TestLogger.cs`) and what
//! `LoggingRegistrationExecutor` sets up: a log manager, registered in the locator, whose loggers
//! all write to one [`TestLogger`].

use ferroui_splat::{AppLocator, FuncLogManager, IFullLogger, ILogManager, ILogger, LogLevel, WrappingFullLogger};
use std::cell::{Cell, RefCell};
use std::error::Error;
use std::rc::Rc;

/// A logger that keeps every message it is given (`TestLogger`).
pub struct TestLogger {
    /// `Messages`: the message, the name of the type that wrote it, and its level.
    messages: RefCell<Vec<(String, &'static str, LogLevel)>>,
    /// `Level`.
    level: Cell<LogLevel>,
}

impl TestLogger {
    /// A logger at the level `Debug` with no message.
    pub fn new() -> Rc<Self> {
        Rc::new(Self { messages: RefCell::new(Vec::new()), level: Cell::new(LogLevel::Debug) })
    }

    /// The messages written so far (`Messages`).
    pub fn messages(&self) -> Vec<(String, &'static str, LogLevel)> {
        self.messages.borrow().clone()
    }

    /// The number of messages written at `level`.
    pub fn count(&self, level: LogLevel) -> usize {
        self.messages.borrow().iter().filter(|(_, _, message_level)| *message_level == level).count()
    }

    /// Sets the level (`Level { set; }`).
    pub fn set_level(&self, level: LogLevel) {
        self.level.set(level);
    }
}

impl ILogger for TestLogger {
    fn level(&self) -> LogLevel {
        self.level.get()
    }

    fn write(&self, message: &str, log_level: LogLevel) {
        self.write_with_type(message, std::any::type_name::<TestLogger>(), log_level);
    }

    fn write_exception(&self, _exception: &dyn Error, message: &str, log_level: LogLevel) {
        self.write(message, log_level);
    }

    fn write_with_type(&self, message: &str, type_name: &'static str, log_level: LogLevel) {
        self.messages.borrow_mut().push((message.to_owned(), type_name, log_level));
    }

    fn write_exception_with_type(&self, _exception: &dyn Error, message: &str, _type_name: &'static str, log_level: LogLevel) {
        self.messages.borrow_mut().push((message.to_owned(), std::any::type_name::<TestLogger>(), log_level));
    }
}

/// Registers in the locator a log manager whose loggers write to a new [`TestLogger`], and returns
/// that logger (`LoggingRegistrationExecutor.ConfigureAppBuilder`). The locator is per thread, and
/// each test runs on its own thread.
pub fn register_test_logger() -> Rc<TestLogger> {
    let logger = TestLogger::new();
    let target = logger.clone();
    let manager: Rc<dyn ILogManager> = Rc::new(FuncLogManager::new(move |_| {
        let target: Rc<dyn ILogger> = target.clone();
        Rc::new(WrappingFullLogger::new(target)) as Rc<dyn IFullLogger>
    }));
    AppLocator::register::<dyn ILogManager>(move || Some(manager.clone()));
    logger
}
