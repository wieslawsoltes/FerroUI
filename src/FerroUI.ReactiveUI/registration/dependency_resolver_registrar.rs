//! `DependencyResolverRegistrar` (`ReactiveUI.Core/Registration/DependencyResolverRegistrar.cs`).

use super::IRegistrar;
use ferroui_splat::service_location::{ErasedService, ServiceFactory, ServiceType};
use ferroui_splat::IMutableDependencyResolver;
use std::fmt;
use std::rc::Rc;

/// The [`IRegistrar`] that forwards every registration to a Splat resolver
/// (`DependencyResolverRegistrar`).
pub struct DependencyResolverRegistrar {
    /// `_resolver`.
    resolver: Rc<dyn IMutableDependencyResolver>,
}

impl fmt::Debug for DependencyResolverRegistrar {
    /// `[DebuggerDisplay("DependencyResolverRegistrar")]`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("DependencyResolverRegistrar")
    }
}

impl DependencyResolverRegistrar {
    /// A registrar of `resolver` (`DependencyResolverRegistrar(IMutableDependencyResolver)`).
    pub fn new(resolver: Rc<dyn IMutableDependencyResolver>) -> Self {
        Self { resolver }
    }
}

impl IRegistrar for DependencyResolverRegistrar {
    fn register_constant_erased(&self, factory: Box<dyn FnOnce() -> ErasedService>, service_type: ServiceType, contract: Option<&str>) {
        self.resolver.register_constant_erased(Some(factory()), None, service_type, contract);
    }

    fn register_lazy_singleton_erased(&self, factory: ServiceFactory, service_type: ServiceType, contract: Option<&str>) {
        self.resolver.register_lazy_singleton_erased(factory, None, service_type, contract);
    }

    fn register_erased(&self, factory: ServiceFactory, service_type: ServiceType, contract: Option<&str>) {
        self.resolver.register_erased(factory, service_type, contract);
    }
}
