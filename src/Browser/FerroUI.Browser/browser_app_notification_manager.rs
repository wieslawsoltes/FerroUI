//! The notifications of a web page through the Notification API (#1276).
//!
//! Not from upstream. The page's `Notification` objects show the notifications of
//! `AppNotificationManager`: the first text is the title, the other texts and the attribution text
//! the body; the tag (with the group) is the tag of the notification, so that one replaces
//! another; an app logo and a hero image with an `http:`, `https:` or `data:` URI are the icon
//! and the image. A click on a notification raises `NotificationInvoked` with its arguments. The
//! buttons and inputs need the notifications of a service worker, which the platform does not
//! register: they are not shown. Browsers allow notifications for secure pages that the user
//! allowed: `register` asks the user, and `show` fails until the user allowed them.

use ferroui_base::platform::notifications::{
    AppNotification, AppNotificationActivatedEventArgs, AppNotificationError, AppNotificationManager,
    AppNotificationScenario, IAppNotificationManagerImpl,
};
use std::cell::RefCell;
use std::collections::BTreeMap;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    /// `Notification`.
    #[wasm_bindgen(js_name = Notification)]
    type JsNotification;

    #[wasm_bindgen(catch, constructor, js_class = "Notification")]
    fn new(title: &str, options: &JsValue) -> Result<JsNotification, JsValue>;

    #[wasm_bindgen(static_method_of = JsNotification, js_class = "Notification", getter)]
    fn permission() -> String;

    #[wasm_bindgen(catch, static_method_of = JsNotification, js_class = "Notification", js_name = requestPermission)]
    fn request_permission() -> Result<JsValue, JsValue>;

    #[wasm_bindgen(method)]
    fn close(this: &JsNotification);

    #[wasm_bindgen(method, setter = onclick)]
    fn set_onclick(this: &JsNotification, handler: Option<&js_sys::Function>);
}

/// A notification the page shows.
struct Entry {
    id: u32,
    tag: String,
    group: String,
    notification: JsNotification,
    _on_click: Closure<dyn FnMut()>,
}

/// The notification service of a web page: the Notification API. Registered by
/// `use_platform_app_notifications()` of the browser platform.
#[derive(Default)]
pub struct BrowserAppNotificationManager {
    entries: RefCell<Vec<Entry>>,
}

impl BrowserAppNotificationManager {
    /// The service of the page.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers the Notification API as the notification service of the application.
    pub fn register() {
        ferroui_base::platform::notifications::register_app_notification_manager_impl(std::rc::Rc::new(Self::new()));
    }

    fn remove(&self, matches: impl Fn(&Entry) -> bool) {
        self.entries.borrow_mut().retain(|entry| {
            if matches(entry) {
                entry.notification.set_onclick(None);
                entry.notification.close();
                false
            } else {
                true
            }
        });
    }
}

/// Whether the page has the Notification API (a secure page of a browser that has it).
fn has_notification_api() -> bool {
    cfg!(target_arch = "wasm32")
        && js_sys::Reflect::has(&js_sys::global(), &JsValue::from_str("Notification")).unwrap_or(false)
}

/// The URI of an image the page can load.
fn web_image(uri: &str) -> Option<&str> {
    let scheme = uri.split_once(':').map_or("", |(scheme, _)| scheme);
    ["http", "https", "data", "blob"].iter().any(|known| scheme.eq_ignore_ascii_case(known)).then_some(uri)
}

/// The options of the `Notification` of `notification`.
fn options(notification: &AppNotification, tag: &str) -> JsValue {
    let options = js_sys::Object::new();
    let set = |key: &str, value: JsValue| {
        let _ = js_sys::Reflect::set(&options, &JsValue::from_str(key), &value);
    };
    let mut lines: Vec<&str> = notification.texts().iter().skip(1).map(String::as_str).collect();
    if let Some(attribution) = notification.attribution_text() {
        lines.push(attribution);
    }
    set("body", JsValue::from_str(&lines.join("\n")));
    set("tag", JsValue::from_str(tag));
    if let Some(icon) = notification.app_logo_override().and_then(|(uri, _)| web_image(uri)) {
        set("icon", JsValue::from_str(icon));
    }
    if let Some(image) = notification.hero_image().and_then(web_image) {
        set("image", JsValue::from_str(image));
    }
    if notification.is_audio_muted() {
        set("silent", JsValue::TRUE);
    }
    if notification.scenario() != AppNotificationScenario::Default || notification.has_inputs() {
        set("requireInteraction", JsValue::TRUE);
    }
    if let Some(time) = notification.time_stamp() {
        let unix_ms = time.to_universal_time().subtract_date_time(ferroui_base::utilities::DateTime::UNIX_EPOCH);
        set("timestamp", JsValue::from_f64(unix_ms.total_milliseconds()));
    }
    options.into()
}

impl IAppNotificationManagerImpl for BrowserAppNotificationManager {
    fn is_supported(&self) -> bool {
        has_notification_api() && JsNotification::permission() != "denied"
    }

    fn register(&self) {
        if has_notification_api() && JsNotification::permission() == "default" {
            // The browser asks the user; the answer is the permission `show` reads.
            let _ = JsNotification::request_permission();
        }
    }

    fn show(&self, notification: &AppNotification) -> Result<(), AppNotificationError> {
        if !has_notification_api() {
            return Err(AppNotificationError::NotSupported);
        }
        if JsNotification::permission() != "granted" {
            return Err(AppNotificationError::Disabled);
        }
        let tag = if notification.tag().is_empty() {
            format!("ferroui.{}", notification.id())
        } else {
            format!("{}.{}", notification.group(), notification.tag())
        };
        let shown = JsNotification::new(notification.title(), &options(notification, &tag))
            .map_err(|error| AppNotificationError::Failed(format!("{error:?}")))?;
        let argument = notification.argument_string();
        let on_click = Closure::<dyn FnMut()>::new(move || {
            AppNotificationManager::raise_notification_invoked(&AppNotificationActivatedEventArgs::new(
                argument.clone(),
                BTreeMap::new(),
            ));
        });
        shown.set_onclick(Some(on_click.as_ref().unchecked_ref()));
        let mut entries = self.entries.borrow_mut();
        // The page replaces a notification with the same tag itself.
        entries.retain(|entry| !(entry.id == notification.id() || (entry.tag == notification.tag()
            && !entry.tag.is_empty()
            && entry.group == notification.group())));
        entries.push(Entry {
            id: notification.id(),
            tag: notification.tag().to_string(),
            group: notification.group().to_string(),
            notification: shown,
            _on_click: on_click,
        });
        Ok(())
    }

    fn remove_by_id(&self, id: u32) {
        self.remove(|entry| entry.id == id);
    }

    fn remove_by_tag(&self, tag: &str, group: Option<&str>) {
        self.remove(|entry| entry.tag == tag && group.is_none_or(|group| entry.group == group));
    }

    fn remove_by_group(&self, group: &str) {
        self.remove(|entry| entry.group == group);
    }

    fn remove_all(&self) {
        self.remove(|_| true);
    }
}
