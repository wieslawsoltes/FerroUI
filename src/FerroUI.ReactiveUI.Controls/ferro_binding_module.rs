//! `FerroBindingModule` (ReactiveUI.Binding's integration, `...BindingModule.cs`).

use crate::{FerroCreatesCommandBinding, FerroObjectObservableForProperty, FerroViewThreadInvoker};
use ferroui_reactiveui::{ICreatesCommandBinding, ICreatesObservableForProperty, IViewThreadInvoker};
use ferroui_splat::{IModule, IMutableDependencyResolver, MutableDependencyResolverExt};
use std::rc::Rc;

/// Registers the property observer of FerroUI objects, the command binder of FerroUI controls and
/// the view thread invoker with the dependency resolver (`FerroBindingModule`).
#[derive(Debug, Default, Clone, Copy)]
pub struct FerroBindingModule;

impl FerroBindingModule {
    /// The module (`new FerroBindingModule()`).
    pub fn new() -> Self {
        Self
    }
}

impl IModule for FerroBindingModule {
    /// Registers [`FerroObjectObservableForProperty`], [`FerroCreatesCommandBinding`] and the shared
    /// [`FerroViewThreadInvoker`] as lazy singletons (`Configure`).
    fn configure(&self, resolver: &dyn IMutableDependencyResolver) {
        register_binding_integration(resolver);
    }
}

/// Registers what [`FerroBindingModule`] registers with `resolver`: the property observer of
/// FerroUI objects (a lazy singleton of `dyn ICreatesObservableForProperty`), the command binder of
/// FerroUI controls (`dyn ICreatesCommandBinding`) and the shared view thread invoker
/// (`dyn IViewThreadInvoker`).
// Deviation (DEVIATIONS.md, ReactiveUI controls): the null check of the resolver has no
// counterpart; the body of `Configure` is also a function, for the builder of the application.
pub fn register_binding_integration(resolver: &dyn IMutableDependencyResolver) {
    resolver.register_lazy_singleton::<dyn ICreatesObservableForProperty>(|| {
        Some(Rc::new(FerroObjectObservableForProperty::new()) as Rc<dyn ICreatesObservableForProperty>)
    });
    resolver.register_lazy_singleton::<dyn ICreatesCommandBinding>(|| {
        Some(Rc::new(FerroCreatesCommandBinding::new()) as Rc<dyn ICreatesCommandBinding>)
    });
    resolver.register_lazy_singleton::<dyn IViewThreadInvoker>(|| Some(FerroViewThreadInvoker::instance() as Rc<dyn IViewThreadInvoker>));
}
