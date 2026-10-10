//! The type table of this crate: its namespaces, its classes, what it states about itself for
//! markup, its embedded assets and the loader table of its compiled markup.

use crate::generic_markup_extension::{GenericMarkupExtension, GenericMarkupExtensionOfColor};
use crate::generic_value_converter::{GenericValueConverter, GenericValueConverterOfSolidColorBrush};
use crate::markup::XamlClass;
use crate::{view_models, App, MainWindow, TestItemView};
use ferroui_base::data::converters::IValueConverter;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::media::{Color, SolidColorBrush};
use ferroui_base::metadata::{MarkupAssembly, MarkupType, MarkupTyped};
use ferroui_base::TypeInfo;
use std::rc::Rc;

/// The dotted namespaces of the modules of this crate. A type belongs to the namespace of the
/// longest module path that is a prefix of the path of its declaring module.
const NAMESPACES: &[(&str, &str)] = &[("binding_demo", "BindingDemo"), ("binding_demo::view_models", "BindingDemo.ViewModels")];

/// What this crate states about itself for markup.
pub static ASSEMBLY: MarkupAssembly = MarkupAssembly {
    name: "BindingDemo",
    crate_name: "binding_demo",
    xmlns_definitions: &[],
    xmlns_prefixes: &[],
    metadata: &[],
};

/// The classes of the root namespace `BindingDemo` (`X::TYPE`).
const ROOT_TYPES: &[&TypeInfo] = &[App::TYPE, MainWindow::TYPE, TestItemView::TYPE];

/// The classes of the root namespace that have a document (`&X::XAML_CLASS`).
pub(crate) const ROOT_CLASSES: &[&XamlClass] = &[&App::XAML_CLASS, &MainWindow::XAML_CLASS, &TestItemView::XAML_CLASS];

/// The types of the root namespace declared with `ferro_markup_type!`
/// (`<X as MarkupTyped>::MARKUP`): the instantiations of the generic markup extension and of
/// the generic value converter the documents name.
const ROOT_MARKUP_TYPES: &[&MarkupType] = &[
    <GenericMarkupExtensionOfColor as MarkupTyped>::MARKUP,
    <GenericValueConverterOfSolidColorBrush as MarkupTyped>::MARKUP,
];

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
        for types in [ROOT_TYPES, view_models::TYPES] {
            TypeInfo::register_all(types);
        }
        for markup_types in [ROOT_MARKUP_TYPES, view_models::MARKUP_TYPES] {
            MarkupType::register_all(markup_types);
        }
        ValueTypes::register_global(register_value_types);
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
    ValueTypes::register_reference::<GenericMarkupExtension<Color>>();
    ValueTypes::register_reference::<GenericValueConverter<SolidColorBrush>>();
    ValueTypes::register_cast::<GenericValueConverter<SolidColorBrush>, Rc<dyn IValueConverter>>(
        GenericValueConverter::<SolidColorBrush>::as_value_converter,
    );
    view_models::register_value_types();
}
