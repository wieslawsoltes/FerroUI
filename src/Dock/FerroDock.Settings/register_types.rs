//! Original settings assembly identity and shared native metadata registration.
use crate::*;
use ferroui_base::metadata::{
    MarkupAssembly, MarkupType, MarkupTyped, XmlnsDefinition, FERRO_XML_NAMESPACE,
};
use ferroui_base::{StaticType, TypeInfo};
const NAMESPACES: &[(&str, &str)] = &[("ferrodock_settings", "FerroDock.Settings")];
/// Original framework XML namespace mapping for the native settings assembly.
pub static ASSEMBLY: MarkupAssembly = MarkupAssembly {
    name: "FerroDock.Settings",
    crate_name: "ferrodock_settings",
    xmlns_definitions: &[XmlnsDefinition {
        xml_namespace: FERRO_XML_NAMESPACE,
        namespace: "FerroDock.Settings",
    }],
    xmlns_prefixes: &[],
    metadata: &[],
};
ferroui_base::ferro_rust_paths! {classes:[crate::DockProperties],types:[crate::DockSettings,crate::DockSettingsOptions,crate::DockFloatingWindowOwnerPolicy,crate::DockGlobalDockingPreset,crate::DockCommandBarMergingScope,crate::WindowDragDockScope],contracts:[],generics:[],generic_contracts:[]}
/// Registers original settings/property metadata without initializing an application.
pub fn register_types() {
    ferrodock_model_ferro::register_types();
    ferroui_controls::register_types();
    TypeInfo::register_namespaces(NAMESPACES);
    TypeInfo::register_all(&[<DockProperties as StaticType>::TYPE]);
    register_rust_paths();
    MarkupType::register_all(&[
        DockSettings::MARKUP,
        DockSettingsOptions::MARKUP,
        DockFloatingWindowOwnerPolicy::MARKUP,
        DockGlobalDockingPreset::MARKUP,
        DockCommandBarMergingScope::MARKUP,
        WindowDragDockScope::MARKUP,
    ]);
    MarkupAssembly::register(&ASSEMBLY);
}
