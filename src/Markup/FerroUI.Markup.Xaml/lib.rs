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
mod markup_extension;
mod object_casts;
mod register_types;
mod rust_paths;
mod runtime_xaml_loader_configuration;
mod runtime_xaml_loader_document;
mod xaml_load_exception;
mod xaml_types;

pub use eager_parent_stack_enumerator::EagerParentStackEnumerator;
pub use extensions::ServiceProviderExtensions;
pub use ferro_xaml_loader::{CompiledXamlLoader, FerroXamlLoader, IRuntimeXamlLoader};
pub use markup_extension::MarkupExtension;
pub use object_casts::{FromXamlObject, XamlResourceNode};
pub use register_types::{register_types, ASSEMBLY};
pub use runtime_xaml_loader_configuration::{
    RuntimeXamlDiagnostic, RuntimeXamlDiagnosticSeverity, RuntimeXamlLoaderConfiguration, XamlDiagnosticFunc,
};
pub use runtime_xaml_loader_document::RuntimeXamlLoaderDocument;
pub use xaml_load_exception::XamlLoadException;
pub use xaml_types::{IProvideValueTarget, IRootObjectProvider, IUriContext, IXamlTypeResolver};

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

/// The equivalent of an exception thrown through an untyped (metadata)
/// call: metadata invokers have no error channel for methods, so the error
/// of a fallible member surfaces as a panic that carries its message.
pub(crate) fn throw<T>(result: Result<T, XamlLoadException>) -> T {
    match result {
        Ok(value) => value,
        Err(error) => panic!("{error}"),
    }
}

#[cfg(test)]
mod test_support;
