//! The badge of a packaged application on Windows (#1276).
//!
//! Not from upstream. `BadgeUpdateManager` shows the badge of `BadgeNotificationManager` on the
//! application's taskbar button and Start tile from the XML of a badge (`<badge value="5"/>`,
//! `<badge value="newMessage"/>`); Windows shows a count above 99 as 99+. Windows keeps badges
//! for applications with a package identity (an MSIX package, or a sparse package of an
//! unpackaged application): `is_supported` is `false` for a process without one. The overlay
//! icon of the taskbar button, which an application without an identity could show, is not set.

use crate::interop::unmanaged_methods::{get_module_handle, get_proc_address};
use crate::win32_platform::Win32Platform;
use crate::win_rt::{
    HStringInterop, IBadgeNotificationFactory, IBadgeUpdateManagerStatics, IBadgeUpdater, IXmlDocument,
    IXmlDocumentIO, NativeWinRTMethods,
};
use ferroui_base::platform::notifications::{BadgeNotificationGlyph, IBadgeNotificationManagerImpl};
use ferroui_microcom::{ComPtr, HResult};
use std::cell::OnceCell;

/// The XML of a badge that shows `value` (a count, or the value of a glyph).
pub(crate) fn badge_payload(value: &str) -> String {
    format!("<badge value=\"{value}\"/>")
}

/// The badge service of Windows: the badge of the packaged application. Registered by
/// `use_platform_badge_notifications()` of the desktop platform.
#[derive(Default)]
pub struct Win32BadgeNotificationManager {
    updater: OnceCell<Option<ComPtr<IBadgeUpdater>>>,
}

impl Win32BadgeNotificationManager {
    /// The service of the application's package.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers the badge of the application's package as the badge service of the
    /// application.
    pub fn register() {
        ferroui_base::platform::notifications::register_badge_notification_manager_impl(std::rc::Rc::new(Self::new()));
    }

    fn updater(&self) -> Option<ComPtr<IBadgeUpdater>> {
        self.updater
            .get_or_init(|| {
                if Win32Platform::windows_version().major < 10 || !has_package_identity() {
                    return None;
                }
                let statics = NativeWinRTMethods::create_activation_factory::<IBadgeUpdateManagerStatics>(
                    "Windows.UI.Notifications.BadgeUpdateManager",
                )
                .ok()?;
                statics.create_badge_updater_for_application().ok().flatten()
            })
            .clone()
    }

    fn update(&self, value: &str) {
        let Some(updater) = self.updater() else { return };
        let _ = (|| -> Result<(), HResult> {
            let document = NativeWinRTMethods::create_instance::<IXmlDocumentIO>("Windows.Data.Xml.Dom.XmlDocument")?;
            let xml = HStringInterop::new(Some(&badge_payload(value)))?;
            document.load_xml(xml.handle())?;
            let document = document.cast::<IXmlDocument>()?;
            let factory = NativeWinRTMethods::create_activation_factory::<IBadgeNotificationFactory>(
                "Windows.UI.Notifications.BadgeNotification",
            )?;
            let badge = factory.create_badge_notification(Some(&document))?.ok_or(HResult::POINTER)?;
            updater.update(Some(&badge))
        })();
    }
}

/// Whether the process has a package identity: `GetCurrentPackageFullName` (Windows 8 and later,
/// looked up so that the process starts on a system without it) answers with the size of the name
/// instead of `APPMODEL_ERROR_NO_PACKAGE`.
fn has_package_identity() -> bool {
    /// `ERROR_INSUFFICIENT_BUFFER`: the process has a package, whose name does not fit in 0
    /// characters.
    const ERROR_INSUFFICIENT_BUFFER: i32 = 122;
    type GetCurrentPackageFullName = unsafe extern "system" fn(length: *mut u32, name: *mut u16) -> i32;
    let Some(function) = get_proc_address(get_module_handle(Some("kernel32.dll")), c"GetCurrentPackageFullName") else {
        return false;
    };
    // SAFETY: the export of `kernel32.dll` has this signature.
    let function: GetCurrentPackageFullName = unsafe { std::mem::transmute(function) };
    let mut length = 0u32;
    // SAFETY: a length of 0 and no buffer: the function writes the length it needs.
    unsafe { function(&mut length, std::ptr::null_mut()) == ERROR_INSUFFICIENT_BUFFER }
}

impl IBadgeNotificationManagerImpl for Win32BadgeNotificationManager {
    fn is_supported(&self) -> bool {
        self.updater().is_some()
    }

    fn set_count(&self, count: u32) {
        self.update(&count.to_string());
    }

    fn set_glyph(&self, glyph: BadgeNotificationGlyph) {
        self.update(glyph.badge_value());
    }

    fn clear(&self) {
        if let Some(updater) = self.updater() {
            let _ = updater.clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_payload_is_the_xml_of_a_badge() {
        assert_eq!(badge_payload("5"), "<badge value=\"5\"/>");
        assert_eq!(
            badge_payload(BadgeNotificationGlyph::NewMessage.badge_value()),
            "<badge value=\"newMessage\"/>"
        );
    }

    /// Against the system: a test process has no package identity, so the badge is not
    /// supported and the calls change nothing.
    #[test]
    fn a_process_without_a_package_has_no_badge() {
        let manager = Win32BadgeNotificationManager::new();
        assert!(!has_package_identity());
        assert!(!manager.is_supported());
        manager.set_count(3);
        manager.clear();
    }
}
