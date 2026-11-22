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

    /// Writes through a platform stream and waits for the provider to finish
    /// the write. The callback flushes its buffers before returning success.
    fn write_with_async(
        &self,
        write: Box<dyn FnOnce(&mut dyn Write) -> std::io::Result<()>>,
    ) -> LocalBoxFuture<std::io::Result<()>> {
        let target = self.open_write_async();
        Box::pin(async move { write(target.await?.as_mut()) })
    }
}
