/// Enumeration of types for an [`INotification`](super::INotification).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum NotificationType {
    #[default]
    Information = 0,
    Success = 1,
    Warning = 2,
    Error = 3,
}
