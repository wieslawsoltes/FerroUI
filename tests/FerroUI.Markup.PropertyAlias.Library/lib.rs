//! An attached property with markup aliases on `Control`, for the tests of the alias.

extern crate self as ferroui_markup_property_alias_library;

use ferroui_base::metadata::{MarkupAssembly, XmlnsDefinition, FERRO_XML_NAMESPACE};
use ferroui_base::{ferro_properties, ferro_static_type, AttachedProperty, FerroProperty, StaticType, TypeInfo};
use ferroui_controls::Control;

/// Owns the attached property the aliases stand for.
pub struct AliasedProperties;

ferro_static_type!(AliasedProperties);

ferro_properties! {
    impl AliasedProperties {
        /// Defines the `Fade` attached property.
        pub fn fade_property() -> AttachedProperty<f64> {
            FerroProperty::register_attached::<AliasedProperties, Control, _>("Fade", 0.0)
        }
    }
}

impl AliasedProperties {
    /// The value of `Fade`.
    pub fn get_fade(element: &Control) -> f64 {
        element.get_value(Self::fade_property())
    }

    /// Sets the value of `Fade`.
    pub fn set_fade(element: &Control, value: f64) {
        element.set_value(Self::fade_property(), value);
    }
}

/// `Control.Fade` is the alias. `Control.Width` is one too, which the real property must beat.
pub static ASSEMBLY: MarkupAssembly = MarkupAssembly {
    name: "FerroUI.Markup.PropertyAlias.Library",
    crate_name: "ferroui_markup_property_alias_library",
    xmlns_definitions: &[XmlnsDefinition { xml_namespace: FERRO_XML_NAMESPACE, namespace: "FerroUI.Markup.PropertyAlias" }],
    xmlns_prefixes: &[],
    metadata: &[
        ("FerroXamlPropertyAlias", "FerroUI.Controls.Control.FadeAmount=FerroUI.Markup.PropertyAlias.AliasedProperties.Fade"),
        ("FerroXamlPropertyAlias", "FerroUI.Controls.Control.Width=FerroUI.Markup.PropertyAlias.AliasedProperties.Fade"),
        ("FerroXamlPropertyAlias", "FerroUI.Controls.Control.Twice=FerroUI.Markup.PropertyAlias.AliasedProperties.Fade"),
    ],
};

ferroui_base::ferro_rust_paths! {
    classes: [],
    types: [crate::AliasedProperties],
    contracts: [], generics: [], generic_contracts: [],
}

const NAMESPACES: &[(&str, &str)] = &[("ferroui_markup_property_alias_library", "FerroUI.Markup.PropertyAlias")];

const TYPES: &[&TypeInfo] = &[<AliasedProperties as StaticType>::TYPE];

/// Registers the types of this crate for markup.
pub fn register_types() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        ferroui_controls::register_types();
        TypeInfo::register_namespaces(NAMESPACES);
        TypeInfo::register_table(TYPES);
        register_rust_paths();
        MarkupAssembly::register_table(&ASSEMBLY);
    });
}
