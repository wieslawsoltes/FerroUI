//! The launcher: opens a URI with the application the system has for it,
//! and previews a file.

use ferroui_base::input::LocalBoxFuture;
use ferroui_base::platform::storage::{ILauncher, IStorageItem};
use block2::RcBlock;
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, Bool};
use objc2::MainThreadMarker;
use objc2_foundation::{NSDictionary, NSString, NSURL};
use objc2_ui_kit::{UIApplication, UIApplicationOpenExternalURLOptionsKey, UIDocumentInteractionController};
use ferroui_system::task::oneshot;
use std::cell::RefCell;
use std::rc::Rc;
use url::Url;

/// The launcher of iOS.
#[derive(Default)]
pub struct IosLauncher;

impl IosLauncher {
    /// Creates the launcher.
    pub fn new() -> Self {
        Self
    }
}

/// The URL of the file a storage item stands for: the URL of an item of
/// the storage provider of the platform, or the local path of another.
fn url_of_storage_item(storage_item: &Rc<dyn IStorageItem>) -> Option<Retained<NSURL>> {
    crate::storage::ios_storage_item::url_of(&**storage_item).or_else(|| {
        storage_item.try_get_local_path().map(|local_path| NSURL::fileURLWithPath(&NSString::from_str(&local_path)))
    })
}

impl ILauncher for IosLauncher {
    fn launch_uri_async(&self, uri: &Url) -> LocalBoxFuture<bool> {
        // The launcher of a top-level is asked on the main thread.
        let Some(mtm) = MainThreadMarker::new() else {
            return Box::pin(std::future::ready(false));
        };

        let application = UIApplication::sharedApplication(mtm);
        let url = NSURL::URLWithString(&NSString::from_str(uri.as_str()));
        match url {
            Some(url) if application.canOpenURL(&url) => {
                let (completion, future) = oneshot::channel();
                let completion = RefCell::new(Some(completion));
                let block = RcBlock::new(move |success: Bool| {
                    if let Some(completion) = completion.take() {
                        // A launch whose wait was dropped does not want the answer.
                        let _ = completion.send(success.as_bool());
                    }
                });
                let options = NSDictionary::<UIApplicationOpenExternalURLOptionsKey, AnyObject>::new();
                // SAFETY: the options are an empty dictionary, which is
                // of every type of key and value. UIKit calls the
                // completion handler on the main queue, the thread the
                // completion and the future that waits for it belong to.
                unsafe { application.openURL_options_completionHandler(&url, &options, Some(&block)) };
                // Deviation (DEVIATIONS.md, iOS platform): a completion handler that UIKit releases
                // without calling it answers `false`; the task of the reference would never
                // complete.
                Box::pin(async move { future.await.unwrap_or(false) })
            }
            _ => Box::pin(std::future::ready(false)),
        }
    }

    fn launch_file_async(&self, storage_item: Rc<dyn IStorageItem>) -> LocalBoxFuture<bool> {
        let Some(mtm) = MainThreadMarker::new() else {
            return Box::pin(std::future::ready(false));
        };

        if let Some(uri) = url_of_storage_item(&storage_item) {
            let document_controller = UIDocumentInteractionController::new(mtm);
            document_controller.setName(Some(&NSString::from_str(&storage_item.name())));
            document_controller.setURL(Some(&uri));

            let result = document_controller.presentPreviewAnimated(true);
            return Box::pin(std::future::ready(result));
        }

        Box::pin(std::future::ready(false))
    }
}
