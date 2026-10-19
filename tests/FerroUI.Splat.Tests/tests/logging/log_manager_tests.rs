//! Ported from `Splat.Logging.Tests/LogManagerTests.cs` of Splat.
//! Adapted: `GetLogger(typeof(T))` is `get_logger(std::any::type_name::<T>())` and `GetLogger<T>()`
//! is `get_logger_for::<T>()`. `IsTypeOf<WrappingFullLogger>` on a `Rc<dyn IFullLogger>` is not
//! checkable: the tests check what the logger writes instead. The resolver registers one shared
//! `TextLogger` (upstream a new one per resolution) so that the test can read it; the logger of
//! `DefaultLogManager` writes to it with the short name of the type in front. The type of the cache
//! of the log manager is `MemoizingMRUCache<&'static str, Rc<dyn IFullLogger>>` (upstream
//! `MemoizingMRUCache<Type, IFullLogger>`); its logger is a full logger over a `NullLogger`.

use crate::tests::common::{panic_payload, same, IMockLogTarget, TextLogger};
use ferroui_splat::{
    DefaultLogManager, FuncLogManager, IFullLogger, ILogManager, ILogger, LogLevel, LogManagerExt,
    LoggingException, MemoizingMRUCache, ModernDependencyResolver, MutableDependencyResolverExt,
    WrappingFullLogger,
};
use std::cell::Cell;
use std::rc::Rc;

/// The type the loggers are for (`typeof(LogManagerTests)`).
#[allow(dead_code)] // only named as a type
struct LogManagerTests;

fn resolver_with_logger(text_logger: &Rc<TextLogger>) -> Rc<ModernDependencyResolver> {
    let resolver = Rc::new(ModernDependencyResolver::new());
    let text_logger = text_logger.clone();
    resolver.register::<dyn ILogger>(move || Some(text_logger.clone() as Rc<dyn ILogger>));
    resolver
}

#[test]
fn default_log_manager_should_create_wrapping_full_logger() {
    let text_logger = Rc::new(TextLogger::new());
    let resolver = resolver_with_logger(&text_logger);
    let log_manager = DefaultLogManager::new(resolver);
    let logger = log_manager.get_logger(std::any::type_name::<LogManagerTests>());

    logger.info("message");

    assert_eq!(text_logger.logs(), vec![(LogLevel::Info, "LogManagerTests: message".to_owned())]);
}

#[test]
fn func_log_manager_should_use_factory() {
    const EXPECTED_FACTORY_CALL_COUNT: i32 = 2;
    let call_count = Rc::new(Cell::new(0));
    let text_logger = Rc::new(TextLogger::new());
    let counter = call_count.clone();
    let target = text_logger.clone();
    let log_manager = FuncLogManager::new(move |_| {
        counter.set(counter.get() + 1);
        Rc::new(WrappingFullLogger::new(target.clone())) as Rc<dyn IFullLogger>
    });

    let logger1 = log_manager.get_logger(std::any::type_name::<LogManagerTests>());
    let logger2 = log_manager.get_logger_for::<LogManagerTests>();

    assert_eq!(call_count.get(), EXPECTED_FACTORY_CALL_COUNT);
    logger1.debug("first");
    logger2.debug("second");
    assert_eq!(
        text_logger.logs(),
        vec![(LogLevel::Debug, "first".to_owned()), (LogLevel::Debug, "second".to_owned())]
    );
}

#[test]
fn log_manager_mixin_should_create_full_logger() {
    let text_logger = Rc::new(TextLogger::new());
    let target = text_logger.clone();
    let log_manager =
        FuncLogManager::new(move |_| Rc::new(WrappingFullLogger::new(target.clone())) as Rc<dyn IFullLogger>);
    let logger = log_manager.get_logger_for::<LogManagerTests>();

    logger.warn("message");

    assert_eq!(text_logger.logs(), vec![(LogLevel::Warn, "message".to_owned())]);
}

#[test]
fn default_log_manager_should_return_shared_null_logger_for_cache_type() {
    let text_logger = Rc::new(TextLogger::new());
    let resolver = resolver_with_logger(&text_logger);
    let log_manager = DefaultLogManager::new(resolver);
    let cache_type = std::any::type_name::<MemoizingMRUCache<&'static str, Rc<dyn IFullLogger>>>();

    let first = log_manager.get_logger(cache_type);
    let second = log_manager.get_logger(cache_type);

    assert!(same(&first, &second));
    first.fatal("message");
    assert!(text_logger.logs().is_empty());
}

#[test]
fn default_log_manager_should_throw_when_resolver_has_no_logger() {
    let resolver = Rc::new(ModernDependencyResolver::new());
    let log_manager = DefaultLogManager::new(resolver);

    let payload = panic_payload(|| {
        let _ = log_manager.get_logger(std::any::type_name::<LogManagerTests>());
    })
    .expect("get_logger panics without a logger");
    assert!(payload.downcast_ref::<LoggingException>().is_some());
}
