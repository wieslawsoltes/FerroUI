use super::{BclStorageFile, BclStorageFolder, StorageBookmarkHelper, StorageProviderHelpers};
use crate::animation::TimeSpan;
use crate::platform::storage::{IStorageFolder, IStorageItem, StorageItemProperties};
use crate::utilities::{DateTimeOffset, Uri, UriKind};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::SystemTime;

/// An entry of the local file system: a file or a directory, by its full
/// path (the counterpart of the file-system info objects of the .NET base
/// library). The entry does not have to exist.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FileSystemInfo {
    /// A file.
    File(PathBuf),
    /// A directory.
    Directory(PathBuf),
}

/// The absolute form of the path without `.` and `..` parts; does not
/// access the file system.
fn full_path(path: &Path) -> PathBuf {
    use std::path::Component;

    let absolute = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
    let mut result = PathBuf::new();
    for component in absolute.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                // The parent of a root is the root.
                if matches!(result.components().next_back(), Some(Component::Normal(_))) {
                    result.pop();
                }
            }
            other => result.push(other),
        }
    }
    result
}

impl FileSystemInfo {
    /// The file at `path`, which is made absolute.
    pub fn file(path: impl AsRef<Path>) -> Self {
        Self::File(full_path(path.as_ref()))
    }

    /// The directory at `path`, which is made absolute.
    pub fn directory(path: impl AsRef<Path>) -> Self {
        Self::Directory(full_path(path.as_ref()))
    }

    /// The full path of the entry.
    pub fn path(&self) -> &Path {
        match self {
            Self::File(path) | Self::Directory(path) => path,
        }
    }

    /// The full path of the entry as text (`FullName`).
    pub fn full_name(&self) -> String {
        self.path().to_string_lossy().into_owned()
    }

    /// The name of the entry: the last part of its path, or the whole path
    /// for a root.
    pub fn name(&self) -> String {
        match self.path().file_name() {
            Some(name) => name.to_string_lossy().into_owned(),
            None => self.full_name(),
        }
    }

    /// Whether the entry is a directory.
    pub fn is_directory(&self) -> bool {
        matches!(self, Self::Directory(_))
    }

    /// Whether the entry exists and is of the kind it is expected to be.
    pub fn exists(&self) -> bool {
        match self {
            Self::File(path) => path.is_file(),
            Self::Directory(path) => path.is_dir(),
        }
    }
}

fn to_date_time_offset(time: SystemTime) -> DateTimeOffset {
    const NANOSECONDS_PER_TICK: u128 = 100;

    let ticks = match time.duration_since(SystemTime::UNIX_EPOCH) {
        Ok(after) => (after.as_nanos() / NANOSECONDS_PER_TICK) as i64,
        Err(before) => -((before.duration().as_nanos() / NANOSECONDS_PER_TICK) as i64),
    };
    DateTimeOffset::UNIX_EPOCH.add(TimeSpan::from_ticks(ticks))
}

/// The base of the storage items backed by an entry of the local file
/// system: the entry, and the operations on it.
///
/// This is an implementation detail of the platform backends.
pub struct BclStorageItem {
    file_system_info: FileSystemInfo,
}

impl BclStorageItem {
    /// Creates the base of an item over a file-system entry.
    ///
    /// # Panics
    /// Panics when the entry is a directory that does not exist.
    pub fn new(file_system_info: FileSystemInfo) -> Self {
        if file_system_info.is_directory() && !file_system_info.exists() {
            panic!("Directory must exist (Parameter 'fileSystemInfo')");
        }
        Self { file_system_info }
    }

    /// The file-system entry of the item.
    pub fn file_system_info(&self) -> &FileSystemInfo {
        &self.file_system_info
    }

    pub fn name(&self) -> String {
        self.file_system_info.name()
    }

    pub fn can_bookmark(&self) -> bool {
        true
    }

    pub fn path(&self) -> Uri {
        Self::get_path_core(&self.file_system_info)
    }

    pub fn get_basic_properties(&self) -> StorageItemProperties {
        Self::get_basic_properties_async_core(&self.file_system_info)
    }

    pub fn get_parent(&self) -> Option<Rc<dyn IStorageFolder>> {
        Self::get_parent_core(&self.file_system_info)
            .and_then(|parent| Self::wrap_file_system_info(Some(parent)))
            .and_then(|parent| parent.as_storage_folder())
    }

    pub fn delete(&self) -> io::Result<()> {
        Self::delete_core(&self.file_system_info)
    }

    pub fn move_to(&self, destination: &dyn IStorageFolder) -> io::Result<Option<Rc<dyn IStorageItem>>> {
        Ok(Self::wrap_file_system_info(Self::move_core(&self.file_system_info, destination)?))
    }

    pub fn save_bookmark(&self) -> Option<String> {
        let path = self.file_system_info.full_name();
        Some(StorageBookmarkHelper::encode_bcl_bookmark(&path))
    }

    /// The storage item of a file-system entry: a folder for a directory,
    /// a file for a file.
    pub fn wrap_file_system_info(file_system_info: Option<FileSystemInfo>) -> Option<Rc<dyn IStorageItem>> {
        match file_system_info? {
            directory_info @ FileSystemInfo::Directory(_) => Some(BclStorageFolder::new(directory_info)),
            file_info @ FileSystemInfo::File(_) => Some(BclStorageFile::new(file_info)),
        }
    }

    pub fn delete_core(file_system_info: &FileSystemInfo) -> io::Result<()> {
        match file_system_info {
            FileSystemInfo::Directory(path) => fs::remove_dir_all(path),
            FileSystemInfo::File(path) => fs::remove_file(path),
        }
    }

    pub fn get_path_core(file_system_info: &FileSystemInfo) -> Uri {
        if file_system_info.path().parent().is_some() {
            return StorageProviderHelpers::uri_from_file_path(
                &file_system_info.full_name(),
                file_system_info.is_directory(),
            );
        }

        Uri::new(&file_system_info.name(), UriKind::Relative)
            .expect("the name of a file-system root is a valid relative URI")
    }

    pub fn get_basic_properties_async_core(file_system_info: &FileSystemInfo) -> StorageItemProperties {
        if file_system_info.exists() {
            if let Ok(metadata) = fs::metadata(file_system_info.path()) {
                let size = if file_system_info.is_directory() { 0 } else { metadata.len() };
                let date_modified = metadata.modified().ok().map(to_date_time_offset);
                // File systems without a creation time report the time of
                // the last change.
                let date_created = metadata.created().ok().map(to_date_time_offset).or(date_modified);
                return StorageItemProperties::new(Some(size), date_created, date_modified);
            }
        }

        StorageItemProperties::default()
    }

    pub fn get_parent_core(file_system_info: &FileSystemInfo) -> Option<FileSystemInfo> {
        file_system_info.path().parent().map(|parent| FileSystemInfo::Directory(parent.to_path_buf()))
    }

    pub fn move_core(
        file_system_info: &FileSystemInfo,
        destination: &dyn IStorageFolder,
    ) -> io::Result<Option<FileSystemInfo>> {
        let destination: &dyn IStorageItem = destination;
        let Some(destination_path) = destination.try_get_local_path() else {
            return Ok(None);
        };

        let new_path = Path::new(&destination_path).join(file_system_info.name());
        if new_path.exists() {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "Cannot create a file when that file already exists.",
            ));
        }

        match file_system_info {
            FileSystemInfo::Directory(path) => {
                fs::rename(path, &new_path)?;
                Ok(Some(FileSystemInfo::directory(new_path)))
            }
            FileSystemInfo::File(path) => {
                match fs::rename(path, &new_path) {
                    // A file is copied to another volume.
                    Err(error) if error.kind() == io::ErrorKind::CrossesDevices => {
                        fs::copy(path, &new_path)?;
                        fs::remove_file(path)?;
                    }
                    result => result?,
                }
                Ok(Some(FileSystemInfo::file(new_path)))
            }
        }
    }

    pub fn open_read_core(file_info: &Path) -> io::Result<fs::File> {
        fs::File::open(file_info)
    }

    pub fn open_write_core(file_info: &Path) -> io::Result<fs::File> {
        fs::File::create(file_info)
    }

    /// The entries of a directory: the directories, then the files.
    pub fn get_items_core(directory_info: &Path) -> io::Result<Vec<FileSystemInfo>> {
        let mut directories = Vec::new();
        let mut files = Vec::new();
        for entry in fs::read_dir(directory_info)? {
            let path = entry?.path();
            if path.is_dir() {
                directories.push(FileSystemInfo::Directory(path));
            } else {
                files.push(FileSystemInfo::File(path));
            }
        }
        directories.append(&mut files);
        Ok(directories)
    }

    pub fn get_folder_core(directory_info: &Path, name: &str) -> Option<FileSystemInfo> {
        let path = directory_info.join(name);
        path.is_dir().then(|| FileSystemInfo::directory(path))
    }

    pub fn get_file_core(directory_info: &Path, name: &str) -> Option<FileSystemInfo> {
        let path = directory_info.join(name);
        path.is_file().then(|| FileSystemInfo::file(path))
    }

    pub fn create_file_core(directory_info: &Path, name: &str) -> io::Result<FileSystemInfo> {
        let file_name = directory_info.join(name);
        fs::File::create(&file_name)?;
        Ok(FileSystemInfo::file(file_name))
    }

    pub fn create_folder_core(directory_info: &Path, name: &str) -> io::Result<FileSystemInfo> {
        let path = full_path(&directory_info.join(name));
        if !path.starts_with(directory_info) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("The directory specified, '{name}', is not a subdirectory of '{}'.", directory_info.display()),
            ));
        }
        fs::create_dir_all(&path)?;
        Ok(FileSystemInfo::Directory(path))
    }
}

/// Implements the storage item contracts of a class deriving from
/// [`BclStorageItem`] (a struct with the base in its `base` field), with
/// the given conversions to the file and folder contracts.
macro_rules! impl_bcl_storage_item {
    ($type_:ty, as_storage_file: $as_file:expr, as_storage_folder: $as_folder:expr) => {
        impl $crate::reactive::IDisposable for $type_ {
            fn dispose(&self) {}
        }

        impl $crate::platform::storage::IStorageItem for $type_ {
            fn name(&self) -> String {
                self.base.name()
            }

            fn path(&self) -> $crate::utilities::Uri {
                self.base.path()
            }

            fn get_basic_properties_async(
                &self,
            ) -> $crate::input::LocalBoxFuture<$crate::platform::storage::StorageItemProperties> {
                Box::pin(std::future::ready(self.base.get_basic_properties()))
            }

            fn can_bookmark(&self) -> bool {
                self.base.can_bookmark()
            }

            fn save_bookmark_async(&self) -> $crate::input::LocalBoxFuture<Option<String>> {
                Box::pin(std::future::ready(self.base.save_bookmark()))
            }

            fn get_parent_async(
                &self,
            ) -> $crate::input::LocalBoxFuture<Option<std::rc::Rc<dyn $crate::platform::storage::IStorageFolder>>> {
                Box::pin(std::future::ready(self.base.get_parent()))
            }

            fn delete_async(&self) -> $crate::input::LocalBoxFuture<std::io::Result<()>> {
                Box::pin(std::future::ready(self.base.delete()))
            }

            fn move_async(
                &self,
                destination: std::rc::Rc<dyn $crate::platform::storage::IStorageFolder>,
            ) -> $crate::input::LocalBoxFuture<
                std::io::Result<Option<std::rc::Rc<dyn $crate::platform::storage::IStorageItem>>>,
            > {
                Box::pin(std::future::ready(self.base.move_to(&*destination)))
            }

            fn as_storage_file(
                self: std::rc::Rc<Self>,
            ) -> Option<std::rc::Rc<dyn $crate::platform::storage::IStorageFile>> {
                let convert: fn(std::rc::Rc<Self>) -> Option<std::rc::Rc<dyn $crate::platform::storage::IStorageFile>> =
                    $as_file;
                convert(self)
            }

            fn as_storage_folder(
                self: std::rc::Rc<Self>,
            ) -> Option<std::rc::Rc<dyn $crate::platform::storage::IStorageFolder>> {
                let convert: fn(
                    std::rc::Rc<Self>,
                ) -> Option<std::rc::Rc<dyn $crate::platform::storage::IStorageFolder>> = $as_folder;
                convert(self)
            }

            fn as_storage_item_with_file_system_info(
                &self,
            ) -> Option<&dyn $crate::platform::storage::IStorageItemWithFileSystemInfo> {
                Some(self)
            }
        }

        impl $crate::platform::storage::IStorageItemWithFileSystemInfo for $type_ {
            fn file_system_info_full_name(&self) -> String {
                self.base.file_system_info().full_name()
            }
        }

        impl $crate::platform::storage::IStorageBookmarkItem for $type_ {
            fn release_bookmark_async(&self) -> $crate::input::LocalBoxFuture<()> {
                Box::pin(std::future::ready(()))
            }
        }
    };
}

pub(super) use impl_bcl_storage_item;
