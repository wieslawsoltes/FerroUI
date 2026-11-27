//! The notifications of a FreeDesktop session over `org.freedesktop.Notifications` (#1276).
//!
//! Not from upstream. The notification server of the desktop (GNOME Shell, Plasma, dunst and
//! the others) shows the notifications of `AppNotificationManager`: the first text is the
//! summary, the other texts and the attribution text the body; the buttons are the actions of
//! the notification and a click on its body is the action `default`; the sound event is the
//! `sound-name` hint (sound theme names), the progress bar the `value` hint, and an image with a
//! `file:` URI the `image-path` hint. The specification has no inputs: the text boxes and combo
//! boxes of a notification are not shown, and its activation has no user input.

use crate::dbus_helper::DBusHelper;
use crate::signal_watch::watch_stream;
use ferroui_base::platform::notifications::{
    AppNotification, AppNotificationActivatedEventArgs, AppNotificationError, AppNotificationManager,
    AppNotificationScenario, AppNotificationSoundEvent, IAppNotificationManagerImpl,
};
use ferroui_base::reactive::IDisposable;
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, HashMap};
use std::rc::{Rc, Weak};
use zbus::Connection;
use zbus::Variant;

const SERVICE: &str = "org.freedesktop.Notifications";
const PATH: &str = "/org/freedesktop/Notifications";

/// `org.freedesktop.Notifications`, with the members that are called and the signals that are
/// listened to.
#[zbus::proxy(interface = "org.freedesktop.Notifications", assume_defaults = false)]
pub trait Notifications {
    #[allow(clippy::too_many_arguments)]
    fn notify(
        &self,
        app_name: &str,
        replaces_id: u32,
        app_icon: &str,
        summary: &str,
        body: &str,
        actions: &[&str],
        hints: HashMap<&str, Variant<'_>>,
        expire_timeout: i32,
    ) -> zbus::Result<u32>;

    fn close_notification(&self, id: u32) -> zbus::Result<()>;

    #[zbus(signal)]
    fn action_invoked(&self, id: u32, action_key: &str) -> zbus::Result<()>;

    #[zbus(signal)]
    fn notification_closed(&self, id: u32, reason: u32) -> zbus::Result<()>;
}

/// The action key of a click on the body of a notification.
const DEFAULT_ACTION: &str = "default";

/// A notification the server shows, or is asked to show.
struct Entry {
    id: u32,
    tag: String,
    group: String,
    /// The id the server gave it; `None` until `Notify` returns.
    server_id: Option<u32>,
    /// Removed before `Notify` returned: closed once it does.
    close_when_shown: bool,
    argument: String,
    button_arguments: Vec<String>,
}

/// The notification service of a FreeDesktop session: the notification server of the desktop
/// over D-Bus. Registered by `use_platform_app_notifications()` of the Linux platform.
pub struct DBusAppNotificationManager {
    connection: Option<Connection>,
    app_name: String,
    proxy: RefCell<Option<NotificationsProxy<'static>>>,
    entries: RefCell<Vec<Entry>>,
    subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
    registered: Cell<bool>,
    this: Weak<Self>,
}

impl DBusAppNotificationManager {
    /// The service over the default connection of the process (no service without a session
    /// bus), named after the executable.
    pub fn new() -> Rc<Self> {
        Self::with_connection(DBusHelper::default_connection(), default_app_name())
    }

    /// The service over `connection`, which shows the notifications as `app_name`.
    pub fn with_connection(connection: Option<Connection>, app_name: impl Into<String>) -> Rc<Self> {
        let app_name = app_name.into();
        Rc::new_cyclic(|this| Self {
            connection,
            app_name,
            proxy: RefCell::new(None),
            entries: RefCell::new(Vec::new()),
            subscriptions: RefCell::new(Vec::new()),
            registered: Cell::new(false),
            this: this.clone(),
        })
    }

    /// Registers the service of the session bus as the notification service of the application.
    pub fn register() {
        ferroui_base::platform::notifications::register_app_notification_manager_impl(Self::new());
    }

    async fn proxy(&self) -> Option<NotificationsProxy<'static>> {
        if let Some(proxy) = self.proxy.borrow().clone() {
            return Some(proxy);
        }
        let connection = self.connection.clone()?;
        let proxy = NotificationsProxy::builder(&connection)
            .destination(SERVICE)
            .path(PATH)
            .cache_properties(zbus::proxy::CacheProperties::No)
            .build()
            .await
            .ok()?;
        *self.proxy.borrow_mut() = Some(proxy.clone());
        Some(proxy)
    }

    fn spawn(&self, task: impl std::future::Future<Output = ()> + 'static) {
        drop(Dispatcher::ui_thread().spawn_local_with_priority(task, DispatcherPriority::DEFAULT));
    }

    fn close(&self, matches: impl Fn(&Entry) -> bool) {
        let mut to_close = Vec::new();
        self.entries.borrow_mut().retain_mut(|entry| {
            if !matches(entry) {
                return true;
            }
            match entry.server_id {
                Some(server_id) => {
                    to_close.push(server_id);
                    false
                }
                None => {
                    entry.close_when_shown = true;
                    true
                }
            }
        });
        if to_close.is_empty() {
            return;
        }
        let this = self.this.clone();
        self.spawn(async move {
            let Some(this) = this.upgrade() else { return };
            let Some(proxy) = this.proxy().await else { return };
            for server_id in to_close {
                let _ = proxy.close_notification(server_id).await;
            }
        });
    }

    fn action_invoked(&self, server_id: u32, action_key: &str) {
        let argument = self.entries.borrow().iter().find(|entry| entry.server_id == Some(server_id)).and_then(
            |entry| {
                if action_key == DEFAULT_ACTION {
                    Some(entry.argument.clone())
                } else {
                    action_key.parse::<usize>().ok().and_then(|index| entry.button_arguments.get(index).cloned())
                }
            },
        );
        if let Some(argument) = argument {
            AppNotificationManager::raise_notification_invoked(&AppNotificationActivatedEventArgs::new(
                argument,
                BTreeMap::new(),
            ));
        }
    }

    fn notification_closed(&self, server_id: u32) {
        self.entries.borrow_mut().retain(|entry| entry.server_id != Some(server_id));
    }
}

impl IAppNotificationManagerImpl for DBusAppNotificationManager {
    fn is_supported(&self) -> bool {
        self.connection.is_some()
    }

    fn register(&self) {
        if self.registered.replace(true) {
            return;
        }
        let this = self.this.clone();
        self.spawn(async move {
            let Some(manager) = this.upgrade() else { return };
            let Some(proxy) = manager.proxy().await else { return };
            drop(manager);
            let invoked = proxy.receive_action_invoked().await;
            let closed = proxy.receive_notification_closed().await;
            let Some(manager) = this.upgrade() else { return };
            if !manager.registered.get() {
                return;
            }
            let mut subscriptions = manager.subscriptions.borrow_mut();
            if let Ok(invoked) = invoked {
                let this = this.clone();
                subscriptions.push(watch_stream(invoked, move |signal| {
                    if let (Some(manager), Ok(args)) = (this.upgrade(), signal.args()) {
                        manager.action_invoked(args.id, args.action_key);
                    }
                }));
            }
            if let Ok(closed) = closed {
                let this = this.clone();
                subscriptions.push(watch_stream(closed, move |signal| {
                    if let (Some(manager), Ok(args)) = (this.upgrade(), signal.args()) {
                        manager.notification_closed(args.id);
                    }
                }));
            }
        });
    }

    fn unregister(&self) {
        self.registered.set(false);
        for subscription in self.subscriptions.take() {
            subscription.dispose();
        }
    }

    fn show(&self, notification: &AppNotification) -> Result<(), AppNotificationError> {
        if self.connection.is_none() {
            return Err(AppNotificationError::NotSupported);
        }
        // The notification it replaces: the one with its id, or with its tag and group.
        let replaces = self
            .entries
            .borrow()
            .iter()
            .find(|entry| {
                entry.id == notification.id()
                    || (!notification.tag().is_empty()
                        && entry.tag == notification.tag()
                        && entry.group == notification.group())
            })
            .and_then(|entry| entry.server_id);
        self.entries.borrow_mut().retain(|entry| {
            !(entry.id == notification.id()
                || (!notification.tag().is_empty()
                    && entry.tag == notification.tag()
                    && entry.group == notification.group()))
        });
        self.entries.borrow_mut().push(Entry {
            id: notification.id(),
            tag: notification.tag().to_string(),
            group: notification.group().to_string(),
            server_id: None,
            close_when_shown: false,
            argument: notification.argument_string(),
            button_arguments: notification.buttons().iter().map(|button| button.argument_string()).collect(),
        });

        let request = NotifyRequest::of(notification, replaces.unwrap_or(0));
        let id = notification.id();
        let this = self.this.clone();
        self.spawn(async move {
            let Some(manager) = this.upgrade() else { return };
            let Some(proxy) = manager.proxy().await else { return };
            let result = proxy
                .notify(
                    &manager.app_name,
                    request.replaces_id,
                    "",
                    &request.summary,
                    &request.body,
                    &request.actions.iter().map(String::as_str).collect::<Vec<_>>(),
                    request.hints(),
                    request.expire_timeout,
                )
                .await;
            let Ok(server_id) = result else {
                manager.entries.borrow_mut().retain(|entry| entry.id != id);
                return;
            };
            let close = {
                let mut entries = manager.entries.borrow_mut();
                match entries.iter_mut().find(|entry| entry.id == id) {
                    Some(entry) if entry.close_when_shown => {
                        entries.retain(|entry| entry.id != id);
                        true
                    }
                    Some(entry) => {
                        entry.server_id = Some(server_id);
                        false
                    }
                    None => false,
                }
            };
            if close {
                let _ = proxy.close_notification(server_id).await;
            }
        });
        Ok(())
    }

    fn remove_by_id(&self, id: u32) {
        self.close(|entry| entry.id == id);
    }

    fn remove_by_tag(&self, tag: &str, group: Option<&str>) {
        self.close(|entry| entry.tag == tag && group.is_none_or(|group| entry.group == group));
    }

    fn remove_by_group(&self, group: &str) {
        self.close(|entry| entry.group == group);
    }

    fn remove_all(&self) {
        self.close(|_| true);
    }
}

/// The arguments of `Notify` for a notification.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct NotifyRequest {
    pub(crate) replaces_id: u32,
    pub(crate) summary: String,
    pub(crate) body: String,
    pub(crate) actions: Vec<String>,
    pub(crate) urgency: u8,
    pub(crate) sound_name: Option<&'static str>,
    pub(crate) suppress_sound: bool,
    pub(crate) value: Option<i32>,
    pub(crate) image_path: Option<String>,
    pub(crate) expire_timeout: i32,
}

impl NotifyRequest {
    pub(crate) fn of(notification: &AppNotification, replaces_id: u32) -> Self {
        let mut lines: Vec<&str> = notification.texts().iter().skip(1).map(String::as_str).collect();
        if let Some(attribution) = notification.attribution_text() {
            lines.push(attribution);
        }
        if let Some(progress) = notification.progress_bar() {
            if !progress.status().is_empty() {
                lines.push(progress.status());
            }
        }
        // The servers that show markup (most of them) read the body as markup.
        let body = escape_markup(&lines.join("\n"));
        let mut actions = vec![DEFAULT_ACTION.to_string(), String::new()];
        for (index, button) in notification.buttons().iter().enumerate() {
            actions.push(index.to_string());
            actions.push(button.content().to_string());
        }
        let scenario = notification.scenario();
        let urgent = matches!(
            scenario,
            AppNotificationScenario::Urgent | AppNotificationScenario::Alarm | AppNotificationScenario::IncomingCall
        );
        let stays = scenario != AppNotificationScenario::Default || notification.has_inputs();
        let image_path = notification
            .hero_image()
            .or(notification.app_logo_override().map(|(uri, _)| uri))
            .and_then(file_path);
        Self {
            replaces_id,
            summary: notification.title().to_string(),
            body,
            actions,
            urgency: if urgent { 2 } else { 1 },
            sound_name: notification.audio_event().map(sound_name),
            suppress_sound: notification.is_audio_muted(),
            value: notification.progress_bar().map(|progress| (progress.value() * 100.0).round() as i32),
            image_path,
            // A notification that waits for the user stays until it is dismissed.
            expire_timeout: if stays { 0 } else { -1 },
        }
    }

    fn hints(&self) -> HashMap<&'static str, Variant<'_>> {
        let mut hints = HashMap::new();
        hints.insert("urgency", Variant::U8(self.urgency));
        if let Some(sound_name) = self.sound_name {
            hints.insert("sound-name", Variant::from(sound_name));
        }
        if self.suppress_sound {
            hints.insert("suppress-sound", Variant::Bool(true));
        }
        if let Some(value) = self.value {
            hints.insert("value", Variant::I32(value));
        }
        if let Some(path) = &self.image_path {
            hints.insert("image-path", Variant::from(path.as_str()));
        }
        hints
    }
}

/// The name of the sound of the FreeDesktop sound theme for a sound event.
fn sound_name(event: AppNotificationSoundEvent) -> &'static str {
    match event {
        AppNotificationSoundEvent::Default => "message",
        AppNotificationSoundEvent::IM | AppNotificationSoundEvent::SMS => "message-new-instant",
        AppNotificationSoundEvent::Mail => "message-new-email",
        AppNotificationSoundEvent::Reminder | AppNotificationSoundEvent::Alarm => "alarm-clock-elapsed",
        AppNotificationSoundEvent::Call => "phone-incoming-call",
    }
}

/// The path of a `file:` URI; `None` for any other.
fn file_path(uri: &str) -> Option<String> {
    let url = url::Url::parse(uri).ok()?;
    (url.scheme() == "file").then(|| url.to_file_path().ok()).flatten().map(|path| path.display().to_string())
}

fn escape_markup(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

fn default_app_name() -> String {
    std::env::current_exe()
        .ok()
        .and_then(|path| path.file_stem().map(|stem| stem.to_string_lossy().into_owned()))
        .unwrap_or_default()
}

#[cfg(test)]
#[path = "dbus_app_notification_manager_tests.rs"]
mod tests;
