use crate::ferro_native_platform::FerroNativePlatform;
use crate::interop::*;
use crate::frn_string::frn_string_array_to_vec;
use crate::mac_os_activatable_lifetime::MacOSActivatableLifetime;
use crate::storage_provider_api::StorageProviderApi;
use ferroui_base::platform::storage::file_io::StorageProviderHelpers;
use ferroui_base::platform::storage::IStorageItem;
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::utilities::{HandlerList, Uri, UriKind};
use ferroui_base::{FerroLocator, LocatorExtensions};
use ferroui_controls::application_lifetimes::{
    ActivationKind, FileActivatedEventArgs, ProtocolActivatedEventArgs, ShutdownRequestedEventArgs,
};
use ferroui_controls::platform::IPlatformLifetimeEventsImpl;
use std::rc::{Rc, Weak};

/// Receives the application events of the native side and exposes them as
/// the platform lifetime events.
pub struct FerroNativeApplicationPlatform {
    weak_self: Weak<FerroNativeApplicationPlatform>,
    platform: Weak<FerroNativePlatform>,
    shutdown_requested: HandlerList<dyn Fn(&ShutdownRequestedEventArgs)>,
}

impl FerroNativeApplicationPlatform {
    pub(crate) fn new(platform: Weak<FerroNativePlatform>) -> Rc<FerroNativeApplicationPlatform> {
        Rc::new_cyclic(|weak_self| FerroNativeApplicationPlatform {
            weak_self: weak_self.clone(),
            platform,
            shutdown_requested: HandlerList::new(),
        })
    }

    /// The reply to a shutdown request of the system for the outcome of the
    /// shutdown-requested event.
    fn shutdown_reply(e: &ShutdownRequestedEventArgs, is_os_shutdown: bool) -> FrnShutdownReply {
        if e.cancel() {
            return FrnShutdownReply::ShutdownReplyCancel;
        }

        // If we know the main loop is going to exit (e.g. via a classic desktop application lifetime),
        // tell the native side it doesn't have to exit, allowing the managed side to complete its shutdown.
        if e.will_exit_main_loop() && !is_os_shutdown {
            return FrnShutdownReply::ShutdownReplyDeferToManagedLoop;
        }

        FrnShutdownReply::ShutdownReplyTerminateNow
    }
}

impl IPlatformLifetimeEventsImpl for FerroNativeApplicationPlatform {
    fn shutdown_requested(&self, handler: Rc<dyn Fn(&ShutdownRequestedEventArgs)>) -> Rc<dyn IDisposable> {
        let token = self.shutdown_requested.add(handler);
        let this = self.weak_self.clone();
        Disposable::create(move || {
            if let Some(this) = this.upgrade() {
                this.shutdown_requested.remove(token);
            }
        })
    }
}

/// The activatable lifetime of this backend, if it is the registered one.
fn activatable_lifetime() -> Option<Rc<MacOSActivatableLifetime>> {
    FerroLocator::current().get_service::<MacOSActivatableLifetime>()
}

/// The storage API of this backend, if it is the registered storage
/// provider factory.
fn storage_api() -> Option<Rc<StorageProviderApi>> {
    FerroLocator::current().get_service::<StorageProviderApi>()
}

impl IFrnApplicationEventsImpl for FerroNativeApplicationPlatform {
    fn files_opened(&self, urls: Option<&IFrnStringArray>) {
        crate::callback_base::guard((), || {
            if let (Some(lifetime), Some(storage_api)) = (activatable_lifetime(), storage_api()) {
                let file_paths = urls.map(frn_string_array_to_vec).unwrap_or_default();
                let mut files: Vec<Rc<dyn IStorageItem>> = Vec::with_capacity(file_paths.len());
                for file_path in &file_paths {
                    if let Some(file) = StorageProviderHelpers::try_get_uri_from_file_path(file_path, false)
                        .and_then(|file_uri| storage_api.try_get_storage_item(Some(&file_uri), false))
                    {
                        files.push(file.into_item());
                    }
                }

                if !files.is_empty() {
                    lifetime.on_activated_with(FileActivatedEventArgs::new(files).into());
                }
            }
        })
    }

    fn urls_opened(&self, urls: Option<&IFrnStringArray>) {
        crate::callback_base::guard((), || {
            if let (Some(lifetime), Some(storage_api)) = (activatable_lifetime(), storage_api()) {
                let mut files: Vec<Rc<dyn IStorageItem>> = Vec::new();
                let mut uris: Vec<Uri> = Vec::new();
                for url in urls.map(frn_string_array_to_vec).unwrap_or_default() {
                    if let Some(uri) = Uri::try_create(&url, UriKind::RelativeOrAbsolute) {
                        if uri.scheme() == "file" {
                            if let Some(file) = storage_api.try_get_storage_item(Some(&uri), false) {
                                files.push(file.into_item());
                            }
                        } else {
                            uris.push(uri);
                        }
                    }
                }

                for uri in uris {
                    lifetime.on_activated_with(ProtocolActivatedEventArgs::new(uri).into());
                }
                if !files.is_empty() {
                    lifetime.on_activated_with(FileActivatedEventArgs::new(files).into());
                }
            }
        })
    }

    fn try_shutdown(&self, is_os_shutdown: bool) -> FrnShutdownReply {
        crate::callback_base::guard(FrnShutdownReply::ShutdownReplyTerminateNow, || {
            if self.shutdown_requested.is_empty() {
                return FrnShutdownReply::ShutdownReplyTerminateNow;
            }

            let e = ShutdownRequestedEventArgs::with_is_os_shutdown(is_os_shutdown);
            for (_, handler) in self.shutdown_requested.snapshot().iter() {
                handler(&e);
            }

            Self::shutdown_reply(&e, is_os_shutdown)
        })
    }

    fn on_reopen(&self) {
        crate::callback_base::guard((), || {
            if let Some(lifetime) = activatable_lifetime() {
                lifetime.on_activated(ActivationKind::Reopen);
            }
        })
    }

    fn on_hide(&self) {}

    fn on_unhide(&self) {}

    fn on_activate(&self) {
        crate::callback_base::guard((), || {
            if let Some(lifetime) = activatable_lifetime() {
                lifetime.on_activated(ActivationKind::Background);
            }
        })
    }

    fn on_deactivate(&self) {
        crate::callback_base::guard((), || {
            if let Some(lifetime) = activatable_lifetime() {
                lifetime.on_deactivated(ActivationKind::Background);
            }
        })
    }

    fn on_terminating(&self) {
        crate::callback_base::guard((), || {
            // The OS is terminating us directly: nothing else gets to run, dispose now.
            if let Some(platform) = self.platform.upgrade() {
                platform.dispose();
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reply(cancel: bool, will_exit_main_loop: bool, is_os_shutdown: bool) -> FrnShutdownReply {
        let e = ShutdownRequestedEventArgs::with_is_os_shutdown(is_os_shutdown);
        e.set_cancel(cancel);
        e.set_will_exit_main_loop(will_exit_main_loop);
        FerroNativeApplicationPlatform::shutdown_reply(&e, is_os_shutdown)
    }

    #[test]
    fn shutdown_replies() {
        assert_eq!(reply(true, false, false), FrnShutdownReply::ShutdownReplyCancel);
        assert_eq!(reply(true, true, true), FrnShutdownReply::ShutdownReplyCancel);
        assert_eq!(reply(false, true, false), FrnShutdownReply::ShutdownReplyDeferToManagedLoop);
        // The OS shutting down does not wait for the managed loop.
        assert_eq!(reply(false, true, true), FrnShutdownReply::ShutdownReplyTerminateNow);
        assert_eq!(reply(false, false, false), FrnShutdownReply::ShutdownReplyTerminateNow);
    }

    #[test]
    fn shutdown_without_subscribers_terminates_now() {
        let platform = FerroNativeApplicationPlatform::new(Weak::new());
        assert_eq!(platform.try_shutdown(false), FrnShutdownReply::ShutdownReplyTerminateNow);

        let subscription = platform.shutdown_requested(Rc::new(|e| e.set_cancel(true)));
        assert_eq!(platform.try_shutdown(false), FrnShutdownReply::ShutdownReplyCancel);

        subscription.dispose();
        assert_eq!(platform.try_shutdown(false), FrnShutdownReply::ShutdownReplyTerminateNow);
    }
}
