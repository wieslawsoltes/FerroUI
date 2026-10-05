use super::file_io::StorageProviderHelpers;
use super::{IStorageFile, IStorageFolder, IStorageItem, IStorageProvider};
use crate::input::LocalBoxFuture;
use std::rc::Rc;

/// Group of public extensions for the [`IStorageProvider`] contract.
impl dyn IStorageProvider {
    /// Attempts to read a file from the file-system by its path.
    ///
    /// `file_path` is the path of the item to retrieve in URI format.
    ///
    /// Resolves to the file, or `None` if it does not exist.
    pub fn try_get_file_from_path_str_async(&self, file_path: &str) -> LocalBoxFuture<Option<Rc<dyn IStorageFile>>> {
        // We can avoid double escaping of the path by checking for the
        // file-system backed provider.
        #[cfg(not(target_arch = "wasm32"))]
        if self.is_file_system_backed() {
            use super::file_io::BclStorageItemHandle;

            let file = match StorageProviderHelpers::try_create_bcl_storage_item(Some(file_path)) {
                Some(BclStorageItemHandle::File(file)) => Some(file as Rc<dyn IStorageFile>),
                _ => None,
            };
            return Box::pin(std::future::ready(file));
        }

        if let Some(uri) = StorageProviderHelpers::try_get_uri_from_file_path(file_path, false) {
            return self.try_get_file_from_path_async(&uri);
        }

        Box::pin(std::future::ready(None))
    }

    /// Attempts to read a folder from the file-system by its path.
    ///
    /// `folder_path` is the path of the folder to retrieve.
    ///
    /// Resolves to the folder, or `None` if it does not exist.
    pub fn try_get_folder_from_path_str_async(
        &self,
        folder_path: &str,
    ) -> LocalBoxFuture<Option<Rc<dyn IStorageFolder>>> {
        // We can avoid double escaping of the path by checking for the
        // file-system backed provider.
        #[cfg(not(target_arch = "wasm32"))]
        if self.is_file_system_backed() {
            use super::file_io::BclStorageItemHandle;

            let folder = match StorageProviderHelpers::try_create_bcl_storage_item(Some(folder_path)) {
                Some(BclStorageItemHandle::Folder(folder)) => Some(folder as Rc<dyn IStorageFolder>),
                _ => None,
            };
            return Box::pin(std::future::ready(folder));
        }

        if let Some(uri) = StorageProviderHelpers::try_get_uri_from_file_path(folder_path, true) {
            return self.try_get_folder_from_path_async(&uri);
        }

        Box::pin(std::future::ready(None))
    }
}

/// Group of public extensions for the [`IStorageItem`] contract.
impl dyn IStorageItem + '_ {
    /// Gets the local operating system path for the specified storage item.
    ///
    /// Returns the local path, or `None` if the storage item is not a local
    /// path.
    ///
    /// The storage item can be a file or a folder (which convert to this
    /// contract).
    pub fn try_get_local_path(&self) -> Option<String> {
        // We can avoid double escaping of the path by checking for a
        // file-system backed item.
        // Ideally, the local path of `path()` should also work, as that's
        // the only way available to the users.
        if let Some(storage_item) = self.as_storage_item_with_file_system_info() {
            return Some(storage_item.file_system_info_full_name());
        }

        StorageProviderHelpers::try_get_path_from_file_uri(Some(&self.path()))
    }
}
