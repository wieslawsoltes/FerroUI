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

    /// The write of a provider that commits a write itself (the browser's file system): it writes
    /// through its own stream and returns the future of the commit. Every other provider hands
    /// the callback back, and [`write_with_async`](dyn IStorageFile::write_with_async) writes
    /// through [`open_write_async`](Self::open_write_async). Kept out of the vtable's default
    /// body so that an application that never writes this way does not link the write.
    fn write_committed_async(
        &self,
        write: StorageWriteCallback,
    ) -> Result<LocalBoxFuture<std::io::Result<()>>, StorageWriteCallback> {
        Err(write)
    }
}

/// The write a caller hands to [`write_with_async`](dyn IStorageFile::write_with_async).
pub type StorageWriteCallback = Box<dyn FnOnce(&mut dyn Write) -> std::io::Result<()>>;

impl dyn IStorageFile + '_ {
    /// Writes through a platform stream and waits for the provider to finish
    /// the write. The callback flushes its buffers before returning success.
    pub fn write_with_async(&self, write: StorageWriteCallback) -> LocalBoxFuture<std::io::Result<()>> {
        match self.write_committed_async(write) {
            Ok(commit) => commit,
            Err(write) => {
                let target = self.open_write_async();
                Box::pin(async move { write(target.await?.as_mut()) })
            }
        }
    }
}
