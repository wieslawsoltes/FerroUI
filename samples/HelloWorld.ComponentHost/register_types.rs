//! The type table of this crate: its namespace, its classes, what it states about itself for
//! markup, its embedded assets and the loader table of its compiled markup.

use crate::markup::XamlClass;
use crate::{App, MainWindow};
use ferroui_base::metadata::MarkupAssembly;
use ferroui_base::TypeInfo;
use ferroui_view::prelude::*;

/// The dotted namespaces of the modules of this crate.
const NAMESPACES: &[(&str, &str)] = &[("hello_world_component_host", "HelloWorldComponentHost")];

/// What this crate states about itself for markup.
pub static ASSEMBLY: MarkupAssembly = MarkupAssembly {
    name: "HelloWorldComponentHost",
    crate_name: "hello_world_component_host",
    xmlns_definitions: &[],
    xmlns_prefixes: &[],
    metadata: &[],
};

/// The classes of the namespace `HelloWorldComponentHost` (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[App::TYPE, MainWindow::TYPE];

/// The classes of the namespace that have a document (`&X::XAML_CLASS`).
pub(crate) const CLASSES: &[&XamlClass] = &[&App::XAML_CLASS, &MainWindow::XAML_CLASS];

/// Registers the namespace, the types, the assembly, the embedded assets and the compiled
/// markup of this crate (and of the crates it is built on), and the component `Hello`. Cheap and
/// idempotent.
pub fn register_types() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        // The runtime library of compiled markup (the markup extensions the compiled
        // documents create), which registers the controls.
        ferroui_markup_xaml::register_types();
        ferroui_themes_winui::register_types();
        // `ComponentHost` for markup, and the component the window names.
        ferroui_view::register_types();
        ferroui_view::register_component("Hello", || rsx! { crate::Hello {} });
        TypeInfo::register_namespaces(NAMESPACES);
        TypeInfo::register_all(TYPES);
        MarkupAssembly::register(&ASSEMBLY);
        crate::SAMPLE.register_assets();
        // The loader table of the compiled markup, which the build of the crate generates.
        crate::compiled_markup::register();
    });
}
