//! The public Rust paths of the registered types of this crate, by the shortest path another crate
//! names it by; the emitter of Rust source of the markup compiler reads them. The types of the
//! features of the crate are listed in modules of their own, built with the features.

mod core_paths {
    ferroui_base::ferro_rust_paths! {
        classes: [
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
        ],
        types: [
            crate::AsyncActionMode,
            crate::StorageFileToReadStreamConverter,
            crate::StorageFileToWriteStreamConverter,
            crate::StorageItemToPathConverter,
        ],
        contracts: [
        ],
        generics: [
        ],
        generic_contracts: [
        ],
    }
}

#[cfg(feature = "file-system")]
mod file_system_paths {
    ferroui_base::ferro_rust_paths! {
        classes: [
            crate::file_system::CreateDirectoryAction,
            crate::file_system::DeleteDirectoryAction,
            crate::file_system::DeleteFileAction,
            crate::file_system::FileSystemWatcherTrigger,
            crate::file_system::WriteTextToFileAction,
        ],
        types: [
            crate::file_system::WatcherChangeTypes,
        ],
        contracts: [
        ],
        generics: [
        ],
        generic_contracts: [
        ],
    }
}

#[cfg(feature = "network")]
mod network_paths {
    ferroui_base::ferro_rust_paths! {
        classes: [
            crate::network::HttpRequestAction,
            crate::network::NetworkInformationTrigger,
        ],
        types: [
        ],
        contracts: [
        ],
        generics: [
        ],
        generic_contracts: [
        ],
    }
}

/// Records the public Rust paths of this crate.
pub(crate) fn register_rust_paths() {
    core_paths::register_rust_paths();
    #[cfg(feature = "file-system")]
    file_system_paths::register_rust_paths();
    #[cfg(feature = "network")]
    network_paths::register_rust_paths();
}
