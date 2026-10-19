//! Service location (`Splat.Core/ServiceLocation` and `Splat/ServiceLocation`): the interfaces of
//! a dependency resolver, the three resolvers of Splat, the locator that holds the current one
//! ([`AppLocator`], [`Locator`]) and the mixins over them.
//!
//! A service is keyed by the type of the service, `S: ?Sized + 'static`, which may be a trait
//! object, and an optional contract; a registration is a factory that returns `Option<Rc<S>>`. The
//! interfaces are object safe, so that a resolver is held as `Rc<dyn IDependencyResolver>`: their
//! methods take the type as a [`ServiceType`] and the service type-erased ([`ErasedService`]). The
//! generic API callers use (`register::<S>`, `get_service::<S>`, ...) is in the extension traits
//! [`ReadonlyDependencyResolverExt`] and [`MutableDependencyResolverExt`], implemented for every
//! resolver.

mod app_locator;
pub(crate) mod common;
mod dependency_resolver;
mod dependency_resolver_mixins;
pub(crate) mod global_generic_first;
pub(crate) mod instance_generic_first;
mod internal_locator;
mod locator;
mod modern_dependency_resolver;
mod resolver_mixins;
mod service_location_initialization;
mod service_type;

pub use app_locator::AppLocator;
pub use dependency_resolver::{
    IDependencyResolver, IMutableDependencyResolver, IReadonlyDependencyResolver,
    MutableDependencyResolverExt, ReadonlyDependencyResolverExt,
};
pub use dependency_resolver_mixins::DependencyResolverMixins;
pub use global_generic_first::GlobalGenericFirstDependencyResolver;
pub use instance_generic_first::InstanceGenericFirstDependencyResolver;
#[doc(hidden)]
pub use internal_locator::InternalLocator;
pub use locator::Locator;
pub use modern_dependency_resolver::ModernDependencyResolver;
pub use resolver_mixins::ResolverMixins;
pub use service_location_initialization::{initialize_splat, ServiceLocationInitializationExt};
pub use service_type::{
    erase_service, unerase_service, ErasedService, LazyDisposal, RegistrationCallback,
    ServiceFactory, ServiceType,
};
