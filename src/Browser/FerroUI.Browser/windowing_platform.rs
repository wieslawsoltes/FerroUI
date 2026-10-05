use crate::browser_activatable_lifetime::BrowserActivatableLifetime;
use crate::browser_platform_settings::BrowserPlatformSettings;
use crate::browser_runtime_platform::BrowserRuntimePlatform;
use crate::browser_single_threaded_dispatcher_impl::BrowserSingleThreadedDispatcherImpl;
use crate::cursor::CssCursorFactory;
use crate::interop::JsObject;
use crate::win_stubs::IconLoaderStub;
use ferroui_base::input::platform::{KeyGestureFormatInfo, PlatformHotkeyConfiguration};
use ferroui_base::input::{IKeyboardDevice, KeyboardDevice};
use ferroui_base::platform::{ICursorFactory, IPlatformSettings, IRuntimePlatform};
use ferroui_base::threading::Dispatcher;
use ferroui_base::FerroLocator;
use ferroui_controls::application_lifetimes::IActivatableLifetime;
use ferroui_controls::platform::{
    IPlatformIconLoader, ITopLevelImpl, ITrayIconImpl, IWindowImpl, IWindowingPlatform,
};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

thread_local! {
    // Capture the initial global object of the page.
    static GLOBAL_THIS: RefCell<Option<JsObject>> = const { RefCell::new(None) };
    static KEYBOARD: RefCell<Option<Rc<KeyboardDevice>>> = const { RefCell::new(None) };
}

/// The windowing platform of the browser. There are no windows: content is
/// shown by a view over an element of the page.
#[derive(Default)]
pub struct BrowserWindowingPlatform;

impl BrowserWindowingPlatform {
    /// The global object of the page.
    ///
    /// # Panics
    /// Panics when the browser backend has not been initialized.
    pub fn global_this() -> JsObject {
        match GLOBAL_THIS.with(|global_this| global_this.borrow().clone()) {
            Some(global_this) => global_this,
            None => panic!("Browser backend wasn't initialized. GlobalThis is null."),
        }
    }

    pub(crate) fn set_global_this(value: JsObject) {
        GLOBAL_THIS.with(|global_this| *global_this.borrow_mut() = Some(value));
    }

    /// The keyboard device of the platform.
    ///
    /// # Panics
    /// Panics when the windowing platform has not been registered.
    pub fn keyboard() -> Rc<KeyboardDevice> {
        match KEYBOARD.with(|keyboard| keyboard.borrow().clone()) {
            Some(keyboard) => keyboard,
            None => panic!("BrowserWindowingPlatform not registered."),
        }
    }

    /// Sets the keyboard device of the platform without registering the
    /// platform, for the tests of the input handler.
    #[cfg(test)]
    pub(crate) fn set_keyboard_for_unit_tests(keyboard: Option<Rc<KeyboardDevice>>) {
        KEYBOARD.with(|slot| *slot.borrow_mut() = keyboard);
    }

    /// Registers the services of the platform with the current service
    /// locator and installs the dispatcher of the browser.
    pub fn register() {
        let instance: Rc<dyn IWindowingPlatform> = Rc::new(BrowserWindowingPlatform);

        let keyboard = KeyboardDevice::new();
        KEYBOARD.with(|slot| *slot.borrow_mut() = Some(keyboard.clone()));
        let keyboard_device: Rc<dyn IKeyboardDevice> = keyboard;
        let platform_settings = BrowserPlatformSettings::new();
        let activatable_lifetime = BrowserActivatableLifetime::new();
        let locator = FerroLocator::current_mutable();
        locator
            .bind::<dyn IRuntimePlatform>()
            .to_singleton::<BrowserRuntimePlatform>(|instance| instance)
            .bind::<dyn ICursorFactory>()
            .to_singleton::<CssCursorFactory>(|instance| instance)
            .bind::<dyn IKeyboardDevice>()
            .to_constant(keyboard_device)
            .bind::<dyn IPlatformSettings>()
            .to_constant(platform_settings.clone())
            // With their concrete types, for the callbacks of the page: they
            // only go to the services of this backend.
            .bind_to_self(platform_settings)
            .bind::<dyn IWindowingPlatform>()
            .to_constant(instance)
            .bind::<dyn IPlatformIconLoader>()
            .to_singleton::<IconLoaderStub>(|instance| instance)
            .bind_to_self_singleton::<PlatformHotkeyConfiguration>()
            .bind_to_self(Rc::new(KeyGestureFormatInfo::new(Some(HashMap::new()), "Cmd", "Ctrl", "Alt", "Shift")))
            .bind::<dyn IActivatableLifetime>()
            .to_constant(activatable_lifetime.clone())
            .bind_to_self(activatable_lifetime);

        Dispatcher::initialize_ui_thread_dispatcher(BrowserSingleThreadedDispatcherImpl::new());
    }
}

impl IWindowingPlatform for BrowserWindowingPlatform {
    fn create_window(&self) -> Rc<dyn IWindowImpl> {
        panic!(
            "Browser doesn't support windowing platform. In order to display a single-view content, set ISingleViewApplicationLifetime.MainView."
        );
    }

    fn create_embeddable_top_level(&self) -> Rc<dyn ITopLevelImpl> {
        panic!("The method or operation is not implemented.");
    }

    fn create_embeddable_window(&self) -> Rc<dyn IWindowImpl> {
        panic!("Browser doesn't support embeddable windowing platform.");
    }

    fn create_tray_icon(&self) -> Option<Rc<dyn ITrayIconImpl>> {
        None
    }

    fn get_windows_z_order(&self, _windows: &[Rc<dyn IWindowImpl>], _z_order: &mut [i64]) {
        panic!("Specified method is not supported.");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn there_is_no_tray_icon() {
        assert!(BrowserWindowingPlatform.create_tray_icon().is_none());
    }

    #[test]
    #[should_panic(expected = "Browser doesn't support windowing platform")]
    fn windows_cannot_be_created() {
        BrowserWindowingPlatform.create_window();
    }

    #[test]
    #[should_panic(expected = "Browser doesn't support embeddable windowing platform")]
    fn embeddable_windows_cannot_be_created() {
        BrowserWindowingPlatform.create_embeddable_window();
    }

    #[test]
    #[should_panic(expected = "Specified method is not supported")]
    fn the_z_order_of_windows_is_not_available() {
        BrowserWindowingPlatform.get_windows_z_order(&[], &mut []);
    }
}
