use super::{
    FilePickerOpenOptions, FilePickerSaveOptions, FolderPickerOpenOptions, IStorageBookmarkFile,
    IStorageBookmarkFolder, IStorageFile, IStorageFolder, OpenFilePickerResult, SaveFilePickerResult,
    WellKnownFolder,
};
use crate::input::LocalBoxFuture;
use crate::utilities::Uri;
use std::rc::Rc;

/// Opens file and folder pickers and gives access to storage items.
///
/// The picker operations fail with an I/O error when no picker can be
/// shown.
///
/// This interface is not meant to be implemented by applications.
pub trait IStorageProvider {
    /// Returns `true` if it is possible to open a file picker on the
    /// current platform.
    fn can_open(&self) -> bool;

    /// Opens a file picker dialog.
    ///
    /// Resolves to the list of files, empty if the user canceled the
    /// dialog.
    fn open_file_picker_async(
        &self,
        options: FilePickerOpenOptions,
    ) -> LocalBoxFuture<std::io::Result<Vec<Rc<dyn IStorageFile>>>>;

    /// Opens a file picker dialog and returns the selected files along with
    /// additional information.
    ///
    /// This method allows retrieving additional details about the picked
    /// files, such as the selected file type.
    fn open_file_picker_with_result_async(
        &self,
        options: FilePickerOpenOptions,
    ) -> LocalBoxFuture<std::io::Result<OpenFilePickerResult>>;

    /// Returns `true` if it is possible to open a save file picker on the
    /// current platform.
    fn can_save(&self) -> bool;

    /// Opens a save file picker dialog.
    ///
    /// Resolves to the saved [`IStorageFile`], or `None` if the user
    /// canceled the dialog.
    fn save_file_picker_async(
        &self,
        options: FilePickerSaveOptions,
    ) -> LocalBoxFuture<std::io::Result<Option<Rc<dyn IStorageFile>>>>;

    /// Opens a save file picker dialog and returns the selected file along
    /// with additional information.
    ///
    /// This method allows retrieving additional details about the saved
    /// file, such as the selected file type.
    fn save_file_picker_with_result_async(
        &self,
        options: FilePickerSaveOptions,
    ) -> LocalBoxFuture<std::io::Result<SaveFilePickerResult>>;

    /// Returns `true` if it is possible to open a folder picker on the
    /// current platform.
    fn can_pick_folder(&self) -> bool;

    /// Opens a folder picker dialog.
    ///
    /// Resolves to the list of folders, empty if the user canceled the
    /// dialog.
    fn open_folder_picker_async(
        &self,
        options: FolderPickerOpenOptions,
    ) -> LocalBoxFuture<std::io::Result<Vec<Rc<dyn IStorageFolder>>>>;

    /// Opens an [`IStorageBookmarkFile`] from the bookmark ID.
    ///
    /// Resolves to the bookmarked file, or `None` if the OS denied the
    /// request or the bookmark is invalid.
    fn open_file_bookmark_async(&self, bookmark: &str) -> LocalBoxFuture<Option<Rc<dyn IStorageBookmarkFile>>>;

    /// Opens an [`IStorageBookmarkFolder`] from the bookmark ID.
    ///
    /// Resolves to the bookmarked folder, or `None` if the OS denied the
    /// request or the bookmark is invalid.
    fn open_folder_bookmark_async(&self, bookmark: &str) -> LocalBoxFuture<Option<Rc<dyn IStorageBookmarkFolder>>>;

    /// Attempts to read a file from the file-system by its path.
    ///
    /// `file_path` is the path of the item to retrieve in URI format.
    ///
    /// The URI must be an absolute path. It also might ask the user for a
    /// permission, and resolve to `None` if it was denied.
    fn try_get_file_from_path_async(&self, file_path: &Uri) -> LocalBoxFuture<Option<Rc<dyn IStorageFile>>>;

    /// Attempts to read a folder from the file-system by its path.
    ///
    /// `folder_path` is the path of the folder to retrieve in URI format.
    ///
    /// The URI must be an absolute path. It also might ask the user for a
    /// permission, and resolve to `None` if it was denied.
    fn try_get_folder_from_path_async(&self, folder_path: &Uri) -> LocalBoxFuture<Option<Rc<dyn IStorageFolder>>>;

    /// Attempts to read a folder from the file-system by its well-known
    /// folder identifier.
    fn try_get_well_known_folder_async(
        &self,
        well_known_folder: WellKnownFolder,
    ) -> LocalBoxFuture<Option<Rc<dyn IStorageFolder>>>;

    /// Whether the provider is the file-system backed one
    /// (C# `provider is BclStorageProvider`).
    ///
    /// Answered with `true` by the implementations of
    /// [`BclStorageProvider`](super::file_io::BclStorageProvider).
    fn is_file_system_backed(&self) -> bool {
        false
    }
}
