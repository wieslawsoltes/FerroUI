/// Describes the possible positions for notifications that are displayed by
/// a [`WindowNotificationManager`](super::WindowNotificationManager).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum NotificationPosition {
    #[default]
    TopLeft = 0,
    TopRight = 1,
    BottomLeft = 2,
    BottomRight = 3,
    TopCenter = 4,
    BottomCenter = 5,
}
