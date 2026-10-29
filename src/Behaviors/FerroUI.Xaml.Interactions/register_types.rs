//! The type table of this crate: its namespaces, its classes and what it states about itself for
//! markup (the `XmlnsDefinition`s of `Properties/AssemblyInfo.cs`, which map the namespaces of the
//! crate into the default XML namespace of the framework, so documents name the triggers and the
//! actions without a prefix).

use ferroui_base::metadata::{MarkupAssembly, XmlnsDefinition, FERRO_XML_NAMESPACE};
use ferroui_base::{StaticType, TypeInfo};

/// The dotted namespaces of the modules of this crate (the namespaces of the reference, with the
/// name of the framework): the clipboard actions and the storage pickers are in the namespace of
/// the core triggers and actions, as in the reference.
const NAMESPACES: &[(&str, &str)] = &[
    ("ferroui_xaml_interactions", "FerroUI.Xaml.Interactions.Core"),
    ("ferroui_xaml_interactions::file_system", "FerroUI.Xaml.Interactions.FileSystem"),
    ("ferroui_xaml_interactions::network", "FerroUI.Xaml.Interactions.Network"),
];

/// What this crate states about itself for markup: its assembly name and the namespaces the XML
/// namespace of the framework maps to.
pub static ASSEMBLY: MarkupAssembly = MarkupAssembly {
    name: "FerroUI.Xaml.Interactions",
    crate_name: "ferroui_xaml_interactions",
    xmlns_definitions: &[
        XmlnsDefinition { xml_namespace: FERRO_XML_NAMESPACE, namespace: "FerroUI.Xaml.Interactions" },
        XmlnsDefinition { xml_namespace: FERRO_XML_NAMESPACE, namespace: "FerroUI.Xaml.Interactions.Core" },
        XmlnsDefinition { xml_namespace: FERRO_XML_NAMESPACE, namespace: "FerroUI.Xaml.Interactions.FileSystem" },
        XmlnsDefinition { xml_namespace: FERRO_XML_NAMESPACE, namespace: "FerroUI.Xaml.Interactions.Network" },
    ],
    xmlns_prefixes: &[],
    metadata: &[],
};

macro_rules! types {
    ($($type_:ty),* $(,)?) => {
        &[$(<$type_ as StaticType>::TYPE),*]
    };
}

/// The classes of the core triggers and actions, the clipboard actions and the storage pickers.
const TYPES: &[&TypeInfo] = types![
    crate::AsyncActionGroup,
    crate::ButtonOpenFilePickerBehavior,
    crate::ButtonOpenFolderPickerBehavior,
    crate::ButtonSaveFilePickerBehavior,
    crate::CallMethodAction,
    crate::ChangePropertyAction,
    crate::ClearClipboardAction,
    crate::DataTrigger,
    crate::DataTriggerBehavior,
    crate::DebounceAction,
    crate::DelayAction,
    crate::EventTrigger,
    crate::EventTriggerBehavior,
    crate::GetClipboardDataAction,
    crate::GetClipboardFormatsAction,
    crate::GetClipboardTextAction,
    crate::InvokeCommandAction,
    crate::MenuItemOpenFilePickerBehavior,
    crate::MenuItemOpenFolderPickerBehavior,
    crate::MenuItemSaveFilePickerBehavior,
    crate::MultiDataTrigger,
    crate::MultiDataTriggerBehavior,
    crate::ObservableStreamBehavior,
    crate::OpenFilePickerAction,
    crate::OpenFilePickerBehaviorBase,
    crate::OpenFolderPickerAction,
    crate::OpenFolderPickerBehaviorBase,
    crate::PickerActionBase,
    crate::PickerBehaviorBase,
    crate::SaveFilePickerAction,
    crate::SaveFilePickerBehaviorBase,
    crate::SetClipboardDataObjectAction,
    crate::SetClipboardTextAction,
    crate::TaskCompletedTrigger,
    crate::ThrottleAction,
    crate::TimerTrigger,
];

/// The classes of the file system (feature `file-system`).
#[cfg(feature = "file-system")]
const FILE_SYSTEM_TYPES: &[&TypeInfo] = types![
    crate::file_system::CreateDirectoryAction,
    crate::file_system::DeleteDirectoryAction,
    crate::file_system::DeleteFileAction,
    crate::file_system::FileSystemWatcherTrigger,
    crate::file_system::WriteTextToFileAction,
];

/// The classes of the network (feature `network`).
#[cfg(feature = "network")]
const NETWORK_TYPES: &[&TypeInfo] = types![crate::network::HttpRequestAction, crate::network::NetworkInformationTrigger];

/// Registers the namespaces, the types, the markup metadata and the assembly of this crate (and of
/// the crates it is built on). Cheap and idempotent.
pub fn register_types() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        // The object model of the behaviors, and with it the controls and the base crate.
        ferroui_xaml_interactivity::register_types();
        TypeInfo::register_namespaces(NAMESPACES);
        TypeInfo::register_all(TYPES);
        #[cfg(feature = "file-system")]
        TypeInfo::register_all(FILE_SYSTEM_TYPES);
        #[cfg(feature = "network")]
        TypeInfo::register_all(NETWORK_TYPES);
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
    fn every_type_of_this_crate_has_the_core_namespace() {
        register_types();

        for type_ in TYPES {
            assert_eq!("FerroUI.Xaml.Interactions.Core", type_.namespace(), "{type_} has another namespace");
        }
        assert!(TypeInfo::find("FerroUI.Xaml.Interactions.Core", "EventTriggerBehavior").is_some());
    }

    #[test]
    fn the_assembly_maps_the_namespaces_into_the_xml_namespace_of_the_framework() {
        register_types();

        let assembly = MarkupAssembly::find("FerroUI.Xaml.Interactions").expect("the assembly is registered");
        assert!(std::ptr::eq(assembly, &ASSEMBLY));
        assert!(assembly.xmlns_definitions.iter().all(|definition| definition.xml_namespace == FERRO_XML_NAMESPACE
            && definition.namespace.starts_with("FerroUI.Xaml.Interactions")));
    }
}
