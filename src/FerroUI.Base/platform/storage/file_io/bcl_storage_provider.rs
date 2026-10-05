use super::{BclStorageFile, BclStorageFolder, FileSystemInfo, StorageBookmarkHelper, StorageProviderHelpers};
use crate::input::LocalBoxFuture;
use crate::platform::storage::{
    FilePickerOpenOptions, FilePickerSaveOptions, FolderPickerOpenOptions, IStorageBookmarkFile,
    IStorageBookmarkFolder, IStorageFile, IStorageFolder, IStorageProvider, OpenFilePickerResult,
    SaveFilePickerResult, WellKnownFolder,
};
use crate::utilities::Uri;
use std::path::{Path, PathBuf};
use std::rc::Rc;

fn ready<T: 'static>(value: T) -> LocalBoxFuture<T> {
    Box::pin(std::future::ready(value))
}

/// The base of the storage providers that give access to the local file
/// system: a provider implements the pickers and gets the bookmarks, the
/// lookups by path and the well-known folders from here. An implementation
/// of this trait is an [`IStorageProvider`].
///
/// This is an implementation detail of the platform backends.
pub trait BclStorageProvider: 'static {
    /// See [`IStorageProvider::can_open`].
    fn can_open(&self) -> bool;

    /// See [`IStorageProvider::open_file_picker_with_result_async`].
    fn open_file_picker_with_result_async(
        &self,
        options: FilePickerOpenOptions,
    ) -> LocalBoxFuture<std::io::Result<OpenFilePickerResult>>;

    /// See [`IStorageProvider::can_save`].
    fn can_save(&self) -> bool;

    /// See [`IStorageProvider::save_file_picker_with_result_async`].
    fn save_file_picker_with_result_async(
        &self,
        options: FilePickerSaveOptions,
    ) -> LocalBoxFuture<std::io::Result<SaveFilePickerResult>>;

    /// See [`IStorageProvider::can_pick_folder`].
    fn can_pick_folder(&self) -> bool;

    /// See [`IStorageProvider::open_folder_picker_async`].
    fn open_folder_picker_async(
        &self,
        options: FolderPickerOpenOptions,
    ) -> LocalBoxFuture<std::io::Result<Vec<Rc<dyn IStorageFolder>>>>;

    /// See [`IStorageProvider::open_file_bookmark_async`]; the file the
    /// bookmark names, if it exists.
    fn open_file_bookmark_async(&self, bookmark: &str) -> LocalBoxFuture<Option<Rc<dyn IStorageBookmarkFile>>> {
        ready(match open_bookmark(bookmark) {
            Some(BclStorageItemHandle::File(file)) => Some(file as Rc<dyn IStorageBookmarkFile>),
            _ => None,
        })
    }

    /// See [`IStorageProvider::open_folder_bookmark_async`]; the folder
    /// the bookmark names, if it exists.
    fn open_folder_bookmark_async(&self, bookmark: &str) -> LocalBoxFuture<Option<Rc<dyn IStorageBookmarkFolder>>> {
        ready(match open_bookmark(bookmark) {
            Some(BclStorageItemHandle::Folder(folder)) => Some(folder as Rc<dyn IStorageBookmarkFolder>),
            _ => None,
        })
    }

    /// See [`IStorageProvider::try_get_file_from_path_async`]; the file at
    /// the local path of an absolute URI, if it exists.
    fn try_get_file_from_path_async(&self, file_path: &Uri) -> LocalBoxFuture<Option<Rc<dyn IStorageFile>>> {
        if file_path.is_absolute_uri() {
            let file = FileSystemInfo::file(file_path.local_path());
            if file.exists() {
                return ready(Some(BclStorageFile::new(file) as Rc<dyn IStorageFile>));
            }
        }

        ready(None)
    }

    /// See [`IStorageProvider::try_get_folder_from_path_async`]; the
    /// folder at the local path of an absolute URI, if it exists.
    fn try_get_folder_from_path_async(&self, folder_path: &Uri) -> LocalBoxFuture<Option<Rc<dyn IStorageFolder>>> {
        if folder_path.is_absolute_uri() {
            let directory = FileSystemInfo::directory(folder_path.local_path());
            if directory.exists() {
                return ready(Some(BclStorageFolder::new(directory) as Rc<dyn IStorageFolder>));
            }
        }

        ready(None)
    }

    /// See [`IStorageProvider::try_get_well_known_folder_async`].
    fn try_get_well_known_folder_async(
        &self,
        well_known_folder: WellKnownFolder,
    ) -> LocalBoxFuture<Option<Rc<dyn IStorageFolder>>> {
        ready(
            try_get_well_known_folder_core(well_known_folder)
                .map(|directory_info| BclStorageFolder::new(directory_info) as Rc<dyn IStorageFolder>),
        )
    }
}

impl<T: BclStorageProvider> IStorageProvider for T {
    fn can_open(&self) -> bool {
        BclStorageProvider::can_open(self)
    }

    fn open_file_picker_async(
        &self,
        options: FilePickerOpenOptions,
    ) -> LocalBoxFuture<std::io::Result<Vec<Rc<dyn IStorageFile>>>> {
        let result = BclStorageProvider::open_file_picker_with_result_async(self, options);
        Box::pin(async move { Ok(result.await?.files) })
    }

    fn open_file_picker_with_result_async(
        &self,
        options: FilePickerOpenOptions,
    ) -> LocalBoxFuture<std::io::Result<OpenFilePickerResult>> {
        BclStorageProvider::open_file_picker_with_result_async(self, options)
    }

    fn can_save(&self) -> bool {
        BclStorageProvider::can_save(self)
    }

    fn save_file_picker_async(
        &self,
        options: FilePickerSaveOptions,
    ) -> LocalBoxFuture<std::io::Result<Option<Rc<dyn IStorageFile>>>> {
        let result = BclStorageProvider::save_file_picker_with_result_async(self, options);
        Box::pin(async move { Ok(result.await?.file) })
    }

    fn save_file_picker_with_result_async(
        &self,
        options: FilePickerSaveOptions,
    ) -> LocalBoxFuture<std::io::Result<SaveFilePickerResult>> {
        BclStorageProvider::save_file_picker_with_result_async(self, options)
    }

    fn can_pick_folder(&self) -> bool {
        BclStorageProvider::can_pick_folder(self)
    }

    fn open_folder_picker_async(
        &self,
        options: FolderPickerOpenOptions,
    ) -> LocalBoxFuture<std::io::Result<Vec<Rc<dyn IStorageFolder>>>> {
        BclStorageProvider::open_folder_picker_async(self, options)
    }

    fn open_file_bookmark_async(&self, bookmark: &str) -> LocalBoxFuture<Option<Rc<dyn IStorageBookmarkFile>>> {
        BclStorageProvider::open_file_bookmark_async(self, bookmark)
    }

    fn open_folder_bookmark_async(&self, bookmark: &str) -> LocalBoxFuture<Option<Rc<dyn IStorageBookmarkFolder>>> {
        BclStorageProvider::open_folder_bookmark_async(self, bookmark)
    }

    fn try_get_file_from_path_async(&self, file_path: &Uri) -> LocalBoxFuture<Option<Rc<dyn IStorageFile>>> {
        BclStorageProvider::try_get_file_from_path_async(self, file_path)
    }

    fn try_get_folder_from_path_async(&self, folder_path: &Uri) -> LocalBoxFuture<Option<Rc<dyn IStorageFolder>>> {
        BclStorageProvider::try_get_folder_from_path_async(self, folder_path)
    }

    fn try_get_well_known_folder_async(
        &self,
        well_known_folder: WellKnownFolder,
    ) -> LocalBoxFuture<Option<Rc<dyn IStorageFolder>>> {
        BclStorageProvider::try_get_well_known_folder_async(self, well_known_folder)
    }

    fn is_file_system_backed(&self) -> bool {
        true
    }
}

/// A storage item of the local file system, as the class it is.
pub enum BclStorageItemHandle {
    /// A folder.
    Folder(Rc<BclStorageFolder>),
    /// A file.
    File(Rc<BclStorageFile>),
}

fn open_bookmark(bookmark: &str) -> Option<BclStorageItemHandle> {
    let local_path = StorageBookmarkHelper::try_decode_bcl_bookmark(bookmark)?;
    StorageProviderHelpers::try_create_bcl_storage_item(Some(&local_path))
}

fn home_directory() -> Option<PathBuf> {
    let variable = if cfg!(windows) { "USERPROFILE" } else { "HOME" };
    std::env::var_os(variable).filter(|home| !home.is_empty()).map(PathBuf::from)
}

/// The directory `key` names in the user directories configuration of the
/// desktop (`user-dirs.dirs`), if it is configured.
fn read_xdg_user_directory(home: &Path, key: &str) -> Option<PathBuf> {
    let config_home = std::env::var_os("XDG_CONFIG_HOME")
        .filter(|config_home| !config_home.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".config"));
    let content = std::fs::read_to_string(config_home.join("user-dirs.dirs")).ok()?;
    parse_xdg_user_directory(&content, home, key)
}

/// Finds `key="value"` in the content of a `user-dirs.dirs` file. The value
/// is an absolute path or a path relative to the home directory
/// (`$HOME/...`).
pub(super) fn parse_xdg_user_directory(content: &str, home: &Path, key: &str) -> Option<PathBuf> {
    for line in content.lines() {
        let line = line.trim_start();
        let Some(rest) = line.strip_prefix(key) else { continue };
        let Some(rest) = rest.trim_start().strip_prefix('=') else { continue };
        let Some(rest) = rest.trim_start().strip_prefix('"') else { continue };
        let Some(end) = rest.find('"') else { continue };
        let value = &rest[..end];

        if let Some(relative) = value.strip_prefix("$HOME/") {
            return Some(home.join(relative));
        }
        if value.starts_with('/') {
            return Some(PathBuf::from(value));
        }
    }
    None
}

/// The path of a folder of the user, as the special folders of the .NET
/// base library resolve it: the user directories of the desktop on Linux,
/// the conventional directories of the home directory elsewhere. The
/// directory is created when it does not exist.
fn get_from_special_folder(xdg_key: &str, name: &str, macos_name: &str) -> Option<PathBuf> {
    let home = home_directory()?;
    let path = if cfg!(target_os = "macos") {
        home.join(macos_name)
    } else if cfg!(windows) {
        home.join(name)
    } else {
        read_xdg_user_directory(&home, xdg_key).unwrap_or_else(|| home.join(name))
    };
    // Failing to create the directory leaves it missing, which the caller
    // checks.
    let _ = std::fs::create_dir_all(&path);
    Some(path)
}

/// The directory of a well-known folder, if it exists.
pub fn try_get_well_known_folder_core(well_known_folder: WellKnownFolder) -> Option<FileSystemInfo> {
    let folder_path = match well_known_folder {
        WellKnownFolder::Desktop => get_from_special_folder("XDG_DESKTOP_DIR", "Desktop", "Desktop"),
        WellKnownFolder::Documents => get_from_special_folder("XDG_DOCUMENTS_DIR", "Documents", "Documents"),
        WellKnownFolder::Downloads => get_downloads_well_known_folder().map(PathBuf::from),
        WellKnownFolder::Music => get_from_special_folder("XDG_MUSIC_DIR", "Music", "Music"),
        WellKnownFolder::Pictures => get_from_special_folder("XDG_PICTURES_DIR", "Pictures", "Pictures"),
        WellKnownFolder::Videos => get_from_special_folder("XDG_VIDEOS_DIR", "Videos", "Movies"),
    }?;

    let directory = FileSystemInfo::directory(folder_path);
    directory.exists().then_some(directory)
}

/// The path of the downloads folder of the user, which the special folders
/// do not cover.
pub fn get_downloads_well_known_folder() -> Option<String> {
    if cfg!(windows) {
        return home_directory().map(|home| home.join("Downloads").to_string_lossy().into_owned());
    }

    if cfg!(target_os = "linux") {
        if let Some(env_dir) = std::env::var_os("XDG_DOWNLOAD_DIR") {
            if Path::new(&env_dir).is_dir() {
                return Some(env_dir.to_string_lossy().into_owned());
            }
        }
    }

    if cfg!(any(target_os = "linux", target_os = "macos")) {
        // As upstream: the tilde is not expanded, so this only names an
        // existing directory when the current directory has one of that
        // name.
        return Some("~/Downloads".to_owned());
    }

    None
}
