use super::NotificationType;
use crate::unbox_item;
use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::BoxedValue;
use std::rc::Rc;
use std::time::Duration;

/// Represents a notification that can be shown in a window or by the host
/// operating system.
///
/// Not meant to be implemented outside the framework.
pub trait INotification {
    /// Gets the title of the notification.
    fn title(&self) -> Option<String>;

    /// Gets the notification message.
    fn message(&self) -> Option<String>;

    /// Gets the [`NotificationType`] of the notification.
    fn type_(&self) -> NotificationType;

    /// Gets the expiration time of the notification after which it will
    /// automatically close. If the value is [`Duration::ZERO`] then the
    /// notification will remain open until the user closes it.
    fn expiration(&self) -> Duration;

    /// Gets an action to be run when the notification is clicked.
    fn on_click(&self) -> Option<Rc<dyn Fn()>>;

    /// Gets an action to be run when the notification is closed.
    fn on_close(&self) -> Option<Rc<dyn Fn()>>;

    /// The notification as an untyped value: the object itself, as content
    /// controls display it and bindings read it.
    fn to_boxed(self: Rc<Self>) -> BoxedValue;

    /// The identity of the notification: handles to the same notification
    /// compare equal.
    #[doc(hidden)]
    fn reference_id(&self) -> *const () {
        self as *const Self as *const ()
    }
}

/// Notifications are reference types: they compare by identity.
impl PartialEq for dyn INotification {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.reference_id() == other.reference_id()
    }
}

impl dyn INotification {
    /// The notification an untyped value is, if its type declares the
    /// contract (the `is` test of the reference implementation).
    pub fn from_boxed(value: &BoxedValue) -> Option<Rc<dyn INotification>> {
        let notification = ValueTypes::try_cast(value, ValueType::of::<Rc<dyn INotification>>())?;
        unbox_item::<Rc<dyn INotification>>(&Some(notification))
    }
}
