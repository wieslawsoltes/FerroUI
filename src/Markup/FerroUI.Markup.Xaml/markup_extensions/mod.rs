//! The markup extensions of the framework.

pub mod compiled_bindings;

mod compiled_binding_extension;
mod dynamic_resource_extension;
mod on;
mod on_form_factor_extension;
mod on_platform_extension;
mod reflection_binding_extension;
mod relative_source_extension;
mod resolve_by_name_extension;
mod static_resource_extension;

pub use compiled_binding_extension::CompiledBindingExtension;
pub use dynamic_resource_extension::DynamicResourceExtension;
pub use on::{AddChildOfOn, On};
pub use on_form_factor_extension::{OnFormFactorExtension, OnFormFactorExtensionBase, OnFormFactorExtensionOf};
pub use on_platform_extension::{OnPlatformExtension, OnPlatformExtensionBase, OnPlatformExtensionOf};
pub use reflection_binding_extension::ReflectionBindingExtension;
pub use relative_source_extension::RelativeSourceExtension;
pub use resolve_by_name_extension::ResolveByNameExtension;
pub use static_resource_extension::StaticResourceExtension;

#[cfg(test)]
mod markup_extensions_tests;
