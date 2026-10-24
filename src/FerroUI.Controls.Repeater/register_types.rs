//! The type table of this crate: its namespaces, its classes and what it states about itself
//! for markup.

use ferroui_base::metadata::{MarkupAssembly, XmlnsDefinition, FERRO_XML_NAMESPACE};
use ferroui_base::{StaticType, TypeInfo};

/// The dotted namespaces of the modules of this crate (WinUI's namespaces). The static helper
/// types declare `FerroUI.Controls.Repeater` themselves.
const NAMESPACES: &[(&str, &str)] = &[
    ("ferroui_controls_repeater", "FerroUI.Controls"),
    ("ferroui_controls_repeater::repeater_automation_peer", "FerroUI.Automation.Peers"),
];

/// What this crate states about itself for markup: its assembly name and the namespace the XML
/// namespace of the framework maps to.
pub static ASSEMBLY: MarkupAssembly = MarkupAssembly {
    name: "FerroUI.Controls.Repeater",
    crate_name: "ferroui_controls_repeater",
    xmlns_definitions: &[XmlnsDefinition { xml_namespace: FERRO_XML_NAMESPACE, namespace: "FerroUI.Controls" }],
    xmlns_prefixes: &[],
    metadata: &[],
};

macro_rules! types {
    ($($type_:ty),* $(,)?) => {
        &[$(<$type_ as StaticType>::TYPE),*]
    };
}

const TYPES: &[&TypeInfo] = types![
    // FerroUI.Automation.Peers
    crate::RepeaterAutomationPeer,
    // FerroUI.Controls
    crate::ItemsRepeater,
    crate::ItemsRepeaterScrollHost,
    crate::LayoutPanel,
    crate::Layout,
    crate::VirtualizingLayout,
    crate::NonVirtualizingLayout,
    crate::LayoutContext,
    crate::VirtualizingLayoutContext,
    crate::NonVirtualizingLayoutContext,
    crate::StackLayout,
    crate::StackLayoutState,
    crate::FlowLayout,
    crate::FlowLayoutState,
    crate::UniformGridLayout,
    crate::UniformGridLayoutState,
    crate::LinedFlowLayout,
    crate::ElementFactory,
    crate::RecyclePool,
    crate::RecyclingElementFactory,
    crate::RepeaterBindingHelper,
    crate::ItemCollectionTransitionProvider,
    crate::LinedFlowLayoutItemCollectionTransitionProvider,
    crate::animation_manager::RepeaterAnimationProxy,
];

/// The test hooks (WinUI keeps them in a private namespace): registered only with the feature
/// `testing`.
#[cfg(any(test, feature = "testing"))]
const TEST_HOOK_TYPES: &[&TypeInfo] = types![crate::RepeaterTestHooks, crate::LayoutsTestHooks];

/// Registers the namespaces, the types and the assembly of this crate (and of the crates it is
/// built on) and the markup metadata of its types. Cheap and idempotent.
pub fn register_types() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        ferroui_controls::register_types();
        TypeInfo::register_namespaces(NAMESPACES);
        TypeInfo::register_all(TYPES);
        #[cfg(any(test, feature = "testing"))]
        TypeInfo::register_all(TEST_HOOK_TYPES);
        crate::rust_paths::register_rust_paths();
        MarkupAssembly::register(&ASSEMBLY);
        crate::markup::register_types();
        crate::layouts::layouts_markup::register_types();
        crate::services_markup::register_types();
        crate::selection::register_markup_types();
    });
}
