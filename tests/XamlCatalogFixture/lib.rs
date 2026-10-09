//! Test fixture of the XAML compiler (docs/porting/xaml.md, 9.5.17 and 9.10.1, item 2):
//! pages of the ControlCatalog compiled by a build script and populated by their compiled
//! markup. Not a port of an upstream project, and not a copy of the sample: the
//! documents ([`documents::PAGES`]) and the classes ([`pages`], [`view_models`]) are the
//! files of `samples/ControlCatalog`, read where they are. The sample itself is not
//! changed and still loads its documents with the run-time loader.
//!
//! The assembly is the assembly of the sample (`ControlCatalog`), so that the documents
//! name their classes and their assets as they do there. What differs from the sample is
//! [`markup`]: `initialize_component()` of a class calls the compiled markup of its
//! document. `tests` compares each page populated that way with the same class populated
//! by the run-time loader from the same document.

pub mod documents;
pub mod markup;
pub mod pages;
pub mod view_models;

// One module per page (`compiled_check_box_page`, ..: the function `populate` of the document
// of the class) and `compiled_markup` (the loader table of the crate and `register()`).
ferroui_markup_xaml::include_compiled_xaml!();

use ferroui_base::data::core::ValueTypes;
use ferroui_base::metadata::{MarkupAssembly, MarkupType};
use ferroui_base::TypeInfo;

/// The dotted namespaces of the modules of this crate: the namespaces of the sample.
const NAMESPACES: &[(&str, &str)] = &[
    ("xaml_catalog_fixture", "ControlCatalog"),
    ("xaml_catalog_fixture::pages", "ControlCatalog.Pages"),
    ("xaml_catalog_fixture::view_models", "ControlCatalog.ViewModels"),
];

/// What this crate states about itself for markup: the assembly of the sample. The build
/// script reads it from this file.
pub static ASSEMBLY: MarkupAssembly = MarkupAssembly {
    name: "ControlCatalog",
    crate_name: "xaml_catalog_fixture",
    xmlns_definitions: &[],
    xmlns_prefixes: &[],
    metadata: &[],
};

/// Registers the namespaces, the types, the assembly and the compiled markup of this
/// crate (and of the crates it is built on). Cheap and idempotent.
pub fn register_types() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        ferroui_markup_xaml::register_types();
        TypeInfo::register_namespaces(NAMESPACES);
        TypeInfo::register_all(pages::TYPES);
        MarkupType::register_all(view_models::MARKUP_TYPES);
        ValueTypes::register_global(view_models::register_value_types);
        view_models::register_lists();
        MarkupAssembly::register(&ASSEMBLY);
        compiled_markup::register();
    });
}

#[cfg(test)]
mod tests;
