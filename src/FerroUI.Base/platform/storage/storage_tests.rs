//! Tests of the storage contracts and their managed implementations. There
//! are no upstream tests for these; the ported upstream tests of the helpers
//! are in `file_io/storage_provider_helper_tests.rs`.

use super::*;
use crate::input::LocalBoxFuture;
use crate::reactive::IDisposable;
use crate::utilities::Uri;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::task::{Context, Poll, Waker};

fn ready<T>(mut future: LocalBoxFuture<T>) -> T {
    let mut cx = Context::from_waker(Waker::noop());
    match future.as_mut().poll(&mut cx) {
        Poll::Ready(value) => value,
        Poll::Pending => panic!("the future is expected to be complete"),
    }
}

fn now<T: 'static>(value: T) -> LocalBoxFuture<T> {
    Box::pin(std::future::ready(value))
}

struct TestFolder {
    name: &'static str,
    local_path: Option<&'static str>,
}

impl IDisposable for TestFolder {
    fn dispose(&self) {}
}

impl IStorageItem for TestFolder {
    fn name(&self) -> String {
        self.name.to_owned()
    }

    fn path(&self) -> Uri {
        Uri::absolute(&format!("file:///folders/{}%20x", self.name)).unwrap()
    }

    fn get_basic_properties_async(&self) -> LocalBoxFuture<StorageItemProperties> {
        now(StorageItemProperties::new(Some(1), None, None))
    }

    fn can_bookmark(&self) -> bool {
        false
    }

    fn save_bookmark_async(&self) -> LocalBoxFuture<Option<String>> {
        now(None)
    }

    fn get_parent_async(&self) -> LocalBoxFuture<Option<Rc<dyn IStorageFolder>>> {
        now(None)
    }

    fn delete_async(&self) -> LocalBoxFuture<std::io::Result<()>> {
        now(Ok(()))
    }

    fn move_async(
        &self,
        _destination: Rc<dyn IStorageFolder>,
    ) -> LocalBoxFuture<std::io::Result<Option<Rc<dyn IStorageItem>>>> {
        now(Ok(None))
    }

    fn as_storage_folder(self: Rc<Self>) -> Option<Rc<dyn IStorageFolder>> {
        Some(self)
    }

    fn as_storage_item_with_file_system_info(&self) -> Option<&dyn IStorageItemWithFileSystemInfo> {
        self.local_path.is_some().then_some(self as &dyn IStorageItemWithFileSystemInfo)
    }
}

impl IStorageItemWithFileSystemInfo for TestFolder {
    fn file_system_info_full_name(&self) -> String {
        self.local_path.unwrap_or_default().to_owned()
    }
}

impl IStorageFolder for TestFolder {
    fn get_items_async(&self) -> LocalBoxFuture<std::io::Result<Vec<Rc<dyn IStorageItem>>>> {
        now(Ok(Vec::new()))
    }

    fn get_folder_async(&self, _name: &str) -> LocalBoxFuture<Option<Rc<dyn IStorageFolder>>> {
        now(None)
    }

    fn get_file_async(&self, _name: &str) -> LocalBoxFuture<Option<Rc<dyn IStorageFile>>> {
        now(None)
    }

    fn create_file_async(&self, _name: &str) -> LocalBoxFuture<std::io::Result<Option<Rc<dyn IStorageFile>>>> {
        now(Ok(None))
    }

    fn create_folder_async(&self, _name: &str) -> LocalBoxFuture<std::io::Result<Option<Rc<dyn IStorageFolder>>>> {
        now(Ok(None))
    }
}

/// A provider that can only pick folders (when `can_pick_folder`), and
/// knows the folder at one path.
struct TestProvider {
    can_pick_folder: bool,
    known_path: Option<&'static str>,
    requested_paths: RefCell<Vec<String>>,
}

impl TestProvider {
    fn new(can_pick_folder: bool, known_path: Option<&'static str>) -> Rc<Self> {
        Rc::new(Self { can_pick_folder, known_path, requested_paths: RefCell::new(Vec::new()) })
    }
}

impl IStorageProvider for TestProvider {
    fn can_open(&self) -> bool {
        false
    }

    fn open_file_picker_async(
        &self,
        _options: FilePickerOpenOptions,
    ) -> LocalBoxFuture<std::io::Result<Vec<Rc<dyn IStorageFile>>>> {
        now(Ok(Vec::new()))
    }

    fn open_file_picker_with_result_async(
        &self,
        _options: FilePickerOpenOptions,
    ) -> LocalBoxFuture<std::io::Result<OpenFilePickerResult>> {
        now(Ok(OpenFilePickerResult::default()))
    }

    fn can_save(&self) -> bool {
        false
    }

    fn save_file_picker_async(
        &self,
        _options: FilePickerSaveOptions,
    ) -> LocalBoxFuture<std::io::Result<Option<Rc<dyn IStorageFile>>>> {
        now(Ok(None))
    }

    fn save_file_picker_with_result_async(
        &self,
        _options: FilePickerSaveOptions,
    ) -> LocalBoxFuture<std::io::Result<SaveFilePickerResult>> {
        now(Ok(SaveFilePickerResult::default()))
    }

    fn can_pick_folder(&self) -> bool {
        self.can_pick_folder
    }

    fn open_folder_picker_async(
        &self,
        options: FolderPickerOpenOptions,
    ) -> LocalBoxFuture<std::io::Result<Vec<Rc<dyn IStorageFolder>>>> {
        let count = if options.allow_multiple() { 2 } else { 1 };
        let folders = (0..count)
            .map(|_| Rc::new(TestFolder { name: "picked", local_path: None }) as Rc<dyn IStorageFolder>)
            .collect();
        now(Ok(folders))
    }

    fn open_file_bookmark_async(&self, _bookmark: &str) -> LocalBoxFuture<Option<Rc<dyn IStorageBookmarkFile>>> {
        now(None)
    }

    fn open_folder_bookmark_async(&self, _bookmark: &str) -> LocalBoxFuture<Option<Rc<dyn IStorageBookmarkFolder>>> {
        now(None)
    }

    fn try_get_file_from_path_async(&self, _file_path: &Uri) -> LocalBoxFuture<Option<Rc<dyn IStorageFile>>> {
        now(None)
    }

    fn try_get_folder_from_path_async(&self, folder_path: &Uri) -> LocalBoxFuture<Option<Rc<dyn IStorageFolder>>> {
        self.requested_paths.borrow_mut().push(folder_path.absolute_uri().to_owned());
        let found = self.known_path.is_some_and(|known| known == folder_path.absolute_uri());
        now(found.then(|| Rc::new(TestFolder { name: "known", local_path: None }) as Rc<dyn IStorageFolder>))
    }

    fn try_get_well_known_folder_async(
        &self,
        _well_known_folder: WellKnownFolder,
    ) -> LocalBoxFuture<Option<Rc<dyn IStorageFolder>>> {
        now(None)
    }
}

#[test]
fn noop_storage_provider_picks_nothing() {
    let provider = NoopStorageProvider;

    assert!(!provider.can_open());
    assert!(!provider.can_save());
    assert!(!provider.can_pick_folder());
    assert!(ready(provider.open_file_picker_async(FilePickerOpenOptions::new())).unwrap().is_empty());
    let result = ready(provider.open_file_picker_with_result_async(FilePickerOpenOptions::new())).unwrap();
    assert!(result.files.is_empty() && result.selected_file_type.is_none());
    assert!(ready(provider.save_file_picker_async(FilePickerSaveOptions::new())).unwrap().is_none());
    let result = ready(provider.save_file_picker_with_result_async(FilePickerSaveOptions::new())).unwrap();
    assert!(result.file.is_none() && result.selected_file_type.is_none());
    assert!(ready(provider.open_folder_picker_async(FolderPickerOpenOptions::new())).unwrap().is_empty());
}

#[test]
fn noop_launcher_launches_nothing() {
    let launcher = NoopLauncher;
    let folder: Rc<dyn IStorageItem> = Rc::new(TestFolder { name: "f", local_path: None });

    assert!(!ready(launcher.launch_uri_async(&Uri::absolute("https://example.org/").unwrap())));
    assert!(!ready(launcher.launch_file_async(folder)));
}

#[test]
fn fallback_storage_provider_uses_the_first_suitable_provider_and_creates_providers_lazily() {
    let created = Rc::new(Cell::new(0));
    let factory = |provider: Option<Rc<TestProvider>>| -> StorageProviderFactory {
        let created = created.clone();
        Rc::new(move || {
            created.set(created.get() + 1);
            now(provider.clone().map(|provider| provider as Rc<dyn IStorageProvider>))
        })
    };
    let first = TestProvider::new(false, None);
    let second = TestProvider::new(true, Some("file:///known/"));
    let third = TestProvider::new(true, None);
    let provider = FallbackStorageProvider::new(vec![
        factory(Some(first.clone())),
        factory(None),
        factory(Some(second.clone())),
        factory(Some(third.clone())),
    ]);

    assert!(provider.can_open() && provider.can_save() && provider.can_pick_folder());
    assert_eq!(0, created.get());

    // The first provider cannot pick folders, the second factory has no provider.
    let folders = ready(provider.open_folder_picker_async(FolderPickerOpenOptions::new().with_allow_multiple(true)));
    assert_eq!(2, folders.unwrap().len());
    assert_eq!(3, created.get());

    // Created providers are kept.
    let folders = ready(provider.open_folder_picker_async(FolderPickerOpenOptions::new()));
    assert_eq!(1, folders.unwrap().len());
    assert_eq!(3, created.get());

    // No provider can open files.
    let error = ready(provider.open_file_picker_async(FilePickerOpenOptions::new())).err().unwrap();
    assert_eq!("Unable to select a suitable storage provider", error.to_string());
    assert_eq!(4, created.get());

    // The first result that is not `None` wins, and the providers are asked in order.
    let known = Uri::absolute("file:///known/").unwrap();
    let folder = ready(provider.try_get_folder_from_path_async(&known)).unwrap();
    assert_eq!("known", folder.name());
    assert_eq!(1, first.requested_paths.borrow().len());
    assert_eq!(1, second.requested_paths.borrow().len());
    assert_eq!(0, third.requested_paths.borrow().len());
    assert!(ready(provider.try_get_folder_from_path_async(&Uri::absolute("file:///other/").unwrap())).is_none());
    assert_eq!(1, third.requested_paths.borrow().len());

    provider.reset(vec![factory(Some(third.clone()))]);
    assert!(ready(provider.try_get_folder_from_path_async(&known)).is_none());
    assert_eq!(5, created.get());
    assert_eq!(2, third.requested_paths.borrow().len());
    assert_eq!(2, first.requested_paths.borrow().len());
}

#[test]
fn storage_provider_extensions_convert_paths_to_uris() {
    let provider = TestProvider::new(true, Some("file:///home/My%20Folder/"));
    let provider: Rc<dyn IStorageProvider> = provider;

    let folder = ready(provider.try_get_folder_from_path_str_async("/home/My Folder"));
    assert!(folder.is_some());
    assert!(ready(provider.try_get_folder_from_path_str_async("/home/Other")).is_none());
    assert!(ready(provider.try_get_file_from_path_str_async("/home/file.txt")).is_none());
}

#[test]
fn try_get_local_path_prefers_the_file_system_entry_of_the_item() {
    let item: Rc<dyn IStorageItem> = Rc::new(TestFolder { name: "a", local_path: None });
    assert_eq!(Some("/folders/a x"), item.try_get_local_path().as_deref());
    assert!(item.clone().as_storage_folder().is_some());
    assert!(item.clone().as_storage_file().is_none());

    let item: Rc<dyn IStorageItem> = Rc::new(TestFolder { name: "a", local_path: Some("/exact/a%20x") });
    assert_eq!(Some("/exact/a%20x"), item.try_get_local_path().as_deref());

    // A folder handle converts to an item handle.
    let folder: Rc<dyn IStorageFolder> = Rc::new(TestFolder { name: "b", local_path: None });
    let item: Rc<dyn IStorageItem> = folder;
    assert_eq!("b", item.name());
}

#[test]
fn file_picker_file_types_are_shared_and_give_their_extensions() {
    assert!(Rc::ptr_eq(&FilePickerFileTypes::image_all(), &FilePickerFileTypes::image_all()));
    assert!(FilePickerFileTypes::all() != FilePickerFileTypes::text_plain());

    let image_all = FilePickerFileTypes::image_all();
    assert_eq!("All Images", image_all.name());
    assert_eq!("All Images", image_all.to_string());
    assert_eq!(Some(vec!["png", "jpg", "jpeg", "gif", "bmp", "webp"]), image_all.try_get_extensions().as_ref().map(|e| e.iter().map(String::as_str).collect()));
    assert_eq!(Some(&["image/*".to_owned()][..]), image_all.mime_types().as_deref());
    assert_eq!(Some(&["public.image".to_owned()][..]), image_all.apple_uniform_type_identifiers().as_deref());

    // "*.*" has no simple extension, and neither have glob extensions.
    assert_eq!(Some(Vec::new()), FilePickerFileTypes::all().try_get_extensions());
    let projects = FilePickerFileType::new(None).with_patterns(&["*.*proj", "*.sln", "Makefile", "*.tar.gz", "*."]);
    assert_eq!("", projects.name());
    assert_eq!(Some(vec!["sln".to_owned(), "gz".to_owned()]), projects.try_get_extensions());
    assert_eq!(None, FilePickerFileType::new(Some("x")).try_get_extensions());
}

#[test]
fn picker_options_share_the_common_options() {
    let all = FilePickerFileTypes::all();
    let mut options = FilePickerSaveOptions::new().with_default_extension("txt").with_suggested_file_type(all.clone());
    options.set_title(Some("Save".to_owned()));
    options.set_suggested_file_name(Some("a".to_owned()));
    assert_eq!(Some("Save"), options.title());
    assert_eq!(Some("a"), options.suggested_file_name());
    assert_eq!(Some("txt"), options.default_extension());
    assert_eq!(None, options.show_overwrite_prompt());
    assert!(options.suggested_start_location().is_none());
    assert!(options.suggested_file_type() == Some(&all));

    let open = FilePickerOpenOptions::from(PickerOptions::new().with_title("Open")).with_file_type_filter(vec![all]);
    assert_eq!(Some("Open"), open.title());
    assert!(!open.allow_multiple());
    assert_eq!(1, open.file_type_filter().unwrap().len());

    let a = OpenFilePickerResult::default();
    assert!(a == a.clone());
    assert!(SaveFilePickerResult::default() == SaveFilePickerResult::default());
}
