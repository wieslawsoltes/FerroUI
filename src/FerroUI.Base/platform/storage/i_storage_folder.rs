use super::{IStorageFile, IStorageItem};
use crate::input::LocalBoxFuture;
use std::rc::Rc;

/// Manipulates folders and their contents, and provides information about
/// them.
///
/// This interface is not meant to be implemented by applications.
pub trait IStorageFolder: IStorageItem {
    /// Gets the files and subfolders in the current folder.
    ///
    /// When this method completes successfully, it returns a list of the
    /// files and folders in the current folder. Each item in the list is
    /// represented by an [`IStorageItem`] implementation object.
    fn get_items_async(&self) -> LocalBoxFuture<std::io::Result<Vec<Rc<dyn IStorageItem>>>>;

    /// Gets the folder with the specified name from the current folder.
    ///
    /// Resolves to the folder if it exists, `None` otherwise.
    fn get_folder_async(&self, name: &str) -> LocalBoxFuture<Option<Rc<dyn IStorageFolder>>>;

    /// Gets the file with the specified name from the current folder.
    ///
    /// Resolves to the file if it exists, `None` otherwise.
    fn get_file_async(&self, name: &str) -> LocalBoxFuture<Option<Rc<dyn IStorageFile>>>;

    /// Creates a file with the specified name as a child of the current
    /// storage folder.
    fn create_file_async(&self, name: &str) -> LocalBoxFuture<std::io::Result<Option<Rc<dyn IStorageFile>>>>;

    /// Creates a folder with the specified name as a child of the current
    /// storage folder.
    fn create_folder_async(&self, name: &str) -> LocalBoxFuture<std::io::Result<Option<Rc<dyn IStorageFolder>>>>;
}
