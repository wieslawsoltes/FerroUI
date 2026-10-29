//! The actions and the behavior of the notifications of a top level (`Notifications/`).

mod close_notification_action;
mod notification_manager_behavior;
mod show_error_notification_action;
mod show_information_notification_action;
mod show_notification_action;
mod show_success_notification_action;
mod show_warning_notification_action;

pub use close_notification_action::CloseNotificationAction;
pub use notification_manager_behavior::NotificationManagerBehavior;
pub use show_error_notification_action::ShowErrorNotificationAction;
pub use show_information_notification_action::ShowInformationNotificationAction;
pub use show_notification_action::ShowNotificationAction;
pub use show_success_notification_action::ShowSuccessNotificationAction;
pub use show_warning_notification_action::ShowWarningNotificationAction;
