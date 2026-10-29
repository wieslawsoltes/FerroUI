//! The storage pickers (`StorageProvider/`, namespace `FerroUI.Xaml.Interactions.Core`): actions
//! and behaviors that open the file and folder pickers of the storage provider of the top level,
//! and the converters of the storage items.

mod button;
mod converters;
mod core;
mod menu_item;
mod open_file_picker_action;
mod open_file_picker_behavior_base;
mod open_folder_picker_action;
mod open_folder_picker_behavior_base;
mod save_file_picker_action;
mod save_file_picker_behavior_base;
mod utilities;

pub use self::core::{
    FilePickerEventArgs, FolderPickerEventArgs, PickerActionBase, PickerBehaviorBase, SaveFilePickerEventArgs,
};
pub use button::{ButtonOpenFilePickerBehavior, ButtonOpenFolderPickerBehavior, ButtonSaveFilePickerBehavior};
pub use converters::{StorageFileToReadStreamConverter, StorageFileToWriteStreamConverter, StorageItemToPathConverter};
pub use menu_item::{MenuItemOpenFilePickerBehavior, MenuItemOpenFolderPickerBehavior, MenuItemSaveFilePickerBehavior};
pub use open_file_picker_action::{
    OpenFilePickerAction, OpenFilePickerActionImpl, OpenFilePickerActionImplExt, OpenFilePickerActionVTable,
};
pub use open_file_picker_behavior_base::{
    OpenFilePickerBehaviorBase, OpenFilePickerBehaviorBaseImpl, OpenFilePickerBehaviorBaseImplExt,
    OpenFilePickerBehaviorBaseVTable,
};
pub use open_folder_picker_action::{
    OpenFolderPickerAction, OpenFolderPickerActionImpl, OpenFolderPickerActionImplExt, OpenFolderPickerActionVTable,
};
pub use open_folder_picker_behavior_base::{
    OpenFolderPickerBehaviorBase, OpenFolderPickerBehaviorBaseImpl, OpenFolderPickerBehaviorBaseImplExt,
    OpenFolderPickerBehaviorBaseVTable,
};
pub use save_file_picker_action::{
    SaveFilePickerAction, SaveFilePickerActionImpl, SaveFilePickerActionImplExt, SaveFilePickerActionVTable,
};
pub use save_file_picker_behavior_base::{
    SaveFilePickerBehaviorBase, SaveFilePickerBehaviorBaseImpl, SaveFilePickerBehaviorBaseImplExt,
    SaveFilePickerBehaviorBaseVTable,
};
