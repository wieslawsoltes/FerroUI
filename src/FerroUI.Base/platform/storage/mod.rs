//! Storage contracts: file and folder pickers, storage items, bookmarks and
//! the launcher.
//!
//! The implementations backed by the local file system are in [`file_io`];
//! they are not available in the browser.

pub mod file_io;

mod fallback_storage_provider;
mod file_picker_file_type;
mod file_picker_file_types;
mod file_picker_open_options;
mod file_picker_save_options;
mod folder_picker_open_options;
mod i_launcher;
mod i_storage_bookmark_item;
mod i_storage_file;
mod i_storage_folder;
mod i_storage_item;
mod i_storage_provider;
mod noop_storage_provider;
mod open_file_picker_result;
mod picker_options;
mod save_file_picker_result;
mod storage_item_properties;
mod storage_provider_extensions;
mod well_known_folder;

pub use fallback_storage_provider::{FallbackStorageProvider, StorageProviderFactory};
pub use file_picker_file_type::FilePickerFileType;
pub use file_picker_file_types::FilePickerFileTypes;
pub use file_picker_open_options::FilePickerOpenOptions;
pub use file_picker_save_options::FilePickerSaveOptions;
pub use folder_picker_open_options::FolderPickerOpenOptions;
pub use i_launcher::{ILauncher, NoopLauncher};
pub use i_storage_bookmark_item::{
    IStorageBookmarkFile, IStorageBookmarkFolder, IStorageBookmarkItem, IStorageItemWithFileSystemInfo,
};
pub use i_storage_file::IStorageFile;
pub use i_storage_folder::IStorageFolder;
pub use i_storage_item::IStorageItem;
pub use i_storage_provider::IStorageProvider;
pub use noop_storage_provider::NoopStorageProvider;
pub use open_file_picker_result::OpenFilePickerResult;
pub use picker_options::PickerOptions;
pub use save_file_picker_result::SaveFilePickerResult;
pub use storage_item_properties::StorageItemProperties;
pub use well_known_folder::WellKnownFolder;

#[cfg(test)]
mod storage_tests;
