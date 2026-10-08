use crate::storage_provider_api::StorageProviderApi;
use crate::top_level_impl::TopLevelImpl;
use ferroui_base::input::LocalBoxFuture;
use ferroui_base::platform::storage::file_io::{try_get_well_known_folder_core, BclStorageFolder};
use ferroui_base::platform::storage::{
    FilePickerOpenOptions, FilePickerSaveOptions, FolderPickerOpenOptions, IStorageBookmarkFile,
    IStorageBookmarkFolder, IStorageFile, IStorageFolder, IStorageProvider, OpenFilePickerResult,
    SaveFilePickerResult, WellKnownFolder,
};
use ferroui_base::utilities::Uri;
use std::rc::Rc;

/// The storage provider of a top-level of this backend: the pickers are the
/// panels of AppKit, shown as sheets of the top-level.
pub(crate) struct StorageProviderImpl {
    top_level: Rc<TopLevelImpl>,
    native: Rc<StorageProviderApi>,
}

impl StorageProviderImpl {
    pub(crate) fn new(top_level: Rc<TopLevelImpl>, native: Rc<StorageProviderApi>) -> Self {
        Self { top_level, native }
    }
}

impl IStorageProvider for StorageProviderImpl {
    fn can_open(&self) -> bool {
        true
    }

    fn can_save(&self) -> bool {
        true
    }

    fn can_pick_folder(&self) -> bool {
        true
    }

    fn open_file_picker_async(
        &self,
        options: FilePickerOpenOptions,
    ) -> LocalBoxFuture<std::io::Result<Vec<Rc<dyn IStorageFile>>>> {
        let result = self.open_file_picker_with_result_async(options);
        Box::pin(async move { Ok(result.await?.files) })
    }

    fn open_file_picker_with_result_async(
        &self,
        options: FilePickerOpenOptions,
    ) -> LocalBoxFuture<std::io::Result<OpenFilePickerResult>> {
        let dialog = self.native.open_file_dialog(Some(&self.top_level), options);
        Box::pin(async move {
            let (files, selected_type) = dialog.await;
            Ok(OpenFilePickerResult { files, selected_file_type: selected_type })
        })
    }

    fn save_file_picker_async(
        &self,
        options: FilePickerSaveOptions,
    ) -> LocalBoxFuture<std::io::Result<Option<Rc<dyn IStorageFile>>>> {
        let result = self.save_file_picker_with_result_async(options);
        Box::pin(async move { Ok(result.await?.file) })
    }

    fn save_file_picker_with_result_async(
        &self,
        options: FilePickerSaveOptions,
    ) -> LocalBoxFuture<std::io::Result<SaveFilePickerResult>> {
        let dialog = self.native.save_file_dialog(Some(&self.top_level), options);
        Box::pin(async move {
            let (file, selected_type) = dialog.await;
            Ok(SaveFilePickerResult { file, selected_file_type: selected_type })
        })
    }

    fn open_folder_picker_async(
        &self,
        options: FolderPickerOpenOptions,
    ) -> LocalBoxFuture<std::io::Result<Vec<Rc<dyn IStorageFolder>>>> {
        let dialog = self.native.select_folder_dialog(Some(&self.top_level), options);
        Box::pin(async move { Ok(dialog.await) })
    }

    fn open_file_bookmark_async(&self, bookmark: &str) -> LocalBoxFuture<Option<Rc<dyn IStorageBookmarkFile>>> {
        let uri = self.native.read_bookmark(bookmark, false);
        Box::pin(std::future::ready(self.native.try_get_storage_item(uri.as_ref(), false).and_then(|item| item.into_bookmark_file())))
    }

    fn open_folder_bookmark_async(&self, bookmark: &str) -> LocalBoxFuture<Option<Rc<dyn IStorageBookmarkFolder>>> {
        let uri = self.native.read_bookmark(bookmark, true);
        Box::pin(std::future::ready(self.native.try_get_storage_item(uri.as_ref(), false).and_then(|item| item.into_bookmark_folder())))
    }

    fn try_get_file_from_path_async(&self, file_path: &Uri) -> LocalBoxFuture<Option<Rc<dyn IStorageFile>>> {
        Box::pin(std::future::ready(self.native.try_get_storage_item(Some(file_path), false).and_then(|item| item.into_file())))
    }

    fn try_get_folder_from_path_async(&self, folder_path: &Uri) -> LocalBoxFuture<Option<Rc<dyn IStorageFolder>>> {
        Box::pin(std::future::ready(self.native.try_get_storage_item(Some(folder_path), false).and_then(|item| item.into_folder())))
    }

    fn try_get_well_known_folder_async(
        &self,
        well_known_folder: WellKnownFolder,
    ) -> LocalBoxFuture<Option<Rc<dyn IStorageFolder>>> {
        if let Some(directory_info) = try_get_well_known_folder_core(well_known_folder) {
            let folder: Rc<dyn IStorageFolder> = BclStorageFolder::new(directory_info);
            return Box::pin(std::future::ready(Some(folder)));
        }

        Box::pin(std::future::ready(None))
    }
}
