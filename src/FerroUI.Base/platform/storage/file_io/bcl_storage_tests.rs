//! Tests of the storage backed by the local file system, on a temporary
//! directory. Upstream has no unit tests for these classes.

use super::bcl_storage_provider::parse_xdg_user_directory;
use super::{
    BclLauncher, BclStorageFile, BclStorageFolder, BclStorageItem, BclStorageItemHandle, FileSystemInfo,
    SecurityScopedStream, StorageBookmarkHelper, StorageProviderHelpers,
};
use crate::input::LocalBoxFuture;
use crate::platform::storage::{
    ILauncher, IStorageFile, IStorageFolder, IStorageItem, IStorageProvider, NoopStorageProvider,
};
use crate::reactive::Disposable;
use crate::utilities::{DateTimeOffset, Uri, UriKind};
use std::cell::{Cell, RefCell};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::task::{Context, Poll, Waker};

fn ready<T>(mut future: LocalBoxFuture<T>) -> T {
    match future.as_mut().poll(&mut Context::from_waker(Waker::noop())) {
        Poll::Ready(value) => value,
        Poll::Pending => panic!("the future is expected to be complete"),
    }
}

/// A directory of its own for a test, removed when the value is dropped.
struct TempDirectory(PathBuf);

impl TempDirectory {
    fn new(name: &str) -> Self {
        static NEXT: AtomicU32 = AtomicU32::new(0);

        let unique = NEXT.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("ferroui-storage-tests-{}-{unique}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }

    fn folder(&self) -> Rc<BclStorageFolder> {
        BclStorageFolder::new(FileSystemInfo::directory(&self.0))
    }
}

impl Drop for TempDirectory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn names(items: &[Rc<dyn IStorageItem>]) -> Vec<String> {
    items.iter().map(|item| item.name()).collect()
}

fn read_all(file: &dyn IStorageFile) -> String {
    let mut text = String::new();
    ready(file.open_read_async()).unwrap().read_to_string(&mut text).unwrap();
    text
}

fn write_all(file: &dyn IStorageFile, text: &str) {
    let mut stream = ready(file.open_write_async()).unwrap();
    stream.write_all(text.as_bytes()).unwrap();
    stream.flush().unwrap();
}

#[test]
fn files_and_folders_are_created_found_enumerated_and_deleted() {
    let directory = TempDirectory::new("items");
    let folder = directory.folder();
    assert!(ready(folder.get_items_async()).unwrap().is_empty());

    let file = ready(folder.create_file_async("b.txt")).unwrap().expect("the file is created");
    let other = ready(folder.create_file_async("a.txt")).unwrap().expect("the file is created");
    let sub_folder = ready(folder.create_folder_async("sub")).unwrap().expect("the folder is created");
    let nested = ready(folder.create_folder_async("deep/er")).unwrap().expect("the folders are created");
    assert_eq!("b.txt", file.name());
    assert_eq!("sub", sub_folder.name());
    assert_eq!("er", nested.name());
    assert!(directory.path().join("b.txt").is_file());
    assert!(directory.path().join("deep").join("er").is_dir());

    // A folder cannot be created outside of the folder.
    let error = ready(folder.create_folder_async("../outside")).err().expect("the folder is refused");
    assert_eq!(std::io::ErrorKind::InvalidInput, error.kind());

    // The folders come first, then the files.
    let items = ready(folder.get_items_async()).unwrap();
    let mut folders = names(&items[..2]);
    let mut files = names(&items[2..]);
    folders.sort();
    files.sort();
    assert_eq!(["deep", "sub"], folders[..]);
    assert_eq!(["a.txt", "b.txt"], files[..]);
    assert!(items[0].clone().as_storage_folder().is_some() && items[0].clone().as_storage_file().is_none());
    assert!(items[3].clone().as_storage_file().is_some() && items[3].clone().as_storage_folder().is_none());

    // Items are found by name and by kind.
    assert!(ready(folder.get_file_async("a.txt")).is_some());
    assert!(ready(folder.get_file_async("sub")).is_none());
    assert!(ready(folder.get_file_async("missing.txt")).is_none());
    assert!(ready(folder.get_folder_async("sub")).is_some());
    assert!(ready(folder.get_folder_async("a.txt")).is_none());

    // Creating an existing file empties it.
    write_all(&*other, "content");
    assert_eq!("content", read_all(&*other));
    let again = ready(folder.create_file_async("a.txt")).unwrap().unwrap();
    assert_eq!("", read_all(&*again));

    // Deleting: a file, and a folder with its content.
    ready(file.delete_async()).unwrap();
    assert!(!directory.path().join("b.txt").exists());
    assert!(ready(file.delete_async()).is_err());
    let deep = ready(folder.get_folder_async("deep")).unwrap();
    ready(deep.delete_async()).unwrap();
    assert!(!directory.path().join("deep").exists());
    assert_eq!(2, ready(folder.get_items_async()).unwrap().len());
}

#[test]
fn files_are_read_and_written_and_report_their_properties() {
    let directory = TempDirectory::new("content");
    let folder = directory.folder();
    let file = ready(folder.create_file_async("data.txt")).unwrap().unwrap();

    write_all(&*file, "hello storage");
    assert_eq!("hello storage", read_all(&*file));
    // Writing replaces the content.
    write_all(&*file, "short");
    assert_eq!("short", read_all(&*file));

    let now = DateTimeOffset::utc_now();
    let is_recent = |date: Option<DateTimeOffset>| {
        let date = date.expect("the file system reports the date");
        let seconds = now.subtract_date_time_offset(date).ticks().abs() / 10_000_000;
        seconds < 3600
    };

    let properties = ready(file.get_basic_properties_async());
    assert_eq!(Some(5), properties.size());
    assert!(is_recent(properties.date_created()));
    assert!(is_recent(properties.date_modified()));

    // A folder has no size.
    let properties = ready(folder.get_basic_properties_async());
    assert_eq!(Some(0), properties.size());
    assert!(is_recent(properties.date_modified()));

    // An item that is gone has no properties.
    ready(file.delete_async()).unwrap();
    let properties = ready(file.get_basic_properties_async());
    assert_eq!(None, properties.size());
    assert_eq!(None, properties.date_created());
    assert_eq!(None, properties.date_modified());
    assert!(ready(file.open_read_async()).is_err());
}

#[test]
fn items_know_their_name_path_parent_and_local_path() {
    let directory = TempDirectory::new("with space");
    let folder = directory.folder();
    let file = ready(folder.create_file_async("my file [1].txt")).unwrap().unwrap();
    let file_path = directory.path().join("my file [1].txt");

    assert_eq!("my file [1].txt", file.name());
    assert!(file.can_bookmark());
    let item: &dyn IStorageItem = &*file;
    assert_eq!(Some(file_path.to_str().unwrap()), item.try_get_local_path().as_deref());

    // The path is an escaped file URI, which gives the local path back.
    let uri = file.path();
    assert!(uri.is_absolute_uri());
    assert_eq!("file", uri.scheme());
    assert!(uri.absolute_uri().ends_with("/my%20file%20%5B1%5D.txt"), "{uri}");
    assert_eq!(file_path.to_str().unwrap(), uri.local_path());
    assert!(folder.path().absolute_uri().ends_with("with%20space/"), "{}", folder.path());

    let parent = ready(file.get_parent_async()).expect("a file has a parent folder");
    assert_eq!(folder.name(), parent.name());
    let parent: &dyn IStorageItem = &*parent;
    assert_eq!(Some(directory.path().to_str().unwrap()), parent.try_get_local_path().as_deref());

    // A relative path is made absolute.
    let relative = FileSystemInfo::file("some-relative-file.txt");
    assert!(relative.path().is_absolute());
    assert_eq!("some-relative-file.txt", relative.name());
    assert!(!relative.exists());
}

#[cfg(unix)]
#[test]
fn the_root_directory_has_no_parent_and_a_relative_path() {
    let root = BclStorageFolder::new(FileSystemInfo::directory("/"));

    assert_eq!("/", root.name());
    assert!(ready(root.get_parent_async()).is_none());
    assert_eq!(Uri::new("/", UriKind::Relative).unwrap(), root.path());
}

#[test]
#[should_panic(expected = "Directory must exist")]
fn a_folder_is_only_created_over_an_existing_directory() {
    let directory = TempDirectory::new("missing");
    BclStorageFolder::new(FileSystemInfo::directory(directory.path().join("nope")));
}

#[test]
fn files_and_folders_are_moved_to_another_folder() {
    let directory = TempDirectory::new("move");
    let folder = directory.folder();
    let source = ready(folder.create_folder_async("source")).unwrap().unwrap();
    let destination = ready(folder.create_folder_async("destination")).unwrap().unwrap();
    let file = ready(source.create_file_async("file.txt")).unwrap().unwrap();
    write_all(&*file, "moved");
    let inner = ready(source.create_folder_async("inner")).unwrap().unwrap();
    ready(inner.create_file_async("nested.txt")).unwrap().unwrap();

    let moved = ready(file.move_async(destination.clone())).unwrap().expect("the moved item");
    assert_eq!("file.txt", moved.name());
    assert!(!directory.path().join("source").join("file.txt").exists());
    let moved_file = moved.clone().as_storage_file().expect("a moved file is a file");
    assert_eq!("moved", read_all(&*moved_file));
    assert_eq!(
        Some(directory.path().join("destination").join("file.txt").to_str().unwrap()),
        moved.try_get_local_path().as_deref()
    );

    let moved = ready(inner.move_async(destination.clone())).unwrap().expect("the moved item");
    let moved_folder = moved.as_storage_folder().expect("a moved folder is a folder");
    assert!(ready(moved_folder.get_file_async("nested.txt")).is_some());
    assert!(!directory.path().join("source").join("inner").exists());

    // An existing item of the same name is not replaced.
    let duplicate = ready(source.create_file_async("file.txt")).unwrap().unwrap();
    let error = ready(duplicate.move_async(destination.clone())).err().expect("the move is refused");
    assert_eq!(std::io::ErrorKind::AlreadyExists, error.kind());
    assert_eq!("moved", read_all(&*moved_file));
}

#[test]
fn bookmarks_round_trip_through_the_storage_provider() {
    let directory = TempDirectory::new("bookmarks");
    let folder = directory.folder();
    let file = ready(folder.create_file_async("kept.txt")).unwrap().unwrap();
    write_all(&*file, "bookmarked");
    let provider: Rc<dyn IStorageProvider> = Rc::new(NoopStorageProvider);

    let file_bookmark = ready(file.save_bookmark_async()).expect("a file can be bookmarked");
    let folder_bookmark = ready(folder.save_bookmark_async()).expect("a folder can be bookmarked");
    assert_eq!(
        Some(directory.path().join("kept.txt").to_str().unwrap()),
        StorageBookmarkHelper::try_decode_bcl_bookmark(&file_bookmark).as_deref()
    );

    let opened = ready(provider.open_file_bookmark_async(&file_bookmark)).expect("the bookmarked file");
    assert_eq!("kept.txt", opened.name());
    assert_eq!("bookmarked", read_all(&*opened));
    ready(opened.release_bookmark_async());
    let opened = ready(provider.open_folder_bookmark_async(&folder_bookmark)).expect("the bookmarked folder");
    assert_eq!(folder.name(), opened.name());
    assert!(ready(opened.get_file_async("kept.txt")).is_some());

    // A bookmark only opens as what it names.
    assert!(ready(provider.open_folder_bookmark_async(&file_bookmark)).is_none());
    assert!(ready(provider.open_file_bookmark_async(&folder_bookmark)).is_none());
    // Bookmarks of the old format (plain paths) are still opened.
    let plain = directory.path().join("kept.txt");
    assert!(ready(provider.open_file_bookmark_async(plain.to_str().unwrap())).is_some());
    // Invalid bookmarks, and bookmarks of items that are gone, open nothing.
    assert!(ready(provider.open_file_bookmark_async("not a bookmark")).is_none());
    ready(file.delete_async()).unwrap();
    assert!(ready(provider.open_file_bookmark_async(&file_bookmark)).is_none());
}

#[test]
fn the_storage_provider_finds_files_and_folders_by_path() {
    let directory = TempDirectory::new("by path");
    let folder = directory.folder();
    ready(folder.create_file_async("100%.txt")).unwrap().unwrap();
    let file_path = directory.path().join("100%.txt");
    let provider: Rc<dyn IStorageProvider> = Rc::new(NoopStorageProvider);
    assert!(provider.is_file_system_backed());
    assert!(!provider.can_open() && !provider.can_save() && !provider.can_pick_folder());

    // By URI.
    let file_uri = StorageProviderHelpers::uri_from_file_path(file_path.to_str().unwrap(), false);
    let folder_uri = StorageProviderHelpers::uri_from_file_path(directory.path().to_str().unwrap(), true);
    assert_eq!("100%.txt", ready(provider.try_get_file_from_path_async(&file_uri)).expect("the file").name());
    assert!(ready(provider.try_get_folder_from_path_async(&folder_uri)).is_some());
    assert!(ready(provider.try_get_folder_from_path_async(&file_uri)).is_none());
    assert!(ready(provider.try_get_file_from_path_async(&folder_uri)).is_none());
    let relative = Uri::new("100%.txt", UriKind::Relative).unwrap();
    assert!(ready(provider.try_get_file_from_path_async(&relative)).is_none());

    // By path, without going through a URI.
    assert!(ready(provider.try_get_file_from_path_str_async(file_path.to_str().unwrap())).is_some());
    assert!(ready(provider.try_get_folder_from_path_str_async(directory.path().to_str().unwrap())).is_some());
    assert!(ready(provider.try_get_folder_from_path_str_async(file_path.to_str().unwrap())).is_none());
    assert!(ready(provider.try_get_file_from_path_str_async(directory.path().to_str().unwrap())).is_none());
    assert!(ready(provider.try_get_file_from_path_str_async("   ")).is_none());

    assert!(matches!(
        StorageProviderHelpers::try_create_bcl_storage_item(file_path.to_str()),
        Some(BclStorageItemHandle::File(_))
    ));
    assert!(matches!(
        StorageProviderHelpers::try_create_bcl_storage_item(directory.path().to_str()),
        Some(BclStorageItemHandle::Folder(_))
    ));
    assert!(StorageProviderHelpers::try_create_bcl_storage_item(None).is_none());
    assert!(StorageProviderHelpers::try_create_bcl_storage_item(Some("")).is_none());
    assert!(StorageProviderHelpers::try_create_bcl_storage_item(directory.path().join("missing").to_str()).is_none());
}

#[test]
fn user_directories_are_read_from_the_desktop_configuration() {
    let home = Path::new("/home/user");
    let content = "# comment\nXDG_DESKTOP_DIR=\"$HOME/Schreibtisch\"\n  XDG_MUSIC_DIR = \"/mnt/music\"\nXDG_VIDEOS_DIR=\"relative\"\nXDG_PICTURES_DIR=broken\n";

    assert_eq!(
        Some(PathBuf::from("/home/user/Schreibtisch")),
        parse_xdg_user_directory(content, home, "XDG_DESKTOP_DIR")
    );
    assert_eq!(Some(PathBuf::from("/mnt/music")), parse_xdg_user_directory(content, home, "XDG_MUSIC_DIR"));
    assert_eq!(None, parse_xdg_user_directory(content, home, "XDG_VIDEOS_DIR"));
    assert_eq!(None, parse_xdg_user_directory(content, home, "XDG_PICTURES_DIR"));
    assert_eq!(None, parse_xdg_user_directory(content, home, "XDG_DOCUMENTS_DIR"));
}

#[test]
fn security_scoped_stream_ends_its_scope_after_the_file_is_closed() {
    let directory = TempDirectory::new("scoped");
    let path = directory.path().join("scoped.bin");
    let ended = Rc::new(Cell::new(0));
    let scope = {
        let ended = ended.clone();
        Disposable::create(move || ended.set(ended.get() + 1))
    };

    let file = std::fs::OpenOptions::new().read(true).write(true).create(true).truncate(true).open(&path).unwrap();
    let mut stream = SecurityScopedStream::new(file, scope);
    stream.write_all(b"0123456789").unwrap();
    stream.flush().unwrap();
    assert_eq!(10, stream.length().unwrap());
    stream.seek(SeekFrom::Start(4)).unwrap();
    let mut buffer = [0u8; 3];
    stream.read_exact(&mut buffer).unwrap();
    assert_eq!(b"456", &buffer);
    stream.set_length(5).unwrap();
    assert_eq!(5, stream.length().unwrap());
    assert_eq!(0, ended.get());

    drop(stream);

    assert_eq!(1, ended.get());
    assert_eq!(b"01234", std::fs::read(&path).unwrap().as_slice());
}

// --- launcher: only the construction of the command is tested; nothing is launched ---

#[test]
fn the_launch_command_passes_the_target_as_one_argument() {
    let target = "https://example.org/a b?x=1&y=\"2\"";
    let command = BclLauncher::launch_command(target);

    if cfg!(target_os = "linux") {
        assert_eq!(Some(("xdg-open", vec![target.to_owned()])), command);
    } else if cfg!(target_os = "macos") {
        assert_eq!(Some(("open", vec![target.to_owned()])), command);
    } else if cfg!(target_os = "windows") {
        assert_eq!(Some(("rundll32", vec!["url.dll,FileProtocolHandler".to_owned(), target.to_owned()])), command);
    } else {
        assert_eq!(None, command);
    }
}

#[test]
fn the_launcher_refuses_what_it_cannot_launch_without_starting_a_process() {
    let directory = TempDirectory::new("launcher");
    let file = ready(directory.folder().create_file_async("doc.txt")).unwrap().unwrap();
    let asked = Rc::new(RefCell::new(Vec::new()));
    let launcher = BclLauncher::with_can_open_file_or_directory({
        let asked = asked.clone();
        move |local_path| {
            asked.borrow_mut().push(local_path.to_owned());
            false
        }
    });

    // A relative URI is not launched.
    assert!(!ready(launcher.launch_uri_async(&Uri::new("page.html", UriKind::Relative).unwrap())));
    // A file the launcher may not open is not launched.
    let item: Rc<dyn IStorageItem> = file;
    assert!(!ready(launcher.launch_file_async(item)));
    assert_eq!([directory.path().join("doc.txt").to_str().unwrap().to_owned()], asked.borrow()[..]);
}

/// A launcher that records the local paths it is asked to launch.
#[derive(Default)]
struct RecordingLauncher {
    launched: RefCell<Vec<Option<String>>>,
}

impl ILauncher for RecordingLauncher {
    fn launch_uri_async(&self, _uri: &Uri) -> LocalBoxFuture<bool> {
        Box::pin(std::future::ready(false))
    }

    fn launch_file_async(&self, storage_item: Rc<dyn IStorageItem>) -> LocalBoxFuture<bool> {
        self.launched.borrow_mut().push(storage_item.try_get_local_path());
        Box::pin(std::future::ready(true))
    }
}

#[test]
fn launcher_extensions_launch_existing_files_and_directories() {
    let directory = TempDirectory::new("launch info");
    let file_path = directory.path().join("doc.txt");
    std::fs::write(&file_path, b"x").unwrap();
    let recording = Rc::new(RecordingLauncher::default());
    let launcher: Rc<dyn ILauncher> = recording.clone();

    assert!(ready(launcher.launch_file_info_async(&file_path)));
    assert!(ready(launcher.launch_directory_info_async(directory.path())));
    // Missing entries, and entries of the other kind, are not launched.
    assert!(!ready(launcher.launch_file_info_async(&directory.path().join("missing.txt"))));
    assert!(!ready(launcher.launch_directory_info_async(&directory.path().join("missing"))));
    assert!(!ready(launcher.launch_file_info_async(directory.path())));
    assert!(!ready(launcher.launch_directory_info_async(&file_path)));

    assert_eq!(
        [Some(file_path.to_str().unwrap().to_owned()), Some(directory.path().to_str().unwrap().to_owned())],
        recording.launched.borrow()[..]
    );
}

#[test]
fn the_core_operations_work_on_file_system_entries() {
    let directory = TempDirectory::new("core");
    let created = BclStorageItem::create_file_core(directory.path(), "core.txt").unwrap();
    assert_eq!(FileSystemInfo::File(directory.path().join("core.txt")), created);
    assert!(created.exists() && !created.is_directory());
    assert_eq!(Some(FileSystemInfo::Directory(directory.path().to_path_buf())), BclStorageItem::get_parent_core(&created));
    assert_eq!(Some(created.clone()), BclStorageItem::get_file_core(directory.path(), "core.txt"));
    assert_eq!(None, BclStorageItem::get_folder_core(directory.path(), "core.txt"));
    assert_eq!(vec![created.clone()], BclStorageItem::get_items_core(directory.path()).unwrap());

    let file = BclStorageFile::new(created.clone());
    assert_eq!(&created, file.file_system_info());
    BclStorageItem::delete_core(&created).unwrap();
    assert!(!created.exists());
    assert!(BclStorageItem::get_items_core(&directory.path().join("missing")).is_err());
}
