//! The badge of the application's icon (`BadgeNotificationManager`) and the contract of the
//! platforms (`IBadgeNotificationManagerImpl`).
//!
//! Not from upstream: the names are those of the badge notifications of WinUI's app SDK
//! (`BadgeNotificationManager.Current`, `SetBadgeAsCount`, `SetBadgeAsGlyph`, `ClearBadge`,
//! `BadgeNotificationGlyph`). The badge is shown where the platform shows the badge of an
//! application: the Dock tile on macOS, the home screen icon on iOS, the taskbar and the Start
//! tile of a packaged application on Windows and the icon of an installed web application.

use crate::{FerroLocator, LocatorExtensions};
use std::rc::Rc;

/// The glyphs a badge can show in place of a count (`BadgeNotificationGlyph`), in the order of
/// their values.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum BadgeNotificationGlyph {
    /// No glyph: setting it clears the badge.
    #[default]
    None,
    Activity,
    Alarm,
    Alert,
    Attention,
    Available,
    Away,
    Busy,
    Error,
    NewMessage,
    Paused,
    Playing,
    Unavailable,
}

impl BadgeNotificationGlyph {
    /// Every glyph, in the order of their values (`Enum.GetValues`).
    pub const VALUES: [Self; 13] = [
        Self::None,
        Self::Activity,
        Self::Alarm,
        Self::Alert,
        Self::Attention,
        Self::Available,
        Self::Away,
        Self::Busy,
        Self::Error,
        Self::NewMessage,
        Self::Paused,
        Self::Playing,
        Self::Unavailable,
    ];

    /// The name of the glyph, as the enumeration names it.
    pub fn name(self) -> &'static str {
        match self {
            Self::None => "None",
            Self::Activity => "Activity",
            Self::Alarm => "Alarm",
            Self::Alert => "Alert",
            Self::Attention => "Attention",
            Self::Available => "Available",
            Self::Away => "Away",
            Self::Busy => "Busy",
            Self::Error => "Error",
            Self::NewMessage => "NewMessage",
            Self::Paused => "Paused",
            Self::Playing => "Playing",
            Self::Unavailable => "Unavailable",
        }
    }

    /// The `value` of the glyph in the XML of a badge of Windows (`<badge value="..."/>`).
    pub fn badge_value(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Activity => "activity",
            Self::Alarm => "alarm",
            Self::Alert => "alert",
            Self::Attention => "attention",
            Self::Available => "available",
            Self::Away => "away",
            Self::Busy => "busy",
            Self::Error => "error",
            Self::NewMessage => "newMessage",
            Self::Paused => "paused",
            Self::Playing => "playing",
            Self::Unavailable => "unavailable",
        }
    }
}

/// The badge service of a platform.
///
/// Every method is called on the UI thread. The manager calls [`set_count`](Self::set_count)
/// with a count above 0 and [`set_glyph`](Self::set_glyph) with a glyph other than
/// [`BadgeNotificationGlyph::None`]; it calls [`clear`](Self::clear) for the others.
pub trait IBadgeNotificationManagerImpl {
    /// Whether the platform can show the badge of this application: `false`, for example, for an
    /// application without the identity the platform's service needs.
    fn is_supported(&self) -> bool;

    /// Shows `count` on the badge.
    fn set_count(&self, count: u32);

    /// Shows `glyph` on the badge. A platform whose badge cannot show the glyph shows its plain
    /// badge, or none (the platforms document which).
    fn set_glyph(&self, glyph: BadgeNotificationGlyph);

    /// Removes the badge.
    fn clear(&self);
}

/// The badge service the application registered, or `None` (the platform has none, or the
/// application did not ask for it).
pub fn badge_notification_manager_impl() -> Option<Rc<dyn IBadgeNotificationManagerImpl>> {
    FerroLocator::current().get_service::<dyn IBadgeNotificationManagerImpl>()
}

/// Registers `service` as the badge service of the application.
pub fn register_badge_notification_manager_impl(service: Rc<dyn IBadgeNotificationManagerImpl>) {
    FerroLocator::current_mutable()
        .bind::<dyn IBadgeNotificationManagerImpl>()
        .to_constant(service);
}

/// The badge of the application's icon (`BadgeNotificationManager.Current`): a count or a glyph,
/// shown through the badge service the application registered.
///
/// ```no_run
/// use ferroui_base::platform::notifications::BadgeNotificationManager;
///
/// let badge = BadgeNotificationManager::current();
/// if BadgeNotificationManager::is_supported() {
///     badge.set_badge_as_count(3);
/// }
/// // Later, when the user read the messages:
/// badge.clear_badge();
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BadgeNotificationManager;

impl BadgeNotificationManager {
    /// The badge of the application (`Current`).
    pub fn current() -> Self {
        Self
    }

    /// Whether the badge can be shown: the application registered the badge service of a
    /// platform that can show it.
    pub fn is_supported() -> bool {
        badge_notification_manager_impl().is_some_and(|service| service.is_supported())
    }

    /// Shows `count` on the badge (`SetBadgeAsCount`); a count of 0 removes the badge, as on
    /// Windows. Nothing happens when the badge is not supported.
    pub fn set_badge_as_count(&self, count: u32) {
        if let Some(service) = supported() {
            if count == 0 {
                service.clear();
            } else {
                service.set_count(count);
            }
        }
    }

    /// Shows `glyph` on the badge (`SetBadgeAsGlyph`); [`BadgeNotificationGlyph::None`] removes
    /// the badge. Nothing happens when the badge is not supported.
    pub fn set_badge_as_glyph(&self, glyph: BadgeNotificationGlyph) {
        if let Some(service) = supported() {
            if glyph == BadgeNotificationGlyph::None {
                service.clear();
            } else {
                service.set_glyph(glyph);
            }
        }
    }

    /// Removes the badge (`ClearBadge`).
    pub fn clear_badge(&self) {
        if let Some(service) = supported() {
            service.clear();
        }
    }
}

fn supported() -> Option<Rc<dyn IBadgeNotificationManagerImpl>> {
    badge_notification_manager_impl().filter(|service| service.is_supported())
}
