//! The type table of this crate: its namespaces, its classes, what it
//! states about itself for markup, its embedded assets and the loader table
//! of its compiled markup.

use crate::markup::XamlClass;
use crate::{controls, converter, models, pages, view_models, views};
use ferroui_base::data::core::ValueTypes;
use ferroui_base::metadata::{MarkupAssembly, MarkupType};
use ferroui_base::TypeInfo;

/// The dotted namespaces of the modules of this crate. A type belongs to
/// the namespace of the longest module path that is a prefix of the path of
/// its declaring module.
const NAMESPACES: &[(&str, &str)] = &[
    ("control_catalog", "ControlCatalog"),
    ("control_catalog::controls", "ControlCatalog.Controls"),
    ("control_catalog::converter", "ControlCatalog.Converter"),
    ("control_catalog::models", "ControlCatalog.Models"),
    ("control_catalog::pages", "ControlCatalog.Pages"),
    ("control_catalog::pages::open_gl", "ControlCatalog.Pages.OpenGl"),
    ("control_catalog::view_models", "ControlCatalog.ViewModels"),
    ("control_catalog::views", "ControlCatalog.Views"),
];

/// What this crate states about itself for markup.
pub static ASSEMBLY: MarkupAssembly = MarkupAssembly {
    name: "ControlCatalog",
    crate_name: "control_catalog",
    xmlns_definitions: &[],
    xmlns_prefixes: &[],
    metadata: &[],
};

/// The classes of the crate, per namespace (and per directory of the pages).
fn types() -> impl Iterator<Item = &'static TypeInfo> {
    let namespaces: [&[&TypeInfo]; 5] =
        [crate::ROOT_TYPES, controls::TYPES, models::TYPES, view_models::TYPES, views::TYPES];
    namespaces.into_iter().chain(pages::TYPES.iter().copied()).flat_map(|types| types.iter().copied())
}

/// The classes of the crate that have a document.
pub(crate) fn classes() -> impl Iterator<Item = &'static XamlClass> {
    let namespaces: [&[&XamlClass]; 5] =
        [crate::ROOT_CLASSES, controls::CLASSES, models::CLASSES, view_models::CLASSES, views::CLASSES];
    namespaces.into_iter().chain(pages::CLASSES.iter().copied()).flat_map(|classes| classes.iter().copied())
}

/// The types of the crate declared with `ferro_markup_type!` / `ferro_markup_enum!`.
fn markup_types() -> impl Iterator<Item = &'static MarkupType> {
    let namespaces: [&[&MarkupType]; 6] = [
        crate::ROOT_MARKUP_TYPES,
        controls::MARKUP_TYPES,
        converter::MARKUP_TYPES,
        models::MARKUP_TYPES,
        view_models::MARKUP_TYPES,
        views::MARKUP_TYPES,
    ];
    namespaces.into_iter().chain(pages::MARKUP_TYPES.iter().copied()).flat_map(|types| types.iter().copied())
}

/// Registers the namespaces, the types, the assembly, the embedded assets
/// and the compiled markup of this crate (and of the crates it is built
/// on). Cheap and idempotent.
pub fn register_types() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        // The runtime library of compiled markup (the markup extensions the compiled
        // documents create), which registers the controls.
        ferroui_markup_xaml::register_types();
        ferroui_themes_simple::register_types();
        ferroui_themes_fluent::register_types();
        ferroui_controls_color_picker::register_types();
        ferroui_opengl::register_types();
        TypeInfo::register_namespaces(NAMESPACES);
        for type_ in types() {
            TypeInfo::register(type_);
        }
        for markup_type in markup_types() {
            MarkupType::register_all(&[markup_type]);
        }
        ValueTypes::register_global(register_value_types);
        view_models::register_lists();
        pages::register_lists();
        MarkupAssembly::register(&ASSEMBLY);
        crate::assets::register();
        // The loader table of the compiled markup, which the build of the crate generates: a
        // load of a document by its URI creates the class of the document with its
        // constructor, and builds a document without a class from its compiled markup.
        #[cfg(not(feature = "runtime-markup"))]
        crate::compiled_markup::register();
        #[cfg(feature = "runtime-markup")]
        ferroui_markup_xaml::FerroXamlLoader::register_compiled_xaml(ASSEMBLY.name, try_load);
    });
}

fn register_value_types() {
    crate::register_root_value_types();
    controls::register_value_types();
    converter::register_value_types();
    models::register_value_types();
    view_models::register_value_types();
    views::register_value_types();
    pages::register_value_types();
}

/// The loader of the documents of this assembly that have a class, for a build that does
/// not compile the documents (the feature `runtime-markup`): loading the document of a
/// class by URI creates an instance of the class (whose constructor populates it with the
/// run-time loader). The other documents are left to the run-time loader. A build that
/// compiles the documents registers the table the compiler generates instead.
#[cfg(feature = "runtime-markup")]
fn try_load(
    _service_provider: Option<&std::rc::Rc<dyn ferroui_base::metadata::IServiceProvider>>,
    uri: &str,
) -> Result<Option<ferroui_base::BoxedValue>, ferroui_markup_xaml::XamlLoadException> {
    Ok(classes().find(|class| uri.eq_ignore_ascii_case(&class.document_uri())).map(|class| (class.create)()))
}
