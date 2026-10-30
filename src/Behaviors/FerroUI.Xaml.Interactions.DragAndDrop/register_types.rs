//! The type table of this crate: its namespaces, its classes, what it states about itself for
//! markup (the `XmlnsDefinition` of `Properties/AssemblyInfo.cs`, which maps the namespace of the
//! crate into the default XML namespace of the framework, so documents name the behaviors without
//! a prefix), its embedded assets and its compiled markup.

use ferroui_base::metadata::{MarkupAssembly, XmlnsDefinition, FERRO_XML_NAMESPACE};
use ferroui_base::{StaticType, TypeInfo};

/// The dotted namespaces of the modules of this crate (the namespaces of the reference, with the
/// name of the framework). The preview window is in the namespace `..DragAndDrop.Controls`, as in
/// the reference, which does not map that one into the XML namespace of the framework.
const NAMESPACES: &[(&str, &str)] = &[
    ("ferroui_xaml_interactions_draganddrop", "FerroUI.Xaml.Interactions.DragAndDrop"),
    (
        "ferroui_xaml_interactions_draganddrop::managed_drag_drop::drag_preview_window",
        "FerroUI.Xaml.Interactions.DragAndDrop.Controls",
    ),
];

/// What this crate states about itself for markup: its assembly name and the namespace the XML
/// namespace of the framework maps to.
pub static ASSEMBLY: MarkupAssembly = MarkupAssembly {
    name: "FerroUI.Xaml.Interactions.DragAndDrop",
    crate_name: "ferroui_xaml_interactions_draganddrop",
    xmlns_definitions: &[XmlnsDefinition {
        xml_namespace: FERRO_XML_NAMESPACE,
        namespace: "FerroUI.Xaml.Interactions.DragAndDrop",
    }],
    xmlns_prefixes: &[],
    metadata: &[(MarkupAssembly::USE_COMPILED_BINDINGS_BY_DEFAULT, "True")],
};

macro_rules! types {
    ($($type_:ty),* $(,)?) => {
        &[$(<$type_ as StaticType>::TYPE),*]
    };
}

const TYPES: &[&TypeInfo] = types![
    crate::AddPreviewFilesAction,
    crate::ContentControlFilesDropBehavior,
    crate::ContextDragBehavior,
    crate::ContextDragBehaviorBase,
    crate::ContextDragWithDirectionBehavior,
    crate::ContextDropBehavior,
    crate::ContextDropBehaviorBase,
    crate::DragAndDropEventsBehavior,
    crate::DragDropCommandsBehavior,
    crate::DragPreviewWindow,
    crate::DropBehaviorBase,
    crate::FilesDropBehavior,
    crate::FilesPreviewBehavior,
    crate::ManagedContextDragBehavior,
    crate::ManagedContextDropBehavior,
    crate::PanelDragBehavior,
    crate::PanelDropBehavior,
    crate::TextDropBehavior,
    crate::TypedDragBehavior,
    crate::TypedDragBehaviorBase,
];

/// Registers the namespaces, the types, the assembly and the compiled markup of this crate (and of
/// the crates it is built on), and with the feature `document-assets` the styles document as an
/// asset. Cheap and idempotent.
pub fn register_types() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        // The behaviors object model, and with it the runtime library of compiled markup, the
        // controls and the base crate.
        ferroui_xaml_interactivity::register_types();
        TypeInfo::register_namespaces(NAMESPACES);
        TypeInfo::register_all(TYPES);
        crate::rust_paths::register_rust_paths();
        crate::markup_types::register();
        MarkupAssembly::register(&ASSEMBLY);
        // The styles document as an asset: what lets a document the run-time loader loads include
        // it.
        #[cfg(any(test, feature = "document-assets"))]
        crate::assets::register();
        // The loader table of the compiled markup, which the build of the crate generates: a load
        // of the styles document by its URI builds it from its compiled markup.
        crate::compiled_markup::register();
    });
}

#[cfg(test)]
mod tests {
    // Not from the reference: checks the type table of the crate.
    use super::*;

    #[test]
    fn every_type_of_this_crate_has_a_namespace_of_the_crate() {
        register_types();

        for type_ in TYPES {
            assert!(
                type_.namespace().starts_with("FerroUI.Xaml.Interactions.DragAndDrop"),
                "{type_} has another namespace"
            );
        }
        assert!(TypeInfo::find("FerroUI.Xaml.Interactions.DragAndDrop", "ContextDropBehavior").is_some());
        assert_eq!("FerroUI.Xaml.Interactions.DragAndDrop.Controls", crate::DragPreviewWindow::TYPE.namespace());
    }

    #[test]
    fn the_assembly_maps_the_namespace_into_the_xml_namespace_of_the_framework() {
        register_types();

        let assembly =
            MarkupAssembly::find("FerroUI.Xaml.Interactions.DragAndDrop").expect("the assembly is registered");
        assert!(std::ptr::eq(assembly, &ASSEMBLY));
        assert!(assembly
            .xmlns_definitions
            .iter()
            .all(|definition| definition.xml_namespace == FERRO_XML_NAMESPACE
                && definition.namespace == "FerroUI.Xaml.Interactions.DragAndDrop"));
    }
}
