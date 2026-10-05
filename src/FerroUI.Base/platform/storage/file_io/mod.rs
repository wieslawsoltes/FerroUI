//! Helpers shared by the storage providers of the platform backends, and
//! the storage backed by the local file system (not available in the
//! browser).

pub mod path;
mod storage_bookmark_helper;
mod storage_provider_helpers;

pub use storage_bookmark_helper::{DecodeResult, StorageBookmarkHelper};
pub use storage_provider_helpers::StorageProviderHelpers;

#[cfg(not(target_arch = "wasm32"))]
mod bcl_launcher;
#[cfg(not(target_arch = "wasm32"))]
mod bcl_storage_file;
#[cfg(not(target_arch = "wasm32"))]
mod bcl_storage_folder;
#[cfg(not(target_arch = "wasm32"))]
mod bcl_storage_item;
#[cfg(not(target_arch = "wasm32"))]
mod bcl_storage_provider;
#[cfg(not(target_arch = "wasm32"))]
mod security_scoped_stream;

#[cfg(not(target_arch = "wasm32"))]
pub use bcl_launcher::BclLauncher;
#[cfg(not(target_arch = "wasm32"))]
pub use bcl_storage_file::BclStorageFile;
#[cfg(not(target_arch = "wasm32"))]
pub use bcl_storage_folder::BclStorageFolder;
#[cfg(not(target_arch = "wasm32"))]
pub use bcl_storage_item::{BclStorageItem, FileSystemInfo};
#[cfg(not(target_arch = "wasm32"))]
pub use bcl_storage_provider::{
    get_downloads_well_known_folder, try_get_well_known_folder_core, BclStorageItemHandle, BclStorageProvider,
};
#[cfg(not(target_arch = "wasm32"))]
pub use security_scoped_stream::SecurityScopedStream;

#[cfg(all(test, not(target_arch = "wasm32")))]
mod bcl_storage_tests;

#[cfg(test)]
mod storage_provider_helper_tests;
