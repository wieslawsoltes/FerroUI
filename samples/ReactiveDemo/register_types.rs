//! The type table of this crate: its namespace, its classes, what it states about itself for
//! markup, its embedded assets and the loader table of its compiled markup.

use crate::markup::XamlClass;
use crate::{App, MainWindow, NugetDetailsView};
use ferroui_base::metadata::MarkupAssembly;
use ferroui_base::TypeInfo;

/// The dotted namespace of the modules of this crate (the reference has one namespace,
/// `ReactiveDemo`).
const NAMESPACES: &[(&str, &str)] = &[("reactive_demo", "ReactiveDemo")];

/// What this crate states about itself for markup.
pub static ASSEMBLY: MarkupAssembly = MarkupAssembly {
    name: "ReactiveDemo",
    crate_name: "reactive_demo",
    xmlns_definitions: &[],
    xmlns_prefixes: &[],
    metadata: &[],
};

/// The classes of the namespace `ReactiveDemo` (`X::TYPE`).
const ROOT_TYPES: &[&TypeInfo] = &[App::TYPE, MainWindow::TYPE, NugetDetailsView::TYPE];

/// The classes of the namespace that have a document (`&X::XAML_CLASS`).
pub(crate) const ROOT_CLASSES: &[&XamlClass] = &[&App::XAML_CLASS, &MainWindow::XAML_CLASS, &NugetDetailsView::XAML_CLASS];

/// Registers the namespace, the types, the assembly, the embedded assets and the compiled
/// markup of this crate (and of the crates it is built on). Cheap and idempotent.
pub fn register_types() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        // The runtime library of compiled markup (the markup extensions the compiled
        // documents create), which registers the controls.
        ferroui_markup_xaml::register_types();
        ferroui_themes_winui::register_types();
        // The reactive window and user control the documents derive from, and the view host of
        // the items of the list (`http://reactiveui.net`).
        ferroui_reactiveui_controls::register_types();
        TypeInfo::register_namespaces(NAMESPACES);
        TypeInfo::register_all(ROOT_TYPES);
        MarkupAssembly::register(&ASSEMBLY);
        crate::SAMPLE.register_assets();
        // The loader table of the compiled markup, which the build of the crate generates: a
        // load of a document by its URI creates the class of the document with its
        // constructor.
        crate::compiled_markup::register();
    });
}
