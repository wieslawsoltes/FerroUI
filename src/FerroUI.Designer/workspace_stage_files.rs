//! Stage collections use the host's native storage picker. A manifest contains
//! preview configurations and source references, never serialized control graphs.
use super::*;
use crate::PreviewStageManifest;
use ferroui_base::platform::storage::{
    FilePickerFileType, FilePickerOpenOptions, FilePickerSaveOptions,
};
use ferroui_controls::TopLevel;
use std::io::{Read, Write};

impl DesignerWorkspace {
    pub(super) fn request_save_preview_stage_image(self: &Rc<Self>, id: crate::PreviewStageId) {
        if self.file_busy.replace(true) {
            return;
        }
        let this = self.clone();
        Dispatcher::ui_thread()
            .to_task_scheduler()
            .spawn_local(async move {
                let result = this
                    .save_preview_stage_image_file(id, crate::PreviewStageExportOptions::default())
                    .await
                    .map(|_| ());
                this.file_busy.set(false);
                this.report(result);
            });
    }

    /// Freeze the native PNG before opening the save picker. A later source edit or
    /// removal cannot substitute a newer frame for the image the user chose to save.
    pub async fn save_preview_stage_image_file(
        self: &Rc<Self>,
        id: crate::PreviewStageId,
        options: crate::PreviewStageExportOptions,
    ) -> Result<bool, String> {
        let image = self.preview_stage_image(id, options)?;
        let top = TopLevel::get_top_level(Some(&self.root))
            .ok_or("Attach the designer to a window to save a preview image")?;
        let provider = top.storage_provider();
        if !provider.can_save() {
            return Err(
                "This host has no save picker; use preview_stage_image() to capture PNG bytes"
                    .into(),
            );
        }
        let mut options = FilePickerSaveOptions::new();
        options.set_title(Some("Save preview image".into()));
        options.set_default_extension(Some("png".into()));
        options.set_suggested_file_name(Some(format!("Preview-{id}.png")));
        options.set_file_type_choices(Some(vec![
            FilePickerFileType::new(Some("PNG image")).with_patterns(&["*.png"])
        ]));
        options.set_show_overwrite_prompt(Some(true));
        let Some(file) = provider
            .save_file_picker_async(options)
            .await
            .map_err(|error| error.to_string())?
        else {
            return Ok(false);
        };
        #[cfg(not(target_arch = "wasm32"))]
        let local_path = {
            let item: Rc<dyn ferroui_base::platform::storage::IStorageItem> = file.clone();
            item.try_get_local_path()
        };
        #[cfg(target_arch = "wasm32")]
        let local_path: Option<String> = None;
        if let Some(path) = local_path {
            super::files::write_local_bytes(std::path::Path::new(&path), None, &image.png)
                .map_err(|error| error.to_string())?;
            return Ok(true);
        }
        let mut output = file
            .open_write_async()
            .await
            .map_err(|error| error.to_string())?;
        output
            .write_all(&image.png)
            .and_then(|_| output.flush())
            .map_err(|error| error.to_string())?;
        Ok(true)
    }

    pub(super) fn request_save_preview_stages(self: &Rc<Self>) {
        if self.file_busy.replace(true) {
            return;
        }
        let this = self.clone();
        Dispatcher::ui_thread()
            .to_task_scheduler()
            .spawn_local(async move {
                let result = this.save_preview_stages_file().await.map(|_| ());
                this.file_busy.set(false);
                this.report(result);
            });
    }

    pub(super) fn request_load_preview_stages(self: &Rc<Self>) {
        if self.file_busy.replace(true) {
            return;
        }
        let this = self.clone();
        Dispatcher::ui_thread()
            .to_task_scheduler()
            .spawn_local(async move {
                let result = this.load_preview_stages_file().await.map(|_| ());
                this.file_busy.set(false);
                this.report(result);
            });
    }

    /// Export the collection captured when the picker opens. Newer edits remain
    /// local and source-file dirty markers are unaffected by this independent save.
    pub async fn save_preview_stages_file(self: &Rc<Self>) -> Result<bool, String> {
        let manifest = self.preview_stage_manifest().to_json()?;
        let top = TopLevel::get_top_level(Some(&self.root))
            .ok_or("Attach the designer to a window to save preview stages")?;
        let provider = top.storage_provider();
        if !provider.can_save() {
            return Err(
                "This host has no save picker; use preview_stage_manifest().to_json()".into(),
            );
        }
        let mut options = FilePickerSaveOptions::new();
        options.set_title(Some("Save preview stages".into()));
        options.set_default_extension(Some("json".into()));
        options.set_suggested_file_name(Some("Workspace.ferroui-previews.json".into()));
        options.set_file_type_choices(Some(vec![stage_file_type()]));
        options.set_show_overwrite_prompt(Some(true));
        let Some(file) = provider
            .save_file_picker_async(options)
            .await
            .map_err(|error| error.to_string())?
        else {
            return Ok(false);
        };
        #[cfg(not(target_arch = "wasm32"))]
        let local_path = {
            let item: Rc<dyn ferroui_base::platform::storage::IStorageItem> = file.clone();
            item.try_get_local_path()
        };
        #[cfg(target_arch = "wasm32")]
        let local_path: Option<String> = None;
        if let Some(path) = local_path {
            super::files::write_local(std::path::Path::new(&path), None, &manifest)
                .map_err(|error| error.to_string())?;
        } else {
            let mut output = file
                .open_write_async()
                .await
                .map_err(|error| error.to_string())?;
            output
                .write_all(manifest.as_bytes())
                .and_then(|_| output.flush())
                .map_err(|error| error.to_string())?;
        }
        Ok(true)
    }

    /// Restore validated local presentation state. Source URIs are references to
    /// already open documents; this operation never reads arbitrary referenced files.
    pub async fn load_preview_stages_file(self: &Rc<Self>) -> Result<bool, String> {
        let model = self.preview_stage_model();
        let top = TopLevel::get_top_level(Some(&self.root))
            .ok_or("Attach the designer to a window to load preview stages")?;
        let provider = top.storage_provider();
        if !provider.can_open() {
            return Err(
                "This host has no open picker; use restore_preview_stage_manifest()".into(),
            );
        }
        let mut options = FilePickerOpenOptions::new();
        options.set_title(Some("Load preview stages".into()));
        options.set_allow_multiple(false);
        options.set_file_type_filter(Some(vec![stage_file_type()]));
        let files = provider
            .open_file_picker_async(options)
            .await
            .map_err(|error| error.to_string())?;
        let Some(file) = files.into_iter().next() else {
            return Ok(false);
        };
        let stream = file
            .open_read_async()
            .await
            .map_err(|error| error.to_string())?;
        let mut source = String::new();
        stream
            .take(1_048_577)
            .read_to_string(&mut source)
            .map_err(|error| error.to_string())?;
        let manifest = PreviewStageManifest::from_json(&source)?;
        if model != self.preview_stage_model() {
            return Err("The documents or preview stages changed while the file picker was open; load the collection again".into());
        }
        let has_stages = !manifest.stages.is_empty();
        self.restore_preview_stage_manifest(manifest)?;
        if has_stages {
            self.set_preview_stages_visible(true)?;
        }
        Ok(true)
    }
}

fn stage_file_type() -> Rc<FilePickerFileType> {
    FilePickerFileType::new(Some("FerroUI preview stages"))
        .with_patterns(&["*.ferroui-previews.json", "*.json"])
}

#[cfg(test)]
#[path = "workspace_stage_files_tests.rs"]
mod tests;
