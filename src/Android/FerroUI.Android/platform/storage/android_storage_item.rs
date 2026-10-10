//! The storage items of Android: a file or a folder behind a URI of a
//! document provider (`content:`), read, written, listed, created, moved
//! and deleted through the content resolver of the activity and the
//! documents contract.
//!
//! The values the items compute with are functions of this file that the
//! host tests run. The items themselves call Java and exist on Android
//! only. A call that can throw is made by a static method of the Java
//! class `StorageHelper`, which catches the exception and keeps its text:
//! a Java exception inside a call ends the native side, and the reference
//! catches most of them. Where the reference lets an exception reach its
//! caller, the member here returns the error when it has one to return
//! (`std::io::Result`), and otherwise writes it to the log (error level)
//! and answers as if nothing was found.

use ferroui_base::animation::TimeSpan;
use ferroui_base::utilities::DateTimeOffset;
use std::io::{self, Read, Seek, SeekFrom, Write};

/// The class of the Java layer the storage calls go through.
pub(crate) const STORAGE_HELPER: &str = "org/ferroui/android/StorageHelper";

/// `DocumentsContract.Document.COLUMN_DOCUMENT_ID`.
pub(crate) const COLUMN_DOCUMENT_ID: &str = "document_id";
/// `DocumentsContract.Document.COLUMN_MIME_TYPE`.
pub(crate) const COLUMN_MIME_TYPE: &str = "mime_type";
/// `DocumentsContract.Document.COLUMN_DISPLAY_NAME`.
pub(crate) const COLUMN_DISPLAY_NAME: &str = "_display_name";
/// `DocumentsContract.Document.COLUMN_LAST_MODIFIED`.
pub(crate) const COLUMN_LAST_MODIFIED: &str = "last_modified";
/// `DocumentsContract.Document.COLUMN_FLAGS`.
pub(crate) const COLUMN_FLAGS: &str = "flags";
/// `DocumentsContract.Document.COLUMN_SIZE`.
pub(crate) const COLUMN_SIZE: &str = "_size";
/// `DocumentsContract.Document.MIME_TYPE_DIR`.
pub(crate) const MIME_TYPE_DIR: &str = "vnd.android.document/directory";
/// `DocumentsContract.Document.FLAG_VIRTUAL_DOCUMENT`
/// (`DocumentContractFlags.VirtualDocument` of the reference).
pub(crate) const FLAG_VIRTUAL_DOCUMENT: i32 = 1 << 9;
/// `MediaStore.MediaColumns.DISPLAY_NAME`.
pub(crate) const MEDIA_COLUMN_DISPLAY_NAME: &str = "_display_name";
/// `Intent.FLAG_GRANT_READ_URI_PERMISSION`
/// (`ActivityFlags.GrantReadUriPermission` of the reference).
pub(crate) const FLAG_GRANT_READ_URI_PERMISSION: i32 = 0x0000_0001;
/// `Intent.FLAG_GRANT_WRITE_URI_PERMISSION`
/// (`ActivityFlags.GrantWriteUriPermission` of the reference).
pub(crate) const FLAG_GRANT_WRITE_URI_PERMISSION: i32 = 0x0000_0002;
/// `Manifest.permission.READ_EXTERNAL_STORAGE`.
pub(crate) const READ_EXTERNAL_STORAGE: &str = "android.permission.READ_EXTERNAL_STORAGE";
/// The MIME type of a file that is created when the system knows none for
/// the extension of its name.
pub(crate) const DEFAULT_MIME_TYPE: &str = "application/octet-stream";

/// The largest number of milliseconds since 1970 that is a date
/// (the last millisecond of the year 9999).
const MAX_UNIX_TIME_MILLISECONDS: i64 = 253_402_300_799_999;

/// The name of an item whose provider tells none: the last part of the
/// last path segment of its URI
/// (`LastOrDefault()?.Split("/", RemoveEmptyEntries).LastOrDefault() ?? string.Empty`).
pub(crate) fn name_from_last_path_segment(last_path_segment: Option<&str>) -> String {
    last_path_segment
        .and_then(|segment| segment.split('/').filter(|part| !part.is_empty()).next_back())
        .unwrap_or_default()
        .to_string()
}

/// Whether the text of the flags column of a document has the flag of a
/// virtual document.
pub(crate) fn is_virtual_document_flags(value: Option<&str>) -> bool {
    match value {
        Some(value) if !value.is_empty() => match value.trim().parse::<i32>() {
            Ok(flags_int) => flags_int & FLAG_VIRTUAL_DOCUMENT == FLAG_VIRTUAL_DOCUMENT,
            Err(_) => false,
        },
        _ => false,
    }
}

/// The date of the last-modified column of a document
/// (`longValue > 0 ? DateTimeOffset.FromUnixTimeMilliseconds(longValue) : null`).
/// A number that is no date gives none, where the reference catches the
/// exception of the conversion.
pub(crate) fn date_from_unix_time_milliseconds(long_value: i64) -> Option<DateTimeOffset> {
    if long_value <= 0 || long_value > MAX_UNIX_TIME_MILLISECONDS {
        return None;
    }
    Some(DateTimeOffset::UNIX_EPOCH.add(TimeSpan::from_ticks(long_value * 10_000)))
}

/// A column `StorageHelper.queryLongs` read from a row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum LongColumn {
    /// The row has no such column.
    Missing,
    Value(i64),
    /// Reading the column threw; the text of the exception.
    Failed(String),
}

/// Reads an element of the answer of `StorageHelper.queryLongs`: decimal
/// digits, nothing for a column the row does not have, or an exclamation
/// mark and the text of an exception.
pub(crate) fn parse_long_column(text: &str) -> LongColumn {
    if let Some(exception) = text.strip_prefix('!') {
        LongColumn::Failed(exception.to_string())
    } else if text.is_empty() {
        LongColumn::Missing
    } else {
        match text.parse::<i64>() {
            Ok(value) => LongColumn::Value(value),
            Err(error) => LongColumn::Failed(error.to_string()),
        }
    }
}

/// The stream of an asset file descriptor: the whole descriptor when the
/// asset declares no length (`AssetFileDescriptor.UNKNOWN_LENGTH`), else `declared_length` bytes from
/// `start_offset`. This is what `AssetFileDescriptor.createInputStream`
/// and `createOutputStream` give, which the streams of a content resolver
/// are; the port reads and writes the descriptor itself, because a Java
/// stream would need a call into Java for every buffer.
pub(crate) struct AssetStream<S> {
    inner: S,
    remaining: Option<u64>,
}

impl<S: Seek> AssetStream<S> {
    pub(crate) fn new(mut inner: S, start_offset: i64, declared_length: i64) -> io::Result<Self> {
        if declared_length < 0 {
            // The whole descriptor, which may be a pipe: it is not positioned.
            return Ok(Self { inner, remaining: None });
        }
        inner.seek(SeekFrom::Start(start_offset.max(0) as u64))?;
        Ok(Self { inner, remaining: Some(declared_length as u64) })
    }
}

impl<S: Read> Read for AssetStream<S> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self.remaining {
            None => self.inner.read(buf),
            Some(0) => Ok(0),
            Some(remaining) => {
                let count = buf.len().min(usize::try_from(remaining).unwrap_or(usize::MAX));
                let read = self.inner.read(&mut buf[..count])?;
                self.remaining = Some(remaining - read as u64);
                Ok(read)
            }
        }
    }
}

impl<S: Write> Write for AssetStream<S> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self.remaining {
            None => self.inner.write(buf),
            // What is written past the end of the range is dropped, as the stream of the
            // system drops it.
            Some(0) => Ok(buf.len()),
            Some(remaining) => {
                let count = buf.len().min(usize::try_from(remaining).unwrap_or(usize::MAX));
                let written = self.inner.write(&buf[..count])?;
                let remaining = remaining - written as u64;
                self.remaining = Some(remaining);
                Ok(if remaining == 0 { buf.len() } else { written })
            }
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

#[cfg(target_os = "android")]
pub(crate) use imp::{
    create_item, helper, object, take_last_error, text_of, uri_of, Activity, AndroidStorageFile, AndroidStorageFolder,
    AndroidStorageItem, AndroidUri,
};

#[cfg(target_os = "android")]
mod imp {
    use super::{
        date_from_unix_time_milliseconds, is_virtual_document_flags, name_from_last_path_segment, parse_long_column,
        AssetStream, LongColumn, COLUMN_DISPLAY_NAME, COLUMN_DOCUMENT_ID, COLUMN_FLAGS, COLUMN_LAST_MODIFIED,
        COLUMN_MIME_TYPE, COLUMN_SIZE, DEFAULT_MIME_TYPE, FLAG_GRANT_READ_URI_PERMISSION,
        FLAG_GRANT_WRITE_URI_PERMISSION, MEDIA_COLUMN_DISPLAY_NAME, MIME_TYPE_DIR, READ_EXTERNAL_STORAGE,
        STORAGE_HELPER,
    };
    use crate::interop::java::{
        call_int, call_long, call_object, call_static_boolean, call_static_object, new_object, new_string_array,
        string_array_of, string_of, JavaClass, JavaLocal, JavaObject, JavaRef, JavaValue,
    };
    use crate::interop::natives::sdk_int;
    use crate::platform::platform_support::check_permission;
    use crate::platform::storage::android_storage_provider::{all_mime_type, ANDROID_KEY};
    use ferroui_base::input::LocalBoxFuture;
    use ferroui_base::logging::{LogArea, LogEventLevel, Logger};
    use ferroui_base::platform::storage::file_io::StorageBookmarkHelper;
    use ferroui_base::platform::storage::{
        IStorageBookmarkFile, IStorageBookmarkFolder, IStorageBookmarkItem, IStorageFile, IStorageFolder,
        IStorageItem, StorageItemProperties,
    };
    use ferroui_base::reactive::IDisposable;
    use ferroui_base::utilities::{Uri, UriKind};
    use std::any::Any;
    use std::cell::RefCell;
    use std::fmt::Display;
    use std::fs::File;
    use std::io::{self, Read, Write};
    use std::os::fd::FromRawFd;
    use std::rc::{Rc, Weak};

    /// An `android.app.Activity`.
    pub(crate) type Activity = JavaObject;
    /// An `android.net.Uri` (`AndroidUri` of the reference).
    pub(crate) type AndroidUri = JavaObject;

    fn ready<T: 'static>(value: T) -> LocalBoxFuture<T> {
        Box::pin(std::future::ready(value))
    }

    /// The class `StorageHelper` of the Java layer.
    pub(crate) fn helper() -> JavaClass {
        JavaClass::find(STORAGE_HELPER)
    }

    fn documents_contract() -> JavaClass {
        JavaClass::find("android/provider/DocumentsContract")
    }

    /// An object as an argument of a call.
    pub(crate) fn object(value: &dyn JavaRef) -> JavaValue<'_> {
        JavaValue::Object(Some(value))
    }

    fn optional_string(value: Option<&str>) -> JavaValue<'_> {
        match value {
            Some(value) => JavaValue::String(value),
            None => JavaValue::Object(None),
        }
    }

    /// The text of a `java.lang.String` a call returned; none for null.
    pub(crate) fn text_of(value: Option<JavaLocal>) -> Option<String> {
        value.map(|value| string_of(&value))
    }

    fn string_array(values: &[&str]) -> JavaLocal {
        new_string_array(&values.iter().map(|value| (*value).to_string()).collect::<Vec<_>>())
    }

    fn log(level: LogEventLevel, source: Option<&dyn Any>, message_template: &str, value: &dyn Display) {
        if let Some(logger) = Logger::try_get(level, LogArea::ANDROID_PLATFORM) {
            logger.log_with_values(source, message_template, &[value]);
        }
    }

    /// The text of the exception the last call of `StorageHelper` caught;
    /// it is forgotten by the call.
    pub(crate) fn take_last_error() -> Option<String> {
        text_of(call_static_object(&helper(), "takeLastError", "()Ljava/lang/String;", &[]))
    }

    /// The exception the last call of `StorageHelper` caught, as an error.
    fn failure() -> Option<io::Error> {
        take_last_error().map(io::Error::other)
    }

    /// `uri.ToString()`.
    fn uri_to_string(uri: &AndroidUri) -> String {
        text_of(call_object(uri, "toString", "()Ljava/lang/String;", &[])).unwrap_or_default()
    }

    /// `DocumentsContract.GetTreeDocumentId`; the exception for a URI that
    /// is not a tree is caught in Java (`StorageHelper.treeDocumentId`).
    fn get_tree_document_id(uri: &AndroidUri) -> io::Result<String> {
        let id = text_of(call_static_object(
            &helper(),
            "treeDocumentId",
            "(Landroid/net/Uri;)Ljava/lang/String;",
            &[object(uri)],
        ));
        match failure() {
            Some(error) => Err(error),
            None => id.ok_or_else(|| io::Error::other("The tree URI has no document id.")),
        }
    }

    /// `DocumentsContract.GetDocumentId`; the exception for a URI that is
    /// not a document is caught in Java (`StorageHelper.documentId`).
    fn get_document_id(uri: &AndroidUri) -> io::Result<String> {
        let id = text_of(call_static_object(
            &helper(),
            "documentId",
            "(Landroid/net/Uri;)Ljava/lang/String;",
            &[object(uri)],
        ));
        match failure() {
            Some(error) => Err(error),
            None => id.ok_or_else(|| io::Error::other("The document URI has no document id.")),
        }
    }

    /// `DocumentsContract.BuildDocumentUriUsingTree`.
    fn build_document_uri_using_tree(tree_uri: &AndroidUri, document_id: &str) -> Option<AndroidUri> {
        call_static_object(
            &documents_contract(),
            "buildDocumentUriUsingTree",
            "(Landroid/net/Uri;Ljava/lang/String;)Landroid/net/Uri;",
            &[object(tree_uri), JavaValue::String(document_id)],
        )
        .map(|uri| uri.to_global())
    }

    /// `DocumentsContract.BuildChildDocumentsUriUsingTree`.
    fn build_child_documents_uri_using_tree(tree_uri: &AndroidUri, parent_document_id: &str) -> Option<AndroidUri> {
        call_static_object(
            &documents_contract(),
            "buildChildDocumentsUriUsingTree",
            "(Landroid/net/Uri;Ljava/lang/String;)Landroid/net/Uri;",
            &[object(tree_uri), JavaValue::String(parent_document_id)],
        )
        .map(|uri| uri.to_global())
    }

    /// `DocumentsContract.CreateDocument`. The exception the reference lets
    /// reach its caller is caught in Java (`StorageHelper.createDocument`)
    /// and returned as the error.
    fn create_document(
        context: &JavaObject,
        parent_document_uri: &AndroidUri,
        mime_type: &str,
        display_name: &str,
    ) -> io::Result<Option<AndroidUri>> {
        let created = call_static_object(
            &helper(),
            "createDocument",
            "(Landroid/content/Context;Landroid/net/Uri;Ljava/lang/String;Ljava/lang/String;)Landroid/net/Uri;",
            &[
                object(context),
                object(parent_document_uri),
                JavaValue::String(mime_type),
                JavaValue::String(display_name),
            ],
        )
        .map(|uri| uri.to_global());
        match failure() {
            Some(error) => Err(error),
            None => Ok(created),
        }
    }

    /// `DocumentsContract.DeleteDocument`. The exception the reference lets
    /// reach its caller is caught in Java (`StorageHelper.deleteDocument`)
    /// and returned as the error.
    fn delete_document(context: &JavaObject, document_uri: &AndroidUri) -> io::Result<()> {
        let deleted = call_static_boolean(
            &helper(),
            "deleteDocument",
            "(Landroid/content/Context;Landroid/net/Uri;)Z",
            &[object(context), object(document_uri)],
        );
        match failure() {
            Some(error) => Err(error),
            None if deleted => Ok(()),
            None => Err(io::Error::other("The document was not deleted.")),
        }
    }

    /// The rows of the children of a document, as the texts of the columns
    /// of the projection. The exception of the query, which the reference
    /// lets reach its caller, is caught in Java
    /// (`StorageHelper.queryChildren`) and returned as the error.
    fn query_children(context: &JavaObject, children_uri: &AndroidUri, projection: &[&str]) -> io::Result<Vec<String>> {
        let projection = string_array(projection);
        let rows = call_static_object(
            &helper(),
            "queryChildren",
            "(Landroid/content/Context;Landroid/net/Uri;[Ljava/lang/String;)[Ljava/lang/String;",
            &[object(context), object(children_uri), object(&projection)],
        );
        match failure() {
            Some(error) => Err(error),
            None => Ok(rows.map(|rows| string_array_of(&rows)).unwrap_or_default()),
        }
    }

    /// The numbers of columns of the first row of a query
    /// (`StorageHelper.queryLongs`): none without a row, and the text of
    /// the exception when the query threw. With `silent_unsupported` an
    /// `UnsupportedOperationException` of the query is no row.
    fn query_longs(
        context: &JavaObject,
        uri: &AndroidUri,
        columns: &[&str],
        silent_unsupported: bool,
    ) -> Result<Option<Vec<LongColumn>>, String> {
        let columns = string_array(columns);
        let values = call_static_object(
            &helper(),
            "queryLongs",
            "(Landroid/content/Context;Landroid/net/Uri;[Ljava/lang/String;Z)[Ljava/lang/String;",
            &[object(context), object(uri), object(&columns), JavaValue::Boolean(silent_unsupported)],
        );
        if let Some(exception) = take_last_error() {
            return Err(exception);
        }
        Ok(values.map(|values| string_array_of(&values).iter().map(|value| parse_long_column(value)).collect()))
    }

    /// The stream of an `android.content.res.AssetFileDescriptor`: its
    /// file descriptor is taken from the Java object, which does not close
    /// it after that.
    fn stream_of(asset: &JavaLocal) -> io::Result<AssetStream<File>> {
        let start_offset = call_long(asset, "getStartOffset", "()J", &[]);
        let declared_length = call_long(asset, "getDeclaredLength", "()J", &[]);
        let Some(descriptor) =
            call_object(asset, "getParcelFileDescriptor", "()Landroid/os/ParcelFileDescriptor;", &[])
        else {
            return Err(io::Error::other("The asset file descriptor has no file descriptor."));
        };
        let fd = call_int(&descriptor, "detachFd", "()I", &[]);
        if fd < 0 {
            return Err(io::Error::other("The file descriptor of the content is not open."));
        }
        // SAFETY: `ParcelFileDescriptor.detachFd` returns the native descriptor of the
        // Java object and gives its ownership up: the Java object no longer closes it, and
        // nothing else knows the number. It is an open descriptor (not negative, checked
        // above), which the file owns from here and closes once.
        let file = unsafe { File::from_raw_fd(fd) };
        AssetStream::new(file, start_offset, declared_length)
    }

    /// A file or a folder of this backend.
    pub(crate) enum AndroidItem {
        File(Rc<AndroidStorageFile>),
        Folder(Rc<AndroidStorageFolder>),
    }

    impl AndroidItem {
        pub(crate) fn into_storage_item(self) -> Rc<dyn IStorageItem> {
            match self {
                AndroidItem::File(file) => file,
                AndroidItem::Folder(folder) => folder,
            }
        }
    }

    /// What a file and a folder have in common (the abstract base class of
    /// the reference).
    pub(crate) struct AndroidStorageItem {
        activity: RefCell<Option<Activity>>,
        needs_external_files_permission: bool,
        parent: Option<Rc<AndroidStorageFolder>>,
        permission_root: Option<AndroidUri>,
        uri: RefCell<AndroidUri>,
    }

    impl AndroidStorageItem {
        fn new(
            activity: &Activity,
            uri: AndroidUri,
            needs_external_files_permission: bool,
            parent: Option<Rc<AndroidStorageFolder>>,
            permission_root: Option<AndroidUri>,
        ) -> Rc<Self> {
            // `permissionRoot ?? parent?.Uri ?? Uri`, where the reference reads its own URI
            // before the constructor assigns it: the last operand is always null, so an
            // item without a parent and without a root has no permission root.
            let permission_root = permission_root.or_else(|| parent.as_ref().map(|parent| parent.uri()));
            Rc::new(Self {
                activity: RefCell::new(Some(activity.clone())),
                needs_external_files_permission,
                parent,
                permission_root,
                uri: RefCell::new(uri),
            })
        }

        pub(crate) fn uri(&self) -> AndroidUri {
            self.uri.borrow().clone()
        }

        /// The setter of the URI, which nothing of the reference calls either.
        #[allow(dead_code)]
        pub(crate) fn set_uri(&self, value: AndroidUri) {
            *self.uri.borrow_mut() = value;
        }

        /// The activity of the item.
        ///
        /// # Panics
        /// Panics when the item is disposed (the reference throws
        /// `ObjectDisposedException`).
        fn activity(&self) -> Activity {
            self.try_activity().unwrap_or_else(|| panic!("Cannot access a disposed object: AndroidStorageItem."))
        }

        fn try_activity(&self) -> Option<Activity> {
            self.activity.borrow().clone()
        }

        fn name(&self) -> String {
            let activity = self.activity();
            let uri = self.uri();
            Self::get_column_value(&activity, &uri, COLUMN_DISPLAY_NAME, None, None)
                .or_else(|| Self::get_column_value(&activity, &uri, MEDIA_COLUMN_DISPLAY_NAME, None, None))
                .unwrap_or_else(|| {
                    // `Uri.PathSegments?.LastOrDefault()`.
                    let last_path_segment =
                        text_of(call_object(&uri, "getLastPathSegment", "()Ljava/lang/String;", &[]));
                    name_from_last_path_segment(last_path_segment.as_deref())
                })
        }

        /// The URI of the item (`new Uri(Uri.ToString())`).
        ///
        /// # Panics
        /// Panics when the URI is not absolute (the reference throws
        /// `UriFormatException`).
        fn path(&self) -> Uri {
            let text = uri_to_string(&self.uri());
            Uri::try_create(&text, UriKind::Absolute)
                .unwrap_or_else(|| panic!("Invalid URI: The format of the URI could not be determined: {text}"))
        }

        fn can_bookmark(&self) -> bool {
            true
        }

        fn save_bookmark_async(self: &Rc<Self>) -> LocalBoxFuture<Option<String>> {
            let this = self.clone();
            Box::pin(async move {
                if !this.ensure_external_files_permission(false).await {
                    return None;
                }

                // Taking a permission that was not offered as persistable throws; the
                // exception, which the reference lets reach its caller, is caught in Java
                // (`StorageHelper.takePersistableUriPermission`), and no bookmark is made.
                let uri = this.uri();
                let taken = call_static_boolean(
                    &helper(),
                    "takePersistableUriPermission",
                    "(Landroid/content/Context;Landroid/net/Uri;I)Z",
                    &[
                        object(&this.activity()),
                        object(&uri),
                        JavaValue::Int(FLAG_GRANT_WRITE_URI_PERMISSION | FLAG_GRANT_READ_URI_PERMISSION),
                    ],
                );
                if let Some(exception) = take_last_error() {
                    log(
                        LogEventLevel::Error,
                        Some(&*this),
                        "The persistable permission of the URI was not taken: '{Exception}'",
                        &exception,
                    );
                }
                if !taken {
                    return None;
                }

                StorageBookmarkHelper::encode_bookmark(ANDROID_KEY, Some(&uri_to_string(&uri)))
            })
        }

        fn release_bookmark_async(self: &Rc<Self>) -> LocalBoxFuture<()> {
            let this = self.clone();
            Box::pin(async move {
                if !this.ensure_external_files_permission(false).await {
                    return;
                }

                // As above: the exception for a permission that is not held is caught in
                // Java (`StorageHelper.releasePersistableUriPermission`).
                call_static_boolean(
                    &helper(),
                    "releasePersistableUriPermission",
                    "(Landroid/content/Context;Landroid/net/Uri;I)Z",
                    &[
                        object(&this.activity()),
                        object(&this.uri()),
                        JavaValue::Int(FLAG_GRANT_WRITE_URI_PERMISSION | FLAG_GRANT_READ_URI_PERMISSION),
                    ],
                );
                if let Some(exception) = take_last_error() {
                    log(
                        LogEventLevel::Error,
                        Some(&*this),
                        "The persistable permission of the URI was not released: '{Exception}'",
                        &exception,
                    );
                }
            })
        }

        /// The text of a column of the first row the content resolver
        /// answers a query of `content_uri` with. The exception the
        /// reference catches is caught in Java
        /// (`StorageHelper.getColumnValue`) and logged here.
        pub(crate) fn get_column_value(
            context: &JavaObject,
            content_uri: &AndroidUri,
            column: &str,
            selection: Option<&str>,
            selection_args: Option<&[String]>,
        ) -> Option<String> {
            let selection_args = selection_args.map(new_string_array);
            let value = text_of(call_static_object(
                &helper(),
                "getColumnValue",
                "(Landroid/content/Context;Landroid/net/Uri;Ljava/lang/String;Ljava/lang/String;[Ljava/lang/String;)\
                 Ljava/lang/String;",
                &[
                    object(context),
                    object(content_uri),
                    JavaValue::String(column),
                    optional_string(selection),
                    JavaValue::Object(selection_args.as_ref().map(|args| args as &dyn JavaRef)),
                ],
            ));
            if let Some(ex) = take_last_error() {
                log(LogEventLevel::Verbose, None, "File metadata reader failed: '{Exception}'", &ex);
                return None;
            }

            value
        }

        /// `GetParentAsync`, with the folder as its own type.
        async fn get_parent_core(this: Rc<Self>) -> Option<Rc<AndroidStorageFolder>> {
            if !this.ensure_external_files_permission(false).await {
                return None;
            }

            if let Some(parent) = &this.parent {
                return Some(parent.clone());
            }

            // A URI without a path has no parent (the reference passes the null on to the
            // constructor of the file, which throws).
            let path = text_of(call_object(&this.uri(), "getPath", "()Ljava/lang/String;", &[]))?;
            let java_file =
                new_object(&JavaClass::find("java/io/File"), "(Ljava/lang/String;)V", &[JavaValue::String(&path)]);

            // Java file represents files AND directories. Don't be confused.
            let parent_file = call_object(&java_file, "getParentFile", "()Ljava/io/File;", &[])?;
            let android_uri = call_static_object(
                &JavaClass::find("android/net/Uri"),
                "fromFile",
                "(Ljava/io/File;)Landroid/net/Uri;",
                &[object(&parent_file)],
            )?;
            Some(AndroidStorageFolder::new(&this.activity(), android_uri.to_global(), false, None, None))
        }

        fn get_parent_async(self: &Rc<Self>) -> LocalBoxFuture<Option<Rc<dyn IStorageFolder>>> {
            let this = self.clone();
            Box::pin(async move {
                Self::get_parent_core(this).await.map(|parent| parent as Rc<dyn IStorageFolder>)
            })
        }

        fn ensure_external_files_permission(&self, _write: bool) -> LocalBoxFuture<bool> {
            // Starting in API level 33, this permission has no effect.
            if !self.needs_external_files_permission || sdk_int() >= 33 {
                return ready(true);
            }

            check_permission(&self.activity(), READ_EXTERNAL_STORAGE)
        }

        fn dispose(&self) {
            *self.activity.borrow_mut() = None;
        }

        pub(crate) fn permission_root(&self) -> Option<AndroidUri> {
            self.permission_root.clone()
        }
    }

    /// The storage item of a URI: a folder when the provider says the
    /// document is a directory, else a file (`AndroidStorageItem.CreateItem`).
    pub(crate) fn create_item(activity: &Activity, uri: AndroidUri) -> Rc<dyn IStorageItem> {
        let mime_type = AndroidStorageItem::get_column_value(activity, &uri, COLUMN_MIME_TYPE, None, None);
        if mime_type.as_deref() == Some(MIME_TYPE_DIR) {
            AndroidStorageFolder::new(activity, uri, false, None, None)
        } else {
            AndroidStorageFile::new(activity, uri, None, None)
        }
    }

    /// The URI of a storage item of this backend
    /// (`(item as AndroidStorageItem)?.Uri`); none for an item of another
    /// provider. `any` is `IStorageItem::as_any` of the item.
    pub(crate) fn uri_of(any: Option<&dyn Any>) -> Option<AndroidUri> {
        let any = any?;
        if let Some(file) = any.downcast_ref::<AndroidStorageFile>() {
            return Some(file.uri());
        }
        any.downcast_ref::<AndroidStorageFolder>().map(|folder| folder.uri())
    }

    /// The folder of this backend a storage item is
    /// (`item as AndroidStorageFolder`). `any` is `IStorageItem::as_any` of
    /// the item.
    fn folder_of(any: Option<&dyn Any>) -> Option<Rc<AndroidStorageFolder>> {
        any?.downcast_ref::<AndroidStorageFolder>()?.this.upgrade()
    }

    /// The members of a storage item that are those of the base item.
    macro_rules! storage_item {
        ($type:ident, $kind:tt) => {
            impl IDisposable for $type {
                fn dispose(&self) {
                    self.base.dispose();
                }
            }

            impl IStorageItem for $type {
                fn name(&self) -> String {
                    $type::name(self)
                }

                fn path(&self) -> Uri {
                    self.base.path()
                }

                fn get_basic_properties_async(&self) -> LocalBoxFuture<StorageItemProperties> {
                    ready(self.get_basic_properties())
                }

                fn can_bookmark(&self) -> bool {
                    self.base.can_bookmark()
                }

                fn save_bookmark_async(&self) -> LocalBoxFuture<Option<String>> {
                    self.base.save_bookmark_async()
                }

                fn get_parent_async(&self) -> LocalBoxFuture<Option<Rc<dyn IStorageFolder>>> {
                    self.base.get_parent_async()
                }

                fn delete_async(&self) -> LocalBoxFuture<io::Result<()>> {
                    Box::pin($type::delete_core(self.rc()))
                }

                fn move_async(
                    &self,
                    destination: Rc<dyn IStorageFolder>,
                ) -> LocalBoxFuture<io::Result<Option<Rc<dyn IStorageItem>>>> {
                    let this = self.rc();
                    Box::pin(async move {
                        Ok($type::move_core(this, destination).await?.map(|item| item as Rc<dyn IStorageItem>))
                    })
                }

                fn as_storage_file(self: Rc<Self>) -> Option<Rc<dyn IStorageFile>> {
                    storage_item!(@file $kind, self)
                }

                fn as_storage_folder(self: Rc<Self>) -> Option<Rc<dyn IStorageFolder>> {
                    storage_item!(@folder $kind, self)
                }

                fn as_any(&self) -> Option<&dyn Any> {
                    Some(self)
                }
            }

            impl IStorageBookmarkItem for $type {
                fn release_bookmark_async(&self) -> LocalBoxFuture<()> {
                    self.base.release_bookmark_async()
                }
            }

            impl $type {
                /// The item as the shared object it was created as.
                fn rc(&self) -> Rc<Self> {
                    self.this.upgrade().expect("a storage item is called through the reference that keeps it")
                }

                pub(crate) fn uri(&self) -> AndroidUri {
                    self.base.uri()
                }
            }
        };
        (@file file, $this:ident) => { Some($this) };
        (@file folder, $this:ident) => { None };
        (@folder folder, $this:ident) => { Some($this) };
        (@folder file, $this:ident) => { None };
    }

    /// A folder.
    pub(crate) struct AndroidStorageFolder {
        base: Rc<AndroidStorageItem>,
        /// The name of a well-known folder (`WellKnownAndroidStorageFolder`
        /// of the reference, a class derived from the folder that only
        /// replaces the name).
        name: Option<String>,
        this: Weak<AndroidStorageFolder>,
    }

    storage_item!(AndroidStorageFolder, folder);

    impl AndroidStorageFolder {
        pub(crate) fn new(
            activity: &Activity,
            uri: AndroidUri,
            needs_external_files_permission: bool,
            parent: Option<Rc<AndroidStorageFolder>>,
            permission_root: Option<AndroidUri>,
        ) -> Rc<Self> {
            let base = AndroidStorageItem::new(activity, uri, needs_external_files_permission, parent, permission_root);
            Rc::new_cyclic(|this| Self { base, name: None, this: this.clone() })
        }

        /// A folder with a name of its own (the constructor of
        /// `WellKnownAndroidStorageFolder`). As in the reference, nothing
        /// of the backend creates one: the well-known folders are folders
        /// of the file system.
        #[allow(dead_code)]
        pub(crate) fn new_well_known(
            activity: &Activity,
            identifier: &str,
            uri: AndroidUri,
            needs_external_files_permission: bool,
        ) -> Rc<Self> {
            let base = AndroidStorageItem::new(activity, uri, needs_external_files_permission, None, None);
            Rc::new_cyclic(|this| Self { base, name: Some(identifier.to_string()), this: this.clone() })
        }

        fn name(&self) -> String {
            match &self.name {
                Some(name) => name.clone(),
                None => self.base.name(),
            }
        }

        /// `CreateFileAsync`, with the file as its own type.
        pub(crate) async fn create_file_core(
            this: Rc<Self>,
            name: String,
        ) -> io::Result<Option<Rc<AndroidStorageFile>>> {
            // Try to return an existing file to avoid creating file (1).
            let existing_item = Self::get_item_core(this.clone(), name.clone(), false).await?;
            match existing_item {
                Some(AndroidItem::File(existing_file)) => {
                    // The file should be truncated when it is created.
                    drop(existing_file.open_write()?);
                    return Ok(Some(existing_file));
                }
                Some(AndroidItem::Folder(_)) => {
                    // There is an item with the same name but it's not a file. We can't create a file in this case.
                    return Err(io::Error::other(format!(
                        "Can not create '{name}' because a directory with the same name already exists."
                    )));
                }
                None => {}
            }
            // Create new one and return it.
            let tree_uri = this.get_tree_uri()?.1.ok_or_else(|| io::Error::other("The folder has no tree URI."))?;
            let activity = this.base.activity();
            let mime_type = mime_type_from_name(&name).unwrap_or_else(|| DEFAULT_MIME_TYPE.to_string());
            let Some(new_file) = create_document(&activity, &tree_uri, &mime_type, &name)? else {
                return Ok(None);
            };

            Ok(Some(AndroidStorageFile::new(&activity, new_file, Some(this), None)))
        }

        /// `CreateFolderAsync`, with the folder as its own type.
        pub(crate) async fn create_folder_core(
            this: Rc<Self>,
            name: String,
        ) -> io::Result<Option<Rc<AndroidStorageFolder>>> {
            // Try to return an existing folder to avoid creating folder (1).
            let existing_item = Self::get_item_core(this.clone(), name.clone(), true).await?;
            match existing_item {
                Some(AndroidItem::Folder(existing_folder)) => return Ok(Some(existing_folder)),
                Some(AndroidItem::File(_)) => {
                    // There is an item with the same name but it's not a folder. We can't create a folder in this case.
                    return Err(io::Error::other(format!(
                        "Can not create '{name}' because a file with the same name already exists."
                    )));
                }
                None => {}
            }
            // Create new one and return it.
            let tree_uri = this.get_tree_uri()?.1.ok_or_else(|| io::Error::other("The folder has no tree URI."))?;
            let activity = this.base.activity();
            let Some(new_folder) = create_document(&activity, &tree_uri, MIME_TYPE_DIR, &name)? else {
                return Ok(None);
            };

            let permission_root = this.base.permission_root();
            Ok(Some(AndroidStorageFolder::new(&activity, new_folder, false, Some(this), permission_root)))
        }

        async fn delete_core(this: Rc<Self>) -> io::Result<()> {
            if !this.base.ensure_external_files_permission(false).await {
                return Ok(());
            }

            // `Activity != null`: the activity of an item that is not disposed.
            let _ = this.base.activity();
            Self::delete_contents(this.clone(), this).await
        }

        /// The local function `DeleteContents` of `DeleteAsync`: `owner` is
        /// the folder `DeleteAsync` was called on. As in the reference, the
        /// document that is deleted after the items of `storage_folder` is
        /// the one of the tree URI of `owner`, also when `storage_folder`
        /// is a folder below it.
        fn delete_contents(owner: Rc<Self>, storage_folder: Rc<Self>) -> LocalBoxFuture<io::Result<()>> {
            Box::pin(async move {
                for file in Self::get_items_core(storage_folder).await? {
                    match file {
                        AndroidItem::Folder(folder) => Self::delete_contents(owner.clone(), folder).await?,
                        AndroidItem::File(storage_file) => AndroidStorageFile::delete_core(storage_file).await?,
                    }
                }

                let tree_uri = owner.get_tree_uri()?.1.ok_or_else(|| io::Error::other("The folder has no tree URI."))?;
                delete_document(&owner.base.activity(), &tree_uri)
            })
        }

        fn get_basic_properties(&self) -> StorageItemProperties {
            let mut date_modified = None;

            // The reference catches every exception of what follows, the one of a disposed
            // item too.
            let unavailable = |exception: &dyn Display| {
                // Data may not be available for this item or the URI may not be in the expected shape.
                log(
                    LogEventLevel::Verbose,
                    Some(self),
                    "Directory basic properties metadata unavailable: '{Exception}'",
                    exception,
                );
            };
            let Some(activity) = self.base.try_activity() else {
                unavailable(&"Cannot access a disposed object: AndroidStorageItem.");
                return StorageItemProperties::new(None, None, date_modified);
            };

            // When Uri is a tree URI, use its document id to build a document URI.
            // For non-root items, Uri may already be a document URI; use it directly.
            // (`StorageHelper.folderQueryUri` has the two try blocks of the reference.)
            let query_uri = call_static_object(
                &helper(),
                "folderQueryUri",
                "(Landroid/net/Uri;)Landroid/net/Uri;",
                &[object(&self.base.uri())],
            );
            if let Some(exception) = take_last_error() {
                unavailable(&exception);
                return StorageItemProperties::new(None, None, date_modified);
            }

            if let Some(query_uri) = query_uri {
                let query_uri = query_uri.to_global();
                match query_longs(&activity, &query_uri, &[COLUMN_LAST_MODIFIED], false) {
                    Ok(Some(values)) => match values.first() {
                        Some(LongColumn::Value(long_value)) => {
                            date_modified = date_from_unix_time_milliseconds(*long_value);
                        }
                        Some(LongColumn::Failed(exception)) => log(
                            LogEventLevel::Verbose,
                            Some(self),
                            "Directory LastModified metadata reader failed: '{Exception}'",
                            exception,
                        ),
                        Some(LongColumn::Missing) | None => {}
                    },
                    Ok(None) => {}
                    Err(exception) => unavailable(&exception),
                }
            }

            StorageItemProperties::new(None, None, date_modified)
        }

        /// `GetItemsAsync`, with the items as their own types. The
        /// reference enumerates the items as they are read; the contract of
        /// the port is a list.
        pub(crate) async fn get_items_core(this: Rc<Self>) -> io::Result<Vec<AndroidItem>> {
            let mut items = Vec::new();
            if !this.base.ensure_external_files_permission(false).await {
                return Ok(items);
            }

            // An activity always has a content resolver.
            let activity = this.base.activity();

            let (root, children_uri) = this.get_tree_uri()?;

            let projection = [COLUMN_DOCUMENT_ID, COLUMN_MIME_TYPE];
            if let Some(children_uri) = children_uri {
                let rows = query_children(&activity, &children_uri, &projection)?;
                for row in rows.chunks_exact(projection.len()) {
                    let mime = &row[1];
                    let id = &row[0];

                    let is_directory = mime == MIME_TYPE_DIR;
                    let Some(uri) = build_document_uri_using_tree(&root, id) else {
                        continue;
                    };
                    items.push(if is_directory {
                        AndroidItem::Folder(AndroidStorageFolder::new(
                            &activity,
                            uri,
                            false,
                            Some(this.clone()),
                            Some(root.clone()),
                        ))
                    } else {
                        AndroidItem::File(AndroidStorageFile::new(
                            &activity,
                            uri,
                            Some(this.clone()),
                            Some(root.clone()),
                        ))
                    });
                }
            }

            Ok(items)
        }

        /// `MoveAsync`, with the folder as its own type.
        async fn move_core(
            this: Rc<Self>,
            destination: Rc<dyn IStorageFolder>,
        ) -> io::Result<Option<Rc<AndroidStorageFolder>>> {
            // `Activity != null`: the activity of an item that is not disposed.
            let _ = this.base.activity();
            // `(AndroidStorageFolder)destination`: the reference throws `InvalidCastException`.
            let Some(destination) = folder_of(destination.as_any()) else {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "The destination of a folder of a document provider must be a folder of a document provider.",
                ));
            };
            Self::move_recursively(this, destination).await
        }

        fn move_recursively(
            storage_folder: Rc<Self>,
            destination: Rc<Self>,
        ) -> LocalBoxFuture<io::Result<Option<Rc<AndroidStorageFolder>>>> {
            Box::pin(async move {
                let Some(new_destination) = Self::create_folder_core(destination, storage_folder.name()).await? else {
                    return Ok(None);
                };

                let destination = new_destination;

                for file in Self::get_items_core(storage_folder.clone()).await? {
                    match file {
                        AndroidItem::Folder(folder) => {
                            Self::move_recursively(folder, destination.clone()).await?;
                        }
                        AndroidItem::File(file) => {
                            AndroidStorageFile::move_to_folder(file, Some(destination.clone())).await?;
                        }
                    }
                }

                Self::delete_core(storage_folder).await?;

                Ok(Some(destination))
            })
        }

        async fn get_item_core(this: Rc<Self>, name: String, is_directory: bool) -> io::Result<Option<AndroidItem>> {
            if !this.base.ensure_external_files_permission(false).await {
                return Ok(None);
            }

            // An activity always has a content resolver.
            let activity = this.base.activity();

            let (root, children_uri) = this.get_tree_uri()?;

            let projection = [COLUMN_DOCUMENT_ID, COLUMN_MIME_TYPE, COLUMN_DISPLAY_NAME];

            if let Some(children_uri) = children_uri {
                let rows = query_children(&activity, &children_uri, &projection)?;
                for row in rows.chunks_exact(projection.len()) {
                    let id = &row[0];
                    let mime = &row[1];

                    let file_name = &row[2];
                    if *file_name != name {
                        continue;
                    }

                    let mine_directory = mime == MIME_TYPE_DIR;
                    if is_directory != mine_directory {
                        return Ok(None);
                    }

                    let Some(uri) = build_document_uri_using_tree(&root, id) else {
                        return Ok(None);
                    };

                    return Ok(Some(if is_directory {
                        AndroidItem::Folder(AndroidStorageFolder::new(&activity, uri, false, Some(this), Some(root)))
                    } else {
                        AndroidItem::File(AndroidStorageFile::new(&activity, uri, Some(this), Some(root)))
                    }));
                }
            }

            Ok(None)
        }

        fn get_tree_uri(&self) -> io::Result<(AndroidUri, Option<AndroidUri>)> {
            let uri = self.base.uri();
            let root = self.base.permission_root().unwrap_or_else(|| uri.clone());
            // `root != Uri` compares the objects.
            let folder_id =
                if !root.is_same_object(&uri) { get_document_id(&uri)? } else { get_tree_document_id(&uri)? };
            let tree_uri = build_child_documents_uri_using_tree(&root, &folder_id);
            Ok((root, tree_uri))
        }
    }

    /// The MIME type the system knows for the extension of a file name
    /// (`MimeTypeMap.Singleton?.GetMimeTypeFromExtension(MimeTypeMap.GetFileExtensionFromUrl(name))`).
    fn mime_type_from_name(name: &str) -> Option<String> {
        let mime_type_map = JavaClass::find("android/webkit/MimeTypeMap");
        let extension = text_of(call_static_object(
            &mime_type_map,
            "getFileExtensionFromUrl",
            "(Ljava/lang/String;)Ljava/lang/String;",
            &[JavaValue::String(name)],
        ));
        let singleton = call_static_object(&mime_type_map, "getSingleton", "()Landroid/webkit/MimeTypeMap;", &[])?;
        text_of(call_object(
            &singleton,
            "getMimeTypeFromExtension",
            "(Ljava/lang/String;)Ljava/lang/String;",
            &[optional_string(extension.as_deref())],
        ))
    }

    impl IStorageFolder for AndroidStorageFolder {
        fn get_items_async(&self) -> LocalBoxFuture<io::Result<Vec<Rc<dyn IStorageItem>>>> {
            let this = self.rc();
            Box::pin(async move {
                Ok(Self::get_items_core(this).await?.into_iter().map(AndroidItem::into_storage_item).collect())
            })
        }

        fn get_folder_async(&self, name: &str) -> LocalBoxFuture<Option<Rc<dyn IStorageFolder>>> {
            let this = self.rc();
            let name = name.to_string();
            Box::pin(async move {
                match Self::get_item_core(this.clone(), name, true).await {
                    Ok(Some(AndroidItem::Folder(folder))) => Some(folder as Rc<dyn IStorageFolder>),
                    Ok(_) => None,
                    Err(error) => {
                        // The reference lets the exception reach its caller; the member of
                        // the port has no error to return.
                        log(LogEventLevel::Error, Some(&*this), "The folder was not read: '{Exception}'", &error);
                        None
                    }
                }
            })
        }

        fn get_file_async(&self, name: &str) -> LocalBoxFuture<Option<Rc<dyn IStorageFile>>> {
            let this = self.rc();
            let name = name.to_string();
            Box::pin(async move {
                match Self::get_item_core(this.clone(), name, false).await {
                    Ok(Some(AndroidItem::File(file))) => Some(file as Rc<dyn IStorageFile>),
                    Ok(_) => None,
                    Err(error) => {
                        // As above.
                        log(LogEventLevel::Error, Some(&*this), "The folder was not read: '{Exception}'", &error);
                        None
                    }
                }
            })
        }

        fn create_file_async(&self, name: &str) -> LocalBoxFuture<io::Result<Option<Rc<dyn IStorageFile>>>> {
            let this = self.rc();
            let name = name.to_string();
            Box::pin(async move {
                Ok(Self::create_file_core(this, name).await?.map(|file| file as Rc<dyn IStorageFile>))
            })
        }

        fn create_folder_async(&self, name: &str) -> LocalBoxFuture<io::Result<Option<Rc<dyn IStorageFolder>>>> {
            let this = self.rc();
            let name = name.to_string();
            Box::pin(async move {
                Ok(Self::create_folder_core(this, name).await?.map(|folder| folder as Rc<dyn IStorageFolder>))
            })
        }
    }

    impl IStorageBookmarkFolder for AndroidStorageFolder {}

    /// A file.
    pub(crate) struct AndroidStorageFile {
        base: Rc<AndroidStorageItem>,
        this: Weak<AndroidStorageFile>,
    }

    storage_item!(AndroidStorageFile, file);

    impl AndroidStorageFile {
        pub(crate) fn new(
            activity: &Activity,
            uri: AndroidUri,
            parent: Option<Rc<AndroidStorageFolder>>,
            permission_root: Option<AndroidUri>,
        ) -> Rc<Self> {
            let base = AndroidStorageItem::new(activity, uri, false, parent, permission_root);
            Rc::new_cyclic(|this| Self { base, this: this.clone() })
        }

        fn name(&self) -> String {
            self.base.name()
        }

        pub(crate) fn open_read(&self) -> io::Result<AssetStream<File>> {
            self.open_content_stream(&self.base.activity(), &self.base.uri(), false)?
                .ok_or_else(|| io::Error::other("Failed to open content stream"))
        }

        /// The body of `OpenWriteAsync`.
        pub(crate) fn open_write(&self) -> io::Result<AssetStream<File>> {
            self.open_content_stream(&self.base.activity(), &self.base.uri(), true)?
                .ok_or_else(|| io::Error::other("Failed to open content stream"))
        }

        /// The stream of the content of a URI. The reference returns the
        /// stream of the content resolver (`OpenInputStream`,
        /// `OpenOutputStream` with the mode `wt`), which is the stream of
        /// the asset file descriptor the resolver opens; the port opens
        /// that descriptor (`StorageHelper.openAssetFileDescriptor`, where
        /// the exception the reference lets reach its caller is caught and
        /// returned as the error) and reads and writes it itself.
        fn open_content_stream(
            &self,
            context: &JavaObject,
            uri: &AndroidUri,
            is_output: bool,
        ) -> io::Result<Option<AssetStream<File>>> {
            let is_virtual = self.is_virtual_file(context, uri);
            if is_virtual {
                log(LogEventLevel::Verbose, Some(self), "Content URI was virtual: '{Uri}'", &uri_to_string(uri));
                return Self::get_virtual_file_stream(context, uri, is_output);
            }

            let asset = call_static_object(
                &helper(),
                "openAssetFileDescriptor",
                "(Landroid/content/Context;Landroid/net/Uri;Ljava/lang/String;)\
                 Landroid/content/res/AssetFileDescriptor;",
                &[object(context), object(uri), JavaValue::String(if is_output { "wt" } else { "r" })],
            );
            if let Some(error) = failure() {
                return Err(error);
            }
            asset.map(|asset| stream_of(&asset)).transpose()
        }

        fn is_virtual_file(&self, context: &JavaObject, uri: &AndroidUri) -> bool {
            if sdk_int() < 24 {
                return false;
            }

            if !call_static_boolean(
                &documents_contract(),
                "isDocumentUri",
                "(Landroid/content/Context;Landroid/net/Uri;)Z",
                &[object(context), object(uri)],
            ) {
                return false;
            }

            let value = AndroidStorageItem::get_column_value(context, uri, COLUMN_FLAGS, None, None);
            is_virtual_document_flags(value.as_deref())
        }

        /// The stream of a virtual file, as the first type it can be read
        /// as. The exceptions of the two calls, which the reference lets
        /// reach its caller, are caught in Java
        /// (`StorageHelper.getStreamTypes` and
        /// `openTypedAssetFileDescriptor`) and returned as the error.
        fn get_virtual_file_stream(
            context: &JavaObject,
            uri: &AndroidUri,
            _is_output: bool,
        ) -> io::Result<Option<AssetStream<File>>> {
            let mime_types = call_static_object(
                &helper(),
                "getStreamTypes",
                "(Landroid/content/Context;Landroid/net/Uri;Ljava/lang/String;)[Ljava/lang/String;",
                &[object(context), object(uri), JavaValue::String(&all_mime_type())],
            );
            if let Some(error) = failure() {
                return Err(error);
            }
            let mime_types = mime_types.map(|mime_types| string_array_of(&mime_types)).unwrap_or_default();
            if let Some(mime_type) = mime_types.first() {
                let asset = call_static_object(
                    &helper(),
                    "openTypedAssetFileDescriptor",
                    "(Landroid/content/Context;Landroid/net/Uri;Ljava/lang/String;)\
                     Landroid/content/res/AssetFileDescriptor;",
                    &[object(context), object(uri), JavaValue::String(mime_type)],
                );
                if let Some(error) = failure() {
                    return Err(error);
                }

                // `asset?.CreateOutputStream()` or `asset?.CreateInputStream()`: both are
                // the stream of the descriptor of the asset, in its range.
                return asset.map(|asset| stream_of(&asset)).transpose();
            }

            Ok(None)
        }

        fn get_basic_properties(&self) -> StorageItemProperties {
            let mut size = None;
            let item_date = None;
            let mut date_modified = None;

            let activity = self.base.activity();
            let uri = self.base.uri();
            match query_longs(&activity, &uri, &[COLUMN_SIZE, COLUMN_LAST_MODIFIED], true) {
                Ok(Some(values)) => {
                    match values.first() {
                        Some(LongColumn::Value(value)) => size = Some(*value as u64),
                        Some(LongColumn::Failed(exception)) => log(
                            LogEventLevel::Verbose,
                            Some(self),
                            "File Size metadata reader failed: '{Exception}'",
                            exception,
                        ),
                        Some(LongColumn::Missing) | None => {}
                    }

                    match values.get(1) {
                        Some(LongColumn::Value(long_value)) => {
                            date_modified = date_from_unix_time_milliseconds(*long_value);
                        }
                        Some(LongColumn::Failed(exception)) => log(
                            LogEventLevel::Verbose,
                            Some(self),
                            "File LastModified metadata reader failed: '{Exception}'",
                            exception,
                        ),
                        Some(LongColumn::Missing) | None => {}
                    }
                }
                // No row, or an `UnsupportedOperationException`:
                // It's not possible to get parameters of some files/folders.
                Ok(None) => {}
                Err(exception) => {
                    // The reference lets another exception of the query reach its caller;
                    // the member of the port has no error to return.
                    log(
                        LogEventLevel::Error,
                        Some(self),
                        "File basic properties metadata unavailable: '{Exception}'",
                        &exception,
                    );
                }
            }

            StorageItemProperties::new(size, item_date, date_modified)
        }

        pub(crate) async fn delete_core(this: Rc<Self>) -> io::Result<()> {
            if !this.base.ensure_external_files_permission(false).await {
                return Ok(());
            }

            delete_document(&this.base.activity(), &this.base.uri())
        }

        /// `MoveAsync` of the trait: `destination is AndroidStorageFolder`.
        async fn move_core(
            this: Rc<Self>,
            destination: Rc<dyn IStorageFolder>,
        ) -> io::Result<Option<Rc<AndroidStorageFile>>> {
            Self::move_to_folder(this, folder_of(destination.as_any())).await
        }

        /// `MoveAsync`, with the file as its own type; `destination` is
        /// none for a folder of another provider.
        pub(crate) async fn move_to_folder(
            this: Rc<Self>,
            destination: Option<Rc<AndroidStorageFolder>>,
        ) -> io::Result<Option<Rc<AndroidStorageFile>>> {
            if !this.base.ensure_external_files_permission(false).await {
                return Ok(None);
            }

            // `Activity != null`: the activity of an item that is not disposed.
            let activity = this.base.activity();
            let Some(storage_folder) = destination else {
                return Ok(None);
            };

            let mut moved_uri = None;

            if sdk_int() >= 24 {
                let target_parent_uri = storage_folder.uri();
                if let Some(parent_folder) = AndroidStorageItem::get_parent_core(this.base.clone()).await {
                    moved_uri = call_static_object(
                        &helper(),
                        "moveDocument",
                        "(Landroid/content/Context;Landroid/net/Uri;Landroid/net/Uri;Landroid/net/Uri;)\
                         Landroid/net/Uri;",
                        &[
                            object(&activity),
                            object(&this.base.uri()),
                            object(&parent_folder.uri()),
                            object(&target_parent_uri),
                        ],
                    )
                    .map(|uri| uri.to_global());
                    // There are many reason why DocumentContract will fail to move a file. We fallback to
                    // copying below.
                    // (The exception is caught in Java, `StorageHelper.moveDocument`.)
                    let _ = take_last_error();
                }
            }

            if let Some(moved_uri) = moved_uri {
                return Ok(Some(AndroidStorageFile::new(&this.base.activity(), moved_uri, Some(storage_folder), None)));
            }

            Self::move_file_by_copy(this, storage_folder).await
        }

        /// The local function `MoveFileByCopy` of `MoveAsync`.
        async fn move_file_by_copy(
            this: Rc<Self>,
            storage_folder: Rc<AndroidStorageFolder>,
        ) -> io::Result<Option<Rc<AndroidStorageFile>>> {
            let new_file = AndroidStorageFolder::create_file_core(storage_folder.clone(), this.name()).await?;

            let copied: io::Result<Option<Rc<AndroidStorageFile>>> = async {
                if let Some(new_file) = &new_file {
                    {
                        let mut input = this.open_read()?;
                        let mut output = new_file.open_write()?;

                        io::copy(&mut input, &mut output)?;
                        output.flush()?;
                    }

                    Self::delete_core(this.clone()).await?;

                    return Ok(Some(AndroidStorageFile::new(
                        &this.base.activity(),
                        new_file.uri(),
                        Some(storage_folder.clone()),
                        None,
                    )));
                }
                Ok(None)
            }
            .await;

            match copied {
                Ok(Some(moved)) => return Ok(Some(moved)),
                Ok(None) => {}
                Err(_) => {
                    // The reference starts the deletion and does not wait for it; it has
                    // nothing to wait for, so the port deletes here.
                    if let Some(new_file) = new_file {
                        let _ = Self::delete_core(new_file).await;
                    }
                }
            }

            Ok(None)
        }
    }

    impl IStorageFile for AndroidStorageFile {
        fn open_read_async(&self) -> LocalBoxFuture<io::Result<Box<dyn Read>>> {
            ready(self.open_read().map(|stream| Box::new(stream) as Box<dyn Read>))
        }

        fn open_write_async(&self) -> LocalBoxFuture<io::Result<Box<dyn Write>>> {
            ready(self.open_write().map(|stream| Box::new(stream) as Box<dyn Write>))
        }
    }

    impl IStorageBookmarkFile for AndroidStorageFile {}
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests of the Android backend that run without a
    // device.
    use super::*;
    use std::io::Cursor;

    /// `AssetFileDescriptor.UNKNOWN_LENGTH`.
    const UNKNOWN_LENGTH: i64 = -1;

    #[test]
    fn the_sdk_constants_have_the_values_of_the_platform() {
        assert_eq!(512, FLAG_VIRTUAL_DOCUMENT);
        assert_eq!(1, FLAG_GRANT_READ_URI_PERMISSION);
        assert_eq!(2, FLAG_GRANT_WRITE_URI_PERMISSION);
        assert_eq!(COLUMN_DISPLAY_NAME, MEDIA_COLUMN_DISPLAY_NAME);
    }

    #[test]
    fn the_name_is_the_last_part_of_the_last_path_segment() {
        assert_eq!("report.pdf", name_from_last_path_segment(Some("primary:Download/report.pdf")));
        assert_eq!("primary:Download", name_from_last_path_segment(Some("primary:Download/")));
        assert_eq!("primary:Download", name_from_last_path_segment(Some("primary:Download")));
        assert_eq!("", name_from_last_path_segment(Some("/")));
        assert_eq!("", name_from_last_path_segment(Some("")));
        assert_eq!("", name_from_last_path_segment(None));
    }

    #[test]
    fn the_flags_of_a_virtual_document_are_recognised() {
        assert!(is_virtual_document_flags(Some("512")));
        assert!(is_virtual_document_flags(Some("518")));
        assert!(is_virtual_document_flags(Some(" 512 ")));
        assert!(!is_virtual_document_flags(Some("6")));
        assert!(!is_virtual_document_flags(Some("")));
        assert!(!is_virtual_document_flags(Some("virtual")));
        assert!(!is_virtual_document_flags(None));
    }

    #[test]
    fn a_number_of_milliseconds_is_a_date_when_it_is_positive() {
        assert_eq!(None, date_from_unix_time_milliseconds(0));
        assert_eq!(None, date_from_unix_time_milliseconds(-1));
        assert_eq!(
            Some(DateTimeOffset::UNIX_EPOCH.add(TimeSpan::from_ticks(10_000))),
            date_from_unix_time_milliseconds(1)
        );
        assert_eq!(
            Some(DateTimeOffset::UNIX_EPOCH.add(TimeSpan::from_ticks(864_000_000_000))),
            date_from_unix_time_milliseconds(86_400_000)
        );
        assert!(date_from_unix_time_milliseconds(MAX_UNIX_TIME_MILLISECONDS).is_some());
        assert_eq!(None, date_from_unix_time_milliseconds(MAX_UNIX_TIME_MILLISECONDS + 1));
        assert_eq!(None, date_from_unix_time_milliseconds(i64::MAX));
    }

    #[test]
    fn a_column_is_a_number_missing_or_failed() {
        assert_eq!(LongColumn::Value(1024), parse_long_column("1024"));
        assert_eq!(LongColumn::Value(-1), parse_long_column("-1"));
        assert_eq!(LongColumn::Missing, parse_long_column(""));
        assert_eq!(
            LongColumn::Failed("java.lang.IllegalStateException: closed".to_string()),
            parse_long_column("!java.lang.IllegalStateException: closed")
        );
        assert!(matches!(parse_long_column("ten"), LongColumn::Failed(_)));
    }

    #[test]
    fn an_asset_without_a_length_is_read_whole_and_not_positioned() {
        let mut stream = AssetStream::new(Cursor::new(b"0123456789".to_vec()), 4, UNKNOWN_LENGTH).unwrap();
        let mut text = String::new();
        stream.read_to_string(&mut text).unwrap();
        assert_eq!("0123456789", text);
    }

    #[test]
    fn an_asset_with_a_length_is_read_in_its_range() {
        let mut stream = AssetStream::new(Cursor::new(b"0123456789".to_vec()), 2, 5).unwrap();
        let mut text = String::new();
        stream.read_to_string(&mut text).unwrap();
        assert_eq!("23456", text);

        let mut empty = AssetStream::new(Cursor::new(b"0123456789".to_vec()), 2, 0).unwrap();
        let mut text = String::new();
        empty.read_to_string(&mut text).unwrap();
        assert_eq!("", text);
    }

    #[test]
    fn an_asset_with_a_length_is_written_in_its_range() {
        let mut stream = AssetStream::new(Cursor::new(b"0123456789".to_vec()), 2, 3).unwrap();
        // What is past the range is dropped without an error.
        stream.write_all(b"abcdef").unwrap();
        stream.write_all(b"gh").unwrap();
        stream.flush().unwrap();
        assert_eq!(b"01abc56789".to_vec(), stream.inner.into_inner());
    }

    #[test]
    fn an_asset_without_a_length_is_written_whole() {
        let mut stream = AssetStream::new(Cursor::new(Vec::new()), 0, UNKNOWN_LENGTH).unwrap();
        stream.write_all(b"abcdef").unwrap();
        assert_eq!(b"abcdef".to_vec(), stream.inner.into_inner());
    }
}
