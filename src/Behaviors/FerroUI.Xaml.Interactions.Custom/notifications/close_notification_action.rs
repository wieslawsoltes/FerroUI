//! Port of `Notifications/CloseNotificationAction.cs`.

use ferroui_base::data::core::ValueTypes;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, ElementRef, FerroObjectImpl,
    FerroProperty, Nullable, Ref, StyledElementImpl, StyledProperty,
};
use ferroui_controls::NotificationCard;
use ferroui_xaml_interactivity::{StyledElementAction, StyledElementActionImpl};
use std::rc::Rc;

/// Action that closes the specified [`NotificationCard`].
#[repr(C)]
pub struct CloseNotificationAction {
    base: StyledElementAction,
}

ferro_class!(CloseNotificationAction: StyledElementAction);
ferro_class_info!(CloseNotificationAction {
    new: CloseNotificationAction::new,
    markup: {
        content: NotificationCard,
    },
});
ferroui_base::ferro_impl_classes!(CloseNotificationAction: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl CloseNotificationAction {
        /// Gets or sets the notification card to close.
        ///
        /// Deviation (DEVIATIONS.md, Behaviors): the card is usually the element that hosts the
        /// action, which would hold itself through the action; the action does not own the card,
        /// so the value is an element reference.
        pub fn notification_card_property() -> StyledProperty<Option<ElementRef<NotificationCard>>> {
            ValueTypes::register_element_ref::<NotificationCard>();
            FerroProperty::register::<CloseNotificationAction, _>("NotificationCard", None)
        }
    }
}

impl StyledElementActionImpl for CloseNotificationAction {
    /// Executes the action: `sender` is the associated object, `parameter` an optional parameter.
    ///
    /// Returns true if the notification was closed; otherwise, false.
    fn execute(this: &Self, _sender: Option<&BoxedValue>, _parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        if !this.is_enabled() {
            return Some(Rc::new(false));
        }

        let Some(notification_card) = this.notification_card() else {
            return Some(Rc::new(false));
        };

        notification_card.close();
        Some(Rc::new(true))
    }
}

impl CloseNotificationAction {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementAction::construct() }
    }

    /// Creates an action.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The notification card to close: the content of the action in markup.
    pub fn notification_card(&self) -> Option<Ref<NotificationCard>> {
        ElementRef::resolve(&self.get_value(Self::notification_card_property()))
    }

    pub fn set_notification_card(&self, value: impl Into<Nullable<NotificationCard>>) {
        self.set_value(Self::notification_card_property(), ElementRef::from_nullable(value.into().0))
    }
}
