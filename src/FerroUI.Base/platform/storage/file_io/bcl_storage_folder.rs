use super::bcl_storage_item::impl_bcl_storage_item;
use super::{BclStorageItem, FileSystemInfo};
use crate::input::LocalBoxFuture;
use crate::platform::storage::{IStorageBookmarkFolder, IStorageFile, IStorageFolder, IStorageItem};
use std::rc::Rc;

/// A folder of the local file system.
///
/// This is an implementation detail of the platform backends.
pub struct BclStorageFolder {
    base: BclStorageItem,
}

impl BclStorageFolder {
    /// Creates the storage folder of a directory of the file system.
    ///
    /// # Panics
    /// Panics when the entry is not a directory, or does not exist.
    pub fn new(directory_info: FileSystemInfo) -> Rc<Self> {
        assert!(directory_info.is_directory(), "a storage folder is created over a directory");
        Rc::new(Self { base: BclStorageItem::new(directory_info) })
    }

    /// The file-system entry of the folder.
    pub fn file_system_info(&self) -> &FileSystemInfo {
        self.base.file_system_info()
    }
}

impl_bcl_storage_item!(BclStorageFolder, as_storage_file: |_| None, as_storage_folder: |this| Some(this));

fn ready<T: 'static>(value: T) -> LocalBoxFuture<T> {
    Box::pin(std::future::ready(value))
}

fn as_file(file_system_info: Option<FileSystemInfo>) -> Option<Rc<dyn IStorageFile>> {
    BclStorageItem::wrap_file_system_info(file_system_info).and_then(|item| item.as_storage_file())
}

fn as_folder(file_system_info: Option<FileSystemInfo>) -> Option<Rc<dyn IStorageFolder>> {
    BclStorageItem::wrap_file_system_info(file_system_info).and_then(|item| item.as_storage_folder())
}

impl IStorageFolder for BclStorageFolder {
    fn get_items_async(&self) -> LocalBoxFuture<std::io::Result<Vec<Rc<dyn IStorageItem>>>> {
        ready(BclStorageItem::get_items_core(self.file_system_info().path()).map(|items| {
            items.into_iter().filter_map(|item| BclStorageItem::wrap_file_system_info(Some(item))).collect()
        }))
    }

    fn get_folder_async(&self, name: &str) -> LocalBoxFuture<Option<Rc<dyn IStorageFolder>>> {
        ready(as_folder(BclStorageItem::get_folder_core(self.file_system_info().path(), name)))
    }

    fn get_file_async(&self, name: &str) -> LocalBoxFuture<Option<Rc<dyn IStorageFile>>> {
        ready(as_file(BclStorageItem::get_file_core(self.file_system_info().path(), name)))
    }

    fn create_file_async(&self, name: &str) -> LocalBoxFuture<std::io::Result<Option<Rc<dyn IStorageFile>>>> {
        ready(BclStorageItem::create_file_core(self.file_system_info().path(), name).map(|file| as_file(Some(file))))
    }

    fn create_folder_async(&self, name: &str) -> LocalBoxFuture<std::io::Result<Option<Rc<dyn IStorageFolder>>>> {
        ready(
            BclStorageItem::create_folder_core(self.file_system_info().path(), name)
                .map(|folder| as_folder(Some(folder))),
        )
    }
}

impl IStorageBookmarkFolder for BclStorageFolder {}
