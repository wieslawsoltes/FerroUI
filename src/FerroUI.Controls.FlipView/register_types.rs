//! The type table of this crate: its namespace, its class, what it states about itself for
//! markup, its embedded assets and its compiled markup.

use ferroui_base::metadata::{MarkupAssembly, XmlnsDefinition, FERRO_XML_NAMESPACE};
use ferroui_base::{StaticType, TypeInfo};

const NAMESPACES: &[(&str, &str)] = &[("ferroui_controls_flip_view", "FerroUI.Controls")];

/// What this crate states about itself for markup.
pub static ASSEMBLY: MarkupAssembly = MarkupAssembly {
    name: "FerroUI.Controls.FlipView",
    crate_name: "ferroui_controls_flip_view",
    xmlns_definitions: &[XmlnsDefinition { xml_namespace: FERRO_XML_NAMESPACE, namespace: "FerroUI.Controls" }],
    xmlns_prefixes: &[],
    metadata: &[],
};

const TYPES: &[&TypeInfo] = &[<crate::FlipView as StaticType>::TYPE];

/// Registers the types of this crate for markup.
pub fn register_types() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        ferroui_markup_xaml::register_types();
        ferroui_controls::register_types();
        TypeInfo::register_namespaces(NAMESPACES);
        TypeInfo::register_table(TYPES);
        crate::rust_paths::register_rust_paths();
        MarkupAssembly::register_table(&ASSEMBLY);
        #[cfg(any(test, feature = "document-assets"))]
        crate::assets::register();
        crate::compiled_markup::register();
    });
}
