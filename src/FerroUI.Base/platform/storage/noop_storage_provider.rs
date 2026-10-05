use super::{
    FilePickerOpenOptions, FilePickerSaveOptions, FolderPickerOpenOptions, IStorageFolder, OpenFilePickerResult,
    SaveFilePickerResult,
};
use crate::input::LocalBoxFuture;
use std::rc::Rc;

/// A storage provider without pickers: what a top-level uses when its
/// platform has no storage provider. Where there is a local file system it
/// still opens bookmarks and finds files, folders and the well-known
/// folders by path.
pub struct NoopStorageProvider;

fn ready<T: 'static>(value: T) -> LocalBoxFuture<T> {
    Box::pin(std::future::ready(value))
}

impl NoopStorageProvider {
    fn open_file_picker_with_result() -> LocalBoxFuture<std::io::Result<OpenFilePickerResult>> {
        ready(Ok(OpenFilePickerResult::default()))
    }

    fn save_file_picker_with_result() -> LocalBoxFuture<std::io::Result<SaveFilePickerResult>> {
        ready(Ok(SaveFilePickerResult::default()))
    }

    fn open_folder_picker() -> LocalBoxFuture<std::io::Result<Vec<Rc<dyn IStorageFolder>>>> {
        ready(Ok(Vec::new()))
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl super::file_io::BclStorageProvider for NoopStorageProvider {
    fn can_open(&self) -> bool {
        false
    }

    fn open_file_picker_with_result_async(
        &self,
        _options: FilePickerOpenOptions,
    ) -> LocalBoxFuture<std::io::Result<OpenFilePickerResult>> {
        Self::open_file_picker_with_result()
    }

    fn can_save(&self) -> bool {
        false
    }

    fn save_file_picker_with_result_async(
        &self,
        _options: FilePickerSaveOptions,
    ) -> LocalBoxFuture<std::io::Result<SaveFilePickerResult>> {
        Self::save_file_picker_with_result()
    }

    fn can_pick_folder(&self) -> bool {
        false
    }

    fn open_folder_picker_async(
        &self,
        _options: FolderPickerOpenOptions,
    ) -> LocalBoxFuture<std::io::Result<Vec<Rc<dyn IStorageFolder>>>> {
        Self::open_folder_picker()
    }
}

/// Without a local file system (the browser) nothing is found.
#[cfg(target_arch = "wasm32")]
impl super::IStorageProvider for NoopStorageProvider {
    fn can_open(&self) -> bool {
        false
    }

    fn open_file_picker_async(
        &self,
        _options: FilePickerOpenOptions,
    ) -> LocalBoxFuture<std::io::Result<Vec<Rc<dyn super::IStorageFile>>>> {
        ready(Ok(Vec::new()))
    }

    fn open_file_picker_with_result_async(
        &self,
        _options: FilePickerOpenOptions,
    ) -> LocalBoxFuture<std::io::Result<OpenFilePickerResult>> {
        Self::open_file_picker_with_result()
    }

    fn can_save(&self) -> bool {
        false
    }

    fn save_file_picker_async(
        &self,
        _options: FilePickerSaveOptions,
    ) -> LocalBoxFuture<std::io::Result<Option<Rc<dyn super::IStorageFile>>>> {
        ready(Ok(None))
    }

    fn save_file_picker_with_result_async(
        &self,
        _options: FilePickerSaveOptions,
    ) -> LocalBoxFuture<std::io::Result<SaveFilePickerResult>> {
        Self::save_file_picker_with_result()
    }

    fn can_pick_folder(&self) -> bool {
        false
    }

    fn open_folder_picker_async(
        &self,
        _options: FolderPickerOpenOptions,
    ) -> LocalBoxFuture<std::io::Result<Vec<Rc<dyn IStorageFolder>>>> {
        Self::open_folder_picker()
    }

    fn open_file_bookmark_async(&self, _bookmark: &str) -> LocalBoxFuture<Option<Rc<dyn super::IStorageBookmarkFile>>> {
        ready(None)
    }

    fn open_folder_bookmark_async(
        &self,
        _bookmark: &str,
    ) -> LocalBoxFuture<Option<Rc<dyn super::IStorageBookmarkFolder>>> {
        ready(None)
    }

    fn try_get_file_from_path_async(
        &self,
        _file_path: &crate::utilities::Uri,
    ) -> LocalBoxFuture<Option<Rc<dyn super::IStorageFile>>> {
        ready(None)
    }

    fn try_get_folder_from_path_async(
        &self,
        _folder_path: &crate::utilities::Uri,
    ) -> LocalBoxFuture<Option<Rc<dyn IStorageFolder>>> {
        ready(None)
    }

    fn try_get_well_known_folder_async(
        &self,
        _well_known_folder: super::WellKnownFolder,
    ) -> LocalBoxFuture<Option<Rc<dyn IStorageFolder>>> {
        ready(None)
    }
}
