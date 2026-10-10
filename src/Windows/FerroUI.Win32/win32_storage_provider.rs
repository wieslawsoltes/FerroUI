//! Port of `Win32StorageProvider.cs`: the file and folder pickers of a
//! window, through the item dialogs of the shell (`IFileDialog`).
//!
//! As in the reference a dialog runs on a thread of its own, in a
//! single-threaded apartment, and the picker completes when the dialog
//! closes. What the thread is given and what it gives back are values
//! (strings and numbers): the options are read on the UI thread before the
//! thread starts, and the storage items are made on the UI thread from the
//! paths the thread reports, because the options and the items of the port
//! are objects of one thread.

use crate::win32_com::FILEOPENDIALOGOPTIONS;
use ferroui_base::logging::{LogArea, LogEventLevel, Logger};
use ferroui_base::platform::storage::{FilePickerFileType, FilePickerFileTypes};
use std::rc::Rc;

/// `SIGDN_DESKTOPABSOLUTEPARSING`.
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) const SIGDN_DESKTOPABSOLUTEPARSING: u32 = 0x80028000;

pub(crate) const DEFAULT_DIALOG_OPTIONS: FILEOPENDIALOGOPTIONS = FILEOPENDIALOGOPTIONS(
    FILEOPENDIALOGOPTIONS::FOS_PATHMUSTEXIST.0
        | FILEOPENDIALOGOPTIONS::FOS_FORCEFILESYSTEM.0
        | FILEOPENDIALOGOPTIONS::FOS_NOVALIDATE.0
        | FILEOPENDIALOGOPTIONS::FOS_NOTESTFILECREATE.0
        | FILEOPENDIALOGOPTIONS::FOS_DONTADDTORECENT.0,
);

/// The options a dialog is shown with, from the options it was created
/// with and what the picker asks for.
pub(crate) fn dialog_options(
    options: FILEOPENDIALOGOPTIONS,
    open_folder: bool,
    allow_multiple: bool,
    show_overwrite_prompt: Option<bool>,
) -> FILEOPENDIALOGOPTIONS {
    let mut options = options.0;
    options |= DEFAULT_DIALOG_OPTIONS.0;
    if open_folder {
        options |= FILEOPENDIALOGOPTIONS::FOS_PICKFOLDERS.0;
    }
    if allow_multiple {
        options |= FILEOPENDIALOGOPTIONS::FOS_ALLOWMULTISELECT.0;
    }

    if show_overwrite_prompt == Some(false) {
        options &= !FILEOPENDIALOGOPTIONS::FOS_OVERWRITEPROMPT.0;
    }

    FILEOPENDIALOGOPTIONS(options)
}

/// The file type of the one-based index a dialog reports.
pub(crate) fn try_get_selected_file_type(
    file_types: Option<&[Rc<FilePickerFileType>]>,
    index: i32,
) -> Option<Rc<FilePickerFileType>> {
    match file_types {
        Some(file_types) if index >= 1 && index as usize <= file_types.len() => Some(file_types[index as usize - 1].clone()),
        _ => None,
    }
}

/// The file types of a dialog as the name and the pattern list of each:
/// all files when there are none; a file type without patterns is skipped
/// with a warning.
pub(crate) fn filters_to_specs(filters: Option<&[Rc<FilePickerFileType>]>) -> Vec<(String, String)> {
    let all;
    let filters = match filters {
        Some(filters) if !filters.is_empty() => filters,
        _ => {
            all = [FilePickerFileTypes::all()];
            &all[..]
        }
    };

    let mut result = Vec::with_capacity(filters.len());
    for filter in filters {
        match filter.patterns() {
            Some(patterns) if !patterns.is_empty() => result.push((filter.name().to_string(), patterns.join(";"))),
            _ => {
                if let Some(log) = Logger::try_get(LogEventLevel::Warning, LogArea::WIN32_PLATFORM) {
                    let name = if filter.name().is_empty() { "[unnamed]" } else { filter.name() };
                    log.log(None, &format!("Skipping invalid FilePickerFileType '{name}': no patterns defined."));
                }
            }
        }
    }

    result
}

/// What a dialog is shown with: the values of a picker's options, read on
/// the UI thread.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct FilePickerRequest {
    pub(crate) is_open_file: bool,
    pub(crate) open_folder: bool,
    pub(crate) allow_multiple: bool,
    pub(crate) show_overwrite_prompt: Option<bool>,
    pub(crate) title: Option<String>,
    pub(crate) suggested_file_name: Option<String>,
    /// The index of the suggested file type among the filters as they were
    /// given, when it is one of them.
    pub(crate) suggested_file_type_index: Option<usize>,
    /// The local path of the folder the dialog starts in.
    pub(crate) folder_path: Option<String>,
    pub(crate) default_extension: Option<String>,
    pub(crate) filters: Vec<(String, String)>,
}

/// The file types of a picker that a dialog can show: the ones with
/// patterns, in their order. The indices a dialog takes and reports count
/// these. `None` when the picker has no file types.
pub(crate) fn valid_file_types(filters: Option<&[Rc<FilePickerFileType>]>) -> Option<Vec<Rc<FilePickerFileType>>> {
    Some(filters?.iter().filter(|filter| filter.patterns().is_some_and(|patterns| !patterns.is_empty())).cloned().collect())
}

/// The index of a file type among the filters, by identity.
pub(crate) fn index_of_file_type(
    filters: Option<&[Rc<FilePickerFileType>]>,
    suggested_file_type: Option<&Rc<FilePickerFileType>>,
) -> Option<usize> {
    let suggested_file_type = suggested_file_type?;
    filters?.iter().position(|filter| Rc::ptr_eq(filter, suggested_file_type))
}

#[cfg(windows)]
pub(crate) use imp::Win32StorageProvider;

#[cfg(windows)]
mod imp {
    use super::*;
    use crate::interop::unmanaged_methods::{
        co_create_instance, co_initialize_apartment_threaded, co_task_mem_free, co_uninitialize, from_wide,
        sh_create_item_from_parsing_name, to_wide, ShellIds, COMDLG_FILTERSPEC, HRESULT,
    };
    use crate::win32_com::{IFileDialog, IFileOpenDialog, IShellItem};
    use ferroui_base::input::LocalBoxFuture;
    use ferroui_base::platform::storage::file_io::{BclStorageFile, BclStorageFolder, BclStorageProvider, FileSystemInfo};
    use ferroui_base::platform::storage::{
        FilePickerOpenOptions, FilePickerSaveOptions, FolderPickerOpenOptions, IStorageFile, IStorageFolder, IStorageItem,
        OpenFilePickerResult, SaveFilePickerResult,
    };
    use ferroui_base::threading::{Dispatcher, DispatcherPriority};
    use ferroui_microcom::{ComPtr, HResult};
    use std::future::Future;
    use std::io;
    use std::pin::Pin;
    use std::sync::{Arc, Mutex};
    use std::task::{Context, Poll, Waker};

    /// What the thread of a dialog reports: the paths that were picked and
    /// the one-based index of the file type, or what failed.
    type PickerOutcome = Result<(Vec<String>, i32), String>;

    /// What the UI thread and the thread of a dialog share.
    #[derive(Default)]
    struct Shared {
        outcome: Option<PickerOutcome>,
        waker: Option<Waker>,
    }

    /// The picker as a future of the UI thread.
    struct PickerFuture {
        shared: Arc<Mutex<Shared>>,
    }

    impl Future for PickerFuture {
        type Output = PickerOutcome;

        fn poll(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<PickerOutcome> {
            let mut shared = self.shared.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
            match shared.outcome.take() {
                Some(outcome) => Poll::Ready(outcome),
                None => {
                    shared.waker = Some(context.waker().clone());
                    Poll::Pending
                }
            }
        }
    }

    /// COM on the thread of a dialog, ended when the thread is done.
    struct Apartment(bool);

    impl Apartment {
        fn enter() -> Self {
            Self(co_initialize_apartment_threaded() >= 0)
        }
    }

    impl Drop for Apartment {
        fn drop(&mut self) {
            if self.0 {
                co_uninitialize();
            }
        }
    }

    fn failure(what: &str, error: HResult) -> String {
        format!("{what}: {error:?}")
    }

    pub(crate) struct Win32StorageProvider {
        /// The handle of the window of the provider, which owns its
        /// dialogs.
        hwnd: isize,
    }

    impl Win32StorageProvider {
        pub(crate) fn new(hwnd: isize) -> Self {
            Self { hwnd }
        }

        fn show_file_picker_async(&self, request: FilePickerRequest) -> LocalBoxFuture<io::Result<(Vec<String>, i32)>> {
            // TODO13: verify that we're on the correct dispatcher, matching other platforms' implementations.
            // We should then be able to remove the dedicated thread and simply use IFileDialog directly (needs to be reconfirmed).

            let shared = Arc::new(Mutex::new(Shared::default()));
            let hwnd = self.hwnd;

            let thread = {
                let shared = shared.clone();
                std::thread::Builder::new().name("FerroUI file dialog".to_string()).spawn(move || {
                    let outcome = {
                        let _apartment = Apartment::enter();
                        // A panic in the dialog must not leave the picker
                        // waiting for ever.
                        std::panic::catch_unwind(|| show_dialog(hwnd, &request))
                            .unwrap_or_else(|_| Err("the file dialog panicked".to_string()))
                    };
                    shared.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).outcome = Some(outcome);
                    // The continuation runs on the UI thread.
                    Dispatcher::ui_thread().post(
                        move || {
                            let waker = shared.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).waker.take();
                            if let Some(waker) = waker {
                                waker.wake();
                            }
                        },
                        DispatcherPriority::NORMAL,
                    );
                })
            };

            Box::pin(async move {
                if let Err(error) = thread {
                    return Err(error);
                }
                PickerFuture { shared }.await.map_err(io::Error::other)
            })
        }
    }

    fn local_path(folder: Option<&Rc<dyn IStorageFolder>>) -> Option<String> {
        let item: Rc<dyn IStorageItem> = folder?.clone();
        item.try_get_local_path()
    }

    impl BclStorageProvider for Win32StorageProvider {
        fn can_open(&self) -> bool {
            true
        }

        fn can_save(&self) -> bool {
            true
        }

        fn can_pick_folder(&self) -> bool {
            true
        }

        fn open_folder_picker_async(
            &self,
            options: FolderPickerOpenOptions,
        ) -> LocalBoxFuture<io::Result<Vec<Rc<dyn IStorageFolder>>>> {
            let picker = self.show_file_picker_async(FilePickerRequest {
                is_open_file: true,
                open_folder: true,
                allow_multiple: options.allow_multiple(),
                show_overwrite_prompt: Some(false),
                title: options.title().map(str::to_string),
                suggested_file_name: options.suggested_file_name().map(str::to_string),
                suggested_file_type_index: None,
                folder_path: local_path(options.suggested_start_location()),
                default_extension: None,
                filters: Vec::new(),
            });

            Box::pin(async move {
                let (folders, _) = picker.await?;

                Ok(folders
                    .into_iter()
                    .map(|f| BclStorageFolder::new(FileSystemInfo::directory(f)) as Rc<dyn IStorageFolder>)
                    .collect())
            })
        }

        fn open_file_picker_with_result_async(
            &self,
            options: FilePickerOpenOptions,
        ) -> LocalBoxFuture<io::Result<OpenFilePickerResult>> {
            let file_types = valid_file_types(options.file_type_filter());
            let picker = self.show_file_picker_async(FilePickerRequest {
                is_open_file: true,
                open_folder: false,
                allow_multiple: options.allow_multiple(),
                show_overwrite_prompt: Some(false),
                title: options.title().map(str::to_string),
                suggested_file_name: options.suggested_file_name().map(str::to_string),
                suggested_file_type_index: index_of_file_type(file_types.as_deref(), options.suggested_file_type()),
                folder_path: local_path(options.suggested_start_location()),
                default_extension: None,
                filters: filters_to_specs(options.file_type_filter()),
            });

            Box::pin(async move {
                let (files, type_index) = picker.await?;

                let selected_file_type = try_get_selected_file_type(file_types.as_deref(), type_index);

                Ok(OpenFilePickerResult {
                    files: files.into_iter().map(|f| BclStorageFile::new(FileSystemInfo::file(f)) as Rc<dyn IStorageFile>).collect(),
                    selected_file_type,
                })
            })
        }

        fn save_file_picker_with_result_async(
            &self,
            options: FilePickerSaveOptions,
        ) -> LocalBoxFuture<io::Result<SaveFilePickerResult>> {
            let file_types = valid_file_types(options.file_type_choices());
            let picker = self.show_file_picker_async(FilePickerRequest {
                is_open_file: false,
                open_folder: false,
                allow_multiple: false,
                show_overwrite_prompt: options.show_overwrite_prompt(),
                title: options.title().map(str::to_string),
                suggested_file_name: options.suggested_file_name().map(str::to_string),
                suggested_file_type_index: index_of_file_type(file_types.as_deref(), options.suggested_file_type()),
                folder_path: local_path(options.suggested_start_location()),
                default_extension: options.default_extension().map(str::to_string),
                filters: filters_to_specs(options.file_type_choices()),
            });

            Box::pin(async move {
                let (files, type_index) = picker.await?;

                let file = files.into_iter().next().map(|f| BclStorageFile::new(FileSystemInfo::file(f)) as Rc<dyn IStorageFile>);
                let selected_file_type = try_get_selected_file_type(file_types.as_deref(), type_index);

                Ok(SaveFilePickerResult { file, selected_file_type })
            })
        }
    }

    /// Creates the dialog of a request and gives it everything but its
    /// owner: what happens before the dialog is shown.
    pub(crate) fn create_dialog(request: &FilePickerRequest) -> Result<ComPtr<IFileDialog>, String> {
        let clsid = if request.is_open_file { ShellIds::OPEN_FILE_DIALOG } else { ShellIds::SAVE_FILE_DIALOG };
        let iid = ShellIds::I_FILE_DIALOG;
        let instance = co_create_instance(&clsid, 1, &iid).map_err(|code| format!("CreateInstance: {:#010x}", code as u32))?;
        // SAFETY: the pointer was just returned for the identifier of the
        // interface, with a reference the value takes over.
        let frm: ComPtr<IFileDialog> =
            unsafe { ComPtr::from_raw(instance.cast()) }.ok_or_else(|| "CreateInstance returned no object".to_string())?;

        let options = frm.get_options().map_err(|error| failure("GetOptions", error))?;
        let options = dialog_options(options, request.open_folder, request.allow_multiple, request.show_overwrite_prompt);
        frm.set_options(options).map_err(|error| failure("SetOptions", error))?;

        let mut default_extension = to_wide(request.default_extension.as_deref().unwrap_or(""));
        // SAFETY (this call and the two after it): a null-terminated
        // string that lives through the call, which copies it.
        unsafe { frm.set_default_extension(default_extension.as_mut_ptr()) }
            .map_err(|error| failure("SetDefaultExtension", error))?;

        let mut suggested_file_name = to_wide(request.suggested_file_name.as_deref().unwrap_or(""));
        unsafe { frm.set_file_name(suggested_file_name.as_mut_ptr()) }.map_err(|error| failure("SetFileName", error))?;

        let mut title = to_wide(request.title.as_deref().unwrap_or(""));
        unsafe { frm.set_title(title.as_mut_ptr()) }.map_err(|error| failure("SetTitle", error))?;

        if !request.open_folder {
            let strings: Vec<(Vec<u16>, Vec<u16>)> =
                request.filters.iter().map(|(name, spec)| (to_wide(name), to_wide(spec))).collect();
            let mut specs: Vec<COMDLG_FILTERSPEC> =
                strings.iter().map(|(name, spec)| COMDLG_FILTERSPEC { psz_name: name.as_ptr(), psz_spec: spec.as_ptr() }).collect();
            let count = specs.len();
            // SAFETY: an array of `count` file types whose strings live
            // through the call, which copies them.
            unsafe { frm.set_file_types(count as u32, specs.as_mut_ptr().cast()) }
                .map_err(|error| failure("SetFileTypes", error))?;
            if count > 0 {
                // FileTypeIndex is one based, not zero based.
                frm.set_file_type_index(1).map_err(|error| failure("SetFileTypeIndex", error))?;
            }
        }

        if let Some(fi) = request.suggested_file_type_index {
            frm.set_file_type_index(fi as u32 + 1).map_err(|error| failure("SetFileTypeIndex", error))?;
        }

        if let Some(folder_path) = &request.folder_path {
            if let Ok(directory_shell_item) = sh_create_item_from_parsing_name(folder_path, &ShellIds::I_SHELL_ITEM) {
                // SAFETY: the pointer was just returned for the identifier
                // of the interface, with a reference the value takes over.
                if let Some(proxy) = unsafe { ComPtr::<IShellItem>::from_raw(directory_shell_item.cast()) } {
                    frm.set_folder(Some(&proxy)).map_err(|error| failure("SetFolder", error))?;
                    frm.set_default_folder(Some(&proxy)).map_err(|error| failure("SetDefaultFolder", error))?;
                }
            }
        }

        Ok(frm)
    }

    fn show_dialog(hwnd: isize, request: &FilePickerRequest) -> PickerOutcome {
        let mut result = Vec::new();
        let frm = create_dialog(request)?;

        let show_result = frm.show(hwnd);

        let type_index = frm.get_file_type_index().map_err(|error| failure("GetFileTypeIndex", error))? as i32;

        if show_result as u32 == HRESULT::E_CANCELLED {
            return Ok((result, type_index));
        } else if show_result != 0 {
            return Err(format!("The file dialog failed: {:#010x}", show_result as u32));
        }

        if request.allow_multiple {
            let file_open_dialog = frm.cast::<IFileOpenDialog>().map_err(|error| failure("QueryInterface", error))?;
            if let Some(shell_item_array) = file_open_dialog.get_results().map_err(|error| failure("GetResults", error))? {
                let count = shell_item_array.get_count().map_err(|error| failure("GetCount", error))?;

                for i in 0..count {
                    let shell_item = shell_item_array.get_item_at(i).map_err(|error| failure("GetItemAt", error))?;
                    if let Some(selected) = shell_item.as_deref().and_then(get_parsing_name) {
                        result.push(selected);
                    }
                }
            }
        } else if let Some(shell_item) = frm.get_result().map_err(|error| failure("GetResult", error))? {
            if let Some(single_result) = get_parsing_name(&shell_item) {
                result.push(single_result);
            }
        }

        Ok((result, type_index))
    }

    pub(crate) fn get_parsing_name(shell_item: &IShellItem) -> Option<String> {
        get_display_name(shell_item, SIGDN_DESKTOPABSOLUTEPARSING)
    }

    fn get_display_name(shell_item: &IShellItem, sigdn_name: u32) -> Option<String> {
        let mut psz_string: *mut u16 = std::ptr::null_mut();
        // SAFETY: the item writes a string of the COM allocator to the
        // pointer of this frame, which is read up to its terminator and
        // freed here.
        unsafe {
            if shell_item.get_display_name(sigdn_name, &mut psz_string) == 0 && !psz_string.is_null() {
                let mut length = 0;
                while *psz_string.add(length) != 0 {
                    length += 1;
                }
                let name = from_wide(std::slice::from_raw_parts(psz_string, length));
                co_task_mem_free(psz_string.cast());
                return Some(name);
            }
        }
        None
    }

    /// Against the system: what can be said of a dialog without showing
    /// it.
    #[cfg(test)]
    mod tests {
        use super::*;

        fn with_apartment<T>(f: impl FnOnce() -> T) -> T {
            let _apartment = Apartment::enter();
            f()
        }

        #[test]
        fn an_open_dialog_is_created_and_takes_what_the_picker_asks_for() {
            with_apartment(|| {
                let request = FilePickerRequest {
                    is_open_file: true,
                    allow_multiple: true,
                    show_overwrite_prompt: Some(false),
                    title: Some("Open".to_string()),
                    suggested_file_name: Some("a.txt".to_string()),
                    suggested_file_type_index: Some(1),
                    folder_path: std::env::temp_dir().to_str().map(str::to_string),
                    filters: vec![("Text".to_string(), "*.txt".to_string()), ("All".to_string(), "*.*".to_string())],
                    ..Default::default()
                };
                let frm = create_dialog(&request).expect("the dialog is created");
                let options = frm.get_options().expect("the options are read");
                for flag in [
                    FILEOPENDIALOGOPTIONS::FOS_PATHMUSTEXIST,
                    FILEOPENDIALOGOPTIONS::FOS_FORCEFILESYSTEM,
                    FILEOPENDIALOGOPTIONS::FOS_NOVALIDATE,
                    FILEOPENDIALOGOPTIONS::FOS_ALLOWMULTISELECT,
                ] {
                    assert_ne!(options.0 & flag.0, 0, "{flag:?} is not set: {options:?}");
                }
                assert_eq!(options.0 & FILEOPENDIALOGOPTIONS::FOS_PICKFOLDERS.0, 0);
                assert_eq!(frm.get_file_type_index().expect("the index is read"), 2);
                // The folder the dialog starts in is the one that was set.
                let folder = frm.get_folder().expect("the folder is read").expect("the dialog has a folder");
                let name = get_parsing_name(&folder).expect("the folder has a path");
                let expected = std::fs::canonicalize(std::env::temp_dir()).expect("the temporary directory exists");
                let actual = std::fs::canonicalize(&name).expect("the folder of the dialog exists");
                assert_eq!(actual, expected);
                frm.cast::<IFileOpenDialog>().expect("an open dialog has the interface of its results");
            });
        }

        #[test]
        fn a_folder_dialog_and_a_save_dialog_are_created() {
            with_apartment(|| {
                let folder = FilePickerRequest { is_open_file: true, open_folder: true, ..Default::default() };
                let frm = create_dialog(&folder).expect("the folder dialog is created");
                let options = frm.get_options().expect("the options are read");
                assert_ne!(options.0 & FILEOPENDIALOGOPTIONS::FOS_PICKFOLDERS.0, 0, "{options:?}");

                let save = FilePickerRequest {
                    show_overwrite_prompt: Some(false),
                    default_extension: Some("txt".to_string()),
                    filters: vec![("Text".to_string(), "*.txt".to_string())],
                    ..Default::default()
                };
                let frm = create_dialog(&save).expect("the save dialog is created");
                let options = frm.get_options().expect("the options are read");
                assert_eq!(options.0 & FILEOPENDIALOGOPTIONS::FOS_OVERWRITEPROMPT.0, 0, "{options:?}");
                assert_eq!(frm.get_file_type_index().expect("the index is read"), 1);
                assert!(frm.cast::<IFileOpenDialog>().is_err(), "a save dialog has no list of results");
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_options_of_a_dialog() {
        let none = FILEOPENDIALOGOPTIONS(0);
        assert_eq!(dialog_options(none, false, false, None), DEFAULT_DIALOG_OPTIONS);
        assert_eq!(
            dialog_options(none, true, true, None).0,
            DEFAULT_DIALOG_OPTIONS.0 | FILEOPENDIALOGOPTIONS::FOS_PICKFOLDERS.0 | FILEOPENDIALOGOPTIONS::FOS_ALLOWMULTISELECT.0
        );
        // The overwrite prompt of a save dialog stays unless the picker
        // refuses it.
        let save = FILEOPENDIALOGOPTIONS::FOS_OVERWRITEPROMPT;
        assert_ne!(dialog_options(save, false, false, None).0 & FILEOPENDIALOGOPTIONS::FOS_OVERWRITEPROMPT.0, 0);
        assert_ne!(dialog_options(save, false, false, Some(true)).0 & FILEOPENDIALOGOPTIONS::FOS_OVERWRITEPROMPT.0, 0);
        assert_eq!(dialog_options(save, false, false, Some(false)).0 & FILEOPENDIALOGOPTIONS::FOS_OVERWRITEPROMPT.0, 0);
        // The values of the system.
        assert_eq!(DEFAULT_DIALOG_OPTIONS.0, 0x800 | 0x40 | 0x100 | 0x10000 | 0x2000000);
    }

    #[test]
    fn the_selected_file_type_is_the_one_of_the_one_based_index() {
        let types = [FilePickerFileType::new(Some("a")), FilePickerFileType::new(Some("b"))];
        assert!(try_get_selected_file_type(Some(&types), 0).is_none());
        assert!(Rc::ptr_eq(&try_get_selected_file_type(Some(&types), 1).unwrap(), &types[0]));
        assert!(Rc::ptr_eq(&try_get_selected_file_type(Some(&types), 2).unwrap(), &types[1]));
        assert!(try_get_selected_file_type(Some(&types), 3).is_none());
        assert!(try_get_selected_file_type(None, 1).is_none());
    }

    #[test]
    fn the_filters_are_names_with_their_patterns_joined() {
        let text = FilePickerFileType::new(Some("Text")).with_patterns(&["*.txt", "*.md"]);
        let empty = FilePickerFileType::new(Some("Nothing"));
        let images = FilePickerFileType::new(Some("Images")).with_patterns(&["*.png"]);
        let filters = [text.clone(), empty.clone(), images.clone()];
        assert_eq!(
            filters_to_specs(Some(&filters)),
            vec![("Text".to_string(), "*.txt;*.md".to_string()), ("Images".to_string(), "*.png".to_string())]
        );
        // Without filters: all files.
        let all = filters_to_specs(None);
        assert_eq!(all.len(), 1);
        assert_eq!(all, filters_to_specs(Some(&[])));
        assert_eq!(all[0].1, FilePickerFileTypes::all().patterns().unwrap().join(";"));

        assert_eq!(index_of_file_type(Some(&filters), Some(&images)), Some(2));
        // The dialog counts the file types it shows.
        let valid = valid_file_types(Some(&filters)).unwrap();
        assert_eq!(valid.len(), 2);
        assert_eq!(index_of_file_type(Some(&valid), Some(&images)), Some(1));
        assert_eq!(index_of_file_type(Some(&valid), Some(&empty)), None);
        assert!(valid_file_types(None).is_none());
        assert_eq!(index_of_file_type(Some(&filters), Some(&FilePickerFileType::new(Some("Images")))), None);
        assert_eq!(index_of_file_type(None, Some(&images)), None);
        assert_eq!(index_of_file_type(Some(&filters), None), None);
    }
}
