//! The file dialogs of GTK 3 (the port of
//! `NativeDialogs/GtkNativeFileDialogs.cs`): the storage provider a window
//! falls back to when the desktop portal has no file chooser.
//!
//! A dialog is made and answered on the thread of GTK. What crosses to it
//! and back is plain data: the options of a dialog as values
//! ([`GtkDialogRequest`]) and its answer as paths and the index of the
//! chosen filter ([`GtkDialogResult`]). The storage items, which belong to
//! the UI thread, are made there from the answer; the reference makes them
//! on the thread of GTK.

use super::gtk::{start_gtk, Gtk, GtkFileChooserAction, GtkResponseType, GtkWidget, StartGtkTask};
use crate::interop::glib::{panic_message, ConnectedSignal, Glib, GlibTask, GlibTaskCompletion};
use ferroui_base::input::LocalBoxFuture;
use ferroui_base::platform::storage::file_io::{
    BclStorageFile, BclStorageFolder, BclStorageProvider, FileSystemInfo, StorageProviderHelpers,
};
use ferroui_base::platform::storage::{
    FilePickerFileType, FilePickerOpenOptions, FilePickerSaveOptions, FolderPickerOpenOptions, IStorageFile,
    IStorageFolder, IStorageItem, IStorageProvider, OpenFilePickerResult, SaveFilePickerResult,
};
use std::cell::RefCell;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::Path;
use std::rc::Rc;
use std::sync::Mutex;

/// A file type of a dialog as GTK takes it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct GtkFileFilterData {
    pub name: String,
    pub patterns: Vec<String>,
    pub mime_types: Vec<String>,
}

/// The arguments of `ShowDialog`, as values.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct GtkDialogRequest {
    pub title: Option<String>,
    /// The window of the X server the dialog is transient for.
    pub parent_xid: Option<usize>,
    pub action: GtkFileChooserAction,
    pub multi_select: bool,
    /// The local path of the folder the dialog starts in.
    pub initial_folder: Option<String>,
    pub initial_file_name: Option<String>,
    /// The file types that have a pattern or a MIME type, in their order.
    pub filters: Vec<GtkFileFilterData>,
    /// The index, in `filters`, of the suggested file type.
    pub suggested_filter: Option<usize>,
    pub overwrite_prompt: bool,
}

/// The answer of a dialog: the chosen paths (`None` when the dialog was
/// closed without a choice) and the index of the chosen filter.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct GtkDialogResult {
    pub files: Option<Vec<String>>,
    pub selected_filter: Option<usize>,
}

/// The filters of the file types of a dialog and the index of the
/// suggested one, with the file types the filters were made of, so that an
/// index of the answer finds the object of the caller again. A file type
/// without patterns and without MIME types gets no filter, as in the
/// reference.
pub(crate) fn map_filters(
    filters: Option<&[Rc<FilePickerFileType>]>,
    suggested_file_type: Option<&Rc<FilePickerFileType>>,
) -> (Vec<GtkFileFilterData>, Option<usize>, Vec<Rc<FilePickerFileType>>) {
    let mut data = Vec::new();
    let mut sources = Vec::new();
    let mut suggested = None;
    for f in filters.unwrap_or_default() {
        let patterns = f.patterns();
        let mime_types = f.mime_types();
        if patterns.as_ref().is_some_and(|patterns| !patterns.is_empty())
            || mime_types.as_ref().is_some_and(|mime_types| !mime_types.is_empty())
        {
            if suggested_file_type.is_some_and(|suggested| Rc::ptr_eq(suggested, f)) {
                // Every match sets the filter of the chooser: the last one stays.
                suggested = Some(data.len());
            }
            data.push(GtkFileFilterData {
                name: f.name().to_string(),
                patterns: patterns.map(|patterns| patterns.to_vec()).unwrap_or_default(),
                mime_types: mime_types.map(|mime_types| mime_types.to_vec()).unwrap_or_default(),
            });
            sources.push(f.clone());
        }
    }
    (data, suggested, sources)
}

/// The text of the button that accepts.
pub(crate) fn accept_button_text(action: GtkFileChooserAction) -> &'static str {
    match action {
        GtkFileChooserAction::Save => "Save",
        GtkFileChooserAction::SelectFolder => "Select",
        GtkFileChooserAction::Open => "Open",
    }
}

/// What the name of the file a dialog starts with is given to GTK as:
/// `gtk_file_chooser_set_filename()` expects a full path, so a dialog that
/// opens gets the name joined to the start folder; one that saves gets the
/// name alone.
pub(crate) fn initial_file_name_argument(
    action: GtkFileChooserAction,
    folder_local_path: Option<&str>,
    initial_file_name: &str,
) -> String {
    if action == GtkFileChooserAction::Open {
        Path::new(folder_local_path.unwrap_or("")).join(initial_file_name).to_string_lossy().into_owned()
    } else {
        initial_file_name.to_string()
    }
}

/// The local path of a folder, when it has one.
fn local_path(folder: &Rc<dyn IStorageFolder>) -> Option<String> {
    let item: &dyn IStorageItem = &**folder;
    item.try_get_local_path()
}

fn failure(message: impl Into<String>) -> std::io::Error {
    std::io::Error::other(message.into())
}

/// What the handlers of a dialog share.
struct DialogState {
    disposables: RefCell<Vec<ConnectedSignal>>,
}

impl DialogState {
    fn dispose(&self) {
        let disposables = std::mem::take(&mut *self.disposables.borrow_mut());
        for d in &disposables {
            d.dispose();
        }
    }
}

/// `ShowDialog`, on the thread of GTK: makes the dialog, presents it, and
/// completes `tcs` when the user answers.
fn show_dialog(gtk: Gtk, request: GtkDialogRequest, tcs: GlibTaskCompletion<GtkDialogResult>) {
    let action = request.action;
    let dlg = gtk.gtk_file_chooser_dialog_new(request.title.as_deref(), action);

    update_parent(&gtk, dlg, request.parent_xid);
    if request.multi_select {
        gtk.gtk_file_chooser_set_select_multiple(dlg, true);
    }

    gtk.gtk_window_set_modal(dlg, true);
    gtk.gtk_file_chooser_set_local_only(dlg, false);

    let mut filters_dic: Vec<(GtkWidget, usize)> = Vec::new();
    for (index, f) in request.filters.iter().enumerate() {
        let filter = gtk.gtk_file_filter_new();
        filters_dic.push((filter, index));
        gtk.gtk_file_filter_set_name(filter, &f.name);

        for e in &f.patterns {
            gtk.gtk_file_filter_add_pattern(filter, e);
        }

        for e in &f.mime_types {
            gtk.gtk_file_filter_add_mime_type(filter, e);
        }

        gtk.gtk_file_chooser_add_filter(dlg, filter);

        if request.suggested_filter == Some(index) {
            gtk.gtk_file_chooser_set_filter(dlg, filter);
        }
    }

    let state = Rc::new(DialogState { disposables: RefCell::new(Vec::new()) });

    let (close_state, close_tcs) = (state.clone(), tcs.clone());
    // SAFETY: the dialog is alive (it is never destroyed), and `close` of a dialog passes
    // the dialog and the user data.
    let close = unsafe {
        gtk.glib().connect_signal_generic(
            dlg.as_ptr() as usize,
            "close",
            Box::new(move |_| {
                close_tcs.try_set_result(Ok(GtkDialogResult { files: None, selected_filter: None }));
                close_state.dispose();
                false
            }),
        )
    };
    let (response_state, response_tcs) = (state.clone(), tcs.clone());
    // SAFETY: as above; `response` of a dialog passes the dialog, the response and the user
    // data.
    let response = unsafe {
        gtk.glib().connect_signal_dialog_response(
            dlg.as_ptr() as usize,
            "response",
            Box::new(move |_, resp| {
                let mut result = None;
                let mut selected_filter = None;
                if resp == GtkResponseType::Accept as i32 {
                    result = Some(gtk.gtk_file_chooser_get_filenames(dlg));

                    let current_filter = gtk.gtk_file_chooser_get_filter(dlg);
                    selected_filter = current_filter
                        .and_then(|current| filters_dic.iter().find(|(filter, _)| *filter == current))
                        .map(|(_, index)| *index);
                    // GTK doesn't auto-append the extension: the UI thread does that, with the
                    // file type of the chosen filter.
                }

                gtk.gtk_widget_hide(dlg);
                response_state.dispose();
                response_tcs.try_set_result(Ok(GtkDialogResult { files: result, selected_filter }));
                false
            }),
        )
    };
    match (close, response) {
        (Ok(close), Ok(response)) => *state.disposables.borrow_mut() = vec![close, response],
        (close, response) => {
            let error = close.as_ref().err().or(response.as_ref().err()).cloned().unwrap_or_default();
            for connected in [close, response].into_iter().flatten() {
                connected.dispose();
            }
            tcs.try_set_result(Err(error));
            return;
        }
    }

    gtk.gtk_dialog_add_button(dlg, accept_button_text(action), GtkResponseType::Accept);
    gtk.gtk_dialog_add_button(dlg, "Cancel", GtkResponseType::Cancel);

    let folder_local_path = request.initial_folder.as_deref();
    if let Some(folder_local_path) = folder_local_path {
        gtk.gtk_file_chooser_set_current_folder(dlg, folder_local_path);
    }

    if let Some(initial_file_name) = &request.initial_file_name {
        let fn_ = initial_file_name_argument(action, folder_local_path, initial_file_name);

        if action == GtkFileChooserAction::Save {
            gtk.gtk_file_chooser_set_current_name(dlg, &fn_);
        } else {
            gtk.gtk_file_chooser_set_filename(dlg, &fn_);
        }
    }

    gtk.gtk_file_chooser_set_do_overwrite_confirmation(dlg, request.overwrite_prompt);

    gtk.gtk_window_present(dlg);
}

/// `UpdateParent`: the dialog is transient for the window of the platform.
fn update_parent(gtk: &Gtk, chooser: GtkWidget, parent_xid: Option<usize>) {
    let Some(xid) = parent_xid else {
        return;
    };

    gtk.gtk_widget_realize(chooser);
    let window = gtk.gtk_widget_get_window(chooser);
    let parent = gtk.get_foreign_window(xid);
    if let (Some(window), Some(parent)) = (window, parent) {
        gtk.gdk_window_set_transient_for(window, parent);
    }
}

/// Shows a dialog on the thread of GTK and resolves to its answer
/// (`RunOnGlibThread` around `ShowDialog`).
fn run_dialog(request: GtkDialogRequest) -> impl std::future::Future<Output = std::io::Result<GtkDialogResult>> {
    let task = Glib::try_get().map_err(|error| failure(error.to_string())).map(|glib| {
        let (tcs, task) = GlibTask::new();
        glib.g_timeout_add_once(
            0,
            Box::new(move || {
                let failed = tcs.clone();
                let shown = catch_unwind(AssertUnwindSafe(move || {
                    match Gtk::current() {
                        Some(gtk) => show_dialog(gtk, request, tcs),
                        None => tcs.try_set_result(Err("GTK is not initialized on the thread of GLib".to_string())),
                    }
                }));
                if let Err(payload) = shown {
                    failed.try_set_result(Err(panic_message(&*payload)));
                }
            }),
        );
        task
    });
    async move { task?.await.map_err(failure) }
}

/// The storage provider over the dialogs of GTK.
pub struct GtkSystemDialog {
    /// The window of the X server the dialogs are transient for.
    window: usize,
}

static INITIALIZED: Mutex<Option<StartGtkTask>> = Mutex::new(None);

impl GtkSystemDialog {
    /// The provider for a window, when GTK can be started; `None`
    /// otherwise, which sends the window to its next provider.
    pub async fn try_create(window: usize) -> Option<Rc<dyn IStorageProvider>> {
        let initialized =
            INITIALIZED.lock().unwrap_or_else(std::sync::PoisonError::into_inner).get_or_insert_with(start_gtk).clone();

        if initialized.await {
            Some(Rc::new(GtkSystemDialog { window }))
        } else {
            None
        }
    }

    fn parent_xid(&self) -> Option<usize> {
        (self.window != 0).then_some(self.window)
    }
}

impl BclStorageProvider for GtkSystemDialog {
    fn can_open(&self) -> bool {
        true
    }

    fn can_save(&self) -> bool {
        true
    }

    fn can_pick_folder(&self) -> bool {
        true
    }

    fn open_file_picker_with_result_async(
        &self,
        options: FilePickerOpenOptions,
    ) -> LocalBoxFuture<std::io::Result<OpenFilePickerResult>> {
        let (filters, suggested_filter, sources) =
            map_filters(options.file_type_filter(), options.suggested_file_type());
        let dialog = run_dialog(GtkDialogRequest {
            title: options.title().map(str::to_string),
            parent_xid: self.parent_xid(),
            action: GtkFileChooserAction::Open,
            multi_select: options.allow_multiple(),
            initial_folder: options.suggested_start_location().and_then(local_path),
            initial_file_name: None,
            filters,
            suggested_filter,
            overwrite_prompt: false,
        });
        Box::pin(async move {
            let result = dialog.await?;
            let selected_filter = result.selected_filter.and_then(|index| sources.get(index).cloned());

            let storage_files = result
                .files
                .unwrap_or_default()
                .into_iter()
                .filter(|f| Path::new(f).is_file())
                .map(|f| BclStorageFile::new(FileSystemInfo::file(f)) as Rc<dyn IStorageFile>)
                .collect();

            Ok(OpenFilePickerResult { files: storage_files, selected_file_type: selected_filter })
        })
    }

    fn open_folder_picker_async(
        &self,
        options: FolderPickerOpenOptions,
    ) -> LocalBoxFuture<std::io::Result<Vec<Rc<dyn IStorageFolder>>>> {
        let dialog = run_dialog(GtkDialogRequest {
            title: options.title().map(str::to_string),
            parent_xid: self.parent_xid(),
            action: GtkFileChooserAction::SelectFolder,
            multi_select: options.allow_multiple(),
            initial_folder: options.suggested_start_location().and_then(local_path),
            initial_file_name: None,
            filters: Vec::new(),
            suggested_filter: None,
            overwrite_prompt: false,
        });
        Box::pin(async move {
            let result = dialog.await?;
            Ok(result
                .files
                .unwrap_or_default()
                .into_iter()
                .map(|f| BclStorageFolder::new(FileSystemInfo::directory(f)) as Rc<dyn IStorageFolder>)
                .collect())
        })
    }

    fn save_file_picker_with_result_async(
        &self,
        options: FilePickerSaveOptions,
    ) -> LocalBoxFuture<std::io::Result<SaveFilePickerResult>> {
        let (filters, suggested_filter, sources) =
            map_filters(options.file_type_choices(), options.suggested_file_type());
        let dialog = run_dialog(GtkDialogRequest {
            title: options.title().map(str::to_string),
            parent_xid: self.parent_xid(),
            action: GtkFileChooserAction::Save,
            multi_select: false,
            initial_folder: options.suggested_start_location().and_then(local_path),
            initial_file_name: options.suggested_file_name().map(str::to_string),
            filters,
            suggested_filter,
            overwrite_prompt: options.show_overwrite_prompt().unwrap_or(false),
        });
        Box::pin(async move {
            let result = dialog.await?;
            let selected_filter = result.selected_filter.and_then(|index| sources.get(index).cloned());

            // GTK doesn't auto-append the extension, so we need to do that manually
            let file = result
                .files
                .and_then(|files| files.into_iter().next())
                .and_then(|path| {
                    StorageProviderHelpers::name_with_extension(
                        Some(&path),
                        options.default_extension(),
                        selected_filter.as_deref(),
                    )
                })
                .map(|path| BclStorageFile::new(FileSystemInfo::file(path)) as Rc<dyn IStorageFile>);

            Ok(SaveFilePickerResult { file, selected_file_type: selected_filter })
        })
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests for this file.
    use super::*;

    fn file_type(name: &str, patterns: Option<&[&str]>, mime_types: Option<&[&str]>) -> Rc<FilePickerFileType> {
        let file_type = FilePickerFileType::new(Some(name));
        file_type.set_patterns(patterns.map(|patterns| patterns.iter().map(|p| p.to_string()).collect()));
        file_type.set_mime_types(mime_types.map(|mime_types| mime_types.iter().map(|m| m.to_string()).collect()));
        file_type
    }

    #[test]
    fn file_types_become_filters_and_the_suggested_one_is_found_by_identity() {
        let images = file_type("Images", Some(&["*.png", "*.jpg"]), Some(&["image/png"]));
        let empty = file_type("Nothing", None, None);
        let empty_lists = file_type("Empty", Some(&[]), Some(&[]));
        let text = file_type("Text", None, Some(&["text/plain"]));
        // The same content as `text`, another object: not the suggested one.
        let text_twin = file_type("Text", None, Some(&["text/plain"]));
        let all = [images.clone(), empty, empty_lists, text.clone()];

        let (filters, suggested, sources) = map_filters(Some(&all), Some(&text));
        assert_eq!(
            filters,
            [
                GtkFileFilterData {
                    name: "Images".into(),
                    patterns: vec!["*.png".into(), "*.jpg".into()],
                    mime_types: vec!["image/png".into()],
                },
                GtkFileFilterData { name: "Text".into(), patterns: vec![], mime_types: vec!["text/plain".into()] },
            ]
        );
        assert_eq!(suggested, Some(1));
        assert!(Rc::ptr_eq(&sources[0], &images) && Rc::ptr_eq(&sources[1], &text));

        assert_eq!(map_filters(Some(&all), Some(&text_twin)).1, None);
        assert_eq!(map_filters(Some(&all), None).1, None);
        let (filters, suggested, sources) = map_filters(None, Some(&text));
        assert!(filters.is_empty() && suggested.is_none() && sources.is_empty());
    }

    #[test]
    fn the_accept_button_is_named_after_the_action() {
        assert_eq!(accept_button_text(GtkFileChooserAction::Open), "Open");
        assert_eq!(accept_button_text(GtkFileChooserAction::Save), "Save");
        assert_eq!(accept_button_text(GtkFileChooserAction::SelectFolder), "Select");
    }

    #[test]
    fn a_dialog_that_opens_gets_a_full_path_and_one_that_saves_a_name() {
        assert_eq!(initial_file_name_argument(GtkFileChooserAction::Open, Some("/home/u"), "a.txt"), "/home/u/a.txt");
        assert_eq!(initial_file_name_argument(GtkFileChooserAction::Open, None, "a.txt"), "a.txt");
        assert_eq!(initial_file_name_argument(GtkFileChooserAction::Save, Some("/home/u"), "a.txt"), "a.txt");
        assert_eq!(initial_file_name_argument(GtkFileChooserAction::SelectFolder, Some("/home/u"), "d"), "d");
    }

    #[test]
    fn the_constants_of_gtk_have_their_values() {
        assert_eq!(GtkFileChooserAction::Open as i32, 0);
        assert_eq!(GtkFileChooserAction::Save as i32, 1);
        assert_eq!(GtkFileChooserAction::SelectFolder as i32, 2);
        assert_eq!(GtkResponseType::Accept as i32, -3);
        assert_eq!(GtkResponseType::Cancel as i32, -6);
        assert_eq!(GtkResponseType::DeleteEvent as i32, -4);
    }
}
