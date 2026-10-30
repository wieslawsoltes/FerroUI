//! The type table of this crate: its namespaces, its classes, what it states about itself for
//! markup, its embedded assets and its compiled markup.

use ferroui_base::metadata::{MarkupAssembly, XmlnsDefinition, FERRO_XML_NAMESPACE};
use ferroui_base::{StaticType, TypeInfo};

/// The dotted namespaces of the modules of this crate (WinUI's namespaces).
const NAMESPACES: &[(&str, &str)] = &[
    ("ferroui_controls_inking", "FerroUI.Controls"),
    ("ferroui_controls_inking::ink_canvas_automation_peer", "FerroUI.Automation.Peers"),
    ("ferroui_controls_inking::ink_toolbar_automation_peer", "FerroUI.Automation.Peers"),
];

/// What this crate states about itself for markup: its assembly name and the namespace the XML
/// namespace of the framework maps to.
pub static ASSEMBLY: MarkupAssembly = MarkupAssembly {
    name: "FerroUI.Controls.Inking",
    crate_name: "ferroui_controls_inking",
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
    crate::InkCanvasAutomationPeer,
    crate::InkToolbarAutomationPeer,
    crate::InkToolbarToolButtonAutomationPeer,
    crate::InkToolbarMenuButtonAutomationPeer,
    crate::InkToolbarFlyoutItemAutomationPeer,
    crate::InkToolbarStrokeWidthSliderAutomationPeer,
    // FerroUI.Controls
    crate::InkCanvas,
    crate::InkToolbar,
    crate::InkToolbarToolButton,
    crate::InkToolbarPenButton,
    crate::InkToolbarBallpointPenButton,
    crate::InkToolbarPencilButton,
    crate::InkToolbarHighlighterButton,
    crate::InkToolbarEraserButton,
    crate::InkToolbarCustomPen,
    crate::InkToolbarCustomPenButton,
    crate::InkToolbarCustomToolButton,
    crate::InkToolbarToggleButton,
    crate::InkToolbarCustomToggleButton,
    crate::InkToolbarRulerButton,
    crate::InkToolbarMenuButton,
    crate::InkToolbarStencilButton,
    crate::InkToolbarStrokeWidthSlider,
    crate::InkToolbarPenConfigurationControl,
    crate::InkToolbarFlyoutItem,
];

/// Registers the namespaces, the types, the assembly, the markup metadata and the compiled markup
/// of this crate (and of the crates it is built on), and with the feature `document-assets` the
/// theme documents as assets. Cheap and idempotent.
pub fn register_types() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        ferroui_markup_xaml::register_types();
        ferroui_themes_winui::register_types();
        TypeInfo::register_namespaces(NAMESPACES);
        TypeInfo::register_all(TYPES);
        crate::rust_paths::register_rust_paths();
        MarkupAssembly::register(&ASSEMBLY);
        crate::markup::register_markup();
        #[cfg(any(test, feature = "document-assets"))]
        crate::assets::register();
        crate::compiled_markup::register();
    });
}
