use super::{INotification, NotificationType};
use crate::primitives::TemplatedControlImpl;
use crate::{Button, ContentControl, ContentControlImpl, ControlImpl};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::{
    Interactive, InteractiveImpl, RoutedEvent, RoutedEventArgs, RoutedEventHandlerToken, RoutingStrategies,
};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, ferro_routed_event, instantiate,
    AttachedProperty, BoxedValue, DirectProperty, FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Ref,
    StyledElement, StyledElementImpl, StyledProperty, VisualImpl, WeakRef,
};
use std::cell::{Cell, RefCell};

const PC_ERROR: &str = ":error";
const PC_INFORMATION: &str = ":information";
const PC_SUCCESS: &str = ":success";
const PC_WARNING: &str = ":warning";

thread_local! {
    /// The click handlers of the buttons whose `CloseOnClick` is set: what
    /// removing the handler needs when the property is reset.
    static CLOSE_ON_CLICK_HANDLERS: RefCell<Vec<(WeakRef<Button>, RoutedEventHandlerToken)>> =
        const { RefCell::new(Vec::new()) };
}

/// Control that represents and displays a notification.
#[repr(C)]
pub struct NotificationCard {
    base: ContentControl,
    is_closing: Cell<bool>,
}

ferro_class!(NotificationCard: ContentControl);

ferro_class_info!(NotificationCard {
    new: NotificationCard::new,
    markup: {
        attributes: [PseudoClasses(":error", ":information", ":success", ":warning")],
    },
});

ferro_impl_classes!(
    NotificationCard: StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    ContentControlImpl
);

impl FerroObjectImpl for NotificationCard {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.update_notification_type();
    }

    fn on_property_changed(this: &Self, e: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, e);

        if e.property() == ContentControl::content_property().as_property() {
            let content = e.get_new_value::<Option<BoxedValue>>();
            if let Some(notification) = content.as_ref().and_then(<dyn INotification>::from_boxed) {
                this.set_value(Self::notification_type_property(), notification.type_());
            }
        }

        if e.property() == Self::notification_type_property().as_property() {
            this.update_notification_type();
        }

        if e.property() == Self::is_closed_property().as_property() {
            if !this.is_closing() && !this.is_closed() {
                return;
            }

            this.raise_event(&RoutedEventArgs::with_event(Self::notification_closed_event()));
        }
    }
}

ferro_properties! {
    impl NotificationCard {
        /// Defines the `IsClosing` property.
        pub fn is_closing_property() -> DirectProperty<NotificationCard, bool> {
            FerroProperty::register_direct::<NotificationCard, _>("IsClosing", |o| o.is_closing(), None, false)
        }

        /// Defines the `IsClosed` property.
        pub fn is_closed_property() -> StyledProperty<bool> {
            FerroProperty::register::<NotificationCard, _>("IsClosed", false)
        }

        /// Defines the `NotificationType` property.
        pub fn notification_type_property() -> StyledProperty<NotificationType> {
            FerroProperty::register::<NotificationCard, _>("NotificationType", NotificationType::Information)
        }

        /// Defines the `CloseOnClick` attached property.
        pub fn close_on_click_property() -> AttachedProperty<bool> {
            FerroProperty::register_attached::<NotificationCard, Button, _>("CloseOnClick", false)
        }
    }
}

impl NotificationCard {
    ferro_routed_event!(
        /// Defines the `NotificationClosed` event.
        pub fn notification_closed_event() -> RoutedEvent<RoutedEventArgs> {
            RoutedEvent::register::<NotificationCard, _>("NotificationClosed", RoutingStrategies::BUBBLE)
        }
    );

    fn static_constructor() {
        Self::close_on_click_property()
            .changed()
            .add_class_handler::<Button>(Self::on_close_on_click_property_changed);
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: ContentControl::construct(), is_closing: Cell::new(false) }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Determines if the notification is already closing.
    pub fn is_closing(&self) -> bool {
        self.is_closing.get()
    }

    fn set_is_closing(&self, value: bool) {
        self.set_and_raise_cell(Self::is_closing_property(), &self.is_closing, value);
    }

    /// Determines if the notification is closed.
    pub fn is_closed(&self) -> bool {
        self.get_value(Self::is_closed_property())
    }

    pub fn set_is_closed(&self, value: bool) {
        self.set_value(Self::is_closed_property(), value)
    }

    /// The type of the notification.
    pub fn notification_type(&self) -> NotificationType {
        self.get_value(Self::notification_type_property())
    }

    pub fn set_notification_type(&self, value: NotificationType) {
        self.set_value(Self::notification_type_property(), value)
    }

    /// Raised when the notification card has closed.
    pub fn notification_closed(
        &self,
        handler: impl Fn(&Interactive, &RoutedEventArgs) + 'static,
    ) -> RoutedEventHandlerToken {
        self.add_handler(Self::notification_closed_event(), handler)
    }

    /// Gets the value of the `CloseOnClick` attached property of a button.
    pub fn get_close_on_click(obj: &Button) -> bool {
        obj.get_value(Self::close_on_click_property())
    }

    /// Sets the value of the `CloseOnClick` attached property of a button:
    /// whether clicking the button closes the notification card it is in.
    pub fn set_close_on_click(obj: &Button, value: bool) {
        obj.set_value(Self::close_on_click_property(), value)
    }

    fn on_close_on_click_property_changed(button: &Button, e: &FerroPropertyChangedEventArgs<'_>) {
        let value = e.get_new_value::<bool>();
        let button_ref = button.to_ref();

        // The handler of this button, if it has one; entries of buttons that
        // no longer exist are dropped on the way.
        let existing = CLOSE_ON_CLICK_HANDLERS.with(|handlers| {
            let mut handlers = handlers.borrow_mut();
            let mut existing = None;
            handlers.retain(|(candidate, token)| match candidate.upgrade() {
                Some(candidate) if candidate.ptr_eq(&button_ref) => {
                    existing = Some(*token);
                    false
                }
                Some(_) => true,
                None => false,
            });
            existing
        });

        if let Some(token) = existing {
            button.remove_handler(Button::click_event(), token);
        }

        if value {
            let token = button.add_handler(Button::click_event(), Self::button_click);
            CLOSE_ON_CLICK_HANDLERS.with(|handlers| handlers.borrow_mut().push((button_ref.downgrade(), token)));
        }
    }

    /// Called when a button inside the notification is clicked.
    fn button_click(sender: &Interactive, _e: &RoutedEventArgs) {
        let btn: &StyledElement = sender;
        let mut ancestor = btn.parent();
        while let Some(current) = ancestor {
            if let Some(notification) = current.cast::<NotificationCard>() {
                notification.close();
                break;
            }
            ancestor = current.parent();
        }
    }

    /// Closes the notification card.
    pub fn close(&self) {
        if self.is_closing() {
            return;
        }

        self.set_is_closing(true);
    }

    fn update_notification_type(&self) {
        let notification_type = self.notification_type();
        self.pseudo_classes().set(PC_ERROR, notification_type == NotificationType::Error);
        self.pseudo_classes().set(PC_INFORMATION, notification_type == NotificationType::Information);
        self.pseudo_classes().set(PC_SUCCESS, notification_type == NotificationType::Success);
        self.pseudo_classes().set(PC_WARNING, notification_type == NotificationType::Warning);
    }
}
