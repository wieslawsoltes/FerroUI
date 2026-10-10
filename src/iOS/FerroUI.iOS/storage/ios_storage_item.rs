//! The storage items of iOS: a file or a folder behind a URL, read and
//! written inside the security scope of the URL the user opened it, or an
//! ancestor of it, with.

use ferroui_base::utilities::DateTimeOffset;
use ferroui_base::animation::TimeSpan;

/// The path of an item of a folder (`Path.Combine`): the name alone when
/// it is rooted, else the folder, a separator if it lacks one, and the
/// name.
pub fn combine(folder_path: &str, name: &str) -> String {
    if name.starts_with('/') || folder_path.is_empty() {
        return name.to_string();
    }
    if name.is_empty() {
        return folder_path.to_string();
    }
    if folder_path.ends_with('/') {
        format!("{folder_path}{name}")
    } else {
        format!("{folder_path}/{name}")
    }
}

/// The date of a number of seconds since 1970 (`NSDate`).
pub fn date_from_time_interval_since_1970(seconds: f64) -> DateTimeOffset {
    DateTimeOffset::UNIX_EPOCH.add(TimeSpan::from_ticks((seconds * 10_000_000.0) as i64))
}

#[cfg(target_os = "ios")]
pub use uikit::{create_item, url_of, IosStorageFile, IosStorageFolder, IosStorageItem};

#[cfg(target_os = "ios")]
mod uikit {
    use super::{combine, date_from_time_interval_since_1970};
    use crate::storage::ios_storage_provider::PLATFORM_KEY;
    use block2::RcBlock;
    use ferroui_base::input::LocalBoxFuture;
    use ferroui_base::logging::{LogArea, LogEventLevel, Logger};
    use ferroui_base::platform::storage::file_io::{SecurityScopedStream, StorageBookmarkHelper};
    use ferroui_base::platform::storage::{
        IStorageBookmarkFile, IStorageBookmarkFolder, IStorageBookmarkItem, IStorageFile, IStorageFolder,
        IStorageItem, StorageItemProperties, WellKnownFolder,
    };
    use ferroui_base::reactive::{Disposable, IDisposable};
    use ferroui_base::utilities::{Uri, UriKind};
    use objc2::rc::Retained;
    use objc2::runtime::Bool;
    use objc2::{msg_send, MainThreadMarker};
    use objc2_foundation::{
        NSData, NSDirectoryEnumerationOptions, NSError, NSFileCoordinator, NSFileCoordinatorReadingOptions,
        NSFileManager, NSString, NSURLBookmarkCreationOptions, NSURL,
    };
    use objc2_ui_kit::UIDocument;
    use std::any::Any;
    use std::cell::RefCell;
    use std::fs::{File, OpenOptions};
    use std::io::{self, Read, Write};
    use std::ptr::NonNull;
    use std::rc::Rc;

    /// The failure an error of Foundation stands for.
    pub(crate) fn ns_error(error: &NSError) -> io::Error {
        io::Error::other(format!("{} (code {})", error.localizedDescription(), error.code()))
    }

    fn ready<T: 'static>(value: T) -> LocalBoxFuture<T> {
        Box::pin(std::future::ready(value))
    }

    /// Runs `action` inside the security scopes of the given URLs.
    fn scoped<T>(urls: &[&NSURL], action: impl FnOnce() -> T) -> T {
        for url in urls {
            // SAFETY: a URL may always be asked for its security scope;
            // one without a scope answers that it has none.
            unsafe { url.startAccessingSecurityScopedResource() };
        }
        let result = action();
        for url in urls {
            // SAFETY: as above; the stop pairs with the start.
            unsafe { url.stopAccessingSecurityScopedResource() };
        }
        result
    }

    /// What a file and a folder have in common.
    pub struct IosStorageItem {
        url: Retained<NSURL>,
        // Starting to access the security-scoped resource of items that
        // were retrieved from, or created in, a folder fails, because only
        // folders opened directly with the folder picker of the storage
        // provider have security-scoped URLs. The URL of that ancestor is
        // kept so that there is recursive access to an opened folder.
        security_scoped_ancestor_url: Retained<NSURL>,
        file_path: String,
        name: String,
    }

    impl IosStorageItem {
        fn new(url: Retained<NSURL>, security_scoped_ancestor_url: Option<Retained<NSURL>>) -> Self {
            let security_scoped_ancestor_url = security_scoped_ancestor_url.unwrap_or_else(|| url.clone());

            let path_of = |url: &NSURL| url.filePathURL().and_then(|url| url.path()).map(|path| path.to_string());
            // A document knows the URL of its file and the name the
            // system shows for it; it is an object of the main thread.
            let file_name = |file_path: &str| {
                ferroui_base::platform::storage::file_io::path::get_file_name(file_path).to_string()
            };
            let (file_path, name) = match MainThreadMarker::new() {
                Some(mtm) => {
                    let doc = UIDocument::initWithFileURL(mtm.alloc(), &url);
                    // The headers declare the two properties as never
                    // null, and the bindings fail on a null one; the
                    // name of a document that was not opened is null
                    // (seen in the simulator, iOS 26.4), which the
                    // reference allows for. So both are read as optional.
                    // SAFETY: the two properties of a document, a URL
                    // and a string, or null.
                    let (file_url, localized_name): (Option<Retained<NSURL>>, Option<Retained<NSString>>) =
                        unsafe { (msg_send![&*doc, fileURL], msg_send![&*doc, localizedName]) };
                    let file_path = file_url
                        .and_then(|file_url| file_url.path())
                        .map(|path| path.to_string())
                        .or_else(|| path_of(&url))
                        .unwrap_or_default();
                    let name = localized_name.map(|name| name.to_string()).unwrap_or_else(|| file_name(&file_path));
                    (file_path, name)
                }
                None => {
                    let file_path = path_of(&url).unwrap_or_default();
                    let name = file_name(&file_path);
                    (file_path, name)
                }
            };

            Self { url, security_scoped_ancestor_url, file_path, name }
        }

        /// The URL of the item.
        pub fn url(&self) -> &Retained<NSURL> {
            &self.url
        }

        /// The URL whose security scope gives access to the item.
        pub fn security_scoped_ancestor_url(&self) -> &Retained<NSURL> {
            &self.security_scoped_ancestor_url
        }

        /// The path of the file of the item.
        pub fn file_path(&self) -> &str {
            &self.file_path
        }

        fn name(&self) -> String {
            self.name.clone()
        }

        fn path(&self) -> Uri {
            let absolute = self.url.absoluteString().map(|absolute| absolute.to_string()).unwrap_or_default();
            match Uri::try_create(&absolute, UriKind::Absolute) {
                Some(uri) => uri,
                None => match Uri::new(&absolute, UriKind::Relative) {
                    Ok(uri) => uri,
                    Err(error) => panic!("The URL of the storage item is not a URI: {error}"),
                },
            }
        }

        fn get_basic_properties_async(&self) -> LocalBoxFuture<StorageItemProperties> {
            let attributes = NSFileManager::defaultManager().attributesOfItemAtPath_error(&NSString::from_str(&self.file_path));
            let properties = match attributes {
                Err(error) => {
                    if let Some(logger) = Logger::try_get(LogEventLevel::Error, LogArea::IOS_PLATFORM) {
                        logger.log_with_values(
                            None,
                            "GetBasicPropertiesAsync returned an error: {ErrorCode} {ErrorMessage}",
                            &[&error.code(), &error.localizedDescription()],
                        );
                    }
                    StorageItemProperties::default()
                }
                Ok(attributes) => {
                    let date = |date: Option<Retained<objc2_foundation::NSDate>>| {
                        date.map(|date| date_from_time_interval_since_1970(date.timeIntervalSince1970()))
                    };
                    StorageItemProperties::new(
                        Some(attributes.fileSize()),
                        date(attributes.fileCreationDate()),
                        date(attributes.fileModificationDate()),
                    )
                }
            };

            ready(properties)
        }

        fn get_parent_async(&self) -> LocalBoxFuture<Option<Rc<dyn IStorageFolder>>> {
            let parent = self.url.URLByDeletingLastPathComponent().map(|parent| {
                IosStorageFolder::new(parent, Some(self.security_scoped_ancestor_url.clone())) as Rc<dyn IStorageFolder>
            });
            ready(parent)
        }

        fn delete_async(&self) -> LocalBoxFuture<io::Result<()>> {
            let result = scoped(&[&self.security_scoped_ancestor_url], || {
                NSFileManager::defaultManager().removeItemAtURL_error(&self.url).map_err(|error| ns_error(&error))
            });
            ready(result)
        }

        fn move_async(
            &self,
            destination: Rc<dyn IStorageFolder>,
            is_dir: bool,
        ) -> LocalBoxFuture<io::Result<Option<Rc<dyn IStorageItem>>>> {
            let folder = destination.as_any().and_then(|any| any.downcast_ref::<IosStorageFolder>());
            let Some(folder) = folder else {
                return ready(Err(io::Error::other(
                    "Destination folder must be initialized the StorageProvider API.",
                )));
            };

            let result = scoped(&[&self.security_scoped_ancestor_url, &folder.base.security_scoped_ancestor_url], || {
                let new_path = NSURL::fileURLWithPath_isDirectory(
                    &NSString::from_str(&combine(&folder.base.file_path, &self.name)),
                    is_dir,
                );

                NSFileManager::defaultManager()
                    .moveItemAtURL_toURL_error(&self.url, &new_path)
                    .map_err(|error| ns_error(&error))
                    .map(|()| {
                        Some(if is_dir {
                            IosStorageFolder::new(new_path, None) as Rc<dyn IStorageItem>
                        } else {
                            IosStorageFile::new(new_path, None) as Rc<dyn IStorageItem>
                        })
                    })
            });
            ready(result)
        }

        fn save_bookmark_async(&self) -> LocalBoxFuture<Option<String>> {
            // SAFETY: a URL may always be asked for its security scope.
            if !unsafe { self.security_scoped_ancestor_url.startAccessingSecurityScopedResource() } {
                return ready(None);
            }

            let new_bookmark = self.url.bookmarkDataWithOptions_includingResourceValuesForKeys_relativeToURL_error(
                NSURLBookmarkCreationOptions::SuitableForBookmarkFile,
                None,
                None,
            );
            let bookmark = match new_bookmark {
                Err(bookmark_error) => {
                    if let Some(logger) = Logger::try_get(LogEventLevel::Error, LogArea::IOS_PLATFORM) {
                        logger.log_with_values(
                            None,
                            "SaveBookmark returned an error: {ErrorCode} {ErrorMessage}",
                            &[&bookmark_error.code(), &bookmark_error.localizedDescription()],
                        );
                    }
                    None
                }
                Ok(new_bookmark) => StorageBookmarkHelper::encode_bookmark_bytes(PLATFORM_KEY, &new_bookmark.to_vec()),
            };

            // SAFETY: the stop pairs with the start above.
            unsafe { self.security_scoped_ancestor_url.stopAccessingSecurityScopedResource() };
            ready(bookmark)
        }
    }

    /// Creates the item of a URL: a folder for a URL with a directory
    /// path, else a file.
    pub fn create_item(url: Retained<NSURL>, security_scoped_ancestor_url: Option<Retained<NSURL>>) -> Rc<dyn IStorageItem> {
        if url.hasDirectoryPath() {
            IosStorageFolder::new(url, security_scoped_ancestor_url)
        } else {
            IosStorageFile::new(url, security_scoped_ancestor_url)
        }
    }

    /// The URL of a storage item of this platform; none for an item of
    /// another provider.
    pub fn url_of(item: &dyn IStorageItem) -> Option<Retained<NSURL>> {
        let any = item.as_any()?;
        if let Some(file) = any.downcast_ref::<IosStorageFile>() {
            return Some(file.base.url.clone());
        }
        any.downcast_ref::<IosStorageFolder>().map(|folder| folder.base.url.clone())
    }

    /// The members of a storage item that are those of the base item.
    macro_rules! storage_item {
        ($type:ident, $kind:tt) => {
            impl IDisposable for $type {
                fn dispose(&self) {}
            }

            impl IStorageItem for $type {
                fn name(&self) -> String {
                    self.base.name()
                }

                fn path(&self) -> Uri {
                    self.base.path()
                }

                fn get_basic_properties_async(&self) -> LocalBoxFuture<StorageItemProperties> {
                    self.base.get_basic_properties_async()
                }

                fn can_bookmark(&self) -> bool {
                    true
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
                    self.base.move_async(destination, storage_item!(@is_dir $kind))
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
                    // no-op
                    ready(())
                }
            }
        };
        (@is_dir file) => { false };
        (@is_dir folder) => { true };
        (@file file, $this:ident) => { Some($this) };
        (@file folder, $this:ident) => { None };
        (@folder folder, $this:ident) => { Some($this) };
        (@folder file, $this:ident) => { None };
    }

    /// A file.
    pub struct IosStorageFile {
        base: IosStorageItem,
    }

    impl IosStorageFile {
        /// Creates the file of a URL.
        pub fn new(url: Retained<NSURL>, security_scoped_ancestor_url: Option<Retained<NSURL>>) -> Rc<Self> {
            Rc::new(Self { base: IosStorageItem::new(url, security_scoped_ancestor_url) })
        }

        /// What the file has in common with a folder.
        pub fn item(&self) -> &IosStorageItem {
            &self.base
        }

        fn create_stream(&self, write: bool) -> io::Result<SecurityScopedStreamOrFile> {
            let path = self.base.file_path.clone();
            let ancestor = self.base.security_scoped_ancestor_url.clone();
            // SAFETY: a URL may always be asked for its security scope.
            let scope_created = unsafe { ancestor.startAccessingSecurityScopedResource() };

            let stream = if write {
                OpenOptions::new().write(true).create(true).truncate(true).open(&path)
            } else {
                File::open(&path)
            };
            let stream = match stream {
                Ok(stream) => stream,
                Err(error) => {
                    if scope_created {
                        // SAFETY: the stop pairs with the start above.
                        unsafe { ancestor.stopAccessingSecurityScopedResource() };
                    }
                    return Err(error);
                }
            };

            Ok(if scope_created {
                SecurityScopedStreamOrFile::Scoped(SecurityScopedStream::new(
                    stream,
                    Disposable::create(move || {
                        // SAFETY: the stop pairs with the start above.
                        unsafe { ancestor.stopAccessingSecurityScopedResource() };
                    }),
                ))
            } else {
                SecurityScopedStreamOrFile::File(stream)
            })
        }
    }

    /// The stream of a file: inside a security scope that ends with the
    /// stream, or the file itself when the URL has no scope.
    enum SecurityScopedStreamOrFile {
        Scoped(SecurityScopedStream),
        File(File),
    }

    impl Read for SecurityScopedStreamOrFile {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            match self {
                Self::Scoped(stream) => stream.read(buf),
                Self::File(stream) => stream.read(buf),
            }
        }
    }

    impl Write for SecurityScopedStreamOrFile {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            match self {
                Self::Scoped(stream) => stream.write(buf),
                Self::File(stream) => stream.write(buf),
            }
        }

        fn flush(&mut self) -> io::Result<()> {
            match self {
                Self::Scoped(stream) => stream.flush(),
                Self::File(stream) => stream.flush(),
            }
        }
    }

    storage_item!(IosStorageFile, file);

    impl IStorageFile for IosStorageFile {
        fn open_read_async(&self) -> LocalBoxFuture<io::Result<Box<dyn Read>>> {
            ready(self.create_stream(false).map(|stream| Box::new(stream) as Box<dyn Read>))
        }

        fn open_write_async(&self) -> LocalBoxFuture<io::Result<Box<dyn Write>>> {
            ready(self.create_stream(true).map(|stream| Box::new(stream) as Box<dyn Write>))
        }
    }

    impl IStorageBookmarkFile for IosStorageFile {}

    /// A folder.
    pub struct IosStorageFolder {
        base: IosStorageItem,
        well_known_folder: Option<WellKnownFolder>,
    }

    impl IosStorageFolder {
        /// Creates the folder of a URL.
        pub fn new(url: Retained<NSURL>, security_scoped_ancestor_url: Option<Retained<NSURL>>) -> Rc<Self> {
            Rc::new(Self { base: IosStorageItem::new(url, security_scoped_ancestor_url), well_known_folder: None })
        }

        /// Creates a well-known folder.
        pub fn new_well_known(url: Retained<NSURL>, well_known_folder: WellKnownFolder) -> Rc<Self> {
            Rc::new(Self { base: IosStorageItem::new(url, None), well_known_folder: Some(well_known_folder) })
        }

        /// What the folder has in common with a file.
        pub fn item(&self) -> &IosStorageItem {
            &self.base
        }

        /// The well-known folder this folder is, if it is one.
        pub fn well_known_folder(&self) -> Option<WellKnownFolder> {
            self.well_known_folder
        }

        fn get_item(&self, name: &str, is_directory: bool) -> Option<Retained<NSURL>> {
            scoped(&[&self.base.security_scoped_ancestor_url], || {
                let path = combine(&self.base.file_path, name);
                let mut is_directory = Bool::new(is_directory);
                // SAFETY: the pointer is to a variable of this function,
                // which lives for the call.
                let exists = unsafe {
                    NSFileManager::defaultManager().fileExistsAtPath_isDirectory(&NSString::from_str(&path), &mut is_directory)
                };
                exists.then(|| NSURL::fileURLWithPath_isDirectory(&NSString::from_str(&path), is_directory.as_bool()))
            })
        }
    }

    storage_item!(IosStorageFolder, folder);

    impl IStorageFolder for IosStorageFolder {
        fn get_items_async(&self) -> LocalBoxFuture<io::Result<Vec<Rc<dyn IStorageItem>>>> {
            let ancestor = self.base.security_scoped_ancestor_url.clone();
            let result = scoped(&[&ancestor], || {
                // TODO: find out if it can be lazily enumerated.
                let items: Rc<RefCell<Option<io::Result<Vec<Rc<dyn IStorageItem>>>>>> = Rc::new(RefCell::new(None));

                let sink = items.clone();
                let scope = ancestor.clone();
                let reader = RcBlock::new(move |uri: NonNull<NSURL>| {
                    // SAFETY: the URL is valid for the call of the
                    // accessor.
                    let uri = unsafe { uri.as_ref() };
                    let content = NSFileManager::defaultManager()
                        .contentsOfDirectoryAtURL_includingPropertiesForKeys_options_error(
                            uri,
                            None,
                            NSDirectoryEnumerationOptions(0),
                        );
                    *sink.borrow_mut() = Some(match content {
                        Err(error) => Err(ns_error(&error)),
                        Ok(content) => Ok(content.iter().map(|u| create_item(u, Some(scope.clone()))).collect()),
                    });
                });

                let mut error: Option<Retained<NSError>> = None;
                NSFileCoordinator::new().coordinateReadingItemAtURL_options_error_byAccessor(
                    &self.base.url,
                    NSFileCoordinatorReadingOptions::WithoutChanges,
                    Some(&mut error),
                    &reader,
                );

                if let Some(error) = error {
                    return Err(ns_error(&error));
                }

                // The accessor is called before the coordinated read
                // returns; without an error it was called.
                let items = items.borrow_mut().take();
                items.unwrap_or_else(|| Ok(Vec::new()))
            });
            ready(result)
        }

        fn get_folder_async(&self, name: &str) -> LocalBoxFuture<Option<Rc<dyn IStorageFolder>>> {
            let url = self.get_item(name, true);
            ready(url.map(|url| IosStorageFolder::new(url, None) as Rc<dyn IStorageFolder>))
        }

        fn get_file_async(&self, name: &str) -> LocalBoxFuture<Option<Rc<dyn IStorageFile>>> {
            let url = self.get_item(name, false);
            ready(url.map(|url| IosStorageFile::new(url, None) as Rc<dyn IStorageFile>))
        }

        fn create_file_async(&self, name: &str) -> LocalBoxFuture<io::Result<Option<Rc<dyn IStorageFile>>>> {
            let ancestor = &self.base.security_scoped_ancestor_url;
            let file = scoped(&[ancestor], || {
                let path = NSString::from_str(&combine(&self.base.file_path, name));
                // SAFETY: no attributes are passed, so none can be of a
                // wrong type.
                let created =
                    unsafe { NSFileManager::defaultManager().createFileAtPath_contents_attributes(&path, Some(&NSData::new()), None) };
                created.then(|| {
                    IosStorageFile::new(NSURL::fileURLWithPath_isDirectory(&path, false), Some(ancestor.clone()))
                        as Rc<dyn IStorageFile>
                })
            });
            ready(Ok(file))
        }

        fn create_folder_async(&self, name: &str) -> LocalBoxFuture<io::Result<Option<Rc<dyn IStorageFolder>>>> {
            let ancestor = &self.base.security_scoped_ancestor_url;
            let folder = scoped(&[ancestor], || {
                let path = NSString::from_str(&combine(&self.base.file_path, name));
                // SAFETY: no attributes are passed.
                let created = unsafe {
                    NSFileManager::defaultManager()
                        .createDirectoryAtPath_withIntermediateDirectories_attributes_error(&path, true, None)
                };
                created.map_err(|error| ns_error(&error)).map(|()| {
                    Some(IosStorageFolder::new(NSURL::fileURLWithPath_isDirectory(&path, true), Some(ancestor.clone()))
                        as Rc<dyn IStorageFolder>)
                })
            });
            ready(folder)
        }
    }

    impl IStorageBookmarkFolder for IosStorageFolder {}
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests of this file.
    use super::*;

    #[test]
    fn a_name_is_combined_with_the_path_of_its_folder() {
        assert_eq!("/a/b/c.txt", combine("/a/b", "c.txt"));
        assert_eq!("/a/b/c.txt", combine("/a/b/", "c.txt"));
        assert_eq!("/c.txt", combine("/a/b", "/c.txt"));
        assert_eq!("c.txt", combine("", "c.txt"));
        assert_eq!("/a/b", combine("/a/b", ""));
    }

    #[test]
    fn a_date_of_foundation_is_a_date_of_the_framework() {
        assert_eq!(DateTimeOffset::UNIX_EPOCH, date_from_time_interval_since_1970(0.0));
        let date = date_from_time_interval_since_1970(86_400.5);
        assert_eq!(DateTimeOffset::UNIX_EPOCH.add(TimeSpan::from_ticks(864_005_000_000)), date);
    }
}
