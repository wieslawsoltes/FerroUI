//! `UnsynchronizedNotificationException<T>` (`Utilities/UnsynchronizedNotificationException.cs`).

use ferroui_reactive::{Notification, NotificationKind};
use std::fmt;

/// The error of `validate_synchronization`: a notification arrived while another one was still
/// being delivered (`UnsynchronizedNotificationException<T>`).
pub struct UnsynchronizedNotificationException<T> {
    /// The notification that arrived (`IncomingNotification`).
    pub incoming_notification: Notification<T>,
    /// The notification that was being delivered (`PriorNotification`).
    pub prior_notification: Notification<T>,
}

fn kind_name(kind: NotificationKind) -> &'static str {
    match kind {
        NotificationKind::OnNext => "OnNext",
        NotificationKind::OnError => "OnError",
        NotificationKind::OnCompleted => "OnCompleted",
    }
}

impl<T> fmt::Display for UnsynchronizedNotificationException<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Unsynchronized notification received: Another notification is already being processed (incoming {}, prior {})",
            kind_name(self.incoming_notification.kind()),
            kind_name(self.prior_notification.kind())
        )
    }
}

impl<T> fmt::Debug for UnsynchronizedNotificationException<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl<T> std::error::Error for UnsynchronizedNotificationException<T> {}
