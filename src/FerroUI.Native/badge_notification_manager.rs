//! The badge of the application's Dock tile on macOS (#1276).
//!
//! Not from upstream. The badge label of the application's `NSDockTile` shows the badge of
//! `BadgeNotificationManager`: a count is its number, a glyph a text symbol of the same meaning
//! ([`glyph_label`]); clearing removes the label. AppKit, which the platform links already, has
//! the Dock tile: an application without a bundle has one too. An application that is not in the
//! Dock (an accessory application) shows no badge.

use ferroui_base::platform::notifications::{BadgeNotificationGlyph, IBadgeNotificationManagerImpl};
use objc2::rc::Retained;
use objc2::runtime::{AnyClass, AnyObject};
use objc2::msg_send;
use objc2_foundation::NSString;

/// The text the Dock tile shows for `glyph`: a symbol of the same meaning in the text style
/// (`U+FE0E`), as the Dock's badge shows text only.
pub(crate) fn glyph_label(glyph: BadgeNotificationGlyph) -> &'static str {
    match glyph {
        BadgeNotificationGlyph::None => "",
        BadgeNotificationGlyph::Activity => "\u{2026}",
        BadgeNotificationGlyph::Alarm => "!!",
        BadgeNotificationGlyph::Alert => "!",
        BadgeNotificationGlyph::Attention => "\u{2022}",
        BadgeNotificationGlyph::Available => "\u{2713}\u{FE0E}",
        BadgeNotificationGlyph::Away => "\u{25D4}",
        BadgeNotificationGlyph::Busy => "\u{25CF}",
        BadgeNotificationGlyph::Error => "\u{2715}",
        BadgeNotificationGlyph::NewMessage => "\u{2709}\u{FE0E}",
        BadgeNotificationGlyph::Paused => "\u{2016}",
        BadgeNotificationGlyph::Playing => "\u{25B6}\u{FE0E}",
        BadgeNotificationGlyph::Unavailable => "\u{2298}",
    }
}

/// The badge service of macOS: the badge label of the application's Dock tile. Registered by
/// `use_platform_badge_notifications()` of the desktop platform.
#[derive(Default)]
pub struct MacOSBadgeNotificationManager;

impl MacOSBadgeNotificationManager {
    /// The service of the application's Dock tile.
    pub fn new() -> Self {
        Self
    }

    /// Registers the Dock tile as the badge service of the application.
    pub fn register() {
        ferroui_base::platform::notifications::register_badge_notification_manager_impl(std::rc::Rc::new(Self::new()));
    }

    fn dock_tile() -> Option<Retained<AnyObject>> {
        let application = AnyClass::get(c"NSApplication")?;
        // SAFETY: `sharedApplication` of `NSApplication` and its `dockTile`, called on the UI
        // thread (the main thread of the application).
        unsafe {
            let application: Option<Retained<AnyObject>> = msg_send![application, sharedApplication];
            let application = application?;
            msg_send![&application, dockTile]
        }
    }

    fn set_label(label: Option<&str>) {
        let Some(tile) = Self::dock_tile() else { return };
        let label = label.map(NSString::from_str);
        // SAFETY: `setBadgeLabel:` takes a string or nil.
        unsafe {
            let _: () = msg_send![&tile, setBadgeLabel: label.as_deref()];
        }
    }
}

impl IBadgeNotificationManagerImpl for MacOSBadgeNotificationManager {
    fn is_supported(&self) -> bool {
        AnyClass::get(c"NSApplication").is_some()
    }

    fn set_count(&self, count: u32) {
        Self::set_label(Some(&count.to_string()));
    }

    fn set_glyph(&self, glyph: BadgeNotificationGlyph) {
        Self::set_label(Some(glyph_label(glyph)));
    }

    fn clear(&self) {
        Self::set_label(None);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_glyph_but_none_has_a_label() {
        for glyph in BadgeNotificationGlyph::VALUES {
            assert_eq!(glyph_label(glyph).is_empty(), glyph == BadgeNotificationGlyph::None, "{glyph:?}");
        }
    }
}
