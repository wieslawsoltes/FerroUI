//! What this crate states about itself for markup. `WebFonts` is in the namespace
//! `FerroUI.Fonts.Web`, mapped into the XML namespace of the framework as every built-in feature
//! is, so a document writes `<WebFonts.StyleSheet>` with no prefix.

use ferroui_base::metadata::{MarkupAssembly, XmlnsDefinition, FERRO_XML_NAMESPACE};
use ferroui_base::{StaticType, TypeInfo};

const NAMESPACES: &[(&str, &str)] = &[("ferroui_fonts_web", "FerroUI.Fonts.Web")];

/// The assembly of this crate, for markup.
pub static ASSEMBLY: MarkupAssembly = MarkupAssembly {
    name: "FerroUI.Fonts.Web",
    crate_name: "ferroui_fonts_web",
    xmlns_definitions: &[XmlnsDefinition { xml_namespace: FERRO_XML_NAMESPACE, namespace: "FerroUI.Fonts.Web" }],
    xmlns_prefixes: &[],
    metadata: &[],
};

const CLASSES: &[&TypeInfo] = &[<crate::WebFonts as StaticType>::TYPE];

/// Registers `WebFonts` for markup that is loaded at run time, and starts following its property.
/// An application with compiled markup gets the second through `with_web_fonts`.
pub fn register_types() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        ferroui_controls::register_types();
        TypeInfo::register_namespaces(NAMESPACES);
        TypeInfo::register_table(CLASSES);
        crate::rust_paths::register_rust_paths();
        MarkupAssembly::register_table(&ASSEMBLY);
    });
    crate::WebFonts::follow_style_sheets();
}
