//! Ported from `Splat.Tests/ServiceLocation` of Splat.
//!
//! The tests of the internals of the global and instance resolvers (`ArrayHelpers`, `Container`,
//! `ContractContainer`, `Registration`, `ServiceTypeRegistry`, `ContainerCache`,
//! `ContractContainerCache`, `ResolverState`, `ServiceTypeRegistryCache`) are in the crate itself
//! (`src/FerroUI.Splat/tests/service_location`).

mod app_locator_tests;
mod base_dependency_resolver_tests;
mod common;
mod dependency_resolver_mixins_tests;
mod func_dependency_resolver_tests;
mod global_generic_first;
mod instance_generic_first;
mod internal_locator_tests;
mod modern_dependency_resolver_tests;
mod nested_resolution_probe;
mod null_service_type_tests;
mod resolver_mixins_tests;
mod with_resolver_flow_isolation_tests;
