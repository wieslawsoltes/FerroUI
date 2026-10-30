//! The actions and the trigger of the local file system (`FileSystem/`, namespace
//! `FerroUI.Xaml.Interactions.FileSystem`), behind the feature `file-system`: the browser has no
//! local file system.

mod create_directory_action;
mod delete_directory_action;
mod delete_file_action;
mod file_system_watcher_trigger;
mod write_text_to_file_action;

pub use create_directory_action::CreateDirectoryAction;
pub use delete_directory_action::DeleteDirectoryAction;
pub use delete_file_action::DeleteFileAction;
pub use file_system_watcher_trigger::{FileSystemEventArgs, FileSystemWatcherTrigger, WatcherChangeTypes};
pub use write_text_to_file_action::WriteTextToFileAction;

/// The types of this module declared for markup (their namespace is the namespace of the module).
pub(crate) const MARKUP_TYPES: &[&ferroui_base::metadata::MarkupType] =
    &[<WatcherChangeTypes as ferroui_base::metadata::MarkupTyped>::MARKUP];
