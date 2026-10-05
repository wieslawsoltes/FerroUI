use super::blob_readable_stream::BlobReadableStream;
use super::writeable_stream::{settle_pending_closes, WriteableStream};
use crate::browser_app_builder::BrowserPlatformOptions;
use crate::interop::promise_helper::JsError;
use crate::interop::storage_helper::{self, StorageItemProperties};
use crate::interop::{ferro_module, non_null, JsObject};
use ferroui_base::animation::TimeSpan;
use ferroui_base::input::LocalBoxFuture;
use ferroui_base::platform::storage::file_io::{DecodeResult, StorageBookmarkHelper, StorageProviderHelpers};
use ferroui_base::platform::storage::{
    FilePickerFileType, FilePickerFileTypes, FilePickerOpenOptions, FilePickerSaveOptions, FolderPickerOpenOptions,
    IStorageBookmarkFile, IStorageBookmarkFolder, IStorageBookmarkItem, IStorageFile, IStorageFolder, IStorageItem,
    IStorageProvider, OpenFilePickerResult, SaveFilePickerResult, StorageItemProperties as ItemProperties,
    WellKnownFolder,
};
use ferroui_base::reactive::IDisposable;
use ferroui_base::utilities::{DateTimeOffset, Uri, UriKind};
use ferroui_base::{FerroLocator, LocatorExtensions};
use std::any::Any;
use std::cell::RefCell;
use std::io::{self, Read, Write};
use std::rc::{Rc, Weak};
use wasm_bindgen::JsCast;

/// The platform key of the bookmarks of the browser.
pub(crate) const BROWSER_BOOKMARK_KEY: &[u8] = b"browser";
pub(crate) const PICKER_CANCEL_MESSAGE: &str = "The user aborted a request";
pub(crate) const NO_PERMISSIONS_MESSAGE: &str = "Permissions denied";
pub(crate) const FILE_FOLDER_NOT_FOUND_MESSAGE: &str = "A requested file or directory could not be found";
pub(crate) const TYPE_MISSMATCH_MESSAGE: &str = "The path supplied exists, but was not an entry of requested type";

/// The storage provider of the browser: file and folder pickers of the File
/// System Access API (or of the `native-file-system-adapter` polyfill where
/// the browser has none) and bookmarks kept in IndexedDB.
///
/// Every operation first imports the storage bundle of the script side.
#[derive(Default)]
pub struct BrowserStorageProvider;

fn ready<T: 'static>(value: T) -> LocalBoxFuture<T> {
    Box::pin(std::future::ready(value))
}

fn is_cancel(error: &JsError) -> bool {
    error.message().contains(PICKER_CANCEL_MESSAGE)
}

fn denied(error: JsError) -> io::Error {
    if error.message() == NO_PERMISSIONS_MESSAGE {
        io::Error::new(io::ErrorKind::PermissionDenied, "User denied permissions to open the file")
    } else {
        error.into()
    }
}

impl BrowserStorageProvider {
    pub fn new() -> Self {
        Self
    }

    fn prefer_polyfill() -> bool {
        FerroLocator::current()
            .get_service::<BrowserPlatformOptions>()
            .is_some_and(|options| options.prefer_file_dialog_polyfill)
    }

    /// The script object of a start location the provider created itself.
    fn start_in(location: Option<&Rc<dyn IStorageFolder>>) -> Option<JsObject> {
        location.and_then(|location| JsStorageItem::of(&**location)).and_then(JsStorageItem::file_handle)
    }

    fn convert_file_types(input: Option<&[Rc<FilePickerFileType>]>) -> (Option<Vec<JsObject>>, bool) {
        let all = FilePickerFileTypes::all();
        let types: Option<Vec<JsObject>> = input.map(|input| {
            input
                .iter()
                .filter(|t| t.mime_types().is_some_and(|m| !m.is_empty()) && !Rc::ptr_eq(t, &all))
                .map(|t| {
                    storage_helper::create_accept_type(
                        t.name(),
                        t.mime_types().map(|m| m.to_vec()).unwrap_or_default(),
                        t.try_get_extensions().map(|e| e.iter().map(|e| format!(".{e}")).collect()),
                    )
                })
                .collect()
        });
        let types = types.filter(|types| !types.is_empty());

        let include_all = input.is_some_and(|input| input.iter().any(|t| Rc::ptr_eq(t, &all))) || types.is_none();

        (types, !include_all)
    }

    async fn open_file_picker(options: FilePickerOpenOptions) -> io::Result<Vec<Rc<dyn IStorageFile>>> {
        ferro_module::import_storage().await?;
        let start_in = Self::start_in(options.suggested_start_location());

        let (types, exclude_all) = Self::convert_file_types(options.file_type_filter());

        let items = match storage_helper::open_file_dialog(
            start_in.as_ref(),
            options.allow_multiple(),
            types,
            exclude_all,
            Self::prefer_polyfill(),
        )
        .await
        {
            Ok(items) => items,
            Err(error) if is_cancel(&error) => return Ok(Vec::new()),
            Err(error) => return Err(error.into()),
        };
        let Some(items) = non_null(items) else {
            return Ok(Vec::new());
        };

        let items_array = storage_helper::items_array(&items);
        Ok(items_array.into_iter().map(|item| JsStorageFile::new(item) as Rc<dyn IStorageFile>).collect())
    }

    async fn save_file_picker(options: FilePickerSaveOptions) -> io::Result<Option<Rc<dyn IStorageFile>>> {
        ferro_module::import_storage().await?;
        let start_in = Self::start_in(options.suggested_start_location());

        let (types, exclude_all) = Self::convert_file_types(options.file_type_choices());

        let suggested_name = StorageProviderHelpers::name_with_extension(
            options.suggested_file_name(),
            options.default_extension(),
            None,
        );
        match storage_helper::save_file_dialog(
            start_in.as_ref(),
            suggested_name,
            types,
            exclude_all,
            Self::prefer_polyfill(),
        )
        .await
        {
            Ok(item) => Ok(non_null(item).map(|item| JsStorageFile::new(item) as Rc<dyn IStorageFile>)),
            Err(error) if is_cancel(&error) => Ok(None),
            Err(error) => Err(error.into()),
        }
    }

    async fn open_folder_picker(options: FolderPickerOpenOptions) -> io::Result<Vec<Rc<dyn IStorageFolder>>> {
        ferro_module::import_storage().await?;
        let start_in = Self::start_in(options.suggested_start_location());

        match storage_helper::select_folder_dialog(start_in.as_ref(), Self::prefer_polyfill()).await {
            Ok(item) => Ok(non_null(item)
                .map(|item| vec![JsStorageFolder::new(item) as Rc<dyn IStorageFolder>])
                .unwrap_or_default()),
            Err(error) if is_cancel(&error) => Ok(Vec::new()),
            Err(error) => Err(error.into()),
        }
    }

    async fn decode_bookmark(bookmark: String) -> Option<JsStorageItemKind> {
        ferro_module::import_storage().await.ok()?;
        let item = match StorageBookmarkHelper::try_decode_bookmark(BROWSER_BOOKMARK_KEY, Some(&bookmark)) {
            (DecodeResult::Success, Some(bytes)) => {
                storage_helper::open_bookmark(&String::from_utf8_lossy(&bytes)).await.ok()
            }
            // Attempt to decode 11.0 browser bookmarks
            (DecodeResult::InvalidFormat, _) => storage_helper::open_bookmark(&bookmark).await.ok(),
            _ => None,
        };
        JsStorageItemKind::from_item(non_null(item?)?)
    }

    fn well_known_directory_name(well_known_folder: WellKnownFolder) -> &'static str {
        match well_known_folder {
            WellKnownFolder::Desktop => "desktop",
            WellKnownFolder::Documents => "documents",
            WellKnownFolder::Downloads => "downloads",
            WellKnownFolder::Music => "music",
            WellKnownFolder::Pictures => "pictures",
            WellKnownFolder::Videos => "videos",
        }
    }
}

impl IStorageProvider for BrowserStorageProvider {
    fn can_open(&self) -> bool {
        true
    }

    fn open_file_picker_async(
        &self,
        options: FilePickerOpenOptions,
    ) -> LocalBoxFuture<io::Result<Vec<Rc<dyn IStorageFile>>>> {
        Box::pin(Self::open_file_picker(options))
    }

    fn open_file_picker_with_result_async(
        &self,
        options: FilePickerOpenOptions,
    ) -> LocalBoxFuture<io::Result<OpenFilePickerResult>> {
        let files = self.open_file_picker_async(options);
        Box::pin(async move { Ok(OpenFilePickerResult { files: files.await?, selected_file_type: None }) })
    }

    fn can_save(&self) -> bool {
        true
    }

    fn save_file_picker_async(
        &self,
        options: FilePickerSaveOptions,
    ) -> LocalBoxFuture<io::Result<Option<Rc<dyn IStorageFile>>>> {
        Box::pin(Self::save_file_picker(options))
    }

    fn save_file_picker_with_result_async(
        &self,
        options: FilePickerSaveOptions,
    ) -> LocalBoxFuture<io::Result<SaveFilePickerResult>> {
        let file = self.save_file_picker_async(options);
        Box::pin(async move { Ok(SaveFilePickerResult { file: file.await?, selected_file_type: None }) })
    }

    fn can_pick_folder(&self) -> bool {
        true
    }

    fn open_folder_picker_async(
        &self,
        options: FolderPickerOpenOptions,
    ) -> LocalBoxFuture<io::Result<Vec<Rc<dyn IStorageFolder>>>> {
        Box::pin(Self::open_folder_picker(options))
    }

    fn open_file_bookmark_async(&self, bookmark: &str) -> LocalBoxFuture<Option<Rc<dyn IStorageBookmarkFile>>> {
        let bookmark = bookmark.to_string();
        Box::pin(async move {
            match Self::decode_bookmark(bookmark).await? {
                JsStorageItemKind::File(file) => Some(file as Rc<dyn IStorageBookmarkFile>),
                JsStorageItemKind::Folder(_) => None,
            }
        })
    }

    fn open_folder_bookmark_async(&self, bookmark: &str) -> LocalBoxFuture<Option<Rc<dyn IStorageBookmarkFolder>>> {
        let bookmark = bookmark.to_string();
        Box::pin(async move {
            match Self::decode_bookmark(bookmark).await? {
                JsStorageItemKind::Folder(folder) => Some(folder as Rc<dyn IStorageBookmarkFolder>),
                JsStorageItemKind::File(_) => None,
            }
        })
    }

    fn try_get_file_from_path_async(&self, _file_path: &Uri) -> LocalBoxFuture<Option<Rc<dyn IStorageFile>>> {
        ready(None)
    }

    fn try_get_folder_from_path_async(&self, _folder_path: &Uri) -> LocalBoxFuture<Option<Rc<dyn IStorageFolder>>> {
        ready(None)
    }

    fn try_get_well_known_folder_async(
        &self,
        well_known_folder: WellKnownFolder,
    ) -> LocalBoxFuture<Option<Rc<dyn IStorageFolder>>> {
        Box::pin(async move {
            ferro_module::import_storage().await.ok()?;
            let directory =
                storage_helper::create_well_known_directory(Self::well_known_directory_name(well_known_folder));
            Some(JsStorageFolder::new(directory) as Rc<dyn IStorageFolder>)
        })
    }
}

/// A storage item of the script side, as a file or a folder by its `kind`.
enum JsStorageItemKind {
    File(Rc<JsStorageFile>),
    Folder(Rc<JsStorageFolder>),
}

impl JsStorageItemKind {
    fn from_item(item: JsObject) -> Option<Self> {
        match storage_helper::kind_of(&item).as_deref() {
            Some("directory") => Some(Self::Folder(JsStorageFolder::new(item))),
            Some("file") => Some(Self::File(JsStorageFile::new(item))),
            _ => None,
        }
    }

    /// The item for a file system handle the script side returned.
    fn from_handle(handle: JsObject) -> Option<Self> {
        let item = storage_helper::storage_item_from_handle(&handle)?;
        Self::from_item(item)
    }

    fn into_item(self) -> Rc<dyn IStorageItem> {
        match self {
            Self::File(file) => file,
            Self::Folder(folder) => folder,
        }
    }
}

/// What files and folders share: the storage item of the script side.
pub(crate) struct JsStorageItem {
    file_handle: RefCell<Option<JsObject>>,
    this: RefCell<Option<Weak<dyn IStorageItem>>>,
}

impl JsStorageItem {
    fn new(file_handle: JsObject) -> Self {
        Self { file_handle: RefCell::new(Some(file_handle)), this: RefCell::new(None) }
    }

    /// The shared part of `item`, if the provider created it.
    fn of(item: &dyn IStorageItem) -> Option<&JsStorageItem> {
        let item = item.as_any()?;
        if let Some(file) = item.downcast_ref::<JsStorageFile>() {
            return Some(&file.base);
        }
        item.downcast_ref::<JsStorageFolder>().map(|folder| &folder.base)
    }

    /// The script object, `None` once disposed.
    pub(crate) fn file_handle(&self) -> Option<JsObject> {
        self.file_handle.borrow().clone()
    }

    fn require_file_handle(&self) -> io::Result<JsObject> {
        self.file_handle()
            .ok_or_else(|| io::Error::other("Cannot access a disposed object. Object name: 'JSStorageItem'."))
    }

    fn name(&self) -> String {
        self.file_handle().and_then(|handle| storage_helper::name_of(&handle)).unwrap_or_default()
    }

    fn path(&self) -> Uri {
        let name = self.name();
        // A name with a colon would read as a scheme; such a relative reference starts with "./".
        Uri::new(&name, UriKind::Relative)
            .or_else(|_| Uri::new(&format!("./{name}"), UriKind::Relative))
            .expect("a dot segment makes any name a relative reference")
    }

    fn get_basic_properties_async(&self) -> LocalBoxFuture<ItemProperties> {
        let handle = self.file_handle();
        Box::pin(async move {
            let properties = match handle {
                Some(handle) => {
                    // Failures of earlier writes are left for the next opening of the file.
                    let _ = settle_pending_closes(&handle, false).await;
                    storage_helper::get_properties(&handle).await.ok().and_then(non_null)
                }
                None => None,
            };
            let properties = properties.map(JsCast::unchecked_into::<StorageItemProperties>);
            let size = properties.as_ref().and_then(|p| p.size()).map(|size| size as i64);
            let last_modified = properties.as_ref().and_then(|p| p.last_modified()).map(|ms| ms as i64);

            ItemProperties::new(
                size.map(|size| size as u64),
                None,
                last_modified.filter(|ms| *ms > 0).map(from_unix_time_milliseconds),
            )
        })
    }

    fn can_bookmark(&self) -> bool {
        storage_helper::has_native_file_picker()
    }

    fn save_bookmark_async(&self) -> LocalBoxFuture<Option<String>> {
        if !self.can_bookmark() {
            return ready(None);
        }
        let handle = self.file_handle();
        Box::pin(async move {
            let native_bookmark = storage_helper::save_bookmark(&handle?).await.ok()?.as_string()?;
            StorageBookmarkHelper::encode_bookmark(BROWSER_BOOKMARK_KEY, Some(&native_bookmark))
        })
    }

    fn get_parent_async(&self) -> LocalBoxFuture<Option<Rc<dyn IStorageFolder>>> {
        ready(None)
    }

    fn delete_async(&self) -> LocalBoxFuture<io::Result<()>> {
        let handle = self.require_file_handle();
        Box::pin(async move {
            storage_helper::delete_async(&handle?).await?;
            Ok(())
        })
    }

    fn move_async(
        &self,
        destination: Rc<dyn IStorageFolder>,
    ) -> LocalBoxFuture<io::Result<Option<Rc<dyn IStorageItem>>>> {
        let Some(folder) = destination.as_any().and_then(|item| item.downcast_ref::<JsStorageFolder>()) else {
            return ready(Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Destination folder must be initialized the StorageProvider API.",
            )));
        };
        let handle = self.require_file_handle();
        let destination_handle = folder.base.require_file_handle();
        let this = self.this.borrow().clone();
        Box::pin(async move {
            let item_handle = storage_helper::move_async(&handle?, &destination_handle?).await?;
            let Some(item_handle) = non_null(item_handle) else {
                return Ok(None);
            };

            Ok(match JsStorageItemKind::from_handle(item_handle) {
                Some(item) => Some(item.into_item()),
                None => this.and_then(|this| this.upgrade()),
            })
        })
    }

    fn release_bookmark_async(&self) -> LocalBoxFuture<()> {
        if !self.can_bookmark() {
            return ready(());
        }
        let handle = self.file_handle();
        Box::pin(async move {
            if let Some(handle) = handle {
                let _ = storage_helper::delete_bookmark(&handle).await;
            }
        })
    }

    fn dispose(&self) {
        self.file_handle.borrow_mut().take();
    }
}

fn from_unix_time_milliseconds(milliseconds: i64) -> DateTimeOffset {
    DateTimeOffset::UNIX_EPOCH.add(TimeSpan::from_ticks(milliseconds * 10_000))
}

macro_rules! impl_js_storage_item {
    ($type:ty, $kind:ident, $trait:ident) => {
        impl IDisposable for $type {
            fn dispose(&self) {
                self.base.dispose();
            }
        }

        impl IStorageItem for $type {
            fn name(&self) -> String {
                self.base.name()
            }

            fn path(&self) -> Uri {
                self.base.path()
            }

            fn get_basic_properties_async(&self) -> LocalBoxFuture<ItemProperties> {
                self.base.get_basic_properties_async()
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
                self.base.delete_async()
            }

            fn move_async(
                &self,
                destination: Rc<dyn IStorageFolder>,
            ) -> LocalBoxFuture<io::Result<Option<Rc<dyn IStorageItem>>>> {
                self.base.move_async(destination)
            }

            fn $kind(self: Rc<Self>) -> Option<Rc<dyn $trait>> {
                Some(self)
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
    };
}

/// A file of the script side.
pub(crate) struct JsStorageFile {
    base: JsStorageItem,
}

impl JsStorageFile {
    pub(crate) fn new(file_handle: JsObject) -> Rc<Self> {
        let this = Rc::new(Self { base: JsStorageItem::new(file_handle) });
        let weak: Weak<dyn IStorageItem> = Rc::downgrade(&this) as Weak<dyn IStorageItem>;
        *this.base.this.borrow_mut() = Some(weak);
        this
    }

    /// Opens the file for reading. The content is read into memory before
    /// the stream is returned: the stream of the contract is synchronous.
    pub async fn open_read_stream_async(&self) -> io::Result<BlobReadableStream> {
        let handle = self.base.require_file_handle()?;
        settle_pending_closes(&handle, true).await?;
        let blob = storage_helper::open_read(&handle).await.map_err(denied)?;
        let mut stream = BlobReadableStream::new(blob);
        stream.load_async().await?;
        Ok(stream)
    }

    /// Opens the file for writing, truncated.
    pub async fn open_write_stream_async(&self) -> io::Result<WriteableStream> {
        let handle = self.base.require_file_handle()?;
        settle_pending_closes(&handle, true).await?;
        let stream_writer = storage_helper::open_write(&handle).await.map_err(denied)?;

        // Upstream starts from the size the file had, although `openWrite` truncates it
        // (`keepExistingData: false`), so a seek relative to the end landed past the content.
        Ok(WriteableStream::new(stream_writer, Some(handle), 0))
    }
}

impl_js_storage_item!(JsStorageFile, as_storage_file, IStorageFile);

impl IStorageFile for JsStorageFile {
    fn open_read_async(&self) -> LocalBoxFuture<io::Result<Box<dyn Read>>> {
        let handle = self.base.file_handle();
        Box::pin(async move {
            let file = JsStorageFile::detached(handle)?;
            Ok(Box::new(file.open_read_stream_async().await?) as Box<dyn Read>)
        })
    }

    fn open_write_async(&self) -> LocalBoxFuture<io::Result<Box<dyn Write>>> {
        let handle = self.base.file_handle();
        Box::pin(async move {
            let file = JsStorageFile::detached(handle)?;
            Ok(Box::new(file.open_write_stream_async().await?) as Box<dyn Write>)
        })
    }
}

impl JsStorageFile {
    /// A file over the same script object, for a future that must not
    /// borrow `self`.
    fn detached(handle: Option<JsObject>) -> io::Result<JsStorageFile> {
        let base = JsStorageItem { file_handle: RefCell::new(handle), this: RefCell::new(None) };
        base.require_file_handle()?;
        Ok(JsStorageFile { base })
    }
}

impl IStorageBookmarkFile for JsStorageFile {}

/// A folder of the script side.
pub(crate) struct JsStorageFolder {
    base: JsStorageItem,
}

impl JsStorageFolder {
    pub(crate) fn new(file_handle: JsObject) -> Rc<Self> {
        let this = Rc::new(Self { base: JsStorageItem::new(file_handle) });
        let weak: Weak<dyn IStorageItem> = Rc::downgrade(&this) as Weak<dyn IStorageItem>;
        *this.base.this.borrow_mut() = Some(weak);
        this
    }

    fn should_supress_error_on_file_access(error: &JsError) -> bool {
        error.message() == NO_PERMISSIONS_MESSAGE
            || error.message().contains(TYPE_MISSMATCH_MESSAGE)
            || error.message().contains(FILE_FOLDER_NOT_FOUND_MESSAGE)
    }
}

impl_js_storage_item!(JsStorageFolder, as_storage_folder, IStorageFolder);

impl IStorageFolder for JsStorageFolder {
    fn get_items_async(&self) -> LocalBoxFuture<io::Result<Vec<Rc<dyn IStorageItem>>>> {
        let handle = self.base.require_file_handle();
        Box::pin(async move {
            let mut items: Vec<Rc<dyn IStorageItem>> = Vec::new();
            let Some(items_iterator) = storage_helper::get_items_iterator(&handle?) else {
                return Ok(items);
            };

            loop {
                let Some(next_result) = non_null(items_iterator.next().await?) else {
                    break;
                };
                let next_result = next_result.unchecked_into::<storage_helper::ItemsIteratorResult>();

                if next_result.done().unwrap_or(false) {
                    break;
                }

                // 0 - item name, 1 - item instance
                let Some(storage_item) = next_result.item() else {
                    break;
                };

                if let Some(item) = JsStorageItemKind::from_handle(storage_item) {
                    items.push(item.into_item());
                }
            }

            Ok(items)
        })
    }

    fn get_folder_async(&self, name: &str) -> LocalBoxFuture<Option<Rc<dyn IStorageFolder>>> {
        let handle = self.base.file_handle();
        let name = name.to_string();
        Box::pin(async move {
            match storage_helper::get_folder(&handle?, &name).await {
                Ok(folder_handle) => {
                    let storage_folder = storage_helper::storage_item_from_handle(&non_null(folder_handle)?)?;
                    Some(JsStorageFolder::new(storage_folder) as Rc<dyn IStorageFolder>)
                }
                Err(error) if Self::should_supress_error_on_file_access(&error) => None,
                // The contract has no error channel here.
                Err(_) => None,
            }
        })
    }

    fn get_file_async(&self, name: &str) -> LocalBoxFuture<Option<Rc<dyn IStorageFile>>> {
        let handle = self.base.file_handle();
        let name = name.to_string();
        Box::pin(async move {
            match storage_helper::get_file(&handle?, &name).await {
                Ok(file_handle) => {
                    let storage_file = storage_helper::storage_item_from_handle(&non_null(file_handle)?)?;
                    Some(JsStorageFile::new(storage_file) as Rc<dyn IStorageFile>)
                }
                Err(error) if Self::should_supress_error_on_file_access(&error) => None,
                // The contract has no error channel here.
                Err(_) => None,
            }
        })
    }

    fn create_file_async(&self, name: &str) -> LocalBoxFuture<io::Result<Option<Rc<dyn IStorageFile>>>> {
        let handle = self.base.require_file_handle();
        let name = name.to_string();
        Box::pin(async move {
            let file_handle = storage_helper::create_file(&handle?, &name).await.map_err(denied)?;
            let Some(file_handle) = non_null(file_handle) else {
                return Ok(None);
            };

            Ok(storage_helper::storage_item_from_handle(&file_handle)
                .map(|storage_file| JsStorageFile::new(storage_file) as Rc<dyn IStorageFile>))
        })
    }

    fn create_folder_async(&self, name: &str) -> LocalBoxFuture<io::Result<Option<Rc<dyn IStorageFolder>>>> {
        let handle = self.base.require_file_handle();
        let name = name.to_string();
        Box::pin(async move {
            let folder_handle = storage_helper::create_folder(&handle?, &name).await.map_err(denied)?;
            let Some(folder_handle) = non_null(folder_handle) else {
                return Ok(None);
            };

            Ok(storage_helper::storage_item_from_handle(&folder_handle)
                .map(|storage_folder| JsStorageFolder::new(storage_folder) as Rc<dyn IStorageFolder>))
        })
    }
}

impl IStorageBookmarkFolder for JsStorageFolder {}

#[cfg(test)]
mod tests {
    // Not from upstream (upstream has no tests of the browser storage provider): the parts that
    // do not need a page.
    use super::*;

    #[test]
    fn convert_file_types_without_filter_includes_all() {
        let (types, exclude_all) = BrowserStorageProvider::convert_file_types(None);
        assert!(types.is_none());
        assert!(!exclude_all);
    }

    #[test]
    fn convert_file_types_with_only_all_includes_all() {
        let all = [FilePickerFileTypes::all()];
        let (types, exclude_all) = BrowserStorageProvider::convert_file_types(Some(&all));
        assert!(types.is_none());
        assert!(!exclude_all);
    }

    #[test]
    fn convert_file_types_without_mime_types_includes_all() {
        let patterns_only = [FilePickerFileType::new(Some("Text")).with_patterns(&["*.txt"])];
        let (types, exclude_all) = BrowserStorageProvider::convert_file_types(Some(&patterns_only));
        assert!(types.is_none());
        assert!(!exclude_all);
    }

    #[test]
    fn picker_cancel_and_permission_messages_are_recognised() {
        assert!(is_cancel(&JsError::new("The user aborted a request.")));
        assert!(!is_cancel(&JsError::new("Permissions denied")));
        assert_eq!(denied(JsError::new("Permissions denied")).kind(), io::ErrorKind::PermissionDenied);
        assert_eq!(denied(JsError::new("other")).kind(), io::ErrorKind::Other);
        assert!(JsStorageFolder::should_supress_error_on_file_access(&JsError::new(
            "A requested file or directory could not be found at the time an operation was processed."
        )));
        assert!(!JsStorageFolder::should_supress_error_on_file_access(&JsError::new("other")));
    }

    #[test]
    fn last_modified_converts_from_unix_milliseconds() {
        let date = from_unix_time_milliseconds(86_400_000);
        assert_eq!(date, DateTimeOffset::UNIX_EPOCH.add_days(1.0));
    }

    #[test]
    fn well_known_folders_map_to_the_picker_names() {
        assert_eq!(BrowserStorageProvider::well_known_directory_name(WellKnownFolder::Desktop), "desktop");
        assert_eq!(BrowserStorageProvider::well_known_directory_name(WellKnownFolder::Videos), "videos");
    }
}
