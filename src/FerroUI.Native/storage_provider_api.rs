use crate::frn_string::{frn_string_array_to_vec, frn_string_bytes, frn_string_to_string, to_c_string, FrnString, FrnStringArray};
use crate::helpers::ComResultExt;
use crate::interop::*;
use crate::storage_item::{StorageFile, StorageFolder};
use crate::storage_provider_impl::StorageProviderImpl;
use crate::top_level_impl::{top_level_of, TopLevelImpl};
use ferroui_base::input::LocalBoxFuture;
use ferroui_base::logging::{LogArea, LogEventLevel, Logger};
use ferroui_base::platform::storage::file_io::{
    BclStorageFile, BclStorageFolder, DecodeResult, FileSystemInfo, StorageBookmarkHelper, StorageProviderHelpers,
};
use ferroui_base::platform::storage::{
    FilePickerFileType, FilePickerOpenOptions, FilePickerSaveOptions, FolderPickerOpenOptions, IStorageBookmarkFile,
    IStorageBookmarkFolder, IStorageFile, IStorageFolder, IStorageItem, IStorageProvider,
};
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::utilities::{Uri, UriKind};
use ferroui_base::Ref;
use ferroui_controls::platform::IStorageProviderFactory;
use ferroui_controls::TopLevel;
use ferroui_microcom::{ComPtr, HResult};
use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::c_void;
use std::future::Future;
use std::pin::Pin;
use std::rc::{Rc, Weak};
use std::task::{Context, Poll, Waker};

// The native library technically can be used for more than just macOS,
// in which case we should provide different bookmark platform keys, and parse accordingly.
const MAC_OS_KEY: &[u8] = b"macOS";

/// A storage item this backend hands out: an item of the plain file system,
/// or, in a sandboxed application, an item that is accessed inside a
/// security scope.
pub(crate) enum NativeStorageItem {
    BclFile(Rc<BclStorageFile>),
    BclFolder(Rc<BclStorageFolder>),
    File(Rc<StorageFile>),
    Folder(Rc<StorageFolder>),
}

impl NativeStorageItem {
    pub(crate) fn into_item(self) -> Rc<dyn IStorageItem> {
        match self {
            Self::BclFile(file) => file,
            Self::BclFolder(folder) => folder,
            Self::File(file) => file,
            Self::Folder(folder) => folder,
        }
    }

    pub(crate) fn into_file(self) -> Option<Rc<dyn IStorageFile>> {
        match self {
            Self::BclFile(file) => Some(file),
            Self::File(file) => Some(file),
            Self::BclFolder(_) | Self::Folder(_) => None,
        }
    }

    pub(crate) fn into_folder(self) -> Option<Rc<dyn IStorageFolder>> {
        match self {
            Self::BclFolder(folder) => Some(folder),
            Self::Folder(folder) => Some(folder),
            Self::BclFile(_) | Self::File(_) => None,
        }
    }

    pub(crate) fn into_bookmark_file(self) -> Option<Rc<dyn IStorageBookmarkFile>> {
        match self {
            Self::BclFile(file) => Some(file),
            Self::File(file) => Some(file),
            Self::BclFolder(_) | Self::Folder(_) => None,
        }
    }

    pub(crate) fn into_bookmark_folder(self) -> Option<Rc<dyn IStorageBookmarkFolder>> {
        match self {
            Self::BclFolder(folder) => Some(folder),
            Self::Folder(folder) => Some(folder),
            Self::BclFile(_) | Self::File(_) => None,
        }
    }
}

/// The storage services of the native library: the system dialogs,
/// bookmarks and security scopes. Registered as the storage provider
/// factory of the backend.
pub(crate) struct StorageProviderApi {
    // The uses of a security scope are counted on the UI thread only (the
    // reference locks the object; the storage items of this backend are not
    // shared between threads).
    open_scopes: RefCell<HashMap<String, i32>>,
    native: ComPtr<IFrnStorageProvider>,
    sandbox_enabled: bool,
    this: Weak<StorageProviderApi>,
}

impl StorageProviderApi {
    pub(crate) fn new(native: ComPtr<IFrnStorageProvider>, sandbox_enabled: bool) -> Rc<Self> {
        Rc::new_cyclic(|this| Self {
            open_scopes: RefCell::new(HashMap::new()),
            native,
            sandbox_enabled,
            this: this.clone(),
        })
    }

    fn this(&self) -> Rc<Self> {
        self.this.upgrade().expect("the storage provider API is alive while it is used")
    }

    pub(crate) fn try_get_storage_item(&self, item_uri: Option<&Uri>, create: bool) -> Option<NativeStorageItem> {
        if let Some(item_uri) = item_uri {
            if let Some(item_path) = StorageProviderHelpers::try_get_path_from_file_uri(Some(item_uri)) {
                let file_info = FileSystemInfo::file(&item_path);
                if create || file_info.exists() {
                    return Some(if self.sandbox_enabled {
                        NativeStorageItem::File(StorageFile::new(
                            self.this(),
                            file_info,
                            item_uri.clone(),
                            item_uri.clone(),
                        ))
                    } else {
                        NativeStorageItem::BclFile(BclStorageFile::new(file_info))
                    });
                }
                let directory_info = FileSystemInfo::directory(&item_path);
                if create || directory_info.exists() {
                    return Some(if self.sandbox_enabled {
                        NativeStorageItem::Folder(StorageFolder::new(
                            self.this(),
                            directory_info,
                            item_uri.clone(),
                            item_uri.clone(),
                        ))
                    } else {
                        NativeStorageItem::BclFolder(BclStorageFolder::new(directory_info))
                    });
                }
            }
        }

        None
    }

    pub(crate) fn open_security_scope(&self, uri_string: &str) -> Option<Rc<dyn IDisposable>> {
        // Multiple entries are possible.
        // For example, user might open OpenRead stream, and read file properties before closing the file.
        // If we don't check for nested scopes, inner closing scope will break access of the outer scope.
        if self.add_use(uri_string) == 1 {
            let ns_uri_string = IFrnString::from_impl(FrnString::new(uri_string));
            let scope_opened = self.native.open_security_scope(Some(&ns_uri_string));
            if !scope_opened {
                self.remove_use(uri_string);
                if let Some(logger) = Logger::try_get(LogEventLevel::Information, LogArea::MACOS_PLATFORM) {
                    logger.log(None, &format!("OpenSecurityScope returned false for the {uri_string}"));
                }
                return None;
            }
        }

        let api = self.this();
        let uri_string = uri_string.to_owned();
        Some(Disposable::create(move || {
            if api.remove_use(&uri_string) == 0 {
                let ns_uri_string = IFrnString::from_impl(FrnString::new(&uri_string));
                api.native.close_security_scope(Some(&ns_uri_string));
            }
        }))
    }

    fn add_use(&self, uri_string: &str) -> i32 {
        let mut open_scopes = self.open_scopes.borrow_mut();
        let use_value = open_scopes.entry(uri_string.to_owned()).or_insert(0);
        *use_value += 1;
        *use_value
    }

    fn remove_use(&self, uri_string: &str) -> i32 {
        let mut open_scopes = self.open_scopes.borrow_mut();
        let use_value = open_scopes.get(uri_string).copied().unwrap_or(0) - 1;
        if use_value == 0 {
            open_scopes.remove(uri_string);
        } else {
            open_scopes.insert(uri_string.to_owned(), use_value);
        }
        use_value
    }

    pub(crate) fn save_bookmark(&self, uri: &Uri) -> Option<String> {
        let mut error: *mut c_void = std::ptr::null_mut();
        let uri_string = IFrnString::from_impl(FrnString::new(uri.absolute_uri()));
        // SAFETY: `error` is a valid out location for the call; the native
        // side stores an owned string reference there, or leaves it null.
        let bookmark_str = unsafe { self.native.save_bookmark_to_bytes(Some(&uri_string), &mut error) }.check();

        if !error.is_null() {
            // SAFETY: a non-null error is an owned reference to a native
            // string, which the pointer takes over.
            let error_str = unsafe { ComPtr::<IFrnString>::from_raw(error as *mut IFrnString) };
            if let Some(logger) = Logger::try_get(LogEventLevel::Warning, LogArea::MACOS_PLATFORM) {
                let error = error_str.as_ref().and_then(|error| frn_string_to_string(error)).unwrap_or_default();
                logger.log(None, &format!("SaveBookmark for {uri} failed with an error\r\n{error}"));
            }
            return None;
        }

        match bookmark_str {
            Some(bookmark_str) => StorageBookmarkHelper::encode_bookmark_bytes(MAC_OS_KEY, &frn_string_bytes(&bookmark_str)),
            None => None,
        }
    }

    // Support both kinds of bookmarks when reading.
    // Since "save bookmark" implementation will be different depending on the configuration.
    pub(crate) fn read_bookmark(&self, bookmark: &str, is_directory: bool) -> Option<Uri> {
        if let (DecodeResult::Success, Some(mut bytes)) = StorageBookmarkHelper::try_decode_bookmark(MAC_OS_KEY, Some(bookmark)) {
            // SAFETY: the pointer and the length describe the decoded bytes,
            // which live until the call returns; the native side reads them.
            let uri_string =
                unsafe { self.native.read_bookmark_from_bytes(bytes.as_mut_ptr() as *mut c_void, bytes.len() as i32) }
                    .check();
            return uri_string
                .and_then(|uri_string| frn_string_to_string(&uri_string))
                .and_then(|uri_string| Uri::try_create(&uri_string, UriKind::Absolute));
        }
        if let Some(path) = StorageBookmarkHelper::try_decode_bcl_bookmark(bookmark) {
            return Some(StorageProviderHelpers::uri_from_file_path(&path, is_directory));
        }

        None
    }

    pub(crate) fn release_bookmark(&self, uri: &Uri) {
        let uri_string = IFrnString::from_impl(FrnString::new(uri.absolute_uri()));
        self.native.release_bookmark(Some(&uri_string));
    }

    pub(crate) fn open_file_dialog(
        &self,
        top_level: Option<&Rc<TopLevelImpl>>,
        options: FilePickerOpenOptions,
    ) -> LocalBoxFuture<(Vec<Rc<dyn IStorageFile>>, Option<Rc<FilePickerFileType>>)> {
        let file_types = IFrnFilePickerFileTypes::from_impl(FilePickerFileTypesWrapper::new(
            options.file_type_filter().map(<[_]>::to_vec),
            None,
            options.suggested_file_type().cloned(),
        ));
        let suggested_directory =
            options.suggested_start_location().map(|location| location.path().absolute_uri().to_owned()).unwrap_or_default();

        let dialog = self.open_dialog_async(
            |events| {
                self.native.open_file_dialog(
                    top_level.and_then(|top_level| top_level.native()).as_deref(),
                    Some(events),
                    options.allow_multiple(),
                    Some(&to_c_string(options.title().unwrap_or_default())),
                    Some(&to_c_string(&suggested_directory)),
                    Some(&to_c_string(options.suggested_file_name().unwrap_or_default())),
                    Some(&file_types),
                );
            },
            false,
        );

        Box::pin(async move {
            let (items, selected_filter_index) = dialog.await;
            drop(file_types);

            let files = items.into_iter().filter_map(NativeStorageItem::into_file).collect();
            let selected_type = try_get_selected_file_type(options.file_type_filter(), selected_filter_index);

            (files, selected_type)
        })
    }

    pub(crate) fn save_file_dialog(
        &self,
        top_level: Option<&Rc<TopLevelImpl>>,
        options: FilePickerSaveOptions,
    ) -> LocalBoxFuture<(Option<Rc<dyn IStorageFile>>, Option<Rc<FilePickerFileType>>)> {
        let file_types = IFrnFilePickerFileTypes::from_impl(FilePickerFileTypesWrapper::new(
            options.file_type_choices().map(<[_]>::to_vec),
            options.default_extension().map(str::to_owned),
            options.suggested_file_type().cloned(),
        ));
        let suggested_directory =
            options.suggested_start_location().map(|location| location.path().absolute_uri().to_owned()).unwrap_or_default();

        let dialog = self.open_dialog_async(
            |events| {
                self.native.save_file_dialog(
                    top_level.and_then(|top_level| top_level.native()).as_deref(),
                    Some(events),
                    Some(&to_c_string(options.title().unwrap_or_default())),
                    Some(&to_c_string(&suggested_directory)),
                    Some(&to_c_string(options.suggested_file_name().unwrap_or_default())),
                    Some(&file_types),
                );
            },
            true,
        );

        Box::pin(async move {
            let (items, selected_filter_index) = dialog.await;
            drop(file_types);

            let file = items.into_iter().find_map(NativeStorageItem::into_file);
            let selected_type = try_get_selected_file_type(options.file_type_choices(), selected_filter_index);

            (file, selected_type)
        })
    }

    pub(crate) fn select_folder_dialog(
        &self,
        top_level: Option<&Rc<TopLevelImpl>>,
        options: FolderPickerOpenOptions,
    ) -> LocalBoxFuture<Vec<Rc<dyn IStorageFolder>>> {
        let suggested_directory =
            options.suggested_start_location().map(|location| location.path().absolute_uri().to_owned()).unwrap_or_default();

        let dialog = self.open_dialog_async(
            |events| {
                self.native.select_folder_dialog(
                    top_level.and_then(|top_level| top_level.native()).as_deref(),
                    Some(events),
                    options.allow_multiple(),
                    Some(&to_c_string(options.title().unwrap_or_default())),
                    Some(&to_c_string(&suggested_directory)),
                );
            },
            false,
        );

        Box::pin(async move {
            let (items, _) = dialog.await;

            items.into_iter().filter_map(NativeStorageItem::into_folder).collect()
        })
    }

    pub(crate) fn open_dialog_async(
        &self,
        run_dialog: impl FnOnce(&IFrnSystemDialogEvents),
        create: bool,
    ) -> LocalBoxFuture<(Vec<NativeStorageItem>, Option<i32>)> {
        let completion = Rc::new(RefCell::new(Completion::default()));
        let events = IFrnSystemDialogEvents::from_impl(SystemDialogEvents { completion: completion.clone() });
        run_dialog(&events);
        let api = self.this();

        Box::pin(async move {
            let (result, selected_filter_index) = CompletionFuture(completion).await;
            drop(events);

            let items = result
                .iter()
                .filter_map(|f| Uri::try_create(f, UriKind::Absolute))
                .filter_map(|uri| api.try_get_storage_item(Some(&uri), create))
                .collect();

            (items, selected_filter_index)
        })
    }

    pub(crate) fn try_resolve_file_reference_uri(&self, uri: &Uri) -> Option<Uri> {
        let uri_string = IFrnString::from_impl(FrnString::new(uri.absolute_uri()));
        let result_string = self.native.try_resolve_file_reference_uri(Some(&uri_string)).check();

        result_string
            .and_then(|result_string| frn_string_to_string(&result_string))
            .and_then(|result_string| Uri::try_create(&result_string, UriKind::Absolute))
    }
}

impl IStorageProviderFactory for StorageProviderApi {
    fn create_provider(&self, top_level: &Ref<TopLevel>) -> Rc<dyn IStorageProvider> {
        let platform_impl = top_level.platform_impl().expect("the top-level has a platform implementation");
        let top_level_impl = top_level_of(&*platform_impl).expect("the top-level belongs to this backend").clone();
        Rc::new(StorageProviderImpl::new(top_level_impl, self.this()))
    }
}

fn try_get_selected_file_type(
    file_types: Option<&[Rc<FilePickerFileType>]>,
    nullable_index: Option<i32>,
) -> Option<Rc<FilePickerFileType>> {
    match (file_types, nullable_index) {
        (Some(file_types), Some(index)) if index >= 0 && (index as usize) < file_types.len() => {
            Some(file_types[index as usize].clone())
        }
        _ => None,
    }
}

/// The file types of a picker, as native code reads them.
///
/// The strings and arrays handed out are owned by the references native
/// code receives (the reference keeps a list of them to dispose with the
/// wrapper; a reference count needs none).
pub(crate) struct FilePickerFileTypesWrapper {
    types: Option<Vec<Rc<FilePickerFileType>>>,
    default_extension: Option<String>,
    suggested_type: Option<Rc<FilePickerFileType>>,
}

impl FilePickerFileTypesWrapper {
    pub(crate) fn new(
        types: Option<Vec<Rc<FilePickerFileType>>>,
        default_extension: Option<String>,
        suggested_type: Option<Rc<FilePickerFileType>>,
    ) -> Self {
        Self { types, default_extension, suggested_type }
    }

    fn file_type(&self, index: i32) -> &Rc<FilePickerFileType> {
        &self.types.as_ref().expect("the picker has file types")[index as usize]
    }

    fn string_array(items: Option<impl IntoIterator<Item = String>>) -> Result<Option<ComPtr<IFrnStringArray>>, HResult> {
        let items: Vec<String> = items.map(|items| items.into_iter().collect()).unwrap_or_default();
        Ok(Some(IFrnStringArray::from_impl(FrnStringArray::new(items))))
    }
}

impl IFrnFilePickerFileTypesImpl for FilePickerFileTypesWrapper {
    fn get_count(&self) -> i32 {
        crate::callback_base::guard(0, || self.types.as_ref().map_or(0, |types| types.len() as i32))
    }

    fn is_default_type(&self, index: i32) -> bool {
        crate::callback_base::guard(false, || {
            let Some(types) = self.types.as_ref() else {
                return false;
            };

            if let Some(suggested_type) = self.suggested_type.as_ref() {
                if Rc::ptr_eq(&types[index as usize], suggested_type) {
                    return true;
                }
            }

            match self.default_extension.as_ref() {
                Some(default_extension) => types[index as usize]
                    .try_get_extensions()
                    .is_some_and(|extensions| extensions.iter().any(|extension| default_extension.ends_with(extension.as_str()))),
                None => false,
            }
        })
    }

    fn is_any_type(&self, index: i32) -> bool {
        crate::callback_base::guard(false, || {
            let file_type = self.file_type(index);
            file_type.patterns().is_some_and(|patterns| patterns.iter().any(|pattern| pattern == "*.*"))
                || file_type.mime_types().is_some_and(|mime_types| mime_types.iter().any(|mime_type| mime_type == "*.*"))
        })
    }

    fn get_name(&self, index: i32) -> Option<ComPtr<IFrnString>> {
        crate::callback_base::guard(None, || Some(IFrnString::from_impl(FrnString::new(self.file_type(index).name()))))
    }

    fn get_patterns(&self, index: i32) -> Result<Option<ComPtr<IFrnStringArray>>, HResult> {
        crate::callback_base::guard(Ok(None), || {
            Self::string_array(self.file_type(index).patterns().map(|patterns| patterns.to_vec()))
        })
    }

    fn get_extensions(&self, index: i32) -> Result<Option<ComPtr<IFrnStringArray>>, HResult> {
        crate::callback_base::guard(Ok(None), || Self::string_array(self.file_type(index).try_get_extensions()))
    }

    fn get_mime_types(&self, index: i32) -> Result<Option<ComPtr<IFrnStringArray>>, HResult> {
        crate::callback_base::guard(Ok(None), || {
            Self::string_array(self.file_type(index).mime_types().map(|mime_types| mime_types.to_vec()))
        })
    }

    fn get_apple_uniform_type_identifiers(&self, index: i32) -> Result<Option<ComPtr<IFrnStringArray>>, HResult> {
        crate::callback_base::guard(Ok(None), || {
            Self::string_array(
                self.file_type(index).apple_uniform_type_identifiers().map(|identifiers| identifiers.to_vec()),
            )
        })
    }
}

/// The outcome of a system dialog: the URIs of the chosen items and the
/// index of the chosen file type, if the dialog reported one.
type DialogResult = (Vec<String>, Option<i32>);

/// The task completion source of a system dialog.
#[derive(Default)]
struct Completion {
    result: Option<DialogResult>,
    waker: Option<Waker>,
}

struct CompletionFuture(Rc<RefCell<Completion>>);

impl Future for CompletionFuture {
    type Output = DialogResult;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<DialogResult> {
        let mut completion = self.0.borrow_mut();
        match completion.result.take() {
            Some(result) => Poll::Ready(result),
            None => {
                completion.waker = Some(cx.waker().clone());
                Poll::Pending
            }
        }
    }
}

/// Receives the outcome of a system dialog from native code.
pub(crate) struct SystemDialogEvents {
    completion: Rc<RefCell<Completion>>,
}

impl SystemDialogEvents {
    fn complete(&self, ppv: Option<&IFrnStringArray>, selected_filter_index: Option<i32>) {
        let items = ppv.map(frn_string_array_to_vec).unwrap_or_default();
        let type_index = selected_filter_index.filter(|index| *index >= 0);
        let waker = {
            let mut completion = self.completion.borrow_mut();
            if completion.result.is_some() {
                return;
            }
            completion.result = Some((items, type_index));
            completion.waker.take()
        };
        if let Some(waker) = waker {
            waker.wake();
        }
    }
}

impl IFrnSystemDialogEventsImpl for SystemDialogEvents {
    fn on_completed(&self, array: Option<&IFrnStringArray>) {
        crate::callback_base::guard((), || self.complete(array, None))
    }

    fn on_completed_with_filter(&self, array: Option<&IFrnStringArray>, selected_filter_index: i32) {
        crate::callback_base::guard((), || self.complete(array, Some(selected_filter_index)))
    }
}

#[cfg(test)]
mod tests {
    //! Tests of this port; the reference has none for these classes.

    use super::*;

    fn completed(events: &SystemDialogEvents) -> Option<DialogResult> {
        events.completion.borrow_mut().result.take()
    }

    fn events() -> SystemDialogEvents {
        SystemDialogEvents { completion: Rc::new(RefCell::new(Completion::default())) }
    }

    #[test]
    fn dialog_events_complete_with_the_items_and_no_filter() {
        let events = events();
        let array = IFrnStringArray::from_impl(FrnStringArray::new(["file:///a.txt", "file:///b.txt"]));
        events.on_completed(Some(&array));
        assert_eq!(completed(&events), Some((vec!["file:///a.txt".to_owned(), "file:///b.txt".to_owned()], None)));
    }

    #[test]
    fn dialog_events_complete_empty_without_an_array() {
        let events = events();
        events.on_completed(None);
        assert_eq!(completed(&events), Some((Vec::new(), None)));
    }

    #[test]
    fn dialog_events_drop_a_negative_filter_index() {
        let events = events();
        events.on_completed_with_filter(None, -1);
        assert_eq!(completed(&events), Some((Vec::new(), None)));

        let events = self::events();
        events.on_completed_with_filter(None, 2);
        assert_eq!(completed(&events), Some((Vec::new(), Some(2))));
    }

    #[test]
    fn dialog_events_keep_the_first_result() {
        let events = events();
        events.on_completed_with_filter(None, 1);
        events.on_completed(None);
        assert_eq!(completed(&events), Some((Vec::new(), Some(1))));
    }

    #[test]
    fn file_types_wrapper_reports_the_types() {
        let text = FilePickerFileType::new(Some("Text")).with_patterns(&["*.txt", "*.md"]).with_mime_types(&["text/plain"]);
        let all = FilePickerFileType::new(Some("All")).with_patterns(&["*.*"]);
        let wrapper = FilePickerFileTypesWrapper::new(Some(vec![text.clone(), all]), Some("md".to_owned()), None);

        assert_eq!(wrapper.get_count(), 2);
        assert!(wrapper.is_default_type(0));
        assert!(!wrapper.is_default_type(1));
        assert!(!wrapper.is_any_type(0));
        assert!(wrapper.is_any_type(1));
        assert_eq!(frn_string_to_string(&wrapper.get_name(0).unwrap()).as_deref(), Some("Text"));
        assert_eq!(frn_string_array_to_vec(&wrapper.get_patterns(0).unwrap().unwrap()), ["*.txt", "*.md"]);
        assert_eq!(frn_string_array_to_vec(&wrapper.get_mime_types(0).unwrap().unwrap()), ["text/plain"]);
        assert!(frn_string_array_to_vec(&wrapper.get_apple_uniform_type_identifiers(0).unwrap().unwrap()).is_empty());
    }

    #[test]
    fn file_types_wrapper_prefers_the_suggested_type() {
        let first = FilePickerFileType::new(Some("First")).with_patterns(&["*.a"]);
        let second = FilePickerFileType::new(Some("Second")).with_patterns(&["*.b"]);
        let wrapper = FilePickerFileTypesWrapper::new(Some(vec![first, second.clone()]), None, Some(second));

        assert!(!wrapper.is_default_type(0));
        assert!(wrapper.is_default_type(1));
    }

    #[test]
    fn file_types_wrapper_without_types_is_empty() {
        let wrapper = FilePickerFileTypesWrapper::new(None, Some("txt".to_owned()), None);

        assert_eq!(wrapper.get_count(), 0);
        assert!(!wrapper.is_default_type(0));
    }

    #[test]
    fn selected_file_type_is_the_type_at_the_index() {
        let types = vec![FilePickerFileType::new(Some("A")), FilePickerFileType::new(Some("B"))];

        assert!(Rc::ptr_eq(&try_get_selected_file_type(Some(&types), Some(1)).unwrap(), &types[1]));
        assert!(try_get_selected_file_type(Some(&types), Some(2)).is_none());
        assert!(try_get_selected_file_type(Some(&types), Some(-1)).is_none());
        assert!(try_get_selected_file_type(Some(&types), None).is_none());
        assert!(try_get_selected_file_type(None, Some(0)).is_none());
    }
}
