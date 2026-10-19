//! `ILogManager`, `LogManagerExtensions` and `FuncLogManager`.

use super::IFullLogger;
use std::rc::Rc;

/// Hands out the logger of a type (`ILogManager`).
pub trait ILogManager {
    /// The logger for the type named `type_name` (`GetLogger(Type)`).
    fn get_logger(&self, type_name: &'static str) -> Rc<dyn IFullLogger>;
}

/// The generic form of [`ILogManager::get_logger`] (`LogManagerExtensions`).
pub trait LogManagerExt: ILogManager {
    /// The logger for the type `T` (`GetLogger<T>()`).
    fn get_logger_for<T: ?Sized>(&self) -> Rc<dyn IFullLogger> {
        self.get_logger(std::any::type_name::<T>())
    }
}

impl<M: ILogManager + ?Sized> LogManagerExt for M {}

/// A log manager over a function (`FuncLogManager`).
pub struct FuncLogManager {
    get_logger_func: Box<dyn Fn(&'static str) -> Rc<dyn IFullLogger>>,
}

impl FuncLogManager {
    /// A log manager whose loggers `get_logger_func` returns.
    pub fn new(get_logger_func: impl Fn(&'static str) -> Rc<dyn IFullLogger> + 'static) -> Self {
        Self { get_logger_func: Box::new(get_logger_func) }
    }
}

impl ILogManager for FuncLogManager {
    fn get_logger(&self, type_name: &'static str) -> Rc<dyn IFullLogger> {
        (self.get_logger_func)(type_name)
    }
}
