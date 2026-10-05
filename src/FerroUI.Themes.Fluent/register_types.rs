//! The type table of this crate: its namespaces, its classes, what it
//! states about itself for markup, its embedded documents and the loader
//! of the documents with a class.

use crate::accents::SystemAccentColors;
use crate::fluent_theme::{DensityStyle, FluentTheme};
use crate::{ColorPaletteResources, ColorPaletteResourcesCollection};
use ferroui_base::metadata::{into_markup_value, IServiceProvider, MarkupAssembly, MarkupType, MarkupTyped, XmlnsDefinition, FERRO_XML_NAMESPACE};
use ferroui_base::data::core::ValueTypes;
use ferroui_base::{BoxedValue, TypeInfo};
use ferroui_markup_xaml::{FerroXamlLoader, XamlLoadException};
use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;
use std::rc::Rc;

/// The dotted namespaces of the modules of this crate.
const NAMESPACES: &[(&str, &str)] = &[
    ("ferroui_themes_fluent", "FerroUI.Themes.Fluent"),
    ("ferroui_themes_fluent::accents", "FerroUI.Themes.Fluent.Accents"),
];

/// What this crate states about itself for markup: its assembly name and
/// the namespace the XML namespace of the framework maps to.
pub static ASSEMBLY: MarkupAssembly = MarkupAssembly {
    name: "FerroUI.Themes.Fluent",
    crate_name: "ferroui_themes_fluent",
    xmlns_definitions: &[XmlnsDefinition { xml_namespace: FERRO_XML_NAMESPACE, namespace: "FerroUI.Themes.Fluent" }],
    xmlns_prefixes: &[],
    metadata: &[],
};

const TYPES: &[&TypeInfo] = &[
    FluentTheme::TYPE,
    ColorPaletteResources::TYPE,
    ColorPaletteResourcesCollection::TYPE,
    SystemAccentColors::TYPE,
];

const MARKUP_TYPES: &[&MarkupType] = &[<DensityStyle as MarkupTyped>::MARKUP];

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
        MarkupType::register_all(MARKUP_TYPES);
        ValueTypes::register_global(|| ValueTypes::register_nullable::<DensityStyle>());
        MarkupAssembly::register(&ASSEMBLY);
        crate::assets::register();
        FerroRuntimeXamlLoader::register_class_document(FluentTheme::TYPE, FluentTheme::DOCUMENT_URI);
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
    if uri.eq_ignore_ascii_case(FluentTheme::DOCUMENT_URI) {
        return Ok(into_markup_value(FluentTheme::with_service_provider(service_provider.cloned())));
    }
    Ok(None)
}
