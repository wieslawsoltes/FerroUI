//! Port of `Diagnostics/Screenshots/FilePickerHandler.cs`.

use super::BaseRenderToStreamHandler;
use crate::diagnostics::views::MainWindow;
use ferroui_base::input::LocalBoxFuture;
use ferroui_base::platform::storage::{
    FilePickerFileTypes, FilePickerSaveOptions, IStorageFolder, IStorageProvider, WellKnownFolder,
};
use ferroui_base::{Ref, Visual};
use ferroui_controls::{Application, Control, TopLevel};
use std::io::Write;
use std::rc::Rc;

/// Show a FileSavePicker to select where save screenshot
pub struct FilePickerHandler {
    title: String,
    screenshot_root: Option<String>,
}

impl PartialEq for FilePickerHandler {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl Default for FilePickerHandler {
    fn default() -> Self {
        Self::with_title(None, None)
    }
}

impl FilePickerHandler {
    /// Instance FilePickerHandler
    pub fn new() -> Self {
        Self::with_title(None, None)
    }

    /// Instance FilePickerHandler with specificated parameter
    ///
    /// `title` is the title of the save file picker; `screenshot_root` the folder it starts in.
    pub fn with_title(title: Option<String>, screenshot_root: Option<String>) -> Self {
        Self { title: title.unwrap_or_else(|| "Save Screenshot to ...".to_string()), screenshot_root }
    }

    fn get_top_level(control: &Ref<Control>) -> Option<Ref<TopLevel>> {
        // If possible, use devtools main window.
        let dev_tools_top_level: Option<Ref<TopLevel>> = Application::current()
            .and_then(|application| application.application_lifetime())
            .and_then(|lifetime| {
                lifetime.as_classic_desktop_style_application_lifetime().and_then(|desktop| {
                    desktop.windows().into_iter().find(|window| window.is::<MainWindow>()).map(|window| window.upcast())
                })
            });

        let visual: &Visual = control;
        dev_tools_top_level.or_else(|| TopLevel::get_top_level(Some(visual)))
    }
}

impl BaseRenderToStreamHandler for FilePickerHandler {
    fn get_stream(&self, control: &Ref<Control>) -> LocalBoxFuture<std::io::Result<Option<Box<dyn Write>>>> {
        let Some(top_level) = Self::get_top_level(control) else {
            return Box::pin(std::future::ready(Err::<Option<Box<dyn Write>>, _>(std::io::Error::other(
                "No TopLevel is available.",
            ))));
        };
        let storage_provider: Rc<dyn IStorageProvider> = top_level.storage_provider();
        let title = self.title.clone();
        let screenshot_root = self.screenshot_root.clone();

        Box::pin(async move {
            let mut default_folder: Option<Rc<dyn IStorageFolder>> = None;
            if let Some(screenshot_root) = &screenshot_root {
                default_folder = storage_provider.try_get_folder_from_path_str_async(screenshot_root).await;
            }
            if default_folder.is_none() {
                default_folder = storage_provider.try_get_well_known_folder_async(WellKnownFolder::Pictures).await;
            }

            let mut options = FilePickerSaveOptions::new();
            options.set_suggested_start_location(default_folder);
            options.set_title(Some(title));
            options.set_file_type_choices(Some(vec![FilePickerFileTypes::image_png()]));
            options.set_default_extension(Some(".png".to_string()));

            let result = storage_provider.save_file_picker_async(options).await?;
            let Some(result) = result else {
                return Ok(None);
            };

            Ok::<Option<Box<dyn Write>>, std::io::Error>(Some(result.open_write_async().await?))
        })
    }
}
