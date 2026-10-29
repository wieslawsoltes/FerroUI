//! ferroui-xaml-interactions
//!
//! The classic triggers and actions of the behaviors of FerroUI, ported from the
//! `Xaml.Behaviors.Interactions` project of Xaml.Behaviors (see `NOTICE.md`):
//!
//! - triggers on events ([`EventTriggerBehavior`]), on bound data ([`DataTriggerBehavior`],
//!   [`MultiDataTriggerBehavior`]), on timers ([`TimerTrigger`]), on tasks
//!   ([`TaskCompletedTrigger`]) and on observables ([`ObservableStreamBehavior`]);
//! - actions that change a property ([`ChangePropertyAction`], reversible), call a method
//!   ([`CallMethodAction`]) or invoke a command ([`InvokeCommandAction`]), and the action
//!   containers that delay, debounce, throttle or group their actions;
//! - the clipboard actions and the storage pickers (actions and behaviors of buttons and menu
//!   items) of the services of the top level;
//! - behind the features `file-system` and `network`, the actions and the triggers of the local
//!   file system and of the network.
//!
//! The namespaces of the crate are `FerroUI.Xaml.Interactions.Core` (with the clipboard and the
//! storage pickers), `FerroUI.Xaml.Interactions.FileSystem` and
//! `FerroUI.Xaml.Interactions.Network`, mapped into the default XML namespace of the framework:
//! documents name its types without a prefix once [`register_types`] ran.

mod action_task;
#[cfg(any(feature = "file-system", feature = "network"))]
mod background;
mod clipboard;
mod core;
#[cfg(feature = "file-system")]
pub mod file_system;
mod markup_types;
#[cfg(feature = "network")]
pub mod network;
mod platform_handles;
mod register_types;
mod rust_paths;
mod storage_provider;

pub use crate::action_task::{delay, ActionTask, Delay, TaskHandle};
pub use crate::clipboard::{
    ClearClipboardAction, GetClipboardDataAction, GetClipboardFormatsAction, GetClipboardTextAction,
    SetClipboardDataObjectAction, SetClipboardTextAction,
};
pub use crate::core::{
    AsyncActionGroup, AsyncActionMode, CallMethodAction, ChangePropertyAction, DataTrigger, DataTriggerBehavior,
    DebounceAction, DelayAction, EventTrigger, EventTriggerBehavior, InvokeCommandAction, MultiDataTrigger,
    MultiDataTriggerBehavior, ObservableStreamBehavior, ReversibleActionExecution, TaskCompletedTrigger,
    ThrottleAction, TimerTrigger,
};
pub use crate::platform_handles::{
    AsyncDataTransferRef, ClipboardRef, StorageFileRef, StorageFolderRef, StorageItemRef, StorageProviderRef,
};
pub use crate::register_types::{register_types, ASSEMBLY};
pub use crate::storage_provider::{
    ButtonOpenFilePickerBehavior, ButtonOpenFolderPickerBehavior, ButtonSaveFilePickerBehavior, FilePickerEventArgs,
    FolderPickerEventArgs, MenuItemOpenFilePickerBehavior, MenuItemOpenFolderPickerBehavior,
    MenuItemSaveFilePickerBehavior, OpenFilePickerAction, OpenFilePickerActionImpl, OpenFilePickerActionImplExt,
    OpenFilePickerActionVTable, OpenFilePickerBehaviorBase, OpenFilePickerBehaviorBaseImpl,
    OpenFilePickerBehaviorBaseImplExt, OpenFilePickerBehaviorBaseVTable, OpenFolderPickerAction,
    OpenFolderPickerActionImpl, OpenFolderPickerActionImplExt, OpenFolderPickerActionVTable,
    OpenFolderPickerBehaviorBase, OpenFolderPickerBehaviorBaseImpl, OpenFolderPickerBehaviorBaseImplExt,
    OpenFolderPickerBehaviorBaseVTable, PickerActionBase, PickerBehaviorBase, SaveFilePickerAction,
    SaveFilePickerActionImpl, SaveFilePickerActionImplExt, SaveFilePickerActionVTable, SaveFilePickerBehaviorBase,
    SaveFilePickerBehaviorBaseImpl, SaveFilePickerBehaviorBaseImplExt, SaveFilePickerBehaviorBaseVTable,
    SaveFilePickerEventArgs, StorageFileToReadStreamConverter, StorageFileToWriteStreamConverter,
    StorageItemToPathConverter,
};

#[cfg(test)]
mod tests;
