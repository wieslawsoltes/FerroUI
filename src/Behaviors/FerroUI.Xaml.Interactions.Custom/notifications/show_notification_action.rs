//! Port of `Notifications/ShowNotificationAction.cs`.

use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, FerroObjectImpl, FerroProperty, Ref,
    StyledElementImpl, StyledProperty,
};
use ferroui_controls::{INotification, INotificationManager};
use ferroui_xaml_interactivity::{StyledElementAction, StyledElementActionImpl};
use std::rc::Rc;

/// Shows a notification using an [`INotificationManager`].
#[repr(C)]
pub struct ShowNotificationAction {
    base: StyledElementAction,
}

ferro_class!(ShowNotificationAction: StyledElementAction);
ferro_class_info!(ShowNotificationAction {
    new: ShowNotificationAction::new,
    markup: {
        content: Notification,
        property_attributes: [
            NotificationManager: [ResolveByName],
        ],
    },
});
ferroui_base::ferro_impl_classes!(ShowNotificationAction: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl ShowNotificationAction {
        /// Identifies the `NotificationManager` property.
        pub fn notification_manager_property() -> StyledProperty<Option<Rc<dyn INotificationManager>>> {
            FerroProperty::register::<ShowNotificationAction, _>("NotificationManager", None)
        }

        /// Identifies the `Notification` property.
        pub fn notification_property() -> StyledProperty<Option<Rc<dyn INotification>>> {
            FerroProperty::register::<ShowNotificationAction, _>("Notification", None)
        }
    }
}

impl StyledElementActionImpl for ShowNotificationAction {
    fn execute(this: &Self, _sender: Option<&BoxedValue>, _parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        if !this.is_enabled() {
            return Some(Rc::new(false));
        }

        let (Some(manager), Some(notification)) = (this.notification_manager(), this.notification()) else {
            return Some(Rc::new(false));
        };

        manager.show(notification);
        Some(Rc::new(true))
    }
}

impl ShowNotificationAction {
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

    /// Gets or sets the [`INotification`] instance to show. This is a property of the object
    /// model, and the content of the action in markup.
    pub fn notification(&self) -> Option<Rc<dyn INotification>> {
        self.get_value(Self::notification_property())
    }

    pub fn set_notification(&self, value: Option<Rc<dyn INotification>>) {
        self.set_value(Self::notification_property(), value)
    }
}
