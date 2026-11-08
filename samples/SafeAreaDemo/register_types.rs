//! The type table of this crate: its namespaces, its classes, what it states about itself for
//! markup, its embedded assets and the loader table of its compiled markup.

use crate::markup::XamlClass;
use crate::{view_models, views, App, ViewLocator};
use ferroui_base::data::core::ValueTypes;
use ferroui_base::metadata::{MarkupAssembly, MarkupType, MarkupTyped};
use ferroui_base::TypeInfo;
use ferroui_controls::templates::IDataTemplate;
use std::rc::Rc;

/// The dotted namespaces of the modules of this crate. A type belongs to the namespace of the
/// longest module path that is a prefix of the path of its declaring module.
const NAMESPACES: &[(&str, &str)] = &[
    ("safe_area_demo", "SafeAreaDemo"),
    ("safe_area_demo::view_models", "SafeAreaDemo.ViewModels"),
    ("safe_area_demo::views", "SafeAreaDemo.Views"),
];

/// What this crate states about itself for markup.
pub static ASSEMBLY: MarkupAssembly = MarkupAssembly {
    name: "SafeAreaDemo",
    crate_name: "safe_area_demo",
    xmlns_definitions: &[],
    xmlns_prefixes: &[],
    metadata: &[],
};

/// The classes of the root namespace `SafeAreaDemo` (`X::TYPE`).
const ROOT_TYPES: &[&TypeInfo] = &[App::TYPE];

/// The classes of the root namespace that have a document (`&X::XAML_CLASS`).
pub(crate) const CLASSES: &[&XamlClass] = &[&App::XAML_CLASS];

/// The types of the root namespace declared with `ferro_markup_type!`
/// (`<X as MarkupTyped>::MARKUP`).
const ROOT_MARKUP_TYPES: &[&MarkupType] = &[<ViewLocator as MarkupTyped>::MARKUP];

/// Registers the namespaces, the types, the assembly, the embedded assets and the compiled
/// markup of this crate (and of the crates it is built on). Cheap and idempotent.
pub fn register_types() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        // The runtime library of compiled markup (the markup extensions the compiled
        // documents create), which registers the controls.
        ferroui_markup_xaml::register_types();
        ferroui_themes_fluent::register_types();
        TypeInfo::register_namespaces(NAMESPACES);
        for types in [ROOT_TYPES, views::TYPES, view_models::TYPES] {
            TypeInfo::register_all(types);
        }
        // The view locator creates the views by reflection (`Activator.CreateInstance`), from
        // their runtime type: without the tables of types a runtime type names no constructor,
        // so the constructors of the views are registered here, where the classes are named.
        views::register_constructors();
        for markup_types in [ROOT_MARKUP_TYPES, view_models::MARKUP_TYPES] {
            MarkupType::register_all(markup_types);
        }
        ValueTypes::register_global(register_value_types);
        MarkupAssembly::register(&ASSEMBLY);
        crate::SAMPLE.register_assets();
        // The loader table of the compiled markup, which the build of the crate generates: a
        // load of a document by its URI creates the class of the document with its
        // constructor.
        crate::compiled_markup::register();
    });
}

/// What the untyped value conversions must know about the types of this crate.
fn register_value_types() {
    // `ViewLocator : IDataTemplate`: the locator is an item of the data templates of the
    // application.
    ValueTypes::register_reference::<ViewLocator>();
    ValueTypes::register_cast::<ViewLocator, Rc<dyn IDataTemplate>>(ViewLocator::as_data_template);
    view_models::register_value_types();
}
