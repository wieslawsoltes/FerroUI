use super::bcl_storage_item::impl_bcl_storage_item;
use super::{BclStorageItem, FileSystemInfo};
use crate::input::LocalBoxFuture;
use crate::platform::storage::{IStorageBookmarkFile, IStorageFile};
use std::io::{Read, Write};
use std::rc::Rc;

/// A file of the local file system.
///
/// This is an implementation detail of the platform backends.
pub struct BclStorageFile {
    base: BclStorageItem,
}

impl BclStorageFile {
    /// Creates the storage file of a file of the file system.
    ///
    /// # Panics
    /// Panics when the entry is not a file.
    pub fn new(file_info: FileSystemInfo) -> Rc<Self> {
        assert!(!file_info.is_directory(), "a storage file is created over a file");
        Rc::new(Self { base: BclStorageItem::new(file_info) })
    }

    /// The file-system entry of the file.
    pub fn file_system_info(&self) -> &FileSystemInfo {
        self.base.file_system_info()
    }
}

impl_bcl_storage_item!(BclStorageFile, as_storage_file: |this| Some(this), as_storage_folder: |_| None);

impl IStorageFile for BclStorageFile {
    fn open_read_async(&self) -> LocalBoxFuture<std::io::Result<Box<dyn Read>>> {
        let stream = BclStorageItem::open_read_core(self.file_system_info().path());
        Box::pin(std::future::ready(stream.map(|stream| Box::new(stream) as Box<dyn Read>)))
    }

    fn open_write_async(&self) -> LocalBoxFuture<std::io::Result<Box<dyn Write>>> {
        let stream = BclStorageItem::open_write_core(self.file_system_info().path());
        Box::pin(std::future::ready(stream.map(|stream| Box::new(stream) as Box<dyn Write>)))
    }
}

impl IStorageBookmarkFile for BclStorageFile {}
