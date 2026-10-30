//! The type table of this crate: its namespaces, its classes, what it states about itself for
//! markup, its embedded assets and the loader table of its compiled markup.

use crate::markup::XamlClass;
use crate::{App, CaptureHost, MainWindow};
use ferroui_base::data::core::ValueTypes;
use ferroui_base::metadata::{MarkupAssembly, MarkupType};
use ferroui_base::TypeInfo;

/// The dotted namespaces of the modules of this crate. A type belongs to the namespace of the
/// longest module path that is a prefix of the path of its declaring module.
const NAMESPACES: &[(&str, &str)] =
    &[("material_theme_gallery", "MaterialThemeGallery"), ("material_theme_gallery::pages", "MaterialThemeGallery.Pages")];

/// What this crate states about itself for markup.
pub static ASSEMBLY: MarkupAssembly = MarkupAssembly {
    name: "MaterialThemeGallery",
    crate_name: "material_theme_gallery",
    xmlns_definitions: &[],
    xmlns_prefixes: &[],
    metadata: &[],
};

/// The classes of the namespace `MaterialThemeGallery` (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[App::TYPE, CaptureHost::TYPE, MainWindow::TYPE];

/// The classes of the namespace that have a document (`&X::XAML_CLASS`).
pub(crate) const CLASSES: &[&XamlClass] = &[&App::XAML_CLASS, &CaptureHost::XAML_CLASS, &MainWindow::XAML_CLASS];

/// The types of the namespace declared with `ferro_markup_type!` / `ferro_markup_enum!`
/// (`<X as MarkupTyped>::MARKUP`).
pub(crate) const MARKUP_TYPES: &[&MarkupType] = &[];

/// Registers the namespaces, the types, the assembly, the embedded assets and the compiled
/// markup of this crate (and of the crates it is built on). Cheap and idempotent.
pub fn register_types() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        // The runtime library of compiled markup (the markup extensions the compiled
        // documents create), which registers the controls.
        ferroui_markup_xaml::register_types();
        // The managed file chooser of the dialogs page.
        ferroui_dialogs::register_types();
        TypeInfo::register_namespaces(NAMESPACES);
        TypeInfo::register_all(TYPES);
        TypeInfo::register_all(crate::pages::TYPES);
        MarkupType::register_all(MARKUP_TYPES);
        MarkupType::register_all(crate::pages::MARKUP_TYPES);
        ValueTypes::register_global(register_value_types);
        register_lists();
        MarkupAssembly::register(&ASSEMBLY);
        crate::SAMPLE.register_assets();
        // The loader table of the compiled markup, which the build of the crate generates: a
        // load of a document by its URI creates the class of the document with its
        // constructor.
        crate::compiled_markup::register();
    });
}

/// What the untyped value conversions must know about the types of this crate
/// (`ValueTypes::register_reference::<X>()`, nullable forms, casts to contracts).
fn register_value_types() {
    crate::pages::register_value_types();
}

/// Makes the typed lists of this crate known to markup (`ferro_markup_list!`).
fn register_lists() {
    crate::pages::register_lists();
}
