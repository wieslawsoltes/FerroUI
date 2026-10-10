//! The storage provider of iOS: the document pickers of the system for
//! opening files and folders and for saving a file, the image picker for
//! a single picture, bookmarks of security-scoped URLs, and the folders
//! of an application the system knows.

use ferroui_base::platform::storage::{FilePickerFileType, WellKnownFolder};
use std::rc::Rc;

/// The platform key of the bookmarks of this provider.
pub const PLATFORM_KEY: &[u8] = b"ios";

/// How a uniform type of a file type filter is found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UniformTypeSource {
    /// By a file name extension, without its dot.
    Extension(String),
    /// By a uniform type identifier.
    Identifier(String),
    /// By a MIME type.
    MimeType(String),
}

/// The uniform types of a file type filter: per file type its extensions
/// when it has any, else its uniform type identifiers, else its MIME
/// types. `None` without a filter, for which the picker shows content,
/// items and data.
pub fn file_types_to_uniform_type_sources(
    file_picker_file_types: Option<&[Rc<FilePickerFileType>]>,
) -> Option<Vec<UniformTypeSource>> {
    let file_picker_file_types = file_picker_file_types?;
    let mut sources = Vec::new();
    for f in file_picker_file_types {
        if let Some(extensions) = f.try_get_extensions().filter(|extensions| !extensions.is_empty()) {
            sources.extend(extensions.into_iter().map(UniformTypeSource::Extension));
        } else if let Some(identifiers) = f.apple_uniform_type_identifiers().filter(|identifiers| !identifiers.is_empty()) {
            sources.extend(identifiers.iter().cloned().map(UniformTypeSource::Identifier));
        } else if let Some(mime_types) = f.mime_types().filter(|mime_types| !mime_types.is_empty()) {
            sources.extend(mime_types.iter().cloned().map(UniformTypeSource::MimeType));
        }
    }
    Some(sources)
}

/// The `NSSearchPathDirectory` of a well-known folder; none for a folder
/// iOS has no directory for.
pub fn search_path_directory(well_known_folder: WellKnownFolder) -> Option<usize> {
    match well_known_folder {
        WellKnownFolder::Desktop => Some(12),
        WellKnownFolder::Documents => Some(9),
        WellKnownFolder::Downloads => Some(15),
        WellKnownFolder::Music => Some(18),
        WellKnownFolder::Pictures => Some(19),
        WellKnownFolder::Videos => Some(17),
        #[allow(unreachable_patterns)]
        _ => None,
    }
}

#[cfg(target_os = "ios")]
pub(crate) use uikit::{is_text_uniform_type, IosStorageProvider};

#[cfg(target_os = "ios")]
mod uikit {
    use super::{file_types_to_uniform_type_sources, search_path_directory, UniformTypeSource, PLATFORM_KEY};
    use crate::completion::{Completion, CompletionFuture};
    use crate::ferro_view::FerroView;
    use crate::storage::ios_storage_item::{url_of, IosStorageFile, IosStorageFolder};
    use ferroui_base::input::LocalBoxFuture;
    use ferroui_base::logging::{LogArea, LogEventLevel, Logger};
    use ferroui_base::platform::storage::file_io::{DecodeResult, StorageBookmarkHelper, StorageProviderHelpers};
    use ferroui_base::platform::storage::{
        FilePickerOpenOptions, FilePickerSaveOptions, FolderPickerOpenOptions, IStorageBookmarkFile,
        IStorageBookmarkFolder, IStorageFile, IStorageFolder, IStorageProvider, OpenFilePickerResult,
        SaveFilePickerResult, WellKnownFolder,
    };
    use ferroui_base::utilities::Uri;
    use objc2::rc::{Retained, Weak};
    use objc2::runtime::{AnyClass, AnyObject, Bool, NSObject, ProtocolObject};
    use objc2::{define_class, msg_send, AnyThread, DefinedClass, MainThreadMarker, MainThreadOnly, Message};
    use objc2_foundation::{
        NSArray, NSData, NSDataBase64DecodingOptions, NSDictionary, NSFileManager, NSObjectProtocol,
        NSSearchPathDirectory, NSSearchPathDomainMask, NSString, NSURLBookmarkResolutionOptions, NSURL, NSUUID,
    };
    use objc2_ui_kit::{
        UIAdaptivePresentationControllerDelegate, UIDocumentPickerDelegate, UIDocumentPickerViewController,
        UIImagePickerController, UIImagePickerControllerDelegate, UIImagePickerControllerImageURL,
        UIImagePickerControllerInfoKey, UIImagePickerControllerSourceType, UINavigationControllerDelegate,
        UIPresentationController, UIViewController,
    };
    use std::io;
    use std::rc::Rc;

    const _: () = {
        assert!(NSSearchPathDirectory::DesktopDirectory.0 == 12);
        assert!(NSSearchPathDirectory::DocumentDirectory.0 == 9);
        assert!(NSSearchPathDirectory::DownloadsDirectory.0 == 15);
        assert!(NSSearchPathDirectory::MusicDirectory.0 == 18);
        assert!(NSSearchPathDirectory::PicturesDirectory.0 == 19);
        assert!(NSSearchPathDirectory::MoviesDirectory.0 == 17);
    };

    // The uniform types of the system (the UniformTypeIdentifiers
    // framework, iOS 14): the class is found by its name and the
    // constants are declared here, as `interop.rs` declares the functions
    // of Core Foundation.
    #[link(name = "UniformTypeIdentifiers", kind = "framework")]
    extern "C" {
        static UTTypeContent: &'static AnyObject;
        static UTTypeItem: &'static AnyObject;
        static UTTypeData: &'static AnyObject;
        static UTTypeFolder: &'static AnyObject;
        static UTTypeText: &'static AnyObject;
    }

    fn ut_type_class() -> Option<&'static AnyClass> {
        AnyClass::get(c"UTType")
    }

    fn uniform_type(source: &UniformTypeSource) -> Option<Retained<AnyObject>> {
        let class = ut_type_class()?;
        // SAFETY: the three class methods of `UTType` take a string and
        // return a type or nothing.
        unsafe {
            match source {
                UniformTypeSource::Extension(extension) => {
                    msg_send![class, typeWithFilenameExtension: &*NSString::from_str(extension)]
                }
                UniformTypeSource::Identifier(identifier) => {
                    msg_send![class, typeWithIdentifier: &*NSString::from_str(identifier)]
                }
                UniformTypeSource::MimeType(mime_type) => msg_send![class, typeWithMIMEType: &*NSString::from_str(mime_type)],
            }
        }
    }

    /// Whether the uniform type of an identifier is text. A best effort:
    /// an identifier the system does not know is not.
    pub(crate) fn is_text_uniform_type(identifier: &str) -> bool {
        let Some(ut_type) = uniform_type(&UniformTypeSource::Identifier(identifier.to_string())) else {
            return false;
        };
        // SAFETY: `conformsToType:` of a `UTType` takes a `UTType`, here
        // a constant of the framework, and returns a boolean.
        let conforms: Bool = unsafe { msg_send![&*ut_type, conformsToType: UTTypeText] };
        conforms.as_bool()
    }

    type Urls = Vec<Retained<NSURL>>;

    define_class!(
        // SAFETY: `NSObject` may be subclassed; the class does not
        // implement `Drop`. The methods have the signatures the protocol
        // declares.
        #[unsafe(super(NSObject))]
        #[thread_kind = MainThreadOnly]
        #[name = "FerroPickerDelegate"]
        #[ivars = Completion<Urls>]
        struct PickerDelegate;

        unsafe impl NSObjectProtocol for PickerDelegate {}

        unsafe impl UIDocumentPickerDelegate for PickerDelegate {
            #[unsafe(method(documentPickerWasCancelled:))]
            fn was_cancelled(&self, _controller: &UIDocumentPickerViewController) {
                self.ivars().try_set_result(Vec::new());
            }

            #[unsafe(method(documentPicker:didPickDocumentsAtURLs:))]
            fn did_pick_documents(&self, _controller: &UIDocumentPickerViewController, urls: &NSArray<NSURL>) {
                self.ivars().try_set_result(urls.iter().collect());
            }

            #[unsafe(method(documentPicker:didPickDocumentAtURL:))]
            fn did_pick_document(&self, _controller: &UIDocumentPickerViewController, url: &NSURL) {
                self.ivars().try_set_result(vec![url.retain()]);
            }
        }
    );

    define_class!(
        // SAFETY: as for the picker delegate above.
        #[unsafe(super(NSObject))]
        #[thread_kind = MainThreadOnly]
        #[name = "FerroImageOpenPickerDelegate"]
        #[ivars = Completion<Urls>]
        struct ImageOpenPickerDelegate;

        unsafe impl NSObjectProtocol for ImageOpenPickerDelegate {}

        unsafe impl UINavigationControllerDelegate for ImageOpenPickerDelegate {}

        unsafe impl UIImagePickerControllerDelegate for ImageOpenPickerDelegate {
            #[unsafe(method(imagePickerControllerDidCancel:))]
            fn canceled(&self, _picker: &UIImagePickerController) {
                self.ivars().try_set_result(Vec::new());
            }

            #[unsafe(method(imagePickerController:didFinishPickingMediaWithInfo:))]
            fn finished_picking_media(
                &self,
                _picker: &UIImagePickerController,
                info: &NSDictionary<UIImagePickerControllerInfoKey, AnyObject>,
            ) {
                // SAFETY: the key is a constant of UIKit.
                let url = info.objectForKey(unsafe { UIImagePickerControllerImageURL });
                match url.and_then(|url| url.downcast::<NSURL>().ok()) {
                    Some(ns_url) => self.ivars().try_set_result(vec![ns_url]),
                    None => self.ivars().try_set_result(Vec::new()),
                };
            }
        }
    );

    define_class!(
        // SAFETY: as for the picker delegate above.
        #[unsafe(super(NSObject))]
        #[thread_kind = MainThreadOnly]
        #[name = "FerroPresentationControllerDelegate"]
        #[ivars = Completion<Urls>]
        struct UIPresentationControllerDelegate;

        unsafe impl NSObjectProtocol for UIPresentationControllerDelegate {}

        unsafe impl UIAdaptivePresentationControllerDelegate for UIPresentationControllerDelegate {
            #[unsafe(method(presentationControllerDidDismiss:))]
            fn did_dismiss(&self, _presentation_controller: &UIPresentationController) {
                // The picker was dismissed without a choice. A choice
                // that was made stays: the result is set once.
                self.ivars().try_set_result(Vec::new());
            }
        }
    );

    macro_rules! delegate_new {
        ($type:ident) => {
            impl $type {
                fn new(mtm: MainThreadMarker, completion: Completion<Urls>) -> Retained<Self> {
                    let this = mtm.alloc::<Self>().set_ivars(completion);
                    // SAFETY: `init` of the superclass, on the object
                    // that was just allocated and whose instance
                    // variables are set.
                    unsafe { msg_send![super(this), init] }
                }
            }
        };
    }

    delegate_new!(PickerDelegate);
    delegate_new!(ImageOpenPickerDelegate);
    delegate_new!(UIPresentationControllerDelegate);

    /// The delegates of a picker that is shown. UIKit refers to its
    /// delegates weakly, so they live with the future that waits for the
    /// choice.
    struct ShownPicker {
        urls: CompletionFuture<Urls>,
        _picker: Retained<UIViewController>,
        _delegates: Vec<Retained<AnyObject>>,
    }

    fn error<T: 'static>(message: &str) -> LocalBoxFuture<io::Result<T>> {
        Box::pin(std::future::ready(Err(io::Error::other(message.to_string()))))
    }

    /// The storage provider of a view.
    pub struct IosStorageProvider {
        view: Weak<FerroView>,
    }

    impl IosStorageProvider {
        /// Creates the storage provider of `view`.
        pub fn new(view: Weak<FerroView>) -> Rc<Self> {
            Rc::new(Self { view })
        }

        fn mtm() -> io::Result<MainThreadMarker> {
            MainThreadMarker::new().ok_or_else(|| io::Error::other("The pickers are shown from the main thread."))
        }

        fn get_url_from_folder(folder: Option<&Rc<dyn IStorageFolder>>) -> Option<Retained<NSURL>> {
            let folder = folder?;
            url_of(&**folder).or_else(|| NSURL::URLWithString(&NSString::from_str(folder.path().absolute_uri())))
        }

        fn file_types_to_ut_type(options: &FilePickerOpenOptions) -> Retained<NSArray<AnyObject>> {
            match file_types_to_uniform_type_sources(options.file_type_filter()) {
                Some(sources) => {
                    let types: Vec<Retained<AnyObject>> = sources.iter().filter_map(uniform_type).collect();
                    NSArray::from_retained_slice(&types)
                }
                // SAFETY: the three constants of the framework.
                None => NSArray::from_slice(unsafe { &[UTTypeContent, UTTypeItem, UTTypeData] }),
            }
        }

        fn document_picker_for_opening(
            mtm: MainThreadMarker,
            content_types: &NSArray<AnyObject>,
        ) -> Retained<UIDocumentPickerViewController> {
            // SAFETY: the initializer of the document picker for opening
            // takes an array of `UTType` objects, which the array holds,
            // and whether to copy.
            unsafe {
                msg_send![
                    mtm.alloc::<UIDocumentPickerViewController>(),
                    initForOpeningContentTypes: content_types,
                    asCopy: false
                ]
            }
        }

        fn show_document_picker(&self, document_picker: &UIDocumentPickerViewController) -> io::Result<ShownPicker> {
            let mtm = Self::mtm()?;
            let (tcs, urls) = Completion::new();
            let delegate = PickerDelegate::new(mtm, tcs.clone());
            document_picker.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));
            let mut shown = self.show_picker(document_picker, tcs, urls)?;
            shown._delegates.push(Retained::into_super(Retained::into_super(delegate)));
            Ok(shown)
        }

        fn show_picker(
            &self,
            picker: &UIViewController,
            tcs: Completion<Urls>,
            urls: CompletionFuture<Urls>,
        ) -> io::Result<ShownPicker> {
            let mtm = Self::mtm()?;
            let mut delegates: Vec<Retained<AnyObject>> = Vec::new();
            if let Some(presentation_controller) = picker.presentationController() {
                let delegate = UIPresentationControllerDelegate::new(mtm, tcs);
                // SAFETY: the delegate implements the protocol of the
                // property; the controller refers to it weakly, and it
                // lives with the future that waits for the choice.
                unsafe { presentation_controller.setDelegate(Some(ProtocolObject::from_ref(&*delegate))) };
                delegates.push(Retained::into_super(Retained::into_super(delegate)));
            }

            let controller = self.view.load().and_then(|view| view.window()).and_then(|window| window.rootViewController());
            let Some(controller) = controller else {
                return Err(io::Error::other("RootViewController wasn't initialized"));
            };
            controller.presentViewController_animated_completion(picker, true, None);

            Ok(ShownPicker { urls, _picker: picker.retain(), _delegates: delegates })
        }

        fn open_image_picker_async(&self, options: &FilePickerOpenOptions) -> io::Result<ShownPicker> {
            let mtm = Self::mtm()?;
            let image_picker = UIImagePickerController::new(mtm);
            // Deprecated for the photo picker of PhotosUI, which the
            // reference does not use either.
            #[allow(deprecated)]
            image_picker.setSourceType(UIImagePickerControllerSourceType::PhotoLibrary);
            image_picker.setMediaTypes(&NSArray::from_retained_slice(&[NSString::from_str("public.image")]));
            image_picker.setAllowsEditing(false);
            image_picker.setTitle(options.title().map(NSString::from_str).as_deref());

            let (tcs, urls) = Completion::new();
            let delegate = ImageOpenPickerDelegate::new(mtm, tcs.clone());
            let delegate_object: &AnyObject = &delegate;
            // SAFETY: the delegate implements the two protocols the
            // property asks for.
            unsafe { image_picker.setDelegate(Some(delegate_object)) };
            let mut shown = self.show_picker(&image_picker, tcs, urls)?;
            shown._delegates.push(Retained::into_super(Retained::into_super(delegate)));
            Ok(shown)
        }

        fn open_documents_picker_async(&self, options: &FilePickerOpenOptions) -> io::Result<ShownPicker> {
            let mtm = Self::mtm()?;
            let allowed_types = Self::file_types_to_ut_type(options);
            let document_picker = Self::document_picker_for_opening(mtm, &allowed_types);

            document_picker.setDirectoryURL(Self::get_url_from_folder(options.suggested_start_location()).as_deref());
            document_picker.setAllowsMultipleSelection(options.allow_multiple());
            document_picker.setTitle(options.title().map(NSString::from_str).as_deref());

            self.show_document_picker(&document_picker)
        }

        fn get_bookmarked_url(&self, bookmark: &str) -> Option<Retained<NSURL>> {
            let decode_from_ns_data = |ns_data: &NSData| -> Option<Retained<NSURL>> {
                let mut is_stale = Bool::NO;
                // SAFETY: the pointer is to a variable of this closure,
                // which lives for the call.
                let url = unsafe {
                    NSURL::URLByResolvingBookmarkData_options_relativeToURL_bookmarkDataIsStale_error(
                        ns_data,
                        NSURLBookmarkResolutionOptions::WithoutUI,
                        None,
                        &mut is_stale,
                    )
                };
                if is_stale.as_bool() {
                    if let Some(logger) = Logger::try_get(LogEventLevel::Warning, LogArea::IOS_PLATFORM) {
                        logger.log(None, "Stale bookmark detected");
                    }
                }

                match url {
                    Ok(url) => Some(url),
                    Err(error) => {
                        // The reference fails with the error; the
                        // contract of the port answers with no item.
                        if let Some(logger) = Logger::try_get(LogEventLevel::Error, LogArea::IOS_PLATFORM) {
                            logger.log_with_values(None, "The bookmark was not resolved: {Error}", &[&error.localizedDescription()]);
                        }
                        None
                    }
                }
            };

            match StorageBookmarkHelper::try_decode_bookmark(PLATFORM_KEY, Some(bookmark)) {
                (DecodeResult::Success, Some(bytes)) => decode_from_ns_data(&NSData::with_bytes(&bytes)),
                // Attempt to decode the bookmarks of earlier versions,
                // which were the data of the system in base 64.
                (DecodeResult::InvalidFormat, _) => NSData::initWithBase64EncodedString_options(
                    NSData::alloc(),
                    &NSString::from_str(bookmark),
                    NSDataBase64DecodingOptions(0),
                )
                .and_then(|data| decode_from_ns_data(&data)),
                _ => None,
            }
        }

        fn file_url(uri: &Uri) -> Option<(Retained<NSURL>, Retained<NSString>)> {
            let url = NSURL::URLWithString(&NSString::from_str(uri.absolute_uri()))?;
            let path = url.path()?;
            Some((url, path))
        }

        fn exists(path: &NSString) -> Option<bool> {
            let mut is_directory = Bool::NO;
            // SAFETY: the pointer is to a variable of this function,
            // which lives for the call.
            let exists = unsafe { NSFileManager::defaultManager().fileExistsAtPath_isDirectory(path, &mut is_directory) };
            exists.then_some(is_directory.as_bool())
        }
    }

    impl IStorageProvider for IosStorageProvider {
        fn can_open(&self) -> bool {
            true
        }

        fn can_save(&self) -> bool {
            true
        }

        fn can_pick_folder(&self) -> bool {
            true
        }

        fn open_file_picker_async(&self, options: FilePickerOpenOptions) -> LocalBoxFuture<io::Result<Vec<Rc<dyn IStorageFile>>>> {
            let pictures = options
                .suggested_start_location()
                .and_then(|folder| folder.as_any())
                .and_then(|any| any.downcast_ref::<IosStorageFolder>())
                .is_some_and(|folder| folder.well_known_folder() == Some(WellKnownFolder::Pictures));
            let shown = if !options.allow_multiple() && pictures {
                self.open_image_picker_async(&options)
            } else {
                self.open_documents_picker_async(&options)
            };

            Box::pin(async move {
                let shown = shown?;
                let urls = shown.urls.await;
                drop((shown._picker, shown._delegates));
                Ok(urls.into_iter().map(|u| IosStorageFile::new(u, None) as Rc<dyn IStorageFile>).collect())
            })
        }

        fn open_file_picker_with_result_async(
            &self,
            options: FilePickerOpenOptions,
        ) -> LocalBoxFuture<io::Result<OpenFilePickerResult>> {
            let files = self.open_file_picker_async(options);
            Box::pin(async move { Ok(OpenFilePickerResult { files: files.await?, selected_file_type: None }) })
        }

        fn open_file_bookmark_async(&self, bookmark: &str) -> LocalBoxFuture<Option<Rc<dyn IStorageBookmarkFile>>> {
            let file = self.get_bookmarked_url(bookmark).map(|url| IosStorageFile::new(url, None) as Rc<dyn IStorageBookmarkFile>);
            Box::pin(std::future::ready(file))
        }

        fn open_folder_bookmark_async(&self, bookmark: &str) -> LocalBoxFuture<Option<Rc<dyn IStorageBookmarkFolder>>> {
            let folder =
                self.get_bookmarked_url(bookmark).map(|url| IosStorageFolder::new(url, None) as Rc<dyn IStorageBookmarkFolder>);
            Box::pin(std::future::ready(folder))
        }

        fn try_get_file_from_path_async(&self, file_path: &Uri) -> LocalBoxFuture<Option<Rc<dyn IStorageFile>>> {
            let file = Self::file_url(file_path)
                .filter(|(_, path)| {
                    Self::exists(path) == Some(false) && NSFileManager::defaultManager().isReadableFileAtPath(path)
                })
                .map(|(file_url, _)| IosStorageFile::new(file_url, None) as Rc<dyn IStorageFile>);

            Box::pin(std::future::ready(file))
        }

        fn try_get_folder_from_path_async(&self, folder_path: &Uri) -> LocalBoxFuture<Option<Rc<dyn IStorageFolder>>> {
            let folder = Self::file_url(folder_path)
                .filter(|(_, path)| Self::exists(path) == Some(true))
                .map(|(folder_url, _)| IosStorageFolder::new(folder_url, None) as Rc<dyn IStorageFolder>);

            Box::pin(std::future::ready(folder))
        }

        fn try_get_well_known_folder_async(
            &self,
            well_known_folder: WellKnownFolder,
        ) -> LocalBoxFuture<Option<Rc<dyn IStorageFolder>>> {
            let folder = search_path_directory(well_known_folder).and_then(|directory_type| {
                let uri = NSFileManager::defaultManager().URLForDirectory_inDomain_appropriateForURL_create_error(
                    NSSearchPathDirectory(directory_type),
                    NSSearchPathDomainMask::UserDomainMask,
                    None,
                    true,
                );
                match uri {
                    Ok(uri) => Some(IosStorageFolder::new_well_known(uri, well_known_folder) as Rc<dyn IStorageFolder>),
                    Err(error) => {
                        // The reference fails with the error; the
                        // contract of the port answers with no folder.
                        if let Some(logger) = Logger::try_get(LogEventLevel::Error, LogArea::IOS_PLATFORM) {
                            logger.log_with_values(
                                None,
                                "The well-known folder was not found: {Error}",
                                &[&error.localizedDescription()],
                            );
                        }
                        None
                    }
                }
            });

            Box::pin(std::future::ready(folder))
        }

        fn save_file_picker_async(&self, options: FilePickerSaveOptions) -> LocalBoxFuture<io::Result<Option<Rc<dyn IStorageFile>>>> {
            // To save a file, the user is presented with a document
            // picker, which needs a temporary file to "export". When the
            // user picked the location and the name, the picker gives
            // back the URL of the real file, which becomes the storage
            // file. It is weird, but without the temporary file the
            // picker fails.
            let Ok(mtm) = Self::mtm() else {
                return error("The pickers are shown from the main thread.");
            };

            // Create a temporary file to use with the document picker
            let temp_file_name = StorageProviderHelpers::name_with_extension(
                Some(options.suggested_file_name().unwrap_or("document")),
                options.default_extension(),
                options.file_type_choices().and_then(|choices| choices.first()).map(|choice| &**choice),
            )
            .unwrap_or_else(|| "document".to_string());

            let manager = NSFileManager::defaultManager();
            let temp_dir = manager.temporaryDirectory().URLByAppendingPathComponent_isDirectory(&NSUUID::new().UUIDString(), true);
            let Some(temp_dir) = temp_dir else {
                return error("Failed to get temporary directory for save file picker");
            };

            // SAFETY: no attributes are passed.
            let is_directory_created =
                unsafe { manager.createDirectoryAtURL_withIntermediateDirectories_attributes_error(&temp_dir, true, None) };
            if is_directory_created.is_err() {
                return error("Failed to create temporary directory for save file picker");
            }

            let temp_file_url = temp_dir.URLByAppendingPathComponent_isDirectory(&NSString::from_str(&temp_file_name), false);
            let Some(temp_file_url) = temp_file_url else {
                return error("Failed to get temporary directory for save file picker");
            };

            // Create an empty file at the temp location
            NSData::new().writeToURL_atomically(&temp_file_url, false);

            let document_picker = UIDocumentPickerViewController::initForExportingURLs_asCopy(
                mtm.alloc(),
                &NSArray::from_retained_slice(&[temp_file_url]),
                true,
            );
            document_picker.setDirectoryURL(Self::get_url_from_folder(options.suggested_start_location()).as_deref());
            document_picker.setTitle(options.title().map(NSString::from_str).as_deref());

            let shown = self.show_document_picker(&document_picker);

            Box::pin(async move {
                let shown = shown?;
                let urls = shown.urls.await;
                drop((shown._picker, shown._delegates));

                // Clean up the temporary directory
                let _ = NSFileManager::defaultManager().removeItemAtURL_error(&temp_dir);

                Ok(urls.into_iter().next().map(|url| IosStorageFile::new(url, None) as Rc<dyn IStorageFile>))
            })
        }

        fn save_file_picker_with_result_async(
            &self,
            options: FilePickerSaveOptions,
        ) -> LocalBoxFuture<io::Result<SaveFilePickerResult>> {
            let file = self.save_file_picker_async(options);
            Box::pin(async move { Ok(SaveFilePickerResult { file: file.await?, selected_file_type: None }) })
        }

        fn open_folder_picker_async(
            &self,
            options: FolderPickerOpenOptions,
        ) -> LocalBoxFuture<io::Result<Vec<Rc<dyn IStorageFolder>>>> {
            let Ok(mtm) = Self::mtm() else {
                return error("The pickers are shown from the main thread.");
            };
            // SAFETY: the constant of the framework.
            let document_picker = Self::document_picker_for_opening(mtm, &NSArray::from_slice(unsafe { &[UTTypeFolder] }));

            document_picker.setDirectoryURL(Self::get_url_from_folder(options.suggested_start_location()).as_deref());
            document_picker.setAllowsMultipleSelection(options.allow_multiple());

            let shown = self.show_document_picker(&document_picker);
            Box::pin(async move {
                let shown = shown?;
                let urls = shown.urls.await;
                drop((shown._picker, shown._delegates));
                Ok(urls.into_iter().map(|u| IosStorageFolder::new(u, None) as Rc<dyn IStorageFolder>).collect())
            })
        }
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests of this file.
    use super::*;

    #[test]
    fn the_platform_key_is_the_one_of_the_reference() {
        assert_eq!(b"ios", PLATFORM_KEY);
    }

    #[test]
    fn without_a_filter_there_are_no_sources() {
        assert_eq!(None, file_types_to_uniform_type_sources(None));
        assert_eq!(Some(Vec::new()), file_types_to_uniform_type_sources(Some(&[])));
    }

    #[test]
    fn a_file_type_gives_its_extensions_before_its_identifiers_before_its_mime_types() {
        let all = FilePickerFileType::new(Some("Images"))
            .with_patterns(&["*.png", "*.jpg"])
            .with_apple_uniform_type_identifiers(&["public.image"])
            .with_mime_types(&["image/*"]);
        let identifiers = FilePickerFileType::new(Some("Text"))
            .with_apple_uniform_type_identifiers(&["public.plain-text"])
            .with_mime_types(&["text/plain"]);
        let mime = FilePickerFileType::new(Some("Pdf")).with_mime_types(&["application/pdf"]);
        let nothing = FilePickerFileType::new(Some("Nothing"));

        assert_eq!(
            Some(vec![
                UniformTypeSource::Extension("png".to_string()),
                UniformTypeSource::Extension("jpg".to_string()),
                UniformTypeSource::Identifier("public.plain-text".to_string()),
                UniformTypeSource::MimeType("application/pdf".to_string()),
            ]),
            file_types_to_uniform_type_sources(Some(&[all, identifiers, mime, nothing]))
        );
    }

    #[test]
    fn the_well_known_folders_have_their_directories() {
        assert_eq!(Some(12), search_path_directory(WellKnownFolder::Desktop));
        assert_eq!(Some(9), search_path_directory(WellKnownFolder::Documents));
        assert_eq!(Some(15), search_path_directory(WellKnownFolder::Downloads));
        assert_eq!(Some(18), search_path_directory(WellKnownFolder::Music));
        assert_eq!(Some(19), search_path_directory(WellKnownFolder::Pictures));
        assert_eq!(Some(17), search_path_directory(WellKnownFolder::Videos));
    }
}
