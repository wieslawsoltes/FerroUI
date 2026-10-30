//! The type table of this crate: its namespace, its classes, what it states about itself for
//! markup, its embedded assets and the loader table of its compiled markup.

use crate::markup::XamlClass;
use crate::{App, MainWindow, MainWindowViewModel};
use ferroui_base::data::core::ValueTypes;
use ferroui_base::metadata::{MarkupAssembly, MarkupType, MarkupTyped};
use ferroui_base::TypeInfo;

/// The dotted namespaces of the modules of this crate.
const NAMESPACES: &[(&str, &str)] = &[("sample_protranslate_demo", "ProTranslateDemo")];

/// What this crate states about itself for markup.
pub static ASSEMBLY: MarkupAssembly = MarkupAssembly {
    name: "ProTranslateDemo",
    crate_name: "sample_protranslate_demo",
    xmlns_definitions: &[],
    xmlns_prefixes: &[],
    metadata: &[],
};

/// The classes of the namespace `ProTranslateDemo` (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[App::TYPE, MainWindow::TYPE];

/// The classes of the namespace that have a document (`&X::XAML_CLASS`).
pub(crate) const CLASSES: &[&XamlClass] = &[&App::XAML_CLASS, &MainWindow::XAML_CLASS];

/// Registers the namespace, the types, the assembly, the embedded assets and the compiled
/// markup of this crate (and of the crates it is built on). Cheap and idempotent.
pub fn register_types() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        // The runtime library of compiled markup (the markup extensions the compiled
        // documents create), which registers the controls.
        ferroui_markup_xaml::register_types();
        ferroui_themes_winui::register_types();
        ferroui_protranslate::register_types();
        TypeInfo::register_namespaces(NAMESPACES);
        TypeInfo::register_all(TYPES);
        MarkupType::register_all(&[<MainWindowViewModel as MarkupTyped>::MARKUP]);
        ValueTypes::register_global(ValueTypes::register_reference::<MainWindowViewModel>);
        MarkupAssembly::register(&ASSEMBLY);
        crate::SAMPLE.register_assets();
        // The loader table of the compiled markup, which the build of the crate generates.
        crate::compiled_markup::register();
    });
}
