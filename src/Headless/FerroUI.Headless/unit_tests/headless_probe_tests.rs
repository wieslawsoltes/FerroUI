//! Port of `HeadlessProbeTests.cs` of the upstream unit test project of
//! the controls.
//!
//! The tests run on the headless platform with the Simple theme. The
//! headless platform and the theme are built on the controls, so the tests
//! are here, where both are at hand, and not in the crate of the controls.

use super::headless_unit_test_application::HeadlessUnitTestApplication;
use crate::{FerroHeadlessPlatformOptions, HeadlessWindowExtensions};
use ferroui_base::input::platform::{IClipboard, KeyGestureFormatInfo, PlatformHotkeyConfiguration};
use ferroui_base::input::{IKeyboardDevice, MouseButton, RawInputModifiers};
use ferroui_base::platform::{ICursorFactory, IPlatformSettings};
use ferroui_base::rendering::IRenderLoop;
use ferroui_base::{BoxedValue, FerroLocator, LocatorExtensions, Size};
use ferroui_controls::platform::IPlatformIconLoader;
use ferroui_controls::primitives::{Popup, VisualLayerManager};
use ferroui_controls::{Border, Button, Control, Panel, PlacementMode, TextBox, Window};
use std::cell::Cell;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Rc;
use std::sync::Arc;

fn overlay_popups() -> Option<FerroHeadlessPlatformOptions> {
    Some(FerroHeadlessPlatformOptions { overlay_popups: true, ..FerroHeadlessPlatformOptions::default() })
}

fn button_hi() -> ferroui_base::Ref<Button> {
    let button = Button::new();
    button.set_width(100.0);
    button.set_height(30.0);
    let content: BoxedValue = Rc::new("hi".to_string());
    button.set_content(Some(content));
    button
}

fn window_with(content: impl Into<ferroui_base::Ref<Control>>) -> ferroui_base::Ref<Window> {
    let window = Window::new();
    window.set_width(400.0);
    window.set_height(300.0);
    window.set_content(Some(Control::boxed(content.into())));
    window
}

/// A popup placed at the pointer with a child of 50 by 50, in a panel that is the content of
/// a window of 400 by 300.
fn pointer_popup_in_window() -> (ferroui_base::Ref<Popup>, ferroui_base::Ref<Window>) {
    let popup = Popup::new();
    popup.set_placement(PlacementMode::Pointer);
    let child = Border::new();
    child.set_width(50.0);
    child.set_height(50.0);
    popup.set_child(child);
    let panel = Panel::new();
    panel.children().add(&popup);
    let window = window_with(panel.upcast::<Control>());
    (popup, window)
}

#[test]
fn can_show_window_and_lay_out() {
    let app = HeadlessUnitTestApplication::start(None);

    let button = button_hi();
    let window = window_with(button.clone().upcast::<Control>());
    window.show();
    app.run_jobs(None);

    assert_eq!(Size::new(100.0, 30.0), button.bounds().size());
    assert_eq!(Size::new(400.0, 300.0), window.client_size());
}

#[test]
fn can_route_real_input_through_headless_toplevel() {
    let app = HeadlessUnitTestApplication::start(None);

    let clicked = Rc::new(Cell::new(false));
    let button = button_hi();
    button.click({
        let clicked = clicked.clone();
        move |_, _| clicked.set(true)
    });
    let window = window_with(button.clone().upcast::<Control>());
    window.show();
    app.run_jobs(None);

    let pt = button.bounds().center();
    window.mouse_down(pt, MouseButton::Left, RawInputModifiers::NONE);
    window.mouse_up(pt, MouseButton::Left, RawInputModifiers::NONE);
    app.run_jobs(None);

    assert!(clicked.get());
}

#[test]
fn keyboard_input_reaches_focused_control() {
    let app = HeadlessUnitTestApplication::start(None);

    let text_box = TextBox::new();
    text_box.set_width(200.0);
    text_box.set_height(30.0);
    let window = window_with(text_box.clone().upcast::<Control>());
    window.show();
    app.run_jobs(None);

    text_box.focus();
    app.run_jobs(None);
    window.key_text_input("abc");
    app.run_jobs(None);

    assert_eq!(Some("abc".to_string()), text_box.text());
}

#[test]
fn popup_opens_as_real_popup_root() {
    let app = HeadlessUnitTestApplication::start(None);

    let (popup, window) = pointer_popup_in_window();
    window.show();
    app.run_jobs(None);

    popup.open();
    app.run_jobs(None);

    assert!(popup.host().is_some_and(|host| host.as_popup_root().is_some()));
}

#[test]
fn popup_uses_overlay_popup_host_when_overlay_popups_enabled() {
    let app = HeadlessUnitTestApplication::start(overlay_popups());

    let (popup, window) = pointer_popup_in_window();
    window.show();
    app.run_jobs(None);

    popup.open();
    app.run_jobs(None);

    assert!(popup.host().is_some_and(|host| host.as_overlay_popup_host().is_some()));
}

#[test]
fn overlay_popup_requires_an_applied_window_template() {
    let app = HeadlessUnitTestApplication::start(overlay_popups());

    // The overlay layer is looked up through the visual tree, so a window that was
    // never shown has nothing to find.
    let untemplated = Popup::new();
    untemplated.set_placement_target(Window::new());
    let ex = catch_unwind(AssertUnwindSafe(|| untemplated.open())).expect_err("the popup has no overlay layer");
    let message = ex.downcast_ref::<String>().map(String::as_str).or_else(|| ex.downcast_ref::<&str>().copied());
    assert!(message.is_some_and(|message| message.contains("no overlay layer is found")));

    let window = Window::new();
    window.show();
    app.run_jobs(None);
    assert!(window.get_visual_descendants().any(|visual| visual.is::<VisualLayerManager>()));

    let templated = Popup::new();
    templated.set_placement_target(window.clone());
    templated.open();
    assert!(templated.host().is_some_and(|host| host.as_overlay_popup_host().is_some()));
}

#[test]
fn platform_services_are_registered_before_the_first_window_and_stay_stable() {
    let app = HeadlessUnitTestApplication::start(None);

    // The services by their addresses: the comparison of the original is by reference.
    fn address<T: ?Sized>(service: Option<Rc<T>>) -> Option<*const ()> {
        service.map(|service| Rc::as_ptr(&service).cast::<()>())
    }

    fn resolve() -> [Option<*const ()>; 8] {
        let locator = FerroLocator::current();
        [
            address(locator.get_service::<dyn IKeyboardDevice>()),
            address(locator.get_service::<dyn IPlatformSettings>()),
            address(locator.get_service::<dyn ICursorFactory>()),
            address(locator.get_service::<PlatformHotkeyConfiguration>()),
            address(locator.get_service::<dyn IClipboard>()),
            // The render loop is shared with the render thread: the service is the `Arc`.
            locator.get_service::<Arc<dyn IRenderLoop>>().map(|render_loop| Arc::as_ptr(&render_loop).cast::<()>()),
            address(locator.get_service::<dyn IPlatformIconLoader>()),
            address(locator.get_service::<KeyGestureFormatInfo>()),
        ]
    }

    let before = resolve();
    assert!(before.iter().all(Option::is_some));

    Window::new().show();
    app.run_jobs(None);

    // Initializing the platform lazily used to replace these mid-test, leaving anything that
    // resolved early holding a different instance than the window does.
    assert_eq!(before, resolve());
}
