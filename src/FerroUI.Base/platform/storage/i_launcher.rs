use super::IStorageItem;
use crate::input::LocalBoxFuture;
use crate::utilities::Uri;
use std::rc::Rc;

/// Starts the default app associated with the specified file or URI.
pub trait ILauncher {
    /// Starts the default app associated with the URI scheme name for the
    /// specified URI.
    ///
    /// Resolves to `true` if the operation was successful, `false`
    /// otherwise.
    fn launch_uri_async(&self, uri: &Uri) -> LocalBoxFuture<bool>;

    /// Starts the default app associated with the specified storage file or
    /// folder.
    ///
    /// Resolves to `true` if the operation was successful, `false`
    /// otherwise.
    fn launch_file_async(&self, storage_item: Rc<dyn IStorageItem>) -> LocalBoxFuture<bool>;
}

/// A launcher that launches nothing: what a top-level uses when its
/// platform has no launcher.
pub struct NoopLauncher;

impl ILauncher for NoopLauncher {
    fn launch_uri_async(&self, _uri: &Uri) -> LocalBoxFuture<bool> {
        Box::pin(std::future::ready(false))
    }

    fn launch_file_async(&self, _storage_item: Rc<dyn IStorageItem>) -> LocalBoxFuture<bool> {
        Box::pin(std::future::ready(false))
    }
}

/// Extensions of the launcher for entries of the local file system.
#[cfg(not(target_arch = "wasm32"))]
impl dyn ILauncher {
    /// Starts the default app associated with the specified file.
    ///
    /// Resolves to `true` if the operation was successful, `false`
    /// otherwise (also when the file does not exist).
    pub fn launch_file_info_async(&self, file_info: &std::path::Path) -> LocalBoxFuture<bool> {
        use super::file_io::{BclStorageFile, FileSystemInfo};

        let file_info = FileSystemInfo::file(file_info);
        if !file_info.exists() {
            return Box::pin(std::future::ready(false));
        }

        self.launch_file_async(BclStorageFile::new(file_info))
    }

    /// Starts the default app associated with the specified directory.
    ///
    /// Resolves to `true` if the operation was successful, `false`
    /// otherwise (also when the directory does not exist).
    pub fn launch_directory_info_async(&self, directory_info: &std::path::Path) -> LocalBoxFuture<bool> {
        use super::file_io::{BclStorageFolder, FileSystemInfo};

        let directory_info = FileSystemInfo::directory(directory_info);
        if !directory_info.exists() {
            return Box::pin(std::future::ready(false));
        }

        self.launch_file_async(BclStorageFolder::new(directory_info))
    }
}
