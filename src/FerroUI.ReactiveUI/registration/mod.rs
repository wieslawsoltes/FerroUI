//! The registration of ReactiveUI's services (`ReactiveUI.Core/Registration`,
//! `ReactiveUI.Shared/Registration`), with the interfaces it uses (`IRegistrar`,
//! `IWantsToRegisterStuff`, `IPlatformOperations`) and the registration of views
//! (`MutableDependencyResolverExtensions`, `MutableDependencyResolverAOTExtensions`).
//!
//! Not ported: `DependencyResolverMixins` (`ReactiveUI.Shared/Mixins/DependencyResolverMixins.cs`),
//! whose `RegisterViewsForViewModels(Assembly)` and `TypeFactory(TypeInfo)` scan an assembly and
//! create the views by reflection. Views are registered by type with
//! [`MutableDependencyResolverExtensions`] or the builder.

mod dependency_resolver_registrar;
mod i_platform_operations;
mod i_registrar;
mod i_wants_to_register_stuff;
mod mutable_dependency_resolver_aot_extensions;
mod mutable_dependency_resolver_extensions;
mod platform_registrations;
mod registrations;

pub use dependency_resolver_registrar::DependencyResolverRegistrar;
pub use i_platform_operations::IPlatformOperations;
pub use i_registrar::{IRegistrar, RegistrarExt};
pub use i_wants_to_register_stuff::IWantsToRegisterStuff;
#[doc(hidden)]
pub use mutable_dependency_resolver_aot_extensions::MutableDependencyResolverAotExtensions;
pub use mutable_dependency_resolver_extensions::MutableDependencyResolverExtensions;
pub use platform_registrations::PlatformRegistrations;
pub use registrations::Registrations;
