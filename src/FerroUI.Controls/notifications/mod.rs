//! Notification controls and the panels they use.

mod reversible_stack_panel;

pub use reversible_stack_panel::ReversibleStackPanel;

#[cfg(test)]
mod reversible_stack_panel_tests;

mod i_managed_notification_manager;
mod i_notification;
mod i_notification_manager;
mod notification;
mod notification_card;
mod notification_position;
mod notification_type;
mod window_notification_manager;

pub use i_managed_notification_manager::IManagedNotificationManager;
pub use i_notification::INotification;
pub use i_notification_manager::INotificationManager;
pub use notification::{Notification, NotificationOverrides};
pub use notification_card::NotificationCard;
pub use notification_position::NotificationPosition;
pub use notification_type::NotificationType;
pub use window_notification_manager::WindowNotificationManager;

#[cfg(test)]
mod notifications_tests;
