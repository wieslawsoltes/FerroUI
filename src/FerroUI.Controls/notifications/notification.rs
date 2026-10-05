use super::{INotification, NotificationType};
use ferroui_base::data::core::{Maybe, Value, ValueTypes};
use ferroui_base::data::model::{Event, INotifyPropertyChanged, Model};
use ferroui_base::{ferro_model, BoxedValue};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

/// The overridable members of a [`Notification`]: what a class deriving
/// from it can replace. Every member defaults to the implementation of the
/// notification.
///
/// Install the overrides with [`Notification::with_overrides`].
pub trait NotificationOverrides {
    /// Raises the property changed event. `property_name` is `None` when
    /// all properties changed.
    fn on_property_changed(&self, notification: &Notification, property_name: Option<&str>) {
        notification.base_on_property_changed(property_name)
    }
}

/// A notification that can be shown in a window or by the host operating
/// system.
///
/// This class represents a notification that can be displayed either in a
/// window using a
/// [`WindowNotificationManager`](super::WindowNotificationManager) or by the
/// host operating system (to be implemented).
pub struct Notification {
    overrides: Option<Rc<dyn NotificationOverrides>>,
    title: RefCell<Option<String>>,
    message: RefCell<Option<String>>,
    type_: Cell<NotificationType>,
    expiration: Cell<Duration>,
    on_click: RefCell<Option<Rc<dyn Fn()>>>,
    on_close: RefCell<Option<Rc<dyn Fn()>>>,
    property_changed: Event<str>,
}

impl Notification {
    /// The expiration of a notification that states none.
    const DEFAULT_EXPIRATION: Duration = Duration::from_secs(5);

    /// Creates a notification.
    ///
    /// * `title`: the title of the notification.
    /// * `message`: the message to be displayed in the notification.
    /// * `type_`: the [`NotificationType`] of the notification.
    /// * `expiration`: the expiry time at which the notification will close
    ///   (five seconds when `None`). Use [`Duration::ZERO`] for notifications
    ///   that will remain open.
    /// * `on_click`: an action to call when the notification is clicked.
    /// * `on_close`: an action to call when the notification is closed.
    pub fn new(
        title: Option<String>,
        message: Option<String>,
        type_: NotificationType,
        expiration: Option<Duration>,
        on_click: Option<Rc<dyn Fn()>>,
        on_close: Option<Rc<dyn Fn()>>,
    ) -> Rc<Self> {
        Self::create(title, message, type_, expiration, on_click, on_close, None)
    }

    /// Creates a notification without title and message, of the
    /// information type, that expires after five seconds.
    pub fn empty() -> Rc<Self> {
        Self::new(None, None, NotificationType::Information, None, None, None)
    }

    /// Creates a notification whose overridable members are replaced by
    /// `overrides`: the form a class deriving from the notification takes.
    pub fn with_overrides(
        title: Option<String>,
        message: Option<String>,
        type_: NotificationType,
        expiration: Option<Duration>,
        on_click: Option<Rc<dyn Fn()>>,
        on_close: Option<Rc<dyn Fn()>>,
        overrides: Rc<dyn NotificationOverrides>,
    ) -> Rc<Self> {
        Self::create(title, message, type_, expiration, on_click, on_close, Some(overrides))
    }

    fn create(
        title: Option<String>,
        message: Option<String>,
        type_: NotificationType,
        expiration: Option<Duration>,
        on_click: Option<Rc<dyn Fn()>>,
        on_close: Option<Rc<dyn Fn()>>,
        overrides: Option<Rc<dyn NotificationOverrides>>,
    ) -> Rc<Self> {
        Self::ensure_registered();

        let notification = Rc::new(Self {
            overrides,
            title: RefCell::new(None),
            message: RefCell::new(None),
            type_: Cell::new(type_),
            expiration: Cell::new(expiration.unwrap_or(Self::DEFAULT_EXPIRATION)),
            on_click: RefCell::new(on_click),
            on_close: RefCell::new(on_close),
            property_changed: Event::new(),
        });
        // As upstream, the constructor assigns the title and the message
        // through their setters, which notify.
        notification.set_title(title);
        notification.set_message(message);
        notification
    }

    /// Makes the binding metadata of the class and its cast to
    /// [`INotification`] known on the current thread, once.
    fn ensure_registered() {
        thread_local! {
            static REGISTERED: Cell<bool> = const { Cell::new(false) };
        }

        if REGISTERED.replace(true) {
            return;
        }

        <Self as Model>::register();
        ValueTypes::register_boxed_cast::<Notification, Rc<dyn INotification>>(|object| {
            let any: Rc<dyn std::any::Any> = object.clone();
            any.downcast::<Notification>().ok().map(|notification| notification as Rc<dyn INotification>)
        });
    }

    /// The title of the notification.
    pub fn title(&self) -> Option<String> {
        self.title.borrow().clone()
    }

    pub fn set_title(&self, value: Option<String>) {
        if *self.title.borrow() != value {
            *self.title.borrow_mut() = value;
            self.on_property_changed(Some("Title"));
        }
    }

    /// Sets the title; returns the notification for chaining.
    pub fn with_title(self: Rc<Self>, value: Option<String>) -> Rc<Self> {
        self.set_title(value);
        self
    }

    /// The notification message.
    pub fn message(&self) -> Option<String> {
        self.message.borrow().clone()
    }

    pub fn set_message(&self, value: Option<String>) {
        if *self.message.borrow() != value {
            *self.message.borrow_mut() = value;
            self.on_property_changed(Some("Message"));
        }
    }

    /// Sets the message; returns the notification for chaining.
    pub fn with_message(self: Rc<Self>, value: Option<String>) -> Rc<Self> {
        self.set_message(value);
        self
    }

    /// The [`NotificationType`] of the notification.
    pub fn type_(&self) -> NotificationType {
        self.type_.get()
    }

    pub fn set_type(&self, value: NotificationType) {
        self.type_.set(value)
    }

    /// Sets the type; returns the notification for chaining.
    pub fn with_type(self: Rc<Self>, value: NotificationType) -> Rc<Self> {
        self.set_type(value);
        self
    }

    /// The expiration time of the notification after which it will
    /// automatically close. If the value is [`Duration::ZERO`] then the
    /// notification will remain open until the user closes it.
    pub fn expiration(&self) -> Duration {
        self.expiration.get()
    }

    pub fn set_expiration(&self, value: Duration) {
        self.expiration.set(value)
    }

    /// Sets the expiration; returns the notification for chaining.
    pub fn with_expiration(self: Rc<Self>, value: Duration) -> Rc<Self> {
        self.set_expiration(value);
        self
    }

    /// An action to be run when the notification is clicked.
    pub fn on_click(&self) -> Option<Rc<dyn Fn()>> {
        self.on_click.borrow().clone()
    }

    pub fn set_on_click(&self, value: Option<Rc<dyn Fn()>>) {
        *self.on_click.borrow_mut() = value;
    }

    /// Sets the click action; returns the notification for chaining.
    pub fn with_on_click(self: Rc<Self>, value: Option<Rc<dyn Fn()>>) -> Rc<Self> {
        self.set_on_click(value);
        self
    }

    /// An action to be run when the notification is closed.
    pub fn on_close(&self) -> Option<Rc<dyn Fn()>> {
        self.on_close.borrow().clone()
    }

    pub fn set_on_close(&self, value: Option<Rc<dyn Fn()>>) {
        *self.on_close.borrow_mut() = value;
    }

    /// Sets the close action; returns the notification for chaining.
    pub fn with_on_close(self: Rc<Self>, value: Option<Rc<dyn Fn()>>) -> Rc<Self> {
        self.set_on_close(value);
        self
    }

    /// Raises the property changed event. Overridable: see
    /// [`NotificationOverrides::on_property_changed`].
    pub fn on_property_changed(&self, property_name: Option<&str>) {
        match &self.overrides {
            Some(overrides) => overrides.on_property_changed(self, property_name),
            None => self.base_on_property_changed(property_name),
        }
    }

    /// The implementation of
    /// [`on_property_changed`](Self::on_property_changed) of this class, for
    /// overrides.
    pub fn base_on_property_changed(&self, property_name: Option<&str>) {
        // An empty name means that all properties changed.
        self.property_changed.raise(property_name.unwrap_or(""));
    }
}

impl INotification for Notification {
    fn title(&self) -> Option<String> {
        Notification::title(self)
    }

    fn message(&self) -> Option<String> {
        Notification::message(self)
    }

    fn type_(&self) -> NotificationType {
        Notification::type_(self)
    }

    fn expiration(&self) -> Duration {
        Notification::expiration(self)
    }

    fn on_click(&self) -> Option<Rc<dyn Fn()>> {
        Notification::on_click(self)
    }

    fn on_close(&self) -> Option<Rc<dyn Fn()>> {
        Notification::on_close(self)
    }

    fn to_boxed(self: Rc<Self>) -> BoxedValue {
        self
    }
}

impl INotifyPropertyChanged for Notification {
    fn property_changed(&self) -> &Event<str> {
        &self.property_changed
    }
}

// The action properties are not declared: actions are not comparable values.
ferro_model!(Notification, |b| b
    .notify_property_changed()
    .property::<Maybe<String>>("Title", |n| n.title(), |n, v| n.set_title(v))
    .property::<Maybe<String>>("Message", |n| n.message(), |n, v| n.set_message(v))
    .property::<Value<NotificationType>>("Type", |n| n.type_(), |n, v| n.set_type(v))
    .property::<Value<Duration>>("Expiration", |n| n.expiration(), |n, v| n.set_expiration(v)));
