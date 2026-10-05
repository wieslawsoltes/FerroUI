use super::INotificationManager;
use ferroui_base::BoxedValue;

/// Represents a notification manager that can show arbitrary content.
/// Managed notification managers can show any content.
///
/// Because notification managers of this type are implemented purely in the
/// framework, they can display arbitrary content, as opposed to notification
/// managers which display notifications using the host operating system's
/// notification mechanism.
///
/// Not meant to be implemented outside the framework.
pub trait IManagedNotificationManager: INotificationManager {
    /// Shows a notification with the given content.
    fn show_content(&self, content: BoxedValue);

    /// Closes the notifications with the given content.
    fn close_content(&self, content: &BoxedValue);
}

/// Notification managers are reference types: they compare by identity.
impl PartialEq for dyn IManagedNotificationManager {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.reference_id() == other.reference_id()
    }
}
