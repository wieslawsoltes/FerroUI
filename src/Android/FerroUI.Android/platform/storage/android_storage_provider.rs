//! The storage provider of Android: the pickers of the storage access
//! framework (activities of the system that answer with URIs of document
//! providers), bookmarks of those URIs, files and folders by their path,
//! and the folders of the application the system knows.
//!
//! What the provider computes with (the MIME types of a filter, the name a
//! save picker suggests, the URI of a bookmark, the directory of a
//! well-known folder) is in functions of this file that the host tests
//! run; the provider itself calls Java and exists on Android only.

use ferroui_base::platform::storage::file_io::{DecodeResult, StorageBookmarkHelper};
use ferroui_base::platform::storage::{FilePickerFileType, FilePickerFileTypes, WellKnownFolder};
use ferroui_base::utilities::Uri;
use std::rc::Rc;

/// The platform key of the bookmarks of this provider (`AndroidKey`).
pub(crate) const ANDROID_KEY: &[u8] = b"android";
const CONTENT_URI_SCHEME: &str = "content";
const FILE_URI_SCHEME: &str = "file";

/// `Intent.ACTION_OPEN_DOCUMENT`.
pub(crate) const ACTION_OPEN_DOCUMENT: &str = "android.intent.action.OPEN_DOCUMENT";
/// `Intent.ACTION_CREATE_DOCUMENT`.
pub(crate) const ACTION_CREATE_DOCUMENT: &str = "android.intent.action.CREATE_DOCUMENT";
/// `Intent.ACTION_OPEN_DOCUMENT_TREE`.
pub(crate) const ACTION_OPEN_DOCUMENT_TREE: &str = "android.intent.action.OPEN_DOCUMENT_TREE";
/// `Intent.CATEGORY_OPENABLE`.
pub(crate) const CATEGORY_OPENABLE: &str = "android.intent.category.OPENABLE";
/// `Intent.EXTRA_ALLOW_MULTIPLE`.
pub(crate) const EXTRA_ALLOW_MULTIPLE: &str = "android.intent.extra.ALLOW_MULTIPLE";
/// `Intent.EXTRA_MIME_TYPES`.
pub(crate) const EXTRA_MIME_TYPES: &str = "android.intent.extra.MIME_TYPES";
/// `Intent.EXTRA_TITLE`.
pub(crate) const EXTRA_TITLE: &str = "android.intent.extra.TITLE";
/// `DocumentsContract.EXTRA_INITIAL_URI`.
pub(crate) const EXTRA_INITIAL_URI: &str = "android.provider.extra.INITIAL_URI";
/// `Environment.DIRECTORY_DOCUMENTS`.
pub(crate) const DIRECTORY_DOCUMENTS: &str = "Documents";
/// `Environment.DIRECTORY_DOWNLOADS`.
pub(crate) const DIRECTORY_DOWNLOADS: &str = "Download";
/// `Environment.DIRECTORY_MUSIC`.
pub(crate) const DIRECTORY_MUSIC: &str = "Music";
/// `Environment.DIRECTORY_PICTURES`.
pub(crate) const DIRECTORY_PICTURES: &str = "Pictures";
/// `Environment.DIRECTORY_MOVIES`.
pub(crate) const DIRECTORY_MOVIES: &str = "Movies";
/// The name of the extra of a result that carries an error.
pub(crate) const EXTRA_ERROR: &str = "error";

/// The MIME type of every file (`FilePickerFileTypes.All.MimeTypes![0]`).
pub(crate) fn all_mime_type() -> String {
    FilePickerFileTypes::all()
        .mime_types()
        .and_then(|mime_types| mime_types.first().cloned())
        .expect("the file type of all files has a MIME type")
}

/// Whether a path is what the provider reads items from: an absolute link
/// with the scheme `file` or `content`.
pub(crate) fn is_file_or_content_uri(path: &Uri) -> bool {
    path.is_absolute_uri() && matches!(path.scheme(), FILE_URI_SCHEME | CONTENT_URI_SCHEME)
}

/// The type of the directory of the files of the application
/// (`Context.getExternalFilesDir`) a well-known folder is; none for a
/// folder Android has no directory for.
pub(crate) fn well_known_folder_directory(well_known_folder: WellKnownFolder) -> Option<&'static str> {
    match well_known_folder {
        WellKnownFolder::Desktop => None,
        WellKnownFolder::Documents => Some(DIRECTORY_DOCUMENTS),
        WellKnownFolder::Downloads => Some(DIRECTORY_DOWNLOADS),
        WellKnownFolder::Music => Some(DIRECTORY_MUSIC),
        WellKnownFolder::Pictures => Some(DIRECTORY_PICTURES),
        WellKnownFolder::Videos => Some(DIRECTORY_MOVIES),
    }
}

/// The text of the URI of a bookmark (the argument of `AndroidUri.Parse`
/// in `DecodeUriFromBookmark`); none for a bookmark of another platform.
pub(crate) fn decode_uri_text_from_bookmark(bookmark: &str) -> Option<String> {
    match StorageBookmarkHelper::try_decode_bookmark(ANDROID_KEY, Some(bookmark)) {
        (DecodeResult::Success, bytes) => Some(String::from_utf8_lossy(&bytes.unwrap_or_default()).into_owned()),
        // Attempt to decode 11.0 android bookmarks
        (DecodeResult::InvalidFormat, _) => Some(bookmark.to_string()),
        _ => None,
    }
}

/// The MIME types of the file types of a picker, without those of the file
/// type of all files and without repetitions
/// (`Where(t => t != FilePickerFileTypes.All).SelectMany(f => f.MimeTypes ?? []).Distinct()`).
pub(crate) fn mime_types_of(file_types: Option<&[Rc<FilePickerFileType>]>) -> Vec<String> {
    let all = FilePickerFileTypes::all();
    let mut result: Vec<String> = Vec::new();
    for file_type in file_types.unwrap_or_default() {
        if Rc::ptr_eq(file_type, &all) {
            continue;
        }
        if let Some(mime_types) = file_type.mime_types() {
            for mime_type in mime_types.iter() {
                if !result.contains(mime_type) {
                    result.push(mime_type.clone());
                }
            }
        }
    }
    result
}

/// The file name a save picker suggests: the suggested name, with the
/// default extension after it when there is one.
pub(crate) fn suggested_save_file_name(
    suggested_file_name: Option<&str>,
    default_extension: Option<&str>,
) -> Option<String> {
    let mut file_name = suggested_file_name?.to_string();
    if let Some(ext) = default_extension {
        if !ext.starts_with('.') {
            file_name.push('.');
        }
        file_name.push_str(ext);
    }
    Some(file_name)
}

#[cfg(target_os = "android")]
pub(crate) use imp::AndroidStorageProvider;

#[cfg(target_os = "android")]
mod imp {
    use super::{
        all_mime_type, decode_uri_text_from_bookmark, is_file_or_content_uri, mime_types_of,
        suggested_save_file_name, well_known_folder_directory, ACTION_CREATE_DOCUMENT, ACTION_OPEN_DOCUMENT,
        ACTION_OPEN_DOCUMENT_TREE, CATEGORY_OPENABLE, CONTENT_URI_SCHEME, EXTRA_ALLOW_MULTIPLE, EXTRA_ERROR,
        EXTRA_INITIAL_URI, EXTRA_MIME_TYPES, EXTRA_TITLE, FILE_URI_SCHEME,
    };
    use crate::ferro_activity::FerroActivity;
    use crate::i_activity_result_handler::{ActivityResultHandler, Intent, RESULT_OK};
    use crate::interop::java::{
        call_boolean, call_int, call_object, call_static_boolean, call_static_object, call_void, new_object,
        new_string_array, JavaClass, JavaLocal, JavaObject, JavaRef, JavaValue,
    };
    use crate::interop::natives::sdk_int;
    use crate::platform::platform_support::{
        add_activity_result, get_next_request_code, ActivityResultSubscription, Completion,
    };
    use crate::platform::storage::android_storage_item::{
        helper, object, take_last_error, text_of, uri_of, Activity, AndroidStorageFile, AndroidStorageFolder,
        AndroidStorageItem, AndroidUri, COLUMN_MIME_TYPE, MIME_TYPE_DIR,
    };
    use ferroui_base::input::LocalBoxFuture;
    use ferroui_base::platform::storage::file_io::{BclStorageFile, BclStorageFolder, FileSystemInfo};
    use ferroui_base::platform::storage::{
        FilePickerOpenOptions, FilePickerSaveOptions, FolderPickerOpenOptions, IStorageBookmarkFile,
        IStorageBookmarkFolder, IStorageFile, IStorageFolder, IStorageItem, IStorageProvider, OpenFilePickerResult,
        SaveFilePickerResult, WellKnownFolder,
    };
    use ferroui_base::utilities::Uri;
    use std::cell::RefCell;
    use std::io;
    use std::rc::Rc;

    const INTENT: &str = "android/content/Intent";

    fn ready<T: 'static>(value: T) -> LocalBoxFuture<T> {
        Box::pin(std::future::ready(value))
    }

    /// `AndroidUri.Parse`.
    fn parse_uri(uri_string: &str) -> Option<AndroidUri> {
        call_static_object(
            &JavaClass::find("android/net/Uri"),
            "parse",
            "(Ljava/lang/String;)Landroid/net/Uri;",
            &[JavaValue::String(uri_string)],
        )
        .map(|uri| uri.to_global())
    }

    /// `new Intent(action)`. The methods of an intent that set something
    /// return the intent itself, which the functions below let go.
    fn new_intent(action: &str) -> JavaLocal {
        new_object(&JavaClass::find(INTENT), "(Ljava/lang/String;)V", &[JavaValue::String(action)])
    }

    fn add_category(intent: &JavaLocal, category: &str) {
        let _ = call_object(
            intent,
            "addCategory",
            "(Ljava/lang/String;)Landroid/content/Intent;",
            &[JavaValue::String(category)],
        );
    }

    fn set_type(intent: &JavaLocal, mime_type: &str) {
        let _ = call_object(
            intent,
            "setType",
            "(Ljava/lang/String;)Landroid/content/Intent;",
            &[JavaValue::String(mime_type)],
        );
    }

    fn put_extra_boolean(intent: &JavaLocal, name: &str, value: bool) {
        let _ = call_object(
            intent,
            "putExtra",
            "(Ljava/lang/String;Z)Landroid/content/Intent;",
            &[JavaValue::String(name), JavaValue::Boolean(value)],
        );
    }

    fn put_extra_string(intent: &JavaLocal, name: &str, value: &str) {
        let _ = call_object(
            intent,
            "putExtra",
            "(Ljava/lang/String;Ljava/lang/String;)Landroid/content/Intent;",
            &[JavaValue::String(name), JavaValue::String(value)],
        );
    }

    fn put_extra_string_array(intent: &JavaLocal, name: &str, value: &[String]) {
        let value = new_string_array(value);
        let _ = call_object(
            intent,
            "putExtra",
            "(Ljava/lang/String;[Ljava/lang/String;)Landroid/content/Intent;",
            &[JavaValue::String(name), object(&value)],
        );
    }

    fn put_extra_parcelable(intent: &JavaLocal, name: &str, value: &JavaObject) {
        let _ = call_object(
            intent,
            "putExtra",
            "(Ljava/lang/String;Landroid/os/Parcelable;)Landroid/content/Intent;",
            &[JavaValue::String(name), object(value)],
        );
    }

    /// `Intent.CreateChooser`.
    fn create_chooser(intent: &JavaLocal, title: &str) -> Option<JavaLocal> {
        call_static_object(
            &JavaClass::find(INTENT),
            "createChooser",
            "(Landroid/content/Intent;Ljava/lang/CharSequence;)Landroid/content/Intent;",
            &[object(intent), JavaValue::String(title)],
        )
    }

    /// The storage provider of an activity.
    pub(crate) struct AndroidStorageProvider {
        activity: Activity,
    }

    impl AndroidStorageProvider {
        /// Creates the storage provider of `activity`, an
        /// `android.app.Activity`.
        pub(crate) fn new(activity: Activity) -> Rc<Self> {
            Rc::new(Self { activity })
        }

        fn try_get_file_from_path(&self, file_path: &Uri) -> Option<Rc<dyn IStorageFile>> {
            // The text the path was created from: the URI of the system is parsed from what
            // the caller gave.
            let android_uri = parse_uri(file_path.original_string())?;
            let android_uri_path = text_of(call_object(&android_uri, "getPath", "()Ljava/lang/String;", &[]))?;

            // About the READ_EXTERNAL_STORAGE permission:
            // https://developer.android.com/reference/android/Manifest.permission#READ_EXTERNAL_STORAGE
            //  - "Starting in API level 33, this permission has no effect."
            //  - "Also starting in API level 19, this permission is not required
            //     to read or write files in your application-specific directories [...]"
            // Consequently, we don't try to check for that permission here anymore.

            if file_path.scheme() == FILE_URI_SCHEME {
                let java_file = new_object(
                    &JavaClass::find("java/io/File"),
                    "(Ljava/lang/String;)V",
                    &[JavaValue::String(&android_uri_path)],
                );
                if call_boolean(&java_file, "exists", "()Z", &[]) && call_boolean(&java_file, "isFile", "()Z", &[]) {
                    let absolute_path =
                        text_of(call_object(&java_file, "getAbsolutePath", "()Ljava/lang/String;", &[]))?;
                    let file: Rc<dyn IStorageFile> = BclStorageFile::new(FileSystemInfo::file(absolute_path));
                    return Some(file);
                }
                return None;
            } else if file_path.scheme() == CONTENT_URI_SCHEME {
                // The stream is opened and closed in Java (`StorageHelper.canOpenInputStream`),
                // where the exception the reference catches is caught.
                let opened = call_static_boolean(
                    &helper(),
                    "canOpenInputStream",
                    "(Landroid/content/Context;Landroid/net/Uri;)Z",
                    &[object(&self.activity), object(&android_uri)],
                );
                let _ = take_last_error();
                if opened {
                    let file: Rc<dyn IStorageFile> = AndroidStorageFile::new(&self.activity, android_uri, None, None);
                    return Some(file);
                }
            }

            None
        }

        fn try_get_folder_from_path(&self, folder_path: &Uri) -> Option<Rc<dyn IStorageFolder>> {
            let android_uri = parse_uri(folder_path.original_string())?;
            let android_uri_path = text_of(call_object(&android_uri, "getPath", "()Ljava/lang/String;", &[]))?;

            if folder_path.scheme() == FILE_URI_SCHEME {
                let java_file = new_object(
                    &JavaClass::find("java/io/File"),
                    "(Ljava/lang/String;)V",
                    &[JavaValue::String(&android_uri_path)],
                );
                if call_boolean(&java_file, "exists", "()Z", &[]) && call_boolean(&java_file, "isDirectory", "()Z", &[])
                {
                    let absolute_path =
                        text_of(call_object(&java_file, "getAbsolutePath", "()Ljava/lang/String;", &[]))?;
                    let folder: Rc<dyn IStorageFolder> =
                        BclStorageFolder::new(FileSystemInfo::directory(absolute_path));
                    return Some(folder);
                }
                return None;
            } else if folder_path.scheme() == CONTENT_URI_SCHEME {
                // The exceptions the reference catches around what follows are caught in
                // Java, by the methods of `StorageHelper`.
                if sdk_int() >= 21 {
                    if call_static_boolean(
                        &JavaClass::find("android/provider/DocumentsContract"),
                        "isTreeUri",
                        "(Landroid/net/Uri;)Z",
                        &[object(&android_uri)],
                    ) {
                        let doc_id = text_of(call_static_object(
                            &helper(),
                            "treeDocumentId",
                            "(Landroid/net/Uri;)Ljava/lang/String;",
                            &[object(&android_uri)],
                        ));
                        if take_last_error().is_some() {
                            return None;
                        }
                        if doc_id.is_some_and(|doc_id| !doc_id.is_empty()) {
                            let folder: Rc<dyn IStorageFolder> =
                                AndroidStorageFolder::new(&self.activity, android_uri, false, None, None);
                            return Some(folder);
                        }
                    } else {
                        // `DocumentFile.FromSingleUri(...)` of AndroidX, whose `Exists()` is
                        // a query that answers with a row and whose `IsDirectory` is the
                        // MIME type of a directory (docs/porting/android-platform.md 3.3).
                        let exists = call_static_boolean(
                            &helper(),
                            "documentExists",
                            "(Landroid/content/Context;Landroid/net/Uri;)Z",
                            &[object(&self.activity), object(&android_uri)],
                        );
                        let _ = take_last_error();
                        if exists
                            && AndroidStorageItem::get_column_value(
                                &self.activity,
                                &android_uri,
                                COLUMN_MIME_TYPE,
                                None,
                                None,
                            )
                            .as_deref()
                                == Some(MIME_TYPE_DIR)
                        {
                            let folder: Rc<dyn IStorageFolder> =
                                AndroidStorageFolder::new(&self.activity, android_uri, false, None, None);
                            return Some(folder);
                        }
                    }
                }
            }

            None
        }

        fn decode_uri_from_bookmark(bookmark: &str) -> Option<AndroidUri> {
            parse_uri(&decode_uri_text_from_bookmark(bookmark)?)
        }

        /// Starts the activity of a picker and gives the URIs of its
        /// result. As the reference, the activity is started when the
        /// function is called; the result arrives in `onActivityResult` of
        /// the activity, on the UI thread.
        fn start_activity(
            &self,
            picker_intent: Option<&JavaLocal>,
            single_result: bool,
        ) -> LocalBoxFuture<io::Result<Vec<AndroidUri>>> {
            let (tcs, task) = Completion::<Option<JavaObject>>::new();
            let current_request_code = get_next_request_code();

            let Some(main_activity) = FerroActivity::from_java(&self.activity) else {
                return ready(Err(io::Error::other("Main activity must implement IActivityResultHandler interface.")));
            };

            let subscription: Rc<RefCell<Option<ActivityResultSubscription>>> = Rc::new(RefCell::new(None));
            let on_activity_result: ActivityResultHandler = {
                let subscription = subscription.clone();
                // The activity keeps the handler, so the handler does not keep the activity.
                let main_activity = Rc::downgrade(&main_activity);
                Rc::new(move |request_code: i32, result_code: i32, data: Option<&Intent>| {
                    if current_request_code != request_code {
                        return;
                    }

                    let subscription = subscription.borrow_mut().take();
                    if let (Some(subscription), Some(main_activity)) = (subscription, main_activity.upgrade()) {
                        subscription.remove(&*main_activity);
                    }

                    let _ = tcs.try_set_result(if result_code == RESULT_OK { data.cloned() } else { None });
                })
            };
            *subscription.borrow_mut() = Some(add_activity_result(&*main_activity, on_activity_result));
            call_void(
                &self.activity,
                "startActivityForResult",
                "(Landroid/content/Intent;I)V",
                &[
                    JavaValue::Object(picker_intent.map(|intent| intent as &dyn JavaRef)),
                    JavaValue::Int(current_request_code),
                ],
            );

            Box::pin(async move {
                let mut result_list = Vec::with_capacity(1);
                let result = task.await;

                if let Some(result) = &result {
                    // ClipData first to avoid issue with multiple files selection.
                    let clip_data = if single_result {
                        None
                    } else {
                        call_object(result, "getClipData", "()Landroid/content/ClipData;", &[])
                    };
                    if let Some(clip_data) = clip_data {
                        for i in 0..call_int(&clip_data, "getItemCount", "()I", &[]) {
                            let uri = call_object(
                                &clip_data,
                                "getItemAt",
                                "(I)Landroid/content/ClipData$Item;",
                                &[JavaValue::Int(i)],
                            )
                            .and_then(|item| call_object(&item, "getUri", "()Landroid/net/Uri;", &[]));
                            if let Some(uri) = uri {
                                result_list.push(uri.to_global());
                            }
                        }
                    } else if let Some(uri) = call_object(result, "getData", "()Landroid/net/Uri;", &[]) {
                        result_list.push(uri.to_global());
                    }

                    if call_boolean(result, "hasExtra", "(Ljava/lang/String;)Z", &[JavaValue::String(EXTRA_ERROR)]) {
                        let error = text_of(call_object(
                            result,
                            "getStringExtra",
                            "(Ljava/lang/String;)Ljava/lang/String;",
                            &[JavaValue::String(EXTRA_ERROR)],
                        ));
                        return Err(io::Error::other(error.unwrap_or_default()));
                    }
                }

                Ok(result_list)
            })
        }

        fn try_add_extra_initial_uri(intent: &JavaLocal, folder: Option<&Rc<dyn IStorageFolder>>) {
            if sdk_int() >= 26 {
                if let Some(uri) = folder.and_then(|folder| uri_of(folder.as_any())) {
                    put_extra_parcelable(intent, EXTRA_INITIAL_URI, &uri);
                }
            }
        }
    }

    impl IStorageProvider for AndroidStorageProvider {
        fn can_open(&self) -> bool {
            sdk_int() >= 19
        }

        fn can_save(&self) -> bool {
            sdk_int() >= 19
        }

        fn can_pick_folder(&self) -> bool {
            sdk_int() >= 21
        }

        fn open_folder_bookmark_async(&self, bookmark: &str) -> LocalBoxFuture<Option<Rc<dyn IStorageBookmarkFolder>>> {
            let uri = Self::decode_uri_from_bookmark(bookmark);
            ready(uri.map(|uri| {
                AndroidStorageFolder::new(&self.activity, uri, false, None, None) as Rc<dyn IStorageBookmarkFolder>
            }))
        }

        /// # Panics
        /// Panics when the path is not an absolute link with the scheme
        /// `file` or `content` (the reference throws `ArgumentException`).
        fn try_get_file_from_path_async(&self, file_path: &Uri) -> LocalBoxFuture<Option<Rc<dyn IStorageFile>>> {
            if !is_file_or_content_uri(file_path) {
                panic!("File path is expected to be an absolute link with \"file\" or \"content\" scheme.");
            }

            ready(self.try_get_file_from_path(file_path))
        }

        /// # Panics
        /// Panics when the path is not an absolute link with the scheme
        /// `file` or `content` (the reference throws `ArgumentException`).
        fn try_get_folder_from_path_async(&self, folder_path: &Uri) -> LocalBoxFuture<Option<Rc<dyn IStorageFolder>>> {
            if !is_file_or_content_uri(folder_path) {
                panic!("Folder path is expected to be an absolute link with \"file\" or \"content\" scheme.");
            }

            ready(self.try_get_folder_from_path(folder_path))
        }

        fn try_get_well_known_folder_async(
            &self,
            well_known_folder: WellKnownFolder,
        ) -> LocalBoxFuture<Option<Rc<dyn IStorageFolder>>> {
            let Some(dir_code) = well_known_folder_directory(well_known_folder) else {
                return ready(None);
            };

            let dir = call_object(
                &self.activity,
                "getExternalFilesDir",
                "(Ljava/lang/String;)Ljava/io/File;",
                &[JavaValue::String(dir_code)],
            );
            let Some(dir) = dir.filter(|dir| call_boolean(dir, "exists", "()Z", &[])) else {
                return ready(None);
            };

            // From Android 10(API 29), WellKnownFolders points to the app's external files directories, rather than
            // the system's. These paths can be access directly using File apis without need to go though the
            // ContextResolver or requesting permissions.
            let Some(absolute_path) = text_of(call_object(&dir, "getAbsolutePath", "()Ljava/lang/String;", &[])) else {
                return ready(None);
            };
            let folder: Rc<dyn IStorageFolder> = BclStorageFolder::new(FileSystemInfo::directory(absolute_path));
            ready(Some(folder))
        }

        fn open_file_bookmark_async(&self, bookmark: &str) -> LocalBoxFuture<Option<Rc<dyn IStorageBookmarkFile>>> {
            let uri = Self::decode_uri_from_bookmark(bookmark);
            ready(uri.map(|uri| {
                AndroidStorageFile::new(&self.activity, uri, None, None) as Rc<dyn IStorageBookmarkFile>
            }))
        }

        fn open_file_picker_async(
            &self,
            options: FilePickerOpenOptions,
        ) -> LocalBoxFuture<io::Result<Vec<Rc<dyn IStorageFile>>>> {
            let mime_types = mime_types_of(options.file_type_filter());

            let intent = new_intent(ACTION_OPEN_DOCUMENT);
            add_category(&intent, CATEGORY_OPENABLE);
            put_extra_boolean(&intent, EXTRA_ALLOW_MULTIPLE, options.allow_multiple());
            set_type(&intent, &all_mime_type());
            if !mime_types.is_empty() {
                put_extra_string_array(&intent, EXTRA_MIME_TYPES, &mime_types);
            }

            Self::try_add_extra_initial_uri(&intent, options.suggested_start_location());

            let picker_intent = create_chooser(&intent, options.title().unwrap_or("Select file"));

            let uris = self.start_activity(picker_intent.as_ref(), false);
            let activity = self.activity.clone();
            Box::pin(async move {
                let uris = uris.await?;
                Ok(uris
                    .into_iter()
                    .map(|u| AndroidStorageFile::new(&activity, u, None, None) as Rc<dyn IStorageFile>)
                    .collect())
            })
        }

        fn open_file_picker_with_result_async(
            &self,
            options: FilePickerOpenOptions,
        ) -> LocalBoxFuture<io::Result<OpenFilePickerResult>> {
            let files = self.open_file_picker_async(options);
            Box::pin(async move {
                let files = files.await?;
                Ok(OpenFilePickerResult { files, selected_file_type: None })
            })
        }

        fn save_file_picker_async(
            &self,
            options: FilePickerSaveOptions,
        ) -> LocalBoxFuture<io::Result<Option<Rc<dyn IStorageFile>>>> {
            let mime_types = mime_types_of(options.file_type_choices());

            let intent = new_intent(ACTION_CREATE_DOCUMENT);
            add_category(&intent, CATEGORY_OPENABLE);
            set_type(&intent, &all_mime_type());
            if !mime_types.is_empty() {
                put_extra_string_array(&intent, EXTRA_MIME_TYPES, &mime_types);
            }

            let file_name = suggested_save_file_name(options.suggested_file_name(), options.default_extension());
            if let Some(file_name) = file_name {
                put_extra_string(&intent, EXTRA_TITLE, &file_name);
            }

            Self::try_add_extra_initial_uri(&intent, options.suggested_start_location());

            let picker_intent = create_chooser(&intent, options.title().unwrap_or("Save file"));

            let uris = self.start_activity(picker_intent.as_ref(), true);
            let activity = self.activity.clone();
            Box::pin(async move {
                let uris = uris.await?;
                Ok(uris
                    .into_iter()
                    .map(|u| AndroidStorageFile::new(&activity, u, None, None) as Rc<dyn IStorageFile>)
                    .next())
            })
        }

        fn save_file_picker_with_result_async(
            &self,
            options: FilePickerSaveOptions,
        ) -> LocalBoxFuture<io::Result<SaveFilePickerResult>> {
            let file = self.save_file_picker_async(options);
            Box::pin(async move {
                let file = file.await?;
                Ok(SaveFilePickerResult { file, selected_file_type: None })
            })
        }

        fn open_folder_picker_async(
            &self,
            options: FolderPickerOpenOptions,
        ) -> LocalBoxFuture<io::Result<Vec<Rc<dyn IStorageFolder>>>> {
            let intent = new_intent(ACTION_OPEN_DOCUMENT_TREE);
            put_extra_boolean(&intent, EXTRA_ALLOW_MULTIPLE, options.allow_multiple());

            Self::try_add_extra_initial_uri(&intent, options.suggested_start_location());

            let picker_intent = create_chooser(&intent, options.title().unwrap_or("Select folder"));

            let uris = self.start_activity(picker_intent.as_ref(), false);
            let activity = self.activity.clone();
            Box::pin(async move {
                let uris = uris.await?;
                Ok(uris
                    .into_iter()
                    .map(|u| AndroidStorageFolder::new(&activity, u, false, None, None) as Rc<dyn IStorageFolder>)
                    .collect())
            })
        }
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests of the Android backend that run without a
    // device.
    use super::*;
    use ferroui_base::utilities::UriKind;

    #[test]
    fn the_mime_type_of_every_file_is_any_type() {
        assert_eq!("*/*", all_mime_type());
    }

    #[test]
    fn a_path_is_read_when_it_is_an_absolute_file_or_content_link() {
        let uri = |text: &str, kind: UriKind| Uri::try_create(text, kind).expect("the text is a URI");
        assert!(is_file_or_content_uri(&uri("file:///storage/emulated/0/Download/a.txt", UriKind::Absolute)));
        assert!(is_file_or_content_uri(&uri(
            "content://com.android.externalstorage.documents/tree/primary%3ADownload",
            UriKind::Absolute
        )));
        assert!(!is_file_or_content_uri(&uri("https://example.org/a.txt", UriKind::Absolute)));
        assert!(!is_file_or_content_uri(&uri("Download/a.txt", UriKind::Relative)));
    }

    #[test]
    fn a_well_known_folder_is_a_directory_of_the_files_of_the_application() {
        assert_eq!(None, well_known_folder_directory(WellKnownFolder::Desktop));
        assert_eq!(Some("Documents"), well_known_folder_directory(WellKnownFolder::Documents));
        assert_eq!(Some("Download"), well_known_folder_directory(WellKnownFolder::Downloads));
        assert_eq!(Some("Music"), well_known_folder_directory(WellKnownFolder::Music));
        assert_eq!(Some("Pictures"), well_known_folder_directory(WellKnownFolder::Pictures));
        assert_eq!(Some("Movies"), well_known_folder_directory(WellKnownFolder::Videos));
    }

    #[test]
    fn a_bookmark_of_the_provider_is_decoded_to_its_uri() {
        let uri = "content://com.android.externalstorage.documents/tree/primary%3ADownload";
        let bookmark = StorageBookmarkHelper::encode_bookmark(ANDROID_KEY, Some(uri)).expect("the URI is not empty");
        assert_ne!(uri, bookmark);
        assert_eq!(Some(uri.to_string()), decode_uri_text_from_bookmark(&bookmark));
    }

    #[test]
    fn a_bookmark_that_is_a_plain_uri_is_the_uri() {
        let uri = "content://com.android.externalstorage.documents/tree/primary%3ADownload";
        assert_eq!(Some(uri.to_string()), decode_uri_text_from_bookmark(uri));
    }

    #[test]
    fn a_bookmark_of_another_platform_is_not_decoded() {
        let bookmark = StorageBookmarkHelper::encode_bookmark(b"ios", Some("file:///private/var/a.txt"))
            .expect("the bookmark is not empty");
        assert_eq!(None, decode_uri_text_from_bookmark(&bookmark));
    }

    #[test]
    fn the_mime_types_of_a_filter_are_distinct_and_without_those_of_all_files() {
        assert!(mime_types_of(None).is_empty());
        let none: [Rc<FilePickerFileType>; 0] = [];
        assert!(mime_types_of(Some(&none[..])).is_empty());
        let all = [FilePickerFileTypes::all()];
        assert!(mime_types_of(Some(&all[..])).is_empty());

        let custom = FilePickerFileType::new(Some("Text")).with_mime_types(&["text/plain", "text/xml"]);
        let without = FilePickerFileType::new(Some("No MIME types")).with_patterns(&["*.bin"]);
        let types = [
            FilePickerFileTypes::image_png(),
            FilePickerFileTypes::all(),
            FilePickerFileTypes::xml(),
            custom,
            without,
            FilePickerFileTypes::image_png(),
        ];
        assert_eq!(
            vec!["image/png", "application/xml", "text/xml", "text/plain"],
            mime_types_of(Some(&types[..]))
        );
    }

    #[test]
    fn a_file_type_with_the_mime_type_of_all_files_that_is_not_that_file_type_stays() {
        let any = FilePickerFileType::new(Some("Any")).with_mime_types(&["*/*"]);
        let types = [any];
        assert_eq!(vec!["*/*"], mime_types_of(Some(&types[..])));
    }

    #[test]
    fn the_suggested_name_of_a_save_picker_gets_the_default_extension() {
        assert_eq!(None, suggested_save_file_name(None, None));
        assert_eq!(None, suggested_save_file_name(None, Some("txt")));
        assert_eq!(Some("notes".to_string()), suggested_save_file_name(Some("notes"), None));
        assert_eq!(Some("notes.txt".to_string()), suggested_save_file_name(Some("notes"), Some("txt")));
        assert_eq!(Some("notes.txt".to_string()), suggested_save_file_name(Some("notes"), Some(".txt")));
        assert_eq!(Some("notes.md.txt".to_string()), suggested_save_file_name(Some("notes.md"), Some("txt")));
    }
}
