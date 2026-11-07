//! ferroui-markup-xaml
//!
//! The XAML runtime library: what compiled and interpreted markup documents
//! call into. Markup extensions, templates, includes, type converters, the
//! runtime helpers of the XAML compiler (service providers, deferred content,
//! parent stacks) and the loader entry point.

pub mod converters;
pub mod data;
pub mod diagnostics;
pub mod markup_extensions;
pub mod parsers;
pub mod styling;
pub mod templates;
pub mod xaml_il;
pub mod xamlx_runtime;

mod eager_parent_stack_enumerator;
mod extensions;
mod ferro_xaml_loader;
mod populate_overrides;
#[cfg(feature = "source-overlays")]
pub mod source_overlay;
#[cfg(all(feature = "hot-reload", debug_assertions))]
pub mod reload_validation;
#[cfg(all(feature = "hot-reload", debug_assertions))]
pub mod reload_resources;
#[cfg(all(feature = "hot-reload", debug_assertions))]
pub mod reload_events;
mod markup_extension;
mod object_casts;
mod register_types;
mod rust_paths;
mod runtime_xaml_loader_configuration;
mod runtime_xaml_loader_document;
#[doc(hidden)]
pub mod xaml_error_scope;
mod xaml_load_exception;
mod xaml_types;

pub use eager_parent_stack_enumerator::EagerParentStackEnumerator;
pub use extensions::ServiceProviderExtensions;
pub use populate_overrides::{PopulateOverride, PopulateOverrideGuard};
pub use ferro_xaml_loader::{CompiledXamlLoader, FerroXamlLoader, IRuntimeXamlLoader};
pub use markup_extension::MarkupExtension;
pub use object_casts::{FromXamlObject, XamlResourceNode};
pub use register_types::{register_types, ASSEMBLY};
pub use runtime_xaml_loader_configuration::{
    RuntimeXamlDiagnostic, RuntimeXamlDiagnosticSeverity, RuntimeXamlLoaderConfiguration, XamlDiagnosticFunc,
};
pub use runtime_xaml_loader_document::RuntimeXamlLoaderDocument;
pub use xaml_load_exception::XamlLoadError;
#[allow(deprecated)]
pub use xaml_load_exception::XamlLoadException;
pub use xaml_types::{IProvideValueTarget, IRootObjectProvider, IUriContext, IXamlTypeResolver};

/// Includes the compiled markup the build script of the crate generated
/// (`ferroui_build::Build::run`, docs/porting/xaml.md 9.6.2), at the root of the
/// crate: one public module per group of documents (`compiled_xaml` for the
/// default group) and the module `compiled_markup` with the embedded assets
/// (`ASSETS`), the loader table of the crate (`try_load`) and `register()`,
/// which the `register_types()` of the crate calls.
#[macro_export]
macro_rules! include_compiled_xaml {
    () => {
        include!(concat!(env!("OUT_DIR"), "/xaml/mod.rs"));
    };
}

/// Gives a shared plain type identity equality (the reference equality of a
/// class instance), so that it can be held in untyped values.
macro_rules! identity_eq {
    ($($type_:ty),* $(,)?) => {$(
        impl PartialEq for $type_ {
            fn eq(&self, other: &Self) -> bool {
                ::std::ptr::addr_eq(self, other)
            }
        }
    )*};
}
pub(crate) use identity_eq;

/// The equivalent of an exception thrown out of a member without an error
/// channel and without a value to go on with: the error surfaces as a panic
/// that carries its message, inside a scope of
/// [`xaml_error_scope::with_xaml_errors`] too. Members that a load reaches
/// use [`throw_or`] instead.
pub(crate) fn throw<T>(result: Result<T, XamlLoadError>) -> T {
    match result {
        Ok(value) => value,
        Err(error) => xaml_error_scope::throw(error),
    }
}

/// The equivalent of an exception thrown out of a member without an error
/// channel (a contract the framework calls, such as the build of a deferred
/// resource or a template): inside a scope of
/// [`xaml_error_scope::with_xaml_errors`] the error goes to the scope and the
/// member returns `fallback()`, what it produces when there is nothing (no
/// resource, no content); outside a scope it panics with the message of the
/// error, as before (#567).
pub(crate) fn throw_or<T>(result: Result<T, XamlLoadError>, fallback: impl FnOnce() -> T) -> T {
    match result {
        Ok(value) => value,
        Err(error) => match xaml_error_scope::report(error) {
            Ok(()) => fallback(),
            Err(error) => xaml_error_scope::throw(error),
        },
    }
}

#[cfg(test)]
mod test_support;

#[cfg(all(feature = "hot-reload", debug_assertions))]
pub mod reload_templates;

#[cfg(all(feature = "hot-reload", debug_assertions))]
pub mod reload_identity;
