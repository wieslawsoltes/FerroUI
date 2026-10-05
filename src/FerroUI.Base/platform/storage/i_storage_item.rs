use super::{IStorageFile, IStorageFolder, IStorageItemWithFileSystemInfo, StorageItemProperties};
use crate::input::LocalBoxFuture;
use crate::reactive::IDisposable;
use crate::utilities::Uri;
use std::rc::Rc;

/// Manipulates storage items (files and folders) and their contents, and
/// provides information about them.
///
/// This interface inherits [`IDisposable`]. It is recommended to dispose an
/// item when it is not used anymore.
///
/// This interface is not meant to be implemented by applications.
pub trait IStorageItem: IDisposable {
    /// The name of the item including the file name extension if there is
    /// one.
    fn name(&self) -> String;

    /// The file-system path of the item.
    ///
    /// Android backend might return a file path with "content:" scheme.
    /// Browser and iOS backends might return a relative URI.
    fn path(&self) -> Uri;

    /// Gets the basic properties of the current item.
    fn get_basic_properties_async(&self) -> LocalBoxFuture<StorageItemProperties>;

    /// Returns `true` if it is possible to bookmark this file or folder for
    /// future access.
    fn can_bookmark(&self) -> bool;

    /// Saves a bookmark for this file or folder.
    ///
    /// Resolves to an identifier of the bookmark, or `None` if a bookmark
    /// cannot be created.
    fn save_bookmark_async(&self) -> LocalBoxFuture<Option<String>>;

    /// Gets the parent folder of the current storage item.
    fn get_parent_async(&self) -> LocalBoxFuture<Option<Rc<dyn IStorageFolder>>>;

    /// Deletes the current storage item and its contents.
    fn delete_async(&self) -> LocalBoxFuture<std::io::Result<()>>;

    /// Moves the current storage item and its contents to an
    /// [`IStorageFolder`].
    ///
    /// Resolves to the storage item at its new location, if the platform
    /// can provide it.
    fn move_async(
        &self,
        destination: Rc<dyn IStorageFolder>,
    ) -> LocalBoxFuture<std::io::Result<Option<Rc<dyn IStorageItem>>>>;

    /// The item as a file, if it is one (C# `item as IStorageFile`).
    fn as_storage_file(self: Rc<Self>) -> Option<Rc<dyn IStorageFile>> {
        None
    }

    /// The item as a folder, if it is one (C# `item as IStorageFolder`).
    fn as_storage_folder(self: Rc<Self>) -> Option<Rc<dyn IStorageFolder>> {
        None
    }

    /// The item as one that is backed by an entry of the local file
    /// system, if it is one.
    fn as_storage_item_with_file_system_info(&self) -> Option<&dyn IStorageItemWithFileSystemInfo> {
        None
    }
}
