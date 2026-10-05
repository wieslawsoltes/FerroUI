use super::{non_null, JsObject};
use crate::browser_activatable_lifetime::BrowserActivatableLifetime;
use crate::browser_platform_settings::BrowserPlatformSettings;
use crate::browser_screens::BrowserScreens;
use ferroui_base::{FerroLocator, LocatorExtensions};
use wasm_bindgen::prelude::*;

#[wasm_bindgen(raw_module = "./ferroui.js")]
extern "C" {
    /// What `createFerroHost` returns: the elements of a view.
    pub type HostContent;

    #[wasm_bindgen(method, getter, js_name = nativeHost)]
    fn native_host_raw(this: &HostContent) -> JsObject;

    #[wasm_bindgen(method, getter, js_name = inputElement)]
    fn input_element_raw(this: &HostContent) -> JsObject;

    #[wasm_bindgen(js_namespace = FerroDOM, js_name = getGlobalThis)]
    pub fn get_global_this() -> JsObject;

    #[wasm_bindgen(js_namespace = FerroDOM, js_name = getFirstElementById)]
    fn get_first_element_by_id(id: &str, parent: &JsObject) -> JsObject;

    #[wasm_bindgen(js_namespace = FerroDOM, js_name = getFirstElementByClassName)]
    fn get_first_element_by_class_name(class_name: &str, parent: &JsObject) -> JsObject;

    #[wasm_bindgen(js_namespace = FerroDOM, js_name = createFerroHost)]
    fn create_ferro_host_raw(element: &JsObject) -> JsObject;

    /// Whether the document is shown in full screen.
    #[wasm_bindgen(js_namespace = FerroDOM, js_name = isFullscreen)]
    pub fn is_fullscreen(global_this: &JsObject) -> bool;

    // Answers with a promise, which nobody waits for.
    #[wasm_bindgen(js_namespace = FerroDOM, js_name = setFullscreen)]
    fn set_fullscreen_raw(global_this: &JsObject, is_fullscreen: bool) -> JsObject;

    /// The safe area insets of the page: left, top, right, bottom.
    #[wasm_bindgen(js_namespace = FerroDOM, js_name = getSafeAreaPadding)]
    pub fn get_safe_area_padding(global_this: &JsObject) -> Vec<f64>;

    #[wasm_bindgen(js_namespace = FerroDOM, js_name = getDarkMode)]
    pub fn get_dark_mode(global_this: &JsObject) -> Vec<i32>;

    #[wasm_bindgen(js_namespace = FerroDOM, js_name = getNavigatorLanguage)]
    pub fn get_navigator_language(global_this: &JsObject) -> Option<String>;

    #[wasm_bindgen(js_namespace = FerroDOM, js_name = addClass)]
    pub fn add_css_class(element: &JsObject, class_name: &str);

    #[wasm_bindgen(js_namespace = FerroDOM, js_name = initGlobalDomEvents)]
    pub fn init_global_dom_events(global_this: &JsObject);
}

/// Shows the document in full screen or ends the full screen. Nothing waits
/// for the answer: the page may refuse the request (without a user gesture).
pub fn set_fullscreen(global_this: &JsObject, is_fullscreen: bool) {
    drop(set_fullscreen_raw(global_this, is_fullscreen));
}

impl HostContent {
    /// The element native controls are placed in.
    pub fn native_host(&self) -> Option<JsObject> {
        non_null(self.native_host_raw())
    }

    /// The hidden input element text input goes through.
    pub fn input_element(&self) -> Option<JsObject> {
        non_null(self.input_element_raw())
    }
}

/// The element with the given id in the document of `parent`.
pub fn get_element_by_id(id: &str, parent: &JsObject) -> Option<JsObject> {
    non_null(get_first_element_by_id(id, parent))
}

/// The first element with the given class under `parent`.
pub fn get_elements_by_class_name(class_name: &str, parent: &JsObject) -> Option<JsObject> {
    non_null(get_first_element_by_class_name(class_name, parent))
}

/// Turns `element` into the host of a view and creates the elements the
/// view needs inside it.
pub fn create_ferro_host(element: &JsObject) -> Option<HostContent> {
    non_null(create_ferro_host_raw(element)).map(JsCast::unchecked_into)
}

fn platform_settings() -> Option<std::rc::Rc<BrowserPlatformSettings>> {
    FerroLocator::current().get_service::<BrowserPlatformSettings>()
}

/// The colour scheme or the contrast preference of the page changed.
#[wasm_bindgen(js_name = DomHelper_DarkModeChanged)]
pub fn dark_mode_changed(is_dark_mode: bool, is_high_contrast: bool) {
    if let Some(settings) = platform_settings() {
        settings.on_color_values_changed(is_dark_mode, is_high_contrast);
    }
}

/// The page became visible or hidden.
#[wasm_bindgen(js_name = DomHelper_DocumentVisibilityChanged)]
pub fn document_visibility_changed(visibility_state: &str) {
    if let Some(lifetime) = FerroLocator::current().get_service::<BrowserActivatableLifetime>() {
        lifetime.on_visibility_state_changed(visibility_state);
    }
}

/// The preferred language of the browser changed.
#[wasm_bindgen(js_name = DomHelper_LanguageChanged)]
pub fn language_changed(language: Option<String>) {
    if let Some(settings) = platform_settings() {
        settings.on_preferred_language_changed(language.as_deref());
    }
}

/// The screens of the page changed.
#[wasm_bindgen(js_name = DomHelper_ScreensChanged)]
pub fn screens_changed() {
    if let Some(screens) = FerroLocator::current().get_service::<BrowserScreens>() {
        screens.on_changed();
    }
}
