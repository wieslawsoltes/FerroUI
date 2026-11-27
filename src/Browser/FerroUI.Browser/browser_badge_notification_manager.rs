//! The badge of a web application through the Badging API (#1276).
//!
//! Not from upstream. `navigator.setAppBadge` shows the badge of `BadgeNotificationManager` on
//! the icon of the installed web application (the browser shows it for an application the user
//! installed; for a page in a tab the call does nothing). A count is the badge's number; a glyph is
//! the flag badge of the API (`setAppBadge()` without a number), which shows no glyph; clearing
//! is `clearAppBadge`. The promises the calls return are not awaited: a refused call changes
//! nothing.

use ferroui_base::platform::notifications::{BadgeNotificationGlyph, IBadgeNotificationManagerImpl};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

/// The badge service of a web page: the Badging API. Registered by
/// `use_platform_badge_notifications()` of the browser platform.
#[derive(Default)]
pub struct BrowserBadgeNotificationManager;

impl BrowserBadgeNotificationManager {
    /// The service of the page.
    pub fn new() -> Self {
        Self
    }

    /// Registers the Badging API as the badge service of the application.
    pub fn register() {
        ferroui_base::platform::notifications::register_badge_notification_manager_impl(std::rc::Rc::new(Self::new()));
    }
}

ferroui_base::ferro_thread_local! {
    /// The handler of a refused promise: refusals are not reported.
    static IGNORE: Closure<dyn FnMut(JsValue)> = Closure::new(|_: JsValue| {});
}

/// The `navigator` of the page, when it has the Badging API.
fn navigator() -> Option<JsValue> {
    if !cfg!(target_arch = "wasm32") {
        return None;
    }
    let navigator = js_sys::Reflect::get(&js_sys::global(), &JsValue::from_str("navigator")).ok()?;
    let has_api = !navigator.is_undefined()
        && js_sys::Reflect::has(&navigator, &JsValue::from_str("setAppBadge")).unwrap_or(false)
        && js_sys::Reflect::has(&navigator, &JsValue::from_str("clearAppBadge")).unwrap_or(false);
    has_api.then_some(navigator)
}

/// Calls the method `name` of the navigator with `argument` (or none).
fn call(name: &str, argument: Option<JsValue>) {
    let Some(navigator) = navigator() else { return };
    let Ok(method) = js_sys::Reflect::get(&navigator, &JsValue::from_str(name)) else { return };
    let Ok(method) = method.dyn_into::<js_sys::Function>() else { return };
    let result = match argument {
        Some(argument) => method.call1(&navigator, &argument),
        None => method.call0(&navigator),
    };
    if let Ok(promise) = result.and_then(|value| value.dyn_into::<js_sys::Promise>().map_err(JsValue::from)) {
        IGNORE.with(|ignore| {
            let _ = promise.catch(ignore);
        });
    }
}

impl IBadgeNotificationManagerImpl for BrowserBadgeNotificationManager {
    fn is_supported(&self) -> bool {
        navigator().is_some()
    }

    fn set_count(&self, count: u32) {
        call("setAppBadge", Some(JsValue::from_f64(f64::from(count))));
    }

    fn set_glyph(&self, _glyph: BadgeNotificationGlyph) {
        // The flag badge: the API has no glyphs.
        call("setAppBadge", None);
    }

    fn clear(&self) {
        call("clearAppBadge", None);
    }
}
