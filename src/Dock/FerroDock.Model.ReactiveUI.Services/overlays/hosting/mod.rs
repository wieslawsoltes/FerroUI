//! Source host overlay service composition and resolution.
mod host_overlay_services_adapter;
pub use host_overlay_services_adapter::HostOverlayServicesAdapter;
mod host_type_registry;
pub use host_type_registry::{HostServiceValue, HostTypeRegistry};
mod i_host_service_resolver;
pub use i_host_service_resolver::{HostServiceResolverExt, IHostServiceResolver};
mod owner_chain_host_service_resolver;
pub use owner_chain_host_service_resolver::OwnerChainHostServiceResolver;
mod i_host_overlay_services_provider;
pub use i_host_overlay_services_provider::IHostOverlayServicesProvider;
mod host_overlay_services_provider;
pub use host_overlay_services_provider::{
    HostOverlayServicesFactory, HostOverlayServicesFactoryFn, HostOverlayServicesProvider,
};
#[cfg(test)]
pub(crate) mod hosting_test_support;
#[cfg(test)]
mod hosting_tests;
