use super::{
    IManagedNotificationManager, INotification, INotificationManager, NotificationCard, NotificationPosition,
    NotificationType,
};
use crate::primitives::{
    AdornerLayer, TemplateAppliedEventArgs, TemplatedControl, TemplatedControlImpl, TemplatedControlImplExt,
    VisualLayerManager,
};
use crate::{ControlImpl, Controls, Panel, TopLevel};
use ferroui_base::input::{InputElement, InputElementImpl, PointerPressedEventArgs};
use ferroui_base::interactivity::{Interactive, InteractiveImpl, RoutedEventHandlerToken};
use ferroui_base::layout::{HorizontalAlignment, Layoutable, LayoutableImpl, VerticalAlignment};
use ferroui_base::threading::{Dispatcher, DispatcherPriority, DispatcherTimer};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate, BoxedValue, FerroObject,
    FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Ref, StyledElementImpl, StyledProperty, VisualImpl,
    VisualImplExt, VisualTreeAttachmentEventArgs,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

const PC_TOP_LEFT: &str = ":topleft";
const PC_TOP_RIGHT: &str = ":topright";
const PC_BOTTOM_LEFT: &str = ":bottomleft";
const PC_BOTTOM_RIGHT: &str = ":bottomright";
const PC_TOP_CENTER: &str = ":topcenter";
const PC_BOTTOM_CENTER: &str = ":bottomcenter";

/// An [`INotificationManager`] that displays notifications in a window.
#[repr(C)]
pub struct WindowNotificationManager {
    base: TemplatedControl,
    /// The notification cards: a list of its own until the template is
    /// applied, the children of the items panel of the template afterwards.
    items: RefCell<Controls>,
    /// The handler of the template applied event of the hosting top-level.
    top_level_template_applied: Cell<Option<RoutedEventHandlerToken>>,
}

ferro_class!(WindowNotificationManager: TemplatedControl);

ferro_class_info!(WindowNotificationManager {
    new: WindowNotificationManager::new,
    interfaces: [
        Rc<dyn IManagedNotificationManager> => managed_notification_manager_of,
        Rc<dyn INotificationManager> => notification_manager_of,
    ],
    markup: {
        attributes: [
            TemplatePart("PART_Items", type(Ref<Panel>)),
            PseudoClasses(":topleft", ":topright", ":bottomleft", ":bottomright", ":topcenter", ":bottomcenter"),
        ],
    },
});

ferro_impl_classes!(
    WindowNotificationManager: StyledElementImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);

impl FerroObjectImpl for WindowNotificationManager {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.update_pseudo_classes(this.position());
    }

    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        if change.property() == Self::position_property().as_property() {
            this.update_pseudo_classes(change.get_new_value::<NotificationPosition>());
        }
    }
}

impl VisualImpl for WindowNotificationManager {
    fn on_detached_from_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_detached_from_visual_tree(this, e);

        this.items().clear();
    }
}

impl TemplatedControlImpl for WindowNotificationManager {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        Self::parent_on_apply_template(this, e);

        let items_control = e.name_scope().find_as::<Panel>("PART_Items");

        if let Some(items_control) = items_control {
            *this.items.borrow_mut() = items_control.children();
        }
    }
}

ferro_properties! {
    impl WindowNotificationManager {
        /// Defines the `Position` property.
        pub fn position_property() -> StyledProperty<NotificationPosition> {
            FerroProperty::register::<WindowNotificationManager, _>("Position", NotificationPosition::TopRight)
        }

        /// Defines the `MaxItems` property.
        pub fn max_items_property() -> StyledProperty<i32> {
            FerroProperty::register::<WindowNotificationManager, _>("MaxItems", 5)
        }
    }
}

impl WindowNotificationManager {
    fn static_constructor() {
        Layoutable::horizontal_alignment_property()
            .override_default_value::<WindowNotificationManager>(HorizontalAlignment::Stretch);
        Layoutable::vertical_alignment_property()
            .override_default_value::<WindowNotificationManager>(VerticalAlignment::Stretch);
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: TemplatedControl::construct(),
            items: RefCell::new(Controls::new()),
            top_level_template_applied: Cell::new(None),
        }
    }

    /// Creates a notification manager that is not hosted yet.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Creates a notification manager hosted by `host`: the top-level that
    /// will host the control.
    pub fn with_host(host: Option<&Ref<TopLevel>>) -> Ref<Self> {
        let this = Self::new();
        if let Some(host) = host {
            this.install_from_top_level(host);
        }
        this
    }

    /// The list the notification cards are in.
    fn items(&self) -> Controls {
        self.items.borrow().clone()
    }

    /// Currently active notifications.
    pub(crate) fn notifications(&self) -> Vec<Ref<NotificationCard>> {
        self.items().snapshot().iter().filter_map(|item| item.cast::<NotificationCard>()).collect()
    }

    /// Defines which corner of the screen notifications can be displayed in.
    pub fn position(&self) -> NotificationPosition {
        self.get_value(Self::position_property())
    }

    pub fn set_position(&self, value: NotificationPosition) {
        self.set_value(Self::position_property(), value)
    }

    /// Defines the maximum number of notifications visible at once.
    pub fn max_items(&self) -> i32 {
        self.get_value(Self::max_items_property())
    }

    pub fn set_max_items(&self, value: i32) {
        self.set_value(Self::max_items_property(), value)
    }

    /// Shows a notification.
    pub fn show(&self, content: Rc<dyn INotification>) {
        let (type_, expiration, on_click, on_close) =
            (content.type_(), content.expiration(), content.on_click(), content.on_close());
        self.show_with(content.to_boxed(), type_, Some(expiration), on_click, on_close, None);
    }

    /// Shows a notification with the given content: a notification, or any
    /// other content, which is shown as information.
    pub fn show_content(&self, content: BoxedValue) {
        if let Some(notification) = <dyn INotification>::from_boxed(&content) {
            self.show_with(
                content,
                notification.type_(),
                Some(notification.expiration()),
                notification.on_click(),
                notification.on_close(),
                None,
            );
        } else {
            self.show_with(content, NotificationType::Information, None, None, None, None);
        }
    }

    /// Shows a notification.
    ///
    /// * `content`: the content of the notification.
    /// * `type_`: the type of the notification.
    /// * `expiration`: the expiration time of the notification after which
    ///   it will automatically close (five seconds when `None`). If the
    ///   value is zero then the notification will remain open until the user
    ///   closes it.
    /// * `on_click`: an action to be run when the notification is clicked.
    /// * `on_close`: an action to be run when the notification is closed.
    /// * `classes`: style classes to apply.
    pub fn show_with(
        &self,
        content: BoxedValue,
        type_: NotificationType,
        expiration: Option<Duration>,
        on_click: Option<Rc<dyn Fn()>>,
        on_close: Option<Rc<dyn Fn()>>,
        classes: Option<&[&str]>,
    ) {
        Dispatcher::ui_thread().verify_access();

        let notification_control = NotificationCard::new();
        notification_control.set_content(Some(content));
        notification_control.set_notification_type(type_);

        // Add style classes if any
        if let Some(classes) = classes {
            for class in classes {
                notification_control.classes().add(class);
            }
        }

        // The handlers are held by the card: they refer to it, and to the
        // manager, weakly.
        let weak_this = self.to_ref().downgrade();
        let weak_control = notification_control.downgrade();
        notification_control.notification_closed(move |_, _| {
            if let Some(on_close) = &on_close {
                on_close();
            }
            if let (Some(this), Some(notification_control)) = (weak_this.upgrade(), weak_control.upgrade()) {
                this.items().remove(notification_control);
            }
        });

        notification_control.add_handler(
            InputElement::pointer_pressed_event(),
            move |sender: &Interactive, _: &PointerPressedEventArgs| {
                if let Some(on_click) = &on_click {
                    on_click();
                }

                let sender: &FerroObject = sender;
                if let Some(card) = sender.downcast_ref::<NotificationCard>() {
                    card.close();
                }
            },
        );

        self.items().add(notification_control.clone());

        let notifications = self.notifications();
        let open_count = notifications.iter().filter(|i| !i.is_closing()).count();
        if i64::try_from(open_count).unwrap_or(i64::MAX) > i64::from(self.max_items()) {
            if let Some(first) = notifications.iter().find(|i| !i.is_closing()) {
                first.close();
            }
        }

        if expiration == Some(Duration::ZERO) {
            return;
        }

        DispatcherTimer::run_once(
            move || notification_control.close(),
            expiration.unwrap_or(Duration::from_secs(5)),
            DispatcherPriority::DEFAULT,
        );
    }

    /// Closes a notification.
    pub fn close(&self, notification: &Rc<dyn INotification>) {
        self.close_content(&notification.clone().to_boxed())
    }

    /// Closes the notifications with the given content.
    pub fn close_content(&self, content: &BoxedValue) {
        Dispatcher::ui_thread().verify_access();

        // A notification is the same content in either of its untyped forms:
        // the object itself, or a box that holds its contract handle.
        let notification = <dyn INotification>::from_boxed(content);
        for card in self.notifications() {
            let Some(card_content) = card.content() else {
                continue;
            };
            let equals = *card_content == **content
                || match (&notification, <dyn INotification>::from_boxed(&card_content)) {
                    (Some(notification), Some(card_notification)) => **notification == *card_notification,
                    _ => false,
                };
            if equals {
                card.close();
            }
        }
    }

    /// Closes all notifications.
    pub fn close_all(&self) {
        Dispatcher::ui_thread().verify_access();

        for card in self.notifications() {
            card.close();
        }
    }

    /// The manager as the contract of managed notification managers.
    pub fn as_managed_notification_manager(&self) -> Rc<dyn IManagedNotificationManager> {
        Rc::new(ManagerHandle(self.to_ref()))
    }

    /// The manager as the contract of notification managers.
    pub fn as_notification_manager(&self) -> Rc<dyn INotificationManager> {
        Rc::new(ManagerHandle(self.to_ref()))
    }

    /// Installs the manager within the adorner layer of the top-level.
    fn install_from_top_level(&self, top_level: &Ref<TopLevel>) {
        // As upstream, the top-level keeps the manager alive through the
        // handler until its template is applied again.
        let this = self.to_ref();
        let token = top_level.template_applied(move |sender, e| this.top_level_on_template_applied(sender, e));
        self.top_level_template_applied.set(Some(token));

        let adorner =
            top_level.find_descendant_of_type::<VisualLayerManager>(false).and_then(|manager| manager.adorner_layer());
        if let Some(adorner) = adorner {
            adorner.children().add(self.to_ref());
            AdornerLayer::set_adorned_element(self, adorner);
        }
    }

    fn top_level_on_template_applied(&self, sender: &Interactive, _e: &TemplateAppliedEventArgs) {
        if let Some(adorner_layer) = self.parent().and_then(|parent| parent.cast::<AdornerLayer>()) {
            adorner_layer.children().remove(self.to_ref());
            AdornerLayer::set_adorned_element(self, None);
        }

        // Reinstall notification manager on template reapplied.
        let top_level = sender.to_ref().cast::<TopLevel>().expect("the sender of the event is the top-level");
        if let Some(token) = self.top_level_template_applied.take() {
            top_level.remove_handler(TemplatedControl::template_applied_event(), token);
        }
        self.install_from_top_level(&top_level);
    }

    fn update_pseudo_classes(&self, position: NotificationPosition) {
        self.pseudo_classes().set(PC_TOP_LEFT, position == NotificationPosition::TopLeft);
        self.pseudo_classes().set(PC_TOP_RIGHT, position == NotificationPosition::TopRight);
        self.pseudo_classes().set(PC_BOTTOM_LEFT, position == NotificationPosition::BottomLeft);
        self.pseudo_classes().set(PC_BOTTOM_RIGHT, position == NotificationPosition::BottomRight);
        self.pseudo_classes().set(PC_TOP_CENTER, position == NotificationPosition::TopCenter);
        self.pseudo_classes().set(PC_BOTTOM_CENTER, position == NotificationPosition::BottomCenter);
    }
}

/// The handle of a manager as the contracts it implements.
struct ManagerHandle(Ref<WindowNotificationManager>);

impl INotificationManager for ManagerHandle {
    fn show(&self, notification: Rc<dyn INotification>) {
        self.0.show(notification)
    }

    fn close(&self, notification: &Rc<dyn INotification>) {
        self.0.close(notification)
    }

    fn close_all(&self) {
        self.0.close_all()
    }

    fn reference_id(&self) -> *const () {
        let object: &FerroObject = &self.0;
        object as *const FerroObject as *const ()
    }
}

impl IManagedNotificationManager for ManagerHandle {
    fn show_content(&self, content: BoxedValue) {
        self.0.show_content(content)
    }

    fn close_content(&self, content: &BoxedValue) {
        self.0.close_content(content)
    }
}

fn managed_notification_manager_of(manager: Ref<WindowNotificationManager>) -> Rc<dyn IManagedNotificationManager> {
    manager.as_managed_notification_manager()
}

fn notification_manager_of(manager: Ref<WindowNotificationManager>) -> Rc<dyn INotificationManager> {
    manager.as_notification_manager()
}
