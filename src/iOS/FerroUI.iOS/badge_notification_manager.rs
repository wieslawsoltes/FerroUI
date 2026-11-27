//! The badge of the application's icon on iOS (#1276).
//!
//! Not from upstream. The badge count of `UNUserNotificationCenter` (`setBadgeCount:`, iOS 16 and
//! later; `applicationIconBadgeNumber` of `UIApplication` before) shows the count of
//! `BadgeNotificationManager` on the home screen icon. The badge of iOS is a number: a glyph shows
//! no badge. The user allows the badge: the first badge asks with the options the notification
//! service asks for (alerts, sounds and badges), so that the two services agree, and the badge is
//! shown once the user allowed it. The UserNotifications framework is opened on the first use,
//! as the notification service opens it.

use crate::app_notification_manager::{class, framework_loaded, AUTHORIZATION_OPTIONS};
use block2::RcBlock;
use ferroui_base::platform::notifications::{BadgeNotificationGlyph, IBadgeNotificationManagerImpl};
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use objc2::rc::Retained;
use objc2::runtime::{AnyClass, AnyObject, Bool};
use objc2::{msg_send, sel};
use objc2_foundation::{NSBundle, NSError};
use std::cell::Cell;
use std::sync::atomic::{AtomicU32, Ordering};

/// The count the application set last: what the badge shows once the user allowed it.
static COUNT: AtomicU32 = AtomicU32::new(0);

/// The badge service of iOS: the badge count of the application's icon. Registered by
/// `use_platform_badge_notifications()` of the iOS platform.
#[derive(Default)]
pub struct IosBadgeNotificationManager {
    /// Whether the user was asked to allow the badge.
    asked: Cell<bool>,
}

impl IosBadgeNotificationManager {
    /// The service of the application's icon.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers the badge count of the application's icon as the badge service of the
    /// application.
    pub fn register() {
        ferroui_base::platform::notifications::register_badge_notification_manager_impl(std::rc::Rc::new(Self::new()));
    }

    fn center() -> Retained<AnyObject> {
        // SAFETY: the class is in the framework `is_supported` loaded.
        unsafe { msg_send![class(c"UNUserNotificationCenter"), currentNotificationCenter] }
    }

    fn show(&self, count: u32) {
        COUNT.store(count, Ordering::Relaxed);
        if !self.asked.replace(true) {
            // The center calls the handler on a thread of its own: the badge is set on the UI
            // thread, with the count set last.
            let completion = RcBlock::new(|granted: Bool, _error: *mut NSError| {
                if granted.as_bool() {
                    Dispatcher::ui_thread()
                        .post(|| set_badge_count(COUNT.load(Ordering::Relaxed)), DispatcherPriority::DEFAULT);
                }
            });
            // SAFETY: the options and a block of the type the method takes.
            unsafe {
                let _: () = msg_send![
                    &Self::center(),
                    requestAuthorizationWithOptions: AUTHORIZATION_OPTIONS,
                    completionHandler: &*completion
                ];
            }
        }
        set_badge_count(count);
    }
}

/// Sets the badge count of the application's icon, on the UI thread.
fn set_badge_count(count: u32) {
    let count = isize::try_from(count).unwrap_or(isize::MAX);
    let center = IosBadgeNotificationManager::center();
    // SAFETY: `setBadgeCount:withCompletionHandler:` takes a number and a nil handler, where the
    // center answers to it; before iOS 16 `setApplicationIconBadgeNumber:` of the shared
    // application takes the number, on the main thread.
    unsafe {
        let has_badge_count: bool = msg_send![&center, respondsToSelector: sel!(setBadgeCount:withCompletionHandler:)];
        if has_badge_count {
            let _: () = msg_send![
                &center,
                setBadgeCount: count,
                withCompletionHandler: Option::<&block2::Block<dyn Fn(*mut NSError)>>::None
            ];
        } else if let Some(application) = AnyClass::get(c"UIApplication") {
            let application: Option<Retained<AnyObject>> = msg_send![application, sharedApplication];
            if let Some(application) = application {
                let _: () = msg_send![&application, setApplicationIconBadgeNumber: count];
            }
        }
    }
}

impl IBadgeNotificationManagerImpl for IosBadgeNotificationManager {
    fn is_supported(&self) -> bool {
        NSBundle::mainBundle().bundleIdentifier().is_some() && framework_loaded()
    }

    fn set_count(&self, count: u32) {
        self.show(count);
    }

    fn set_glyph(&self, _glyph: BadgeNotificationGlyph) {
        // The badge of iOS is a number.
        self.show(0);
    }

    fn clear(&self) {
        self.show(0);
    }
}
