//! The type table of this crate: its namespace, its classes and what it states about itself for
//! markup.

use ferroui_base::metadata::{MarkupAssembly, MarkupType, MarkupTyped, XmlnsDefinition, FERRO_XML_NAMESPACE};
use ferroui_base::{StaticType, TypeInfo};

const NAMESPACES: &[(&str, &str)] = &[("ferroui_controls_motion", "FerroUI.Controls")];

/// What this crate states about itself for markup.
pub static ASSEMBLY: MarkupAssembly = MarkupAssembly {
    name: "FerroUI.Controls.Motion",
    crate_name: "ferroui_controls_motion",
    xmlns_definitions: &[XmlnsDefinition { xml_namespace: FERRO_XML_NAMESPACE, namespace: "FerroUI.Controls" }],
    xmlns_prefixes: &[],
    // The WinUI names of the implicit transitions (docs/porting/xaml.md, "Property aliases").
    metadata: &[
        ("FerroXamlPropertyAlias", "FerroUI.Visual.OpacityTransition=FerroUI.Controls.ImplicitTransitions.OpacityTransition"),
        ("FerroXamlPropertyAlias", "FerroUI.Visual.RotationTransition=FerroUI.Controls.ImplicitTransitions.RotationTransition"),
        ("FerroXamlPropertyAlias", "FerroUI.Visual.ScaleTransition=FerroUI.Controls.ImplicitTransitions.ScaleTransition"),
        ("FerroXamlPropertyAlias", "FerroUI.Visual.TranslationTransition=FerroUI.Controls.ImplicitTransitions.TranslationTransition"),
        ("FerroXamlPropertyAlias", "FerroUI.Visual.Rotation=FerroUI.Controls.ImplicitTransitions.Rotation"),
        ("FerroXamlPropertyAlias", "FerroUI.Visual.Scale=FerroUI.Controls.ImplicitTransitions.Scale"),
        ("FerroXamlPropertyAlias", "FerroUI.Controls.Border.BackgroundTransition=FerroUI.Controls.ImplicitTransitions.BackgroundTransition"),
        ("FerroXamlPropertyAlias", "FerroUI.Controls.Panel.BackgroundTransition=FerroUI.Controls.ImplicitTransitions.BackgroundTransition"),
        ("FerroXamlPropertyAlias", "FerroUI.Controls.Presenters.ContentPresenter.BackgroundTransition=FerroUI.Controls.ImplicitTransitions.BackgroundTransition"),
    ],
};

const TYPES: &[&TypeInfo] = &[
    <crate::ParallaxView as StaticType>::TYPE,
    <crate::ImplicitTransitions as StaticType>::TYPE,
    <crate::ScalarTransition as StaticType>::TYPE,
    <crate::Vector3Transition as StaticType>::TYPE,
];

ferroui_base::ferro_rust_paths! {
    classes: [crate::ParallaxView, crate::ScalarTransition, crate::Vector3Transition],
    types: [crate::ParallaxSourceOffsetKind, crate::ImplicitTransitions],
    contracts: [], generics: [], generic_contracts: [],
}

/// Registers the types of this crate for markup.
pub fn register_types() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        ferroui_controls::register_types();
        TypeInfo::register_namespaces(NAMESPACES);
        TypeInfo::register_table(TYPES);
        MarkupType::register_table(&[crate::ParallaxSourceOffsetKind::MARKUP]);
        register_rust_paths();
        MarkupAssembly::register_table(&ASSEMBLY);
    });
}
