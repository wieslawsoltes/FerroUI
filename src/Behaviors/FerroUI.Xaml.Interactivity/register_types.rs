//! The type table of this crate: its namespace, its classes and what it states about itself for
//! markup (the `XmlnsDefinition` of `Properties/AssemblyInfo.cs`, which maps the namespace of the
//! crate into the default XML namespace of the framework, so documents name the behaviors
//! without a prefix).

use ferroui_base::metadata::{MarkupAssembly, XmlnsDefinition, FERRO_XML_NAMESPACE};
use ferroui_base::{StaticType, TypeInfo};

/// The dotted namespace of the modules of this crate (the namespace of the reference, with the
/// name of the framework).
const NAMESPACES: &[(&str, &str)] = &[("ferroui_xaml_interactivity", "FerroUI.Xaml.Interactivity")];

/// What this crate states about itself for markup: its assembly name and the namespace the XML
/// namespace of the framework maps to.
pub static ASSEMBLY: MarkupAssembly = MarkupAssembly {
    name: "FerroUI.Xaml.Interactivity",
    crate_name: "ferroui_xaml_interactivity",
    xmlns_definitions: &[XmlnsDefinition { xml_namespace: FERRO_XML_NAMESPACE, namespace: "FerroUI.Xaml.Interactivity" }],
    xmlns_prefixes: &[],
    metadata: &[],
};

macro_rules! types {
    ($($type_:ty),* $(,)?) => {
        &[$(<$type_ as StaticType>::TYPE),*]
    };
}

const TYPES: &[&TypeInfo] = types![
    crate::Action,
    crate::Behavior,
    crate::BehaviorOfT,
    crate::Condition,
    crate::DisposingBehavior,
    crate::DisposingTrigger,
    crate::EventTriggerBase,
    crate::Interaction,
    crate::InteractiveBehaviorBase,
    crate::InteractiveTriggerBase,
    crate::InvokeCommandActionBase,
    crate::InvokeCommandBehaviorBase,
    crate::StyledElementAction,
    crate::StyledElementBehavior,
    crate::StyledElementBehaviorOfT,
    crate::StyledElementTrigger,
    crate::StyledElementTriggerOfT,
    crate::Trigger,
    crate::TriggerOfT,
];

/// Registers the namespace, the types, the markup metadata and the assembly of this crate (and of
/// the crates it is built on). Cheap and idempotent.
pub fn register_types() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        // The runtime library of markup (the template content of the collection templates), and
        // with it the controls and the base crate.
        ferroui_markup_xaml::register_types();
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
            assert_eq!("FerroUI.Xaml.Interactivity", type_.namespace(), "{type_} has another namespace");
        }
        assert!(TypeInfo::find("FerroUI.Xaml.Interactivity", "StyledElementTrigger").is_some());
    }

    #[test]
    fn the_assembly_maps_the_namespace_into_the_xml_namespace_of_the_framework() {
        register_types();

        let assembly = MarkupAssembly::find("FerroUI.Xaml.Interactivity").expect("the assembly is registered");
        assert!(std::ptr::eq(assembly, &ASSEMBLY));
        assert!(assembly
            .xmlns_definitions
            .iter()
            .all(|definition| definition.xml_namespace == FERRO_XML_NAMESPACE
                && definition.namespace == "FerroUI.Xaml.Interactivity"));
    }
}
