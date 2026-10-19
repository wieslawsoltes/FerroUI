//! `ServiceLocationInitializationExtensions`: the default services of Splat.

use super::app_locator::AppLocator;
use super::dependency_resolver::{IMutableDependencyResolver, MutableDependencyResolverExt};
use crate::application_performance_monitoring::{
    DefaultFeatureUsageTrackingManager, IFeatureUsageTrackingManager,
};
use crate::logging::{DebugLogger, DefaultLogManager, ILogManager, ILogger};
use std::rc::Rc;

/// Registers the default services of Splat that are not registered yet (`InitializeSplat`): a
/// [`DefaultLogManager`] over the current resolver as `dyn ILogManager`, a [`DebugLogger`] as
/// `dyn ILogger` and a [`DefaultFeatureUsageTrackingManager`] as
/// `dyn IFeatureUsageTrackingManager`.
pub fn initialize_splat<R: IMutableDependencyResolver + ?Sized>(resolver: &R) {
    register_default_log_manager(resolver);
    register_logger(resolver);
    register_application_performance_monitoring(resolver);
}

fn register_application_performance_monitoring<R: IMutableDependencyResolver + ?Sized>(resolver: &R) {
    if resolver.has_registration::<dyn IFeatureUsageTrackingManager>() {
        return;
    }
    resolver.register_constant::<dyn IFeatureUsageTrackingManager>(Rc::new(
        DefaultFeatureUsageTrackingManager::new(),
    ));
}

fn register_default_log_manager<R: IMutableDependencyResolver + ?Sized>(resolver: &R) {
    if resolver.has_registration::<dyn ILogManager>() {
        return;
    }
    resolver.register::<dyn ILogManager>(|| {
        Some(Rc::new(DefaultLogManager::new(AppLocator::current())) as Rc<dyn ILogManager>)
    });
}

fn register_logger<R: IMutableDependencyResolver + ?Sized>(resolver: &R) {
    if resolver.has_registration::<dyn ILogger>() {
        return;
    }
    resolver.register_constant::<dyn ILogger>(Rc::new(DebugLogger::new()));
}

/// `InitializeSplat` as a method of every resolver.
pub trait ServiceLocationInitializationExt {
    /// `InitializeSplat`.
    fn initialize_splat(&self);
}

impl<R: IMutableDependencyResolver + ?Sized> ServiceLocationInitializationExt for R {
    fn initialize_splat(&self) {
        initialize_splat(self);
    }
}
