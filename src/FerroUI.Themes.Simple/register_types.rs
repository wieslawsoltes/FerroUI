//! The type table of this crate: its namespaces, its classes, what it
//! states about itself for markup, its embedded documents and the loader
//! of the documents with a class.

use crate::simple_theme::SimpleTheme;
use ferroui_base::metadata::{into_markup_value, IServiceProvider, MarkupAssembly, XmlnsDefinition, FERRO_XML_NAMESPACE};
use ferroui_base::{BoxedValue, TypeInfo};
use ferroui_markup_xaml::{FerroXamlLoader, XamlLoadException};
use std::rc::Rc;

/// The dotted namespaces of the modules of this crate.
const NAMESPACES: &[(&str, &str)] = &[("ferroui_themes_simple", "FerroUI.Themes.Simple")];

/// What this crate states about itself for markup: its assembly name and
/// the namespace the XML namespace of the framework maps to.
pub static ASSEMBLY: MarkupAssembly = MarkupAssembly {
    name: "FerroUI.Themes.Simple",
    crate_name: "ferroui_themes_simple",
    xmlns_definitions: &[XmlnsDefinition { xml_namespace: FERRO_XML_NAMESPACE, namespace: "FerroUI.Themes.Simple" }],
    xmlns_prefixes: &[],
    metadata: &[],
};

const TYPES: &[&TypeInfo] = &[SimpleTheme::TYPE];

/// Registers the namespaces, the types, the assembly, the embedded
/// documents and the document loader of this crate (and of the crates it is
/// built on). Cheap and idempotent.
pub fn register_types() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        ferroui_markup_xaml::register_types();
        ferroui_dialogs::register_types();
        TypeInfo::register_namespaces(NAMESPACES);
        TypeInfo::register_all(TYPES);
        crate::rust_paths::register_rust_paths();
        MarkupAssembly::register(&ASSEMBLY);
        crate::assets::register();
        FerroXamlLoader::register_compiled_xaml(ASSEMBLY.name, try_load);
    });
}

/// The loader of the documents of this assembly that have a class: loading
/// the document of a class by URI creates an instance of the class (whose
/// constructor populates it). The other documents are left to the run-time
/// loader.
///
/// This is the table the XAML compiler generates per crate; it is written
/// by hand until the compiler exists.
fn try_load(service_provider: Option<&Rc<dyn IServiceProvider>>, uri: &str) -> Result<Option<BoxedValue>, XamlLoadException> {
    if uri.eq_ignore_ascii_case(SimpleTheme::DOCUMENT_URI) {
        return Ok(into_markup_value(SimpleTheme::with_service_provider(service_provider.cloned())));
    }
    Ok(None)
}
