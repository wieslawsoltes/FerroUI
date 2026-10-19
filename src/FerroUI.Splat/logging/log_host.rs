//! `LogHost`, `IEnableLogger` and `LogHostExtensions`.

use super::{IFullLogger, ILogManager, LoggingException, StaticFullLogger};
use crate::service_location::{AppLocator, ReadonlyDependencyResolverExt};
use std::rc::Rc;

/// The loggers of the application, from the `dyn ILogManager` of the current resolver of
/// [`AppLocator`] (`LogHost`).
pub struct LogHost;

impl LogHost {
    /// A static logger for `LogHost` (`LogHost.Default`).
    ///
    /// Panics with a [`LoggingException`] when the current resolver has no log manager.
    pub fn default_logger() -> StaticFullLogger {
        let Some(factory) = AppLocator::current().get_service::<dyn ILogManager>() else {
            std::panic::panic_any(LoggingException::with_message(
                "ILogManager is null. This should never happen, your dependency resolver is broken",
            ));
        };
        let full_logger = factory.get_logger(std::any::type_name::<LogHost>());
        StaticFullLogger::new(full_logger)
    }
}

/// A type that logs through the logger of its type (`IEnableLogger`).
///
/// `this.Log()` of upstream is the provided [`log`](Self::log).
pub trait IEnableLogger {
    /// The logger of the type, from the `dyn ILogManager` of the current resolver of
    /// [`AppLocator`] (`LogHostExtensions.Log`).
    ///
    /// Panics when the current resolver has no log manager (`InvalidOperationException`).
    fn log(&self) -> Rc<dyn IFullLogger> {
        let Some(factory) = AppLocator::current().get_service::<dyn ILogManager>() else {
            panic!("ILogManager is null. This should never happen, your dependency resolver is broken");
        };
        factory.get_logger(std::any::type_name::<Self>())
    }
}
