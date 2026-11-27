//! The notifications of the operating system and the badge of the application's icon (#1276).
//!
//! Not from upstream: the names are those of the app notifications of WinUI's app SDK
//! (`AppNotificationManager`, `AppNotificationBuilder`, `AppNotification`). An application
//! builds an [`AppNotification`] with an [`AppNotificationBuilder`] and shows it with
//! [`AppNotificationManager::show`]; the notification center of the platform shows it, and a
//! click on it or on one of its buttons raises [`AppNotificationManager::notification_invoked`].
//!
//! The platforms implement [`IAppNotificationManagerImpl`], which the application registers in
//! the [`FerroLocator`](crate::FerroLocator) (the platform crates provide the implementations,
//! registered by their `use_platform_app_notifications()`). Nothing registers one unless the
//! application asks for it, so an application that shows no notification links none of them, and
//! [`AppNotificationManager::is_supported`] is `false` where the platform has no notification
//! service or the application did not ask for one.
//!
//! The badge of the application's icon is [`BadgeNotificationManager`], with the names of the
//! badge notifications of the app SDK, over the [`IBadgeNotificationManagerImpl`] the
//! application registers with `use_platform_badge_notifications()`, in the same way.

mod app_notification;
mod app_notification_manager;
mod badge_notification_manager;

pub use app_notification::{
    AppNotification, AppNotificationBuilder, AppNotificationButton, AppNotificationComboBox,
    AppNotificationImageCrop, AppNotificationProgressBar, AppNotificationScenario,
    AppNotificationSoundEvent, AppNotificationTextBox,
};
pub use app_notification_manager::{
    app_notification_manager_impl, register_app_notification_manager_impl,
    AppNotificationActivatedEventArgs, AppNotificationError, AppNotificationManager,
    IAppNotificationManagerImpl,
};
pub use badge_notification_manager::{
    badge_notification_manager_impl, register_badge_notification_manager_impl,
    BadgeNotificationGlyph, BadgeNotificationManager, IBadgeNotificationManagerImpl,
};

#[cfg(test)]
mod badge_notification_tests;
#[cfg(test)]
mod notifications_tests;
