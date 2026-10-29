//! Port of `Notifications/ShowInformationNotificationAction.cs`.

use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, FerroObjectImpl, FerroProperty, Ref,
    StyledElementImpl, StyledProperty,
};
use ferroui_controls::{INotification, INotificationManager, Notification, NotificationType};
use ferroui_xaml_interactivity::{StyledElementAction, StyledElementActionImpl};
use std::rc::Rc;

/// Shows an information notification using an [`INotificationManager`].
#[repr(C)]
pub struct ShowInformationNotificationAction {
    base: StyledElementAction,
}

ferro_class!(ShowInformationNotificationAction: StyledElementAction);
ferro_class_info!(ShowInformationNotificationAction {
    new: ShowInformationNotificationAction::new,
    markup: {
        property_attributes: [
            NotificationManager: [ResolveByName],
        ],
    },
});
ferroui_base::ferro_impl_classes!(ShowInformationNotificationAction: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl ShowInformationNotificationAction {
        /// Identifies the `NotificationManager` property.
        pub fn notification_manager_property() -> StyledProperty<Option<Rc<dyn INotificationManager>>> {
            FerroProperty::register::<ShowInformationNotificationAction, _>("NotificationManager", None)
        }

        /// Identifies the `Title` property.
        pub fn title_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<ShowInformationNotificationAction, _>("Title", None)
        }

        /// Identifies the `Message` property.
        pub fn message_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<ShowInformationNotificationAction, _>("Message", None)
        }
    }
}

impl StyledElementActionImpl for ShowInformationNotificationAction {
    fn execute(this: &Self, _sender: Option<&BoxedValue>, _parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        if !this.is_enabled() {
            return Some(Rc::new(false));
        }

        let Some(manager) = this.notification_manager() else {
            return Some(Rc::new(false));
        };

        let notification: Rc<dyn INotification> = Notification::new(
            Some(this.title().unwrap_or_default()),
            Some(this.message().unwrap_or_default()),
            NotificationType::Information,
            None,
            None,
            None,
        );
        manager.show(notification);
        Some(Rc::new(true))
    }
}

impl ShowInformationNotificationAction {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementAction::construct() }
    }

    /// Creates an action.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the [`INotificationManager`] used to display the notification. This is a
    /// property of the object model; markup resolves a name given for it to the named manager.
    pub fn notification_manager(&self) -> Option<Rc<dyn INotificationManager>> {
        self.get_value(Self::notification_manager_property())
    }

    pub fn set_notification_manager(&self, value: Option<Rc<dyn INotificationManager>>) {
        self.set_value(Self::notification_manager_property(), value)
    }

    /// Gets or sets the notification title. This is a property of the object model.
    pub fn title(&self) -> Option<String> {
        self.get_value(Self::title_property())
    }

    pub fn set_title(&self, value: Option<String>) {
        self.set_value(Self::title_property(), value)
    }

    /// Gets or sets the notification message. This is a property of the object model.
    pub fn message(&self) -> Option<String> {
        self.get_value(Self::message_property())
    }

    pub fn set_message(&self, value: Option<String>) {
        self.set_value(Self::message_property(), value)
    }
}
