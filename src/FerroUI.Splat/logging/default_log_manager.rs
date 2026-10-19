//! `DefaultLogManager`.

use super::{
    IFullLogger, ILogManager, ILogger, LoggingException, MemoizingMRUCache, NullLogger,
    WrappingFullLogger, WrappingPrefixLogger,
};
use crate::service_location::{IReadonlyDependencyResolver, ReadonlyDependencyResolverExt};
use std::rc::Rc;

const LOGGER_CACHE_SIZE: usize = 64;

/// The log manager Splat registers (`DefaultLogManager`): the logger of a type is the `dyn ILogger`
/// of the resolver, with the name of the type in front of its messages, wrapped in a full logger.
/// The last 64 loggers are cached.
pub struct DefaultLogManager {
    logger_cache: MemoizingMRUCache<&'static str, Rc<dyn IFullLogger>>,
    null_logger: Rc<dyn IFullLogger>,
}

impl DefaultLogManager {
    /// A log manager over the loggers of `dependency_resolver`.
    ///
    /// Getting a logger panics with a [`LoggingException`] when the resolver has no `dyn ILogger`.
    pub fn new(dependency_resolver: Rc<dyn IReadonlyDependencyResolver>) -> Self {
        let logger_cache = MemoizingMRUCache::new(
            move |type_name: &&'static str, _context| -> Rc<dyn IFullLogger> {
                match dependency_resolver.get_service::<dyn ILogger>() {
                    None => std::panic::panic_any(LoggingException::with_message(
                        "Couldn't find an ILogger. This should never happen, your dependency resolver is probably broken.",
                    )),
                    Some(logger) => Rc::new(WrappingFullLogger::new(Rc::new(WrappingPrefixLogger::new(
                        logger, *type_name,
                    )))),
                }
            },
            LOGGER_CACHE_SIZE,
        );
        Self { logger_cache, null_logger: Rc::new(WrappingFullLogger::new(Rc::new(NullLogger::new()))) }
    }
}

impl ILogManager for DefaultLogManager {
    fn get_logger(&self, type_name: &'static str) -> Rc<dyn IFullLogger> {
        // The cache's own logger would log through the cache.
        if type_name == std::any::type_name::<MemoizingMRUCache<&'static str, Rc<dyn IFullLogger>>>() {
            return self.null_logger.clone();
        }
        self.logger_cache.get(type_name)
    }
}
