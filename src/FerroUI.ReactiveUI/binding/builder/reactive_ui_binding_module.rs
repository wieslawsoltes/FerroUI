//! `ReactiveUIBindingModule`.

use crate::binding::interfaces::ICreatesObservableForProperty;
use crate::binding::observable_for_property::{INPCObservableForProperty, POCOObservableForProperty};
use ferroui_splat::{IModule, IMutableDependencyResolver, MutableDependencyResolverExt};
use std::rc::Rc;

/// Registers the property observers of the objects that raise `PropertyChanged` and of plain
/// objects (`ReactiveUIBindingModule`).
///
/// The observers of a platform (FerroUI's objects) are registered by its own module.
#[derive(Debug, Default, Clone, Copy)]
pub struct ReactiveUIBindingModule;

impl IModule for ReactiveUIBindingModule {
    /// Registers [`INPCObservableForProperty`] and [`POCOObservableForProperty`] as lazy singletons
    /// of `dyn ICreatesObservableForProperty` (`Configure`).
    fn configure(&self, resolver: &dyn IMutableDependencyResolver) {
        register_observation_defaults(resolver);
    }
}

/// Registers the default property observers with `resolver`, as [`ReactiveUIBindingModule`] does:
/// [`INPCObservableForProperty`] and [`POCOObservableForProperty`], lazy singletons of
/// `dyn ICreatesObservableForProperty`. A builder calls it, or a test with
/// `register_observation_defaults(&*AppLocator::current_mutable())`.
pub fn register_observation_defaults(resolver: &dyn IMutableDependencyResolver) {
    resolver.register_lazy_singleton::<dyn ICreatesObservableForProperty>(|| {
        Some(Rc::new(INPCObservableForProperty::new()) as Rc<dyn ICreatesObservableForProperty>)
    });
    resolver.register_lazy_singleton::<dyn ICreatesObservableForProperty>(|| {
        Some(Rc::new(POCOObservableForProperty::new()) as Rc<dyn ICreatesObservableForProperty>)
    });
}
