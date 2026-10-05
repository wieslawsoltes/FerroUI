use super::IStorageItem;
use crate::input::LocalBoxFuture;
use std::io::{Read, Write};

/// Represents a file. Provides information about the file and its contents,
/// and ways to manipulate them.
///
/// This interface is not meant to be implemented by applications.
pub trait IStorageFile: IStorageItem {
    /// Opens a stream for read access.
    ///
    /// Fails (among other reasons) when access to the file is denied.
    fn open_read_async(&self) -> LocalBoxFuture<std::io::Result<Box<dyn Read>>>;

    /// Opens a stream for writing to the file.
    ///
    /// Fails (among other reasons) when access to the file is denied.
    fn open_write_async(&self) -> LocalBoxFuture<std::io::Result<Box<dyn Write>>>;
}
