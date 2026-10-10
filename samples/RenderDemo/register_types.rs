//! The type table of this crate: its namespaces, its classes, what it states about itself for
//! markup, its embedded assets and the loader table of its compiled markup.

use crate::markup::XamlClass;
use crate::{controls, pages, view_models, App, MainWindow};
use ferroui_base::data::core::ValueTypes;
use ferroui_base::metadata::{MarkupAssembly, MarkupType};
use ferroui_base::TypeInfo;

/// The dotted namespaces of the modules of this crate. A type belongs to the namespace of the
/// longest module path that is a prefix of the path of its declaring module.
const NAMESPACES: &[(&str, &str)] = &[
    ("render_demo", "RenderDemo"),
    ("render_demo::controls", "RenderDemo.Controls"),
    ("render_demo::pages", "RenderDemo.Pages"),
    ("render_demo::view_models", "RenderDemo.ViewModels"),
];

/// What this crate states about itself for markup.
pub static ASSEMBLY: MarkupAssembly = MarkupAssembly {
    name: "RenderDemo",
    crate_name: "render_demo",
    xmlns_definitions: &[],
    xmlns_prefixes: &[],
    metadata: &[],
};

/// The classes of the root namespace `RenderDemo` (`X::TYPE`).
const ROOT_TYPES: &[&TypeInfo] = &[App::TYPE, MainWindow::TYPE];

/// The classes of the root namespace that have a document (`&X::XAML_CLASS`).
pub(crate) const ROOT_CLASSES: &[&XamlClass] = &[&App::XAML_CLASS, &MainWindow::XAML_CLASS];

/// Registers the namespaces, the types, the assembly, the embedded assets and the compiled
/// markup of this crate (and of the crates it is built on). Cheap and idempotent.
pub fn register_types() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        // The runtime library of compiled markup (the markup extensions the compiled
        // documents create), which registers the controls.
        ferroui_markup_xaml::register_types();
        ferroui_themes_fluent::register_types();
        control_samples::register_types();
        TypeInfo::register_namespaces(NAMESPACES);
        for types in [ROOT_TYPES, controls::TYPES, pages::TYPES, view_models::TYPES] {
            TypeInfo::register_all(types);
        }
        for markup_types in [controls::MARKUP_TYPES, pages::MARKUP_TYPES, view_models::MARKUP_TYPES] {
            MarkupType::register_all(markup_types);
        }
        ValueTypes::register_global(register_value_types);
        controls::register_lists();
        pages::register_lists();
        view_models::register_lists();
        MarkupAssembly::register(&ASSEMBLY);
        crate::SAMPLE.register_assets();
        // The loader table of the compiled markup, which the build of the crate generates: a
        // load of a document by its URI creates the class of the document with its
        // constructor.
        crate::compiled_markup::register();
    });
}

fn register_value_types() {
    controls::register_value_types();
    pages::register_value_types();
    view_models::register_value_types();
}
