//! ferroui-markup-xaml-tests
//!
//! The unit tests of the XAML stack (the counterpart of the upstream
//! `Markup.Xaml.UnitTests` project): markup documents are loaded at run
//! time through the run-time XAML loader and assertions are made on the
//! live objects. One test file per upstream file, under the same folders.
//!
//! The test controls and view models of the suite are declared here with
//! their markup metadata and registered by [`register_types`] under the
//! assembly `FerroUI.Markup.Xaml.UnitTests`.

// Generated code of the emitter's corpus (`emitter/generated.rs`) names the classes of this
// crate by absolute paths, as it names the classes of any other crate.
extern crate self as ferroui_markup_xaml_tests;

use ferroui_base::metadata::{MarkupAssembly, XmlnsDefinition};
use ferroui_base::TypeInfo;

pub mod support;

pub mod emitter;

#[cfg(test)]
mod data;

#[cfg(test)]
mod markup_extensions;

#[cfg(test)]
mod support_bindings;

#[cfg(test)]
mod converters;

#[cfg(test)]
mod selector_tests_property_equals;

#[cfg(test)]
mod setter_tests;

#[cfg(test)]
mod style_tests;

#[cfg(test)]
mod templates;

#[cfg(test)]
mod smoke_tests;

#[cfg(test)]
mod theme_binding_paths;

#[cfg(test)]
mod theme_gap_tests;

#[cfg(test)]
mod theme_readiness_tests;

#[cfg(test)]
mod type_system_drift;

#[cfg(test)]
mod xaml;

/// The dotted namespaces of the modules of this crate.
const NAMESPACES: &[(&str, &str)] = &[
    ("ferroui_markup_xaml_tests", "FerroUI.Markup.Xaml.UnitTests"),
    ("ferroui_markup_xaml_tests::xaml", "FerroUI.Markup.Xaml.UnitTests.Xaml"),
];

const XMLNS_DEFINITIONS: &[XmlnsDefinition] = &[];

/// What this crate states about itself for markup.
pub static ASSEMBLY: MarkupAssembly = MarkupAssembly {
    name: "FerroUI.Markup.Xaml.UnitTests",
    crate_name: "ferroui_markup_xaml_tests",
    xmlns_definitions: XMLNS_DEFINITIONS,
    xmlns_prefixes: &[],
    metadata: &[],
};

/// Registers the namespaces, the types and the assembly of this crate (and
/// of the crates it is built on). Cheap and idempotent.
pub fn register_types() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        ferroui_markup_xaml::register_types();
        TypeInfo::register_namespaces(NAMESPACES);
        MarkupAssembly::register(&ASSEMBLY);
        support::register();
        #[cfg(test)]
        support_bindings::register();
    });
}
