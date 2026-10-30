//! The type table of this crate: its namespaces, its classes, what it
//! states about itself for markup, its embedded documents and the loader
//! of the documents with a class.

use crate::accents::SystemAccentColors;
use crate::converters::{CornerRadiusToThicknessConverter, CornerRadiusToThicknessConverterKind};
use crate::primitives::{InfoBarPanel, TabViewEqualWidthPanel};
use crate::macos_theme::{ControlSize, DensityStyle, MacOSTheme};
use ferroui_base::data::converters::IValueConverter;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::metadata::{MarkupAssembly, MarkupType, MarkupTyped, XmlnsDefinition, FERRO_XML_NAMESPACE};
use ferroui_base::TypeInfo;
use ferroui_markup_xaml::FerroXamlLoader;
use std::rc::Rc;

/// The dotted namespaces of the modules of this crate.
const NAMESPACES: &[(&str, &str)] = &[
    ("ferroui_themes_macos", "FerroUI.Themes.MacOS"),
    ("ferroui_themes_macos::accents", "FerroUI.Themes.MacOS.Accents"),
    ("ferroui_themes_macos::converters", "FerroUI.Themes.MacOS.Converters"),
    ("ferroui_themes_macos::primitives", "FerroUI.Themes.MacOS.Primitives"),
];

/// What this crate states about itself for markup: its assembly name and
/// the namespace the XML namespace of the framework maps to.
pub static ASSEMBLY: MarkupAssembly = MarkupAssembly {
    name: "FerroUI.Themes.MacOS",
    crate_name: "ferroui_themes_macos",
    xmlns_definitions: &[XmlnsDefinition { xml_namespace: FERRO_XML_NAMESPACE, namespace: "FerroUI.Themes.MacOS" }],
    xmlns_prefixes: &[],
    metadata: &[],
};

const TYPES: &[&TypeInfo] =
    &[MacOSTheme::TYPE, SystemAccentColors::TYPE, InfoBarPanel::TYPE, TabViewEqualWidthPanel::TYPE];

const MARKUP_TYPES: &[&MarkupType] = &[
    <ControlSize as MarkupTyped>::MARKUP,
    <DensityStyle as MarkupTyped>::MARKUP,
    <CornerRadiusToThicknessConverter as MarkupTyped>::MARKUP,
    <CornerRadiusToThicknessConverterKind as MarkupTyped>::MARKUP,
];

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
        MarkupType::register_all(MARKUP_TYPES);
        ValueTypes::register_global(register_value_types);
        MarkupAssembly::register(&ASSEMBLY);
        crate::assets::register();
        FerroXamlLoader::register_compiled_xaml(ASSEMBLY.name, crate::compiled_xaml::try_load);
    });
}

/// What the untyped value conversions must know about the types of this crate: the nullable
/// forms of the enumerations and of the converter, and the converter as its contract.
fn register_value_types() {
    ValueTypes::register_nullable::<ControlSize>();
    ValueTypes::register_nullable::<DensityStyle>();
    ValueTypes::register_nullable::<CornerRadiusToThicknessConverterKind>();
    ValueTypes::register_nullable::<Rc<CornerRadiusToThicknessConverter>>();
    ValueTypes::register_cast::<Rc<CornerRadiusToThicknessConverter>, Rc<dyn IValueConverter>>(|c| c.clone());
}
