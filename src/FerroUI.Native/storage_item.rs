use crate::storage_provider_api::StorageProviderApi;
use ferroui_base::input::LocalBoxFuture;
use ferroui_base::platform::storage::file_io::{BclStorageItem, FileSystemInfo, SecurityScopedStream};
use ferroui_base::platform::storage::{
    IStorageBookmarkFile, IStorageBookmarkFolder, IStorageBookmarkItem, IStorageFile, IStorageFolder, IStorageItem,
    IStorageItemWithFileSystemInfo, StorageItemProperties,
};
use ferroui_base::reactive::IDisposable;
use ferroui_base::utilities::Uri;
use std::any::Any;
use std::io::{self, Read, Write};
use std::rc::Rc;

/// The state every storage item of this backend has: the entry of the file
/// system, its URI, and the URI whose security scope gives access to it.
pub(crate) struct StorageItem {
    storage_provider_api: Rc<StorageProviderApi>,
    file_system_info: FileSystemInfo,
    path: Uri,
    scope_owner_uri: Uri,
}

/// A file-system entry wrapped as the storage item of its kind.
enum Wrapped {
    File(Rc<StorageFile>),
    Folder(Rc<StorageFolder>),
}

impl Wrapped {
    fn into_item(self) -> Rc<dyn IStorageItem> {
        match self {
            Wrapped::File(file) => file,
            Wrapped::Folder(folder) => folder,
        }
    }

    fn into_file(self) -> Option<Rc<dyn IStorageFile>> {
        match self {
            Wrapped::File(file) => Some(file),
            Wrapped::Folder(_) => None,
        }
    }

    fn into_folder(self) -> Option<Rc<dyn IStorageFolder>> {
        match self {
            Wrapped::File(_) => None,
            Wrapped::Folder(folder) => Some(folder),
        }
    }
}

impl StorageItem {
    fn new(
        storage_provider_api: Rc<StorageProviderApi>,
        file_system_info: FileSystemInfo,
        uri: Uri,
        scope_owner_uri: Uri,
    ) -> Self {
        Self { storage_provider_api, file_system_info, path: uri, scope_owner_uri }
    }

    fn name(&self) -> String {
        self.file_system_info.name()
    }

    fn path(&self) -> Uri {
        self.path.clone()
    }

    pub(crate) fn scope_owner_uri(&self) -> &Uri {
        &self.scope_owner_uri
    }

    fn get_basic_properties(&self) -> StorageItemProperties {
        let scope = self.open_scope();
        let properties = BclStorageItem::get_basic_properties_async_core(&self.file_system_info);
        dispose(scope);
        properties
    }

    pub(crate) fn file_system_info(&self) -> &FileSystemInfo {
        &self.file_system_info
    }

    pub(crate) fn open_scope(&self) -> Option<Rc<dyn IDisposable>> {
        self.storage_provider_api.open_security_scope(self.scope_owner_uri.absolute_uri())
    }

    fn save_bookmark(&self) -> Option<String> {
        let scope = self.open_scope();
        let bookmark = self.storage_provider_api.save_bookmark(&self.path);
        dispose(scope);
        bookmark
    }

    fn release_bookmark(&self) {
        self.storage_provider_api.release_bookmark(&self.path);
    }

    fn get_parent(&self) -> Option<Rc<dyn IStorageFolder>> {
        let scope = self.open_scope();
        let parent = BclStorageItem::get_parent_core(&self.file_system_info);
        let parent = self.wrap_file_system_info(parent, None).and_then(Wrapped::into_folder);
        dispose(scope);
        parent
    }

    fn delete(&self) -> io::Result<()> {
        let scope = self.open_scope();
        let result = BclStorageItem::delete_core(&self.file_system_info);
        dispose(scope);
        result
    }

    fn move_to(&self, destination: &dyn IStorageFolder) -> io::Result<Option<Rc<dyn IStorageItem>>> {
        let destination_scope = destination
            .as_any()
            .and_then(|destination| destination.downcast_ref::<StorageFolder>())
            .and_then(|destination| destination.base.open_scope());
        let scope = self.open_scope();
        let moved = BclStorageItem::move_core(&self.file_system_info, destination);
        dispose(scope);
        dispose(destination_scope);
        Ok(self.wrap_file_system_info(moved?, None).map(Wrapped::into_item))
    }

    fn wrap_file_system_info(&self, file_system_info: Option<FileSystemInfo>, scoped_owner: Option<&Uri>) -> Option<Wrapped> {
        let file_system_info = file_system_info?;

        // It might not be always correct to assume NSUri from the file path, but that's the best we have here without using native API directly.
        let file_uri = BclStorageItem::get_path_core(&file_system_info);
        let scope_owner = scoped_owner.cloned().unwrap_or_else(|| file_uri.clone());
        Some(if file_system_info.is_directory() {
            Wrapped::Folder(StorageFolder::new(
                self.storage_provider_api.clone(),
                file_system_info,
                file_uri,
                scope_owner,
            ))
        } else {
            Wrapped::File(StorageFile::new(self.storage_provider_api.clone(), file_system_info, file_uri, scope_owner))
        })
    }
}

/// Ends a security scope (the end of a `using` block of the reference).
fn dispose(scope: Option<Rc<dyn IDisposable>>) {
    if let Some(scope) = scope {
        scope.dispose();
    }
}

/// Implements the storage item contracts of a class deriving from
/// [`StorageItem`] (a struct with the base in its `base` field), with the
/// given conversions to the file and folder contracts.
macro_rules! impl_storage_item {
    ($type_:ty, as_storage_file: $as_file:expr, as_storage_folder: $as_folder:expr) => {
        impl IDisposable for $type_ {
            fn dispose(&self) {}
        }

        impl IStorageItem for $type_ {
            fn name(&self) -> String {
                self.base.name()
            }

            fn path(&self) -> Uri {
                self.base.path()
            }

            fn get_basic_properties_async(&self) -> LocalBoxFuture<StorageItemProperties> {
                Box::pin(std::future::ready(self.base.get_basic_properties()))
            }

            fn can_bookmark(&self) -> bool {
                true
            }

            fn save_bookmark_async(&self) -> LocalBoxFuture<Option<String>> {
                Box::pin(std::future::ready(self.base.save_bookmark()))
            }

            fn get_parent_async(&self) -> LocalBoxFuture<Option<Rc<dyn IStorageFolder>>> {
                Box::pin(std::future::ready(self.base.get_parent()))
            }

            fn delete_async(&self) -> LocalBoxFuture<io::Result<()>> {
                Box::pin(std::future::ready(self.base.delete()))
            }

            fn move_async(
                &self,
                destination: Rc<dyn IStorageFolder>,
            ) -> LocalBoxFuture<io::Result<Option<Rc<dyn IStorageItem>>>> {
                Box::pin(std::future::ready(self.base.move_to(&*destination)))
            }

            fn as_storage_file(self: Rc<Self>) -> Option<Rc<dyn IStorageFile>> {
                let convert: fn(Rc<Self>) -> Option<Rc<dyn IStorageFile>> = $as_file;
                convert(self)
            }

            fn as_storage_folder(self: Rc<Self>) -> Option<Rc<dyn IStorageFolder>> {
                let convert: fn(Rc<Self>) -> Option<Rc<dyn IStorageFolder>> = $as_folder;
                convert(self)
            }

            fn as_storage_item_with_file_system_info(&self) -> Option<&dyn IStorageItemWithFileSystemInfo> {
                Some(self)
            }

            fn as_any(&self) -> Option<&dyn Any> {
                Some(self)
            }
        }

        impl IStorageItemWithFileSystemInfo for $type_ {
            fn file_system_info_full_name(&self) -> String {
                self.base.file_system_info().full_name()
            }
        }

        impl IStorageBookmarkItem for $type_ {
            fn release_bookmark_async(&self) -> LocalBoxFuture<()> {
                self.base.release_bookmark();
                Box::pin(std::future::ready(()))
            }
        }
    };
}

/// A file reached through the storage provider of this backend, in a
/// sandboxed application: every access is made inside the security scope of
/// the item the user picked.
pub(crate) struct StorageFile {
    base: StorageItem,
}

impl StorageFile {
    pub(crate) fn new(
        storage_provider_api: Rc<StorageProviderApi>,
        file_info: FileSystemInfo,
        uri: Uri,
        scope_owner_uri: Uri,
    ) -> Rc<Self> {
        Rc::new(Self { base: StorageItem::new(storage_provider_api, file_info, uri, scope_owner_uri) })
    }
}

impl_storage_item!(StorageFile, as_storage_file: |this| Some(this), as_storage_folder: |_| None);

impl IStorageFile for StorageFile {
    fn open_read_async(&self) -> LocalBoxFuture<io::Result<Box<dyn Read>>> {
        let scope = self.base.open_scope();
        let result = BclStorageItem::open_read_core(self.base.file_system_info().path()).map(|inner_stream| match scope {
            Some(scope) => Box::new(SecurityScopedStream::new(inner_stream, scope)) as Box<dyn Read>,
            None => Box::new(inner_stream),
        });
        Box::pin(std::future::ready(result))
    }

    fn open_write_async(&self) -> LocalBoxFuture<io::Result<Box<dyn Write>>> {
        let scope = self.base.open_scope();
        let result = BclStorageItem::open_write_core(self.base.file_system_info().path()).map(|inner_stream| match scope {
            Some(scope) => Box::new(SecurityScopedStream::new(inner_stream, scope)) as Box<dyn Write>,
            None => Box::new(inner_stream),
        });
        Box::pin(std::future::ready(result))
    }
}

impl IStorageBookmarkFile for StorageFile {}

/// A folder reached through the storage provider of this backend, in a
/// sandboxed application. Its items share its security scope.
pub(crate) struct StorageFolder {
    base: StorageItem,
}

impl StorageFolder {
    pub(crate) fn new(
        storage_provider_api: Rc<StorageProviderApi>,
        directory_info: FileSystemInfo,
        uri: Uri,
        scope_owner_uri: Uri,
    ) -> Rc<Self> {
        Rc::new(Self { base: StorageItem::new(storage_provider_api, directory_info, uri, scope_owner_uri) })
    }

    fn directory_info(&self) -> &std::path::Path {
        self.base.file_system_info().path()
    }
}

impl_storage_item!(StorageFolder, as_storage_file: |_| None, as_storage_folder: |this| Some(this));

impl IStorageFolder for StorageFolder {
    fn get_items_async(&self) -> LocalBoxFuture<io::Result<Vec<Rc<dyn IStorageItem>>>> {
        let scope = self.base.open_scope();
        let items = BclStorageItem::get_items_core(self.directory_info()).map(|items| {
            items
                .into_iter()
                .filter_map(|item| self.base.wrap_file_system_info(Some(item), Some(self.base.scope_owner_uri())))
                .map(Wrapped::into_item)
                .collect()
        });
        dispose(scope);
        Box::pin(std::future::ready(items))
    }

    fn get_folder_async(&self, name: &str) -> LocalBoxFuture<Option<Rc<dyn IStorageFolder>>> {
        let scope = self.base.open_scope();
        let item = BclStorageItem::get_folder_core(self.directory_info(), name);
        let folder =
            self.base.wrap_file_system_info(item, Some(self.base.scope_owner_uri())).and_then(Wrapped::into_folder);
        dispose(scope);
        Box::pin(std::future::ready(folder))
    }

    fn get_file_async(&self, name: &str) -> LocalBoxFuture<Option<Rc<dyn IStorageFile>>> {
        let scope = self.base.open_scope();
        let item = BclStorageItem::get_file_core(self.directory_info(), name);
        let file = self.base.wrap_file_system_info(item, Some(self.base.scope_owner_uri())).and_then(Wrapped::into_file);
        dispose(scope);
        Box::pin(std::future::ready(file))
    }

    fn create_file_async(&self, name: &str) -> LocalBoxFuture<io::Result<Option<Rc<dyn IStorageFile>>>> {
        let scope = self.base.open_scope();
        let file = BclStorageItem::create_file_core(self.directory_info(), name).map(|file| {
            self.base.wrap_file_system_info(Some(file), Some(self.base.scope_owner_uri())).and_then(Wrapped::into_file)
        });
        dispose(scope);
        Box::pin(std::future::ready(file))
    }

    fn create_folder_async(&self, name: &str) -> LocalBoxFuture<io::Result<Option<Rc<dyn IStorageFolder>>>> {
        let scope = self.base.open_scope();
        let folder = BclStorageItem::create_folder_core(self.directory_info(), name).map(|folder| {
            self.base.wrap_file_system_info(Some(folder), Some(self.base.scope_owner_uri())).and_then(Wrapped::into_folder)
        });
        dispose(scope);
        Box::pin(std::future::ready(folder))
    }
}

impl IStorageBookmarkFolder for StorageFolder {}
