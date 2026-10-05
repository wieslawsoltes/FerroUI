use super::{IStorageFile, IStorageFolder, IStorageItem};
use crate::input::LocalBoxFuture;

/// A storage item that is backed by an entry of the local file system.
///
/// This is an implementation detail of the storage providers.
///
/// Upstream exposes the file-system info of the entry; the only thing the
/// contracts read from it is its full name.
pub trait IStorageItemWithFileSystemInfo: IStorageItem {
    /// The full path of the file-system entry (C# `FileSystemInfo.FullName`).
    fn file_system_info_full_name(&self) -> String;
}

/// A storage item that was opened from a bookmark.
///
/// This interface is not meant to be implemented by applications.
pub trait IStorageBookmarkItem: IStorageItem {
    /// Releases the bookmark the item was opened from.
    fn release_bookmark_async(&self) -> LocalBoxFuture<()>;
}

/// A file that was opened from a bookmark.
///
/// This interface is not meant to be implemented by applications.
pub trait IStorageBookmarkFile: IStorageFile + IStorageBookmarkItem {}

/// A folder that was opened from a bookmark.
///
/// This interface is not meant to be implemented by applications.
pub trait IStorageBookmarkFolder: IStorageFolder + IStorageBookmarkItem {}
