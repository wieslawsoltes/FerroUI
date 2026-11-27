//! The type table of this crate: its namespaces, its classes, what it states about itself for
//! markup, its embedded assets and its compiled markup.

use ferroui_base::metadata::{MarkupAssembly, XmlnsDefinition, FERRO_XML_NAMESPACE};
use ferroui_base::{StaticType, TypeInfo};

const NAMESPACES: &[(&str, &str)] = &[("ferroui_controls_items_view", "FerroUI.Controls")];

/// What this crate states about itself for markup.
pub static ASSEMBLY: MarkupAssembly = MarkupAssembly {
    name: "FerroUI.Controls.ItemsView",
    crate_name: "ferroui_controls_items_view",
    xmlns_definitions: &[XmlnsDefinition { xml_namespace: FERRO_XML_NAMESPACE, namespace: "FerroUI.Controls" }],
    xmlns_prefixes: &[],
    metadata: &[],
};

const TYPES: &[&TypeInfo] = &[
    <crate::ItemContainer as StaticType>::TYPE,
    <crate::ItemsView as StaticType>::TYPE,
];

/// Registers the namespaces, the types, the assembly and the compiled markup of this crate and
/// of the crates it is built on. Cheap and idempotent.
pub fn register_types() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        ferroui_markup_xaml::register_types();
        ferroui_controls_repeater::register_types();
        TypeInfo::register_namespaces(NAMESPACES);
        TypeInfo::register_table(TYPES);
        crate::rust_paths::register_rust_paths();
        MarkupAssembly::register_table(&ASSEMBLY);
        crate::markup_types::register();
        #[cfg(any(test, feature = "document-assets"))]
        crate::assets::register();
        crate::compiled_markup::register();
    });
}
