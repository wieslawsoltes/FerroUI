use super::INotification;
use std::rc::Rc;

/// Represents a notification manager that can be used to show notifications
/// in a window or using the host operating system.
///
/// Not meant to be implemented outside the framework.
pub trait INotificationManager {
    /// Show a notification.
    fn show(&self, notification: Rc<dyn INotification>);

    /// Closes a notification.
    fn close(&self, notification: &Rc<dyn INotification>);

    /// Closes all notifications.
    fn close_all(&self);

    /// The identity of the manager: handles to the same manager compare
    /// equal.
    #[doc(hidden)]
    fn reference_id(&self) -> *const () {
        self as *const Self as *const ()
    }
}

/// Notification managers are reference types: they compare by identity.
impl PartialEq for dyn INotificationManager {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.reference_id() == other.reference_id()
    }
}
