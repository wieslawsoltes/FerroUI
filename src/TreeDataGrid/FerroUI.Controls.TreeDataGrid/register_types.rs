//! The type table of this crate: its namespaces, its classes, what it states about itself for
//! markup, its embedded assets and its compiled markup.

use ferroui_base::metadata::{MarkupAssembly, XmlnsDefinition, FERRO_XML_NAMESPACE};
use ferroui_base::{StaticType, TypeInfo};

/// The dotted namespaces of the modules of this crate (the reference's namespaces).
const NAMESPACES: &[(&str, &str)] = &[
    ("ferroui_controls_treedatagrid", "FerroUI.Controls"),
    ("ferroui_controls_treedatagrid::automation::peers", "FerroUI.Automation.Peers"),
    ("ferroui_controls_treedatagrid::converters", "FerroUI.Controls.Converters"),
    ("ferroui_controls_treedatagrid::models", "FerroUI.Controls.Models.TreeDataGrid"),
    ("ferroui_controls_treedatagrid::presentation", "FerroUI.Controls.Presentation"),
    ("ferroui_controls_treedatagrid::primitives", "FerroUI.Controls.Primitives"),
    ("ferroui_controls_treedatagrid::selection", "FerroUI.Controls.Selection"),
];

/// What this crate states about itself for markup: its assembly name and the namespaces the XML
/// namespace of the framework maps to (the `XmlnsDefinition` attributes of the reference project).
pub static ASSEMBLY: MarkupAssembly = MarkupAssembly {
    name: "FerroUI.Controls.TreeDataGrid",
    crate_name: "ferroui_controls_treedatagrid",
    xmlns_definitions: &[
        XmlnsDefinition { xml_namespace: FERRO_XML_NAMESPACE, namespace: "FerroUI.Controls" },
        XmlnsDefinition { xml_namespace: FERRO_XML_NAMESPACE, namespace: "FerroUI.Controls.Primitives" },
    ],
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
    crate::automation::peers::TreeDataGridAutomationPeer,
    crate::automation::peers::TreeDataGridCellAutomationPeer,
    crate::automation::peers::TreeDataGridCheckBoxCellAutomationPeer,
    crate::automation::peers::TreeDataGridColumnHeaderAutomationPeer,
    crate::automation::peers::TreeDataGridColumnHeadersPresenterAutomationPeer,
    crate::automation::peers::TreeDataGridRowAutomationPeer,
    // FerroUI.Controls
    crate::TreeDataGrid,
    crate::TreeDataGridBindingProbe,
    // FerroUI.Controls.Primitives
    crate::primitives::TreeDataGridCell,
    crate::primitives::TreeDataGridCellsPresenter,
    crate::primitives::TreeDataGridCheckBoxCell,
    crate::primitives::TreeDataGridColumnHeader,
    crate::primitives::TreeDataGridColumnHeadersPresenter,
    crate::primitives::TreeDataGridExpanderCell,
    crate::primitives::TreeDataGridRow,
    crate::primitives::TreeDataGridRowsPresenter,
    crate::primitives::TreeDataGridTemplateCell,
    crate::primitives::TreeDataGridTextCell,
];

/// Registers the namespaces, the types, the assembly and the compiled markup of this crate (and of
/// the crates it is built on), and with the feature `document-assets` the theme documents as
/// assets. Cheap and idempotent.
pub fn register_types() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        ferroui_markup_xaml::register_types();
        ferroui_treedatagrid_core::register_types();
        TypeInfo::register_namespaces(NAMESPACES);
        TypeInfo::register_all(TYPES);
        crate::rust_paths::register_rust_paths();
        MarkupAssembly::register(&ASSEMBLY);
        crate::markup_types::register();
        #[cfg(any(test, feature = "document-assets"))]
        crate::assets::register();
        crate::compiled_markup::register();
    });
}
