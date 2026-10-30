//! The type table of this crate: its namespace, its types and what it states about itself for
//! markup (`Properties/AssemblyInfo.cs` of the reference: the `XmlnsDefinition` that maps its
//! namespace into the default XML namespace of the framework).
//!
//! The namespace of every type of the reference is `FerroUI.Xaml.Interactions.Custom` (the
//! namespace of the custom behaviors, which forward these types), whatever folder of the
//! reference it comes from; every module of the crate maps to it.

use crate::SelectingItemsControlBehavior;
use ferroui_base::metadata::{MarkupAssembly, XmlnsDefinition, FERRO_XML_NAMESPACE};
use ferroui_base::{StaticType, TypeInfo};

/// The dotted namespace of the modules of this crate.
const NAMESPACES: &[(&str, &str)] = &[("ferroui_xaml_behaviors_animations", "FerroUI.Xaml.Interactions.Custom")];

/// What this crate states about itself for markup: its assembly name and its namespace mapped
/// into the default XML namespace (`[assembly: XmlnsDefinition(.., "..Interactions.Custom")]`).
pub static ASSEMBLY: MarkupAssembly = MarkupAssembly {
    name: "FerroUI.Xaml.Behaviors.Animations",
    crate_name: "ferroui_xaml_behaviors_animations",
    xmlns_definitions: &[XmlnsDefinition {
        xml_namespace: FERRO_XML_NAMESPACE,
        namespace: "FerroUI.Xaml.Interactions.Custom",
    }],
    xmlns_prefixes: &[],
    metadata: &[],
};

/// The types of the object model: the static owner of the `EnableSelectionAnimation` attached
/// property.
const TYPES: &[&TypeInfo] = &[<SelectingItemsControlBehavior as StaticType>::TYPE];

/// Registers the namespace, the types and the assembly of this crate (and of the crates it is
/// built on). Cheap and idempotent.
pub fn register_types() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        ferroui_controls::register_types();
        TypeInfo::register_namespaces(NAMESPACES);
        TypeInfo::register_all(TYPES);
        crate::rust_paths::register_rust_paths();
        crate::markup_types::register();
        MarkupAssembly::register(&ASSEMBLY);
    });
}
