//! File commands use the host's existing native storage provider. No file-system or platform
//! service is created by the embeddable designer.
use super::*;
use ferroui_base::platform::storage::{
    FilePickerFileType, FilePickerOpenOptions, FilePickerSaveOptions, IStorageFile,
};
use ferroui_base::utilities::Uri;
use ferroui_controls::{Border, ThemeVariantScope, TopLevel};
use std::io::{Read, Write};

pub(super) struct PendingDocument {
    source: String,
    uri: String,
    file: Rc<dyn IStorageFile>,
}

impl DesignerWorkspace {
    pub fn is_dirty(&self) -> bool {
        self.session.borrow().document().source() != self.saved_source.borrow().as_str()
    }

    /// Open source at its URI, retaining the previous tab and its history. A newly opened
    /// document starts its own history and is detached from the previous collaboration room.
    pub fn open_document(self: &Rc<Self>, source: String, uri: String) -> Result<(), String> {
        let uri = super::project::document_identity(&uri)?;
        self.capture_tab();
        self.pending_source.set(false);
        self.disconnect_sharing();
        self.session
            .borrow_mut()
            .open_document(source.clone(), uri)?;
        self.reset_canvas_guides();
        *self.file.borrow_mut() = None;
        *self.saved_source.borrow_mut() = source;
        self.runtime_selection.set(None);
        self.surface.set_interactive(false);
        self.refresh(true);
        self.refresh_file_title();
        Ok(())
    }
    /// Return true when a close request was deferred for a native save/discard/cancel prompt.
    /// The host supplies a callback that performs the already-approved close without prompting again.
    pub fn request_close(self: &Rc<Self>, close: impl Fn() + 'static) -> bool {
        if !self.is_dirty() {
            if let Some(uri) = self.inactive_dirty_document() {
                if let Err(error) = self.activate_document(&uri) {
                    self.report(Err(error));
                    return true;
                }
            } else {
                return false;
            }
        }
        *self.pending_close.borrow_mut() = Some(Rc::new(close));
        self.pending_open.borrow_mut().take();
        self.root
            .get_control::<TextBlock>("FilePromptTitle")
            .set_text(Some("Save before closing?"));
        self.root
            .get_control::<TextBlock>("FilePromptText")
            .set_text(Some("The current document has unsaved changes."));
        self.root
            .get_control::<Button>("SaveAndOpen")
            .set_content(Some(Rc::new("Save and close".to_owned())));
        self.root
            .get_control::<Button>("DiscardAndOpen")
            .set_content(Some(Rc::new("Discard and close".to_owned())));
        self.root
            .get_control::<Border>("FilePrompt")
            .set_is_visible(true);
        true
    }
    pub fn set_preview_dark(&self, dark: bool) {
        // The legacy toggle and the staged environment editor share one state model.
        let value = crate::PreviewEnvironment {
            theme: if dark {
                crate::PreviewTheme::Dark
            } else {
                crate::PreviewTheme::Light
            },
            ..self.preview_environment()
        };
        let _ = self.set_preview_environment(value);
    }
    pub(super) fn connect_file_commands(self: &Rc<Self>) {
        for name in [
            "OpenFile",
            "SaveFile",
            "SaveFileAs",
            "SaveAndOpen",
            "DiscardAndOpen",
            "CancelOpen",
            "PreviewTheme",
        ] {
            let weak = Rc::downgrade(self);
            self.root.get_control::<Button>(name).click(move |_, _| {
                let Some(this) = weak.upgrade() else {
                    return;
                };
                match name {
                    "OpenFile" => this.request_open(),
                    "SaveFile" => this.request_save(false, false),
                    "SaveFileAs" => this.request_save(true, false),
                    "SaveAndOpen" => this.request_save(false, true),
                    "DiscardAndOpen" => {
                        if this.pending_close.borrow().is_some() {
                            let saved = this.saved_source.borrow().clone();
                            this.session.borrow_mut().update_source(saved);
                            this.capture_tab();
                        }
                        let result = this.complete_open();
                        this.report(result);
                    }
                    "CancelOpen" => {
                        this.pending_open.borrow_mut().take();
                        this.pending_close.borrow_mut().take();
                        this.root
                            .get_control::<Border>("FilePrompt")
                            .set_is_visible(false);
                    }
                    "PreviewTheme" => {
                        let scope = this
                            .root
                            .get_control::<ThemeVariantScope>("PreviewThemeScope");
                        this.set_preview_dark(
                            scope.requested_theme_variant()
                                != Some(ferroui_base::styling::ThemeVariant::dark()),
                        );
                    }
                    _ => {}
                }
            });
        }
        self.refresh_file_title();
    }
    pub(super) fn refresh_file_title(&self) {
        let session = self.session.borrow();
        let name = self
            .file
            .borrow()
            .as_ref()
            .map(|file| file.name())
            .unwrap_or_else(|| {
                session
                    .document_uri()
                    .rsplit('/')
                    .next()
                    .unwrap_or("Untitled.xaml")
                    .to_owned()
            });
        let dirty = session.document().source() != self.saved_source.borrow().as_str();
        self.root
            .get_control::<TextBlock>("DocumentTitle")
            .set_text(Some(&format!("{name}{}", if dirty { "  •" } else { "" })));
    }
    pub(super) fn request_open(self: &Rc<Self>) {
        if self.file_busy.replace(true) {
            return;
        }
        let this = self.clone();
        Dispatcher::ui_thread()
            .to_task_scheduler()
            .spawn_local(async move {
                let result = this.choose_open().await;
                this.file_busy.set(false);
                this.report(result);
            });
    }
    async fn choose_open(self: &Rc<Self>) -> Result<(), String> {
        let top = TopLevel::get_top_level(Some(&self.root))
            .ok_or("The designer must be attached to a window to open files")?;
        let provider = top.storage_provider();
        if !provider.can_open() {
            return Err(
                "This host has no file picker. Use DesignerWorkspace::open_document from the host."
                    .into(),
            );
        }
        let mut options = FilePickerOpenOptions::new();
        options.set_title(Some("Open FerroUI XAML".into()));
        options.set_allow_multiple(false);
        options.set_file_type_filter(Some(vec![xaml_type()]));
        let files = provider
            .open_file_picker_async(options)
            .await
            .map_err(|e| e.to_string())?;
        let Some(file) = files.into_iter().next() else {
            return Ok(());
        };
        let mut stream = file.open_read_async().await.map_err(|e| e.to_string())?;
        let mut source = String::new();
        stream
            .read_to_string(&mut source)
            .map_err(|e| e.to_string())?;
        let path = file.path();
        let uri = if path.is_absolute_uri() {
            path.absolute_uri().to_owned()
        } else {
            format!(
                "ferres://FerroUI.Designer/{}",
                file.name().replace(' ', "%20")
            )
        };
        let uri = super::project::document_identity(&uri)?;
        let already_open = self.session.borrow().document_uri() == uri
            || self.project.borrow().tabs.contains_key(&uri);
        if already_open {
            self.activate_document(&uri)?;
        } else {
            self.open_document(source, uri)?;
        }
        *self.file.borrow_mut() = Some(file);
        self.rebuild_document_tabs();
        Ok(())
    }

    fn complete_open(self: &Rc<Self>) -> Result<(), String> {
        let document = self.pending_open.borrow_mut().take();
        let Some(document) = document else {
            let close = self.pending_close.borrow_mut().take();
            self.root
                .get_control::<Border>("FilePrompt")
                .set_is_visible(false);
            if let Some(close) = close {
                if let Some(uri) = self.inactive_dirty_document() {
                    self.activate_document(&uri)?;
                    self.request_close(move || close());
                } else {
                    close();
                }
            }
            return Ok(());
        };
        self.open_document(document.source, document.uri)?;
        *self.file.borrow_mut() = Some(document.file);
        self.root
            .get_control::<Border>("FilePrompt")
            .set_is_visible(false);
        self.refresh_file_title();
        Ok(())
    }
    pub(super) fn request_save(self: &Rc<Self>, save_as: bool, then_open: bool) {
        if self.file_busy.replace(true) {
            return;
        }
        let this = self.clone();
        Dispatcher::ui_thread()
            .to_task_scheduler()
            .spawn_local(async move {
                let result = match this.save_file(save_as).await {
                    Ok(true) if then_open => this.complete_open(),
                    Ok(_) => Ok(()),
                    Err(error) => Err(error),
                };
                this.file_busy.set(false);
                this.report(result);
            });
    }
    /// Save the authored buffer, including incomplete XAML. Returns false on picker cancellation.
    /// The saved snapshot is tracked independently of later editor changes, so a concurrent
    /// change is never mistakenly marked saved. Existing files are checked for external changes.
    pub async fn save_file(self: &Rc<Self>, save_as: bool) -> Result<bool, String> {
        let previous_preview_uri = self.session.borrow().document_uri().to_owned();
        let previous_uri = super::project::document_identity(self.session.borrow().document_uri())?;
        let top = TopLevel::get_top_level(Some(&self.root))
            .ok_or("The designer must be attached to a window to save files")?;
        let provider = top.storage_provider();
        let mut file = if save_as {
            None
        } else {
            self.file.borrow().clone()
        };
        if file.is_none() && !save_as {
            let uri = self.session.borrow().document_uri().to_owned();
            if uri.starts_with("file:") {
                if let Ok(uri) = Uri::absolute(&uri) {
                    file = provider.try_get_file_from_path_async(&uri).await;
                }
            }
        }
        let existing = file.is_some();
        if file.is_none() {
            if !provider.can_save() {
                return Err("This host has no save picker; save DesignerSession::document().source() through the host.".into());
            }
            let mut options = FilePickerSaveOptions::new();
            options.set_title(Some("Save FerroUI XAML".into()));
            options.set_default_extension(Some("xaml".into()));
            options.set_suggested_file_name(Some("View.xaml".into()));
            options.set_file_type_choices(Some(vec![xaml_type()]));
            options.set_show_overwrite_prompt(Some(true));
            file = provider
                .save_file_picker_async(options)
                .await
                .map_err(|e| e.to_string())?;
        }
        let Some(file) = file else {
            return Ok(false);
        };
        self.ensure_save_document(&previous_uri)?;
        let destination = file.path();
        let saved_uri = if destination.is_absolute_uri() {
            super::project::document_identity(destination.absolute_uri())?
        } else {
            previous_uri.clone()
        };
        if saved_uri != previous_uri && self.project.borrow().tabs.contains_key(&saved_uri) {
            return Err(
                "That destination is already open in another tab; close or save that tab first"
                    .into(),
            );
        }
        if existing {
            let mut input = file.open_read_async().await.map_err(|e| e.to_string())?;
            let mut external = String::new();
            input
                .read_to_string(&mut external)
                .map_err(|e| e.to_string())?;
            self.ensure_save_document(&previous_uri)?;
            if external != *self.saved_source.borrow() {
                return Err("The file changed outside the designer. Open the updated file or use Save as to preserve your changes.".into());
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        let local_path = {
            let item: Rc<dyn ferroui_base::platform::storage::IStorageItem> = file.clone();
            item.try_get_local_path()
        };
        #[cfg(target_arch = "wasm32")]
        let local_path: Option<String> = None;
        let source;
        if let Some(path) = local_path {
            source = self.session.borrow().document().source().to_owned();
            write_local(
                std::path::Path::new(&path),
                existing
                    .then(|| self.saved_source.borrow().clone())
                    .as_deref(),
                &source,
            )
            .map_err(|e| e.to_string())?;
        } else {
            // Providers may truncate a file while asynchronously opening its write stream.
            // Capture the intended document first; if the user switches tabs during the await,
            // finish that save and update its retained tab rather than saving another buffer.
            source = self.session.borrow().document().source().to_owned();
            let mut output = file.open_write_async().await.map_err(|e| e.to_string())?;
            output
                .write_all(source.as_bytes())
                .and_then(|_| output.flush())
                .map_err(|e| e.to_string())?;
        }
        if super::project::document_identity(self.session.borrow().document_uri())? == previous_uri
        {
            *self.saved_source.borrow_mut() = source.clone();
            self.session
                .borrow_mut()
                .document
                .mark_saved_snapshot(source.clone());
            if saved_uri != previous_uri {
                self.session
                    .borrow_mut()
                    .set_document_uri(saved_uri.clone())?;
                self.rebind_canvas_document_uri();
                self.project.borrow_mut().tabs.remove(&previous_uri);
            }
            *self.file.borrow_mut() = Some(file);
        } else {
            let mut project = self.project.borrow_mut();
            let mut tab = project
                .tabs
                .remove(&previous_uri)
                .ok_or("The document was saved, but its tab was closed while storage was busy")?;
            tab.saved = source.clone();
            tab.file = Some(file);
            tab.document.mark_saved_snapshot(source.clone());
            tab.preview_uri = saved_uri.clone();
            project.tabs.insert(saved_uri.clone(), tab);
        }
        if saved_uri != previous_uri {
            self.preview_stages.board.borrow_mut().retarget_source(&previous_uri, &saved_uri)?;
            if previous_preview_uri != previous_uri {
                self.preview_stages.board.borrow_mut().retarget_source(&previous_preview_uri, &saved_uri)?;
            }
        }
        self.acknowledge_provider_save(&previous_uri, &saved_uri, &source)?;
        self.refresh(true);
        self.refresh_file_title();
        Ok(true)
    }
    fn ensure_save_document(&self, uri: &str) -> Result<(), String> {
        if super::project::document_identity(self.session.borrow().document_uri())? != uri {
            return Err("The active tab changed while the save dialog was open; return to the document and save again".into());
        }
        Ok(())
    }
}
fn xaml_type() -> Rc<FilePickerFileType> {
    FilePickerFileType::new(Some("FerroUI XAML")).with_patterns(&["*.xaml"])
}

// A local file is replaced only after its full candidate has reached storage. A failed write or
// rename leaves the original intact; provider-owned streams remain the fallback for virtual files.
pub(super) fn write_local(
    path: &std::path::Path,
    expected: Option<&str>,
    source: &str,
) -> std::io::Result<()> {
    use std::fs::{self, OpenOptions};
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT_WRITE: AtomicU64 = AtomicU64::new(1);
    let path = if fs::symlink_metadata(path).is_ok_and(|info| info.file_type().is_symlink()) {
        fs::canonicalize(path)?
    } else {
        path.to_path_buf()
    };
    let directory = path
        .parent()
        .ok_or_else(|| std::io::Error::other("File has no parent directory"))?;
    let mut candidate = None;
    for _ in 0..32 {
        let temp = directory.join(format!(
            ".ferroui-designer-{}-{}.tmp",
            std::process::id(),
            NEXT_WRITE.fetch_add(1, Ordering::Relaxed)
        ));
        match OpenOptions::new().write(true).create_new(true).open(&temp) {
            Ok(file) => {
                candidate = Some((temp, file));
                break;
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }
    let (temporary, mut file) =
        candidate.ok_or_else(|| std::io::Error::other("Could not allocate a save candidate"))?;
    let result = (|| {
        if let Ok(metadata) = fs::metadata(&path) {
            file.set_permissions(metadata.permissions())?;
        }
        file.write_all(source.as_bytes())?;
        file.flush()?;
        file.sync_all()?;
        drop(file);
        if let Some(expected) = expected {
            if fs::read_to_string(&path)? != expected {
                return Err(std::io::Error::other(
                    "The file changed outside the designer; use Save as or reopen it",
                ));
            }
        }
        fs::rename(&temporary, &path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn local_save_replaces_full_file_and_preserves_external_changes() {
        let dir =
            std::env::temp_dir().join(format!("ferroui-designer-save-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("View.xaml");
        std::fs::write(&path, "original").unwrap();
        write_local(&path, Some("original"), "edited").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "edited");
        assert!(write_local(&path, Some("original"), "stale").is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "edited");
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 1);
        std::fs::remove_file(path).unwrap();
        std::fs::remove_dir(dir).unwrap();
    }
}
