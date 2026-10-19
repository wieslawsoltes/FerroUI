//! `SplatLoggerModule` and `SplatLogManagerModule` (`Splat.Logging/Builder`).

use super::{DebugLogger, DefaultLogManager, ILogManager, ILogger};
use crate::builder::IModule;
use crate::service_location::{
    IMutableDependencyResolver, IReadonlyDependencyResolver, MutableDependencyResolverExt,
};
use std::rc::Rc;

/// A module that registers a [`DebugLogger`] as `dyn ILogger` when none is registered
/// (`SplatLoggerModule`).
#[derive(Default)]
pub struct SplatLoggerModule;

impl SplatLoggerModule {
    /// The module.
    pub fn new() -> Self {
        Self
    }
}

impl IModule for SplatLoggerModule {
    fn configure(&self, resolver: &dyn IMutableDependencyResolver) {
        if resolver.has_registration::<dyn ILogger>() {
            return;
        }
        let debug_logger: Rc<dyn ILogger> = Rc::new(DebugLogger::new());
        resolver.register::<dyn ILogger>(move || Some(debug_logger.clone()));
    }
}

/// A module that registers a [`DefaultLogManager`] over a resolver as `dyn ILogManager` when none
/// is registered (`SplatLogManagerModule`).
pub struct SplatLogManagerModule {
    current: Rc<dyn IReadonlyDependencyResolver>,
}

impl SplatLogManagerModule {
    /// The module, whose log manager takes its loggers from `current`.
    pub fn new(current: Rc<dyn IReadonlyDependencyResolver>) -> Self {
        Self { current }
    }
}

impl IModule for SplatLogManagerModule {
    fn configure(&self, resolver: &dyn IMutableDependencyResolver) {
        if resolver.has_registration::<dyn ILogManager>() {
            return;
        }
        let current = self.current.clone();
        resolver.register::<dyn ILogManager>(move || {
            Some(Rc::new(DefaultLogManager::new(current.clone())) as Rc<dyn ILogManager>)
        });
    }
}
