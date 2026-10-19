//! Ported from `Splat.Logging.Tests/LogHostCoverageTests.cs` of Splat.
//! Adapted: `[NotInParallel]` is not needed, the locator is per thread; the `AppLocatorScope` is
//! kept. `LogHost.Default` is `LogHost::default_logger()` and `this.Log()` is the provided
//! `IEnableLogger::log`. `IsTypeOf<WrappingFullLogger>` on the logger is not checkable on a
//! `Rc<dyn IFullLogger>`: the test shares the `TextLogger` of the log manager and checks that the
//! full logger writes to it. `Throws<LoggingException>` checks the payload of the panic;
//! `Throws<InvalidOperationException>` is a panic with the plain message of upstream.

use crate::tests::common::{panic_message, panic_payload, panics, AppLocatorScope, IMockLogTarget, TextLogger};
use ferroui_splat::{
    AppLocator, FuncLogManager, IEnableLogger, IFullLogger, ILogManager, IStaticFullLogger, LogHost,
    LogLevel, LoggingException, WrappingFullLogger,
};
use std::rc::Rc;

/// The test class, which logs (`LogHostCoverageTests : IEnableLogger`).
struct LogHostCoverageTests;

impl IEnableLogger for LogHostCoverageTests {}

fn register_log_manager(text_logger: Rc<TextLogger>) {
    AppLocator::register::<dyn ILogManager>(move || {
        let text_logger = text_logger.clone();
        Some(Rc::new(FuncLogManager::new(move |_| {
            Rc::new(WrappingFullLogger::new(text_logger.clone())) as Rc<dyn IFullLogger>
        })) as Rc<dyn ILogManager>)
    });
}

#[test]
fn default_returns_static_full_logger() {
    let _scope = AppLocatorScope::new();
    register_log_manager(Rc::new(TextLogger::new()));

    let logger = LogHost::default_logger();

    assert_eq!(logger.level(), LogLevel::Debug);
}

#[test]
fn default_can_write_without_throwing() {
    let _scope = AppLocatorScope::new();
    let text_logger = Rc::new(TextLogger::new());
    register_log_manager(text_logger.clone());

    assert!(!panics(|| LogHost::default_logger().info("hello", Some("Default_Can_Write_Without_Throwing"))));
    assert_eq!(text_logger.logs().len(), 1);
}

#[test]
fn default_without_log_manager_throws_logging_exception() {
    let _scope = AppLocatorScope::new();
    AppLocator::unregister_all::<dyn ILogManager>();

    let payload = panic_payload(|| {
        let _ = LogHost::default_logger();
    })
    .expect("LogHost::default_logger panics without a log manager");
    assert!(payload.downcast_ref::<LoggingException>().is_some());
}

#[test]
fn log_extension_returns_full_logger() {
    let _scope = AppLocatorScope::new();
    let text_logger = Rc::new(TextLogger::new());
    register_log_manager(text_logger.clone());

    let logger = LogHostCoverageTests.log();
    logger.info("hello");

    assert_eq!(text_logger.logs(), vec![(LogLevel::Info, "hello".to_owned())]);
}

#[test]
fn log_extension_without_log_manager_throws_invalid_operation_exception() {
    let _scope = AppLocatorScope::new();
    AppLocator::unregister_all::<dyn ILogManager>();

    let payload = panic_payload(|| {
        let _ = LogHostCoverageTests.log();
    })
    .expect("log panics without a log manager");
    assert!(payload.downcast_ref::<LoggingException>().is_none());
    assert!(panic_message(payload.as_ref()).contains("ILogManager is null"));
}
