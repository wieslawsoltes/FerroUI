//! The type table of this crate: its namespaces, its classes and what it states about itself for
//! markup (the `XmlnsDefinition`s of `Properties/AssemblyInfo.cs`, which map the namespaces of the
//! crate into the default XML namespace of the framework, so documents name the behaviors without
//! a prefix). Each part of the crate lists its own classes, Rust paths and markup types.

use ferroui_base::metadata::{MarkupAssembly, XmlnsDefinition, FERRO_XML_NAMESPACE};
use ferroui_base::TypeInfo;

/// The dotted namespaces of the modules of this crate (the namespaces of the reference, with the
/// name of the framework).
const NAMESPACES: &[(&str, &str)] = &[
    ("ferroui_xaml_interactions_custom", "FerroUI.Xaml.Interactions.Custom"),
    ("ferroui_xaml_interactions_custom::converters", "FerroUI.Xaml.Interactions.Custom.Converters"),
];

/// What this crate states about itself for markup: its assembly name and the namespaces the XML
/// namespace of the framework maps to.
pub static ASSEMBLY: MarkupAssembly = MarkupAssembly {
    name: "FerroUI.Xaml.Interactions.Custom",
    crate_name: "ferroui_xaml_interactions_custom",
    xmlns_definitions: &[
        XmlnsDefinition { xml_namespace: FERRO_XML_NAMESPACE, namespace: "FerroUI.Xaml.Interactions.Custom" },
        XmlnsDefinition {
            xml_namespace: FERRO_XML_NAMESPACE,
            namespace: "FerroUI.Xaml.Interactions.Custom.Converters",
        },
    ],
    xmlns_prefixes: &[],
    metadata: &[],
};

/// The classes of the parts of the crate.
const TYPE_LISTS: &[&[&TypeInfo]] = &[
    crate::part_a::TYPES,
    crate::part_b::TYPES,
    crate::part_c::TYPES,
    crate::part_d::TYPES,
    crate::converters::TYPES,
];

/// Registers the namespaces, the types, the markup metadata and the assembly of this crate (and of
/// the crates it is built on). Cheap and idempotent.
pub fn register_types() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        // The object model of the behaviors and the animations, and with them the controls and the
        // base crate.
        ferroui_xaml_interactivity::register_types();
        ferroui_xaml_behaviors_animations::register_types();
        TypeInfo::register_namespaces(NAMESPACES);
        for types in TYPE_LISTS {
            TypeInfo::register_all(types);
        }
        crate::part_a::register();
        crate::part_b::register();
        crate::part_c::register();
        crate::part_d::register();
        crate::converters::register();
        MarkupAssembly::register(&ASSEMBLY);
    });
}

#[cfg(test)]
mod tests {
    // Not from the reference: checks the type table of the crate.
    use super::*;

    #[test]
    fn every_class_of_this_crate_has_a_custom_namespace() {
        register_types();

        for types in TYPE_LISTS {
            for type_ in *types {
                assert!(
                    type_.namespace().starts_with("FerroUI.Xaml.Interactions.Custom"),
                    "{type_} has another namespace"
                );
            }
        }
    }

    #[test]
    fn the_assembly_maps_the_namespaces_into_the_xml_namespace_of_the_framework() {
        register_types();

        let assembly =
            MarkupAssembly::find("FerroUI.Xaml.Interactions.Custom").expect("the assembly is registered");
        assert!(std::ptr::eq(assembly, &ASSEMBLY));
        assert!(assembly.xmlns_definitions.iter().all(|definition| definition.xml_namespace == FERRO_XML_NAMESPACE));
    }
}
