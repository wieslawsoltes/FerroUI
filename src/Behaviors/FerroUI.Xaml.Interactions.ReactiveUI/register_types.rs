//! The type table of this crate: its namespace, its classes and what it states about itself for
//! markup (the `XmlnsDefinition` of `Properties/AssemblyInfo.cs`, which maps the namespace of the
//! crate into the default XML namespace of the framework, so documents name the actions and the
//! trigger without a prefix).

use ferroui_base::metadata::{MarkupAssembly, XmlnsDefinition, FERRO_XML_NAMESPACE};
use ferroui_base::{StaticType, TypeInfo};

/// The dotted namespace of the modules of this crate (the namespace of the reference, with the
/// name of the framework).
const NAMESPACES: &[(&str, &str)] = &[("ferroui_xaml_interactions_reactiveui", "FerroUI.Xaml.Interactions.ReactiveUI")];

/// What this crate states about itself for markup: its assembly name and the namespace the XML
/// namespace of the framework maps to.
pub static ASSEMBLY: MarkupAssembly = MarkupAssembly {
    name: "FerroUI.Xaml.Interactions.ReactiveUI",
    crate_name: "ferroui_xaml_interactions_reactiveui",
    xmlns_definitions: &[XmlnsDefinition {
        xml_namespace: FERRO_XML_NAMESPACE,
        namespace: "FerroUI.Xaml.Interactions.ReactiveUI",
    }],
    xmlns_prefixes: &[],
    metadata: &[],
};

macro_rules! types {
    ($($type_:ty),* $(,)?) => {
        &[$(<$type_ as StaticType>::TYPE),*]
    };
}

const TYPES: &[&TypeInfo] = types![
    crate::ClearNavigationStackAction,
    crate::InteractionTriggerBehavior,
    crate::NavigateAction,
    crate::NavigateAndReset,
    crate::NavigateBackAction,
    crate::NavigateToAction,
    crate::NavigateToAndResetAction,
];

/// Registers the namespace, the types, the value types and the assembly of this crate (and of
/// the crates it is built on). Cheap and idempotent.
pub fn register_types() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        // The behaviors object model, and with it the controls and the base crate.
        ferroui_xaml_interactivity::register_types();
        TypeInfo::register_namespaces(NAMESPACES);
        TypeInfo::register_all(TYPES);
        crate::rust_paths::register_rust_paths();
        crate::markup_types::register();
        MarkupAssembly::register(&ASSEMBLY);
    });
}

#[cfg(test)]
mod tests {
    // Not from the reference: checks the type table of the crate.
    use super::*;

    #[test]
    fn every_type_of_this_crate_has_the_namespace_of_the_crate() {
        register_types();

        for type_ in TYPES {
            assert_eq!("FerroUI.Xaml.Interactions.ReactiveUI", type_.namespace(), "{type_} has another namespace");
        }
        assert!(TypeInfo::find("FerroUI.Xaml.Interactions.ReactiveUI", "InteractionTriggerBehavior").is_some());
        assert!(TypeInfo::find("FerroUI.Xaml.Interactions.ReactiveUI", "NavigateAction").is_some());
    }

    #[test]
    fn the_assembly_maps_the_namespace_into_the_xml_namespace_of_the_framework() {
        register_types();

        let assembly =
            MarkupAssembly::find("FerroUI.Xaml.Interactions.ReactiveUI").expect("the assembly is registered");
        assert!(std::ptr::eq(assembly, &ASSEMBLY));
        assert!(assembly
            .xmlns_definitions
            .iter()
            .all(|definition| definition.xml_namespace == FERRO_XML_NAMESPACE
                && definition.namespace == "FerroUI.Xaml.Interactions.ReactiveUI"));
    }
}
