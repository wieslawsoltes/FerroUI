//! Port of the two tests of `Input/GesturesTests.cs` (base unit tests) whose hold gesture opens a context flyout,
//! so they live with the controls. The other tests of the file are in `input/input_tests.rs` of `ferroui-base`.

use crate::flyouts::Flyout;
use crate::mouse_test_helper::MouseTestHelper;
use crate::test_support::TestRoot;
use crate::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use crate::Border;
use ferroui_base::input::platform::PlatformHotkeyConfiguration;
use ferroui_base::input::{InputElement, PointerType};
use ferroui_base::platform::{DefaultPlatformSettings, IPlatformSettings, PlatformColorValues};
use ferroui_base::reactive::IDisposable;
use ferroui_base::threading::Dispatcher;
use ferroui_base::{Point, Ref, Size};
use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

/// Upstream's mock of the platform settings: a hold after 300 milliseconds and a tap size of 16 by 16.
struct HoldPlatformSettings {
    base: DefaultPlatformSettings,
}

impl IPlatformSettings for HoldPlatformSettings {
    fn get_tap_size(&self, _type: PointerType) -> Size {
        Size::new(16.0, 16.0)
    }

    fn get_double_tap_size(&self, type_: PointerType) -> Size {
        self.base.get_double_tap_size(type_)
    }

    fn get_double_tap_time(&self, type_: PointerType) -> Duration {
        self.base.get_double_tap_time(type_)
    }

    fn hold_wait_duration(&self) -> Duration {
        Duration::from_millis(300)
    }

    fn hotkey_configuration(&self) -> Rc<PlatformHotkeyConfiguration> {
        self.base.hotkey_configuration()
    }

    fn preferred_application_language(&self) -> String {
        self.base.preferred_application_language()
    }

    fn get_color_values(&self) -> PlatformColorValues {
        self.base.get_color_values()
    }

    fn color_values_changed(&self, handler: Rc<dyn Fn(&PlatformColorValues)>) -> Rc<dyn IDisposable> {
        self.base.color_values_changed(handler)
    }

    fn preferred_application_language_changed(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        self.base.preferred_application_language_changed(handler)
    }
}

fn settings() -> Rc<dyn IPlatformSettings> {
    Rc::new(HoldPlatformSettings { base: DefaultPlatformSettings::new() })
}

fn start() -> UnitTestApplicationScope {
    UnitTestApplication::start(TestServices::new().with_platform_settings(settings()))
}

/// Upstream's test root answers the platform settings of the locator; the test root of this crate is given them.
fn root_with_child(border: &Ref<Border>) -> Ref<TestRoot> {
    let root = TestRoot::with_child(border);
    root.set_platform_settings(Some(settings()));
    root
}

#[test]
fn started_hold_gesture_should_raise_context_requested_event() {
    let _app = start();

    let flyout = Flyout::new();
    let border = Border::new();
    border.set_context_flyout(&flyout);
    InputElement::set_is_hold_with_mouse_enabled(&border, true);
    let _root = root_with_child(&border);

    let context_requested = Rc::new(Cell::new(false));

    let requested = context_requested.clone();
    let _opened = flyout.opened(move || requested.set(true));

    let mouse = MouseTestHelper::new();
    mouse.down(&border);
    let timers = Dispatcher::timers_for_unit_tests();
    assert_eq!(1, timers.len());
    Dispatcher::force_fire_timer_for_unit_tests(&timers[0]);
    mouse.up(&border);

    assert!(context_requested.get());
}

#[test]
fn cancelled_hold_gesture_should_cancel_context_flyout() {
    let _app = start();

    let flyout = Flyout::new();
    let border = Border::new();
    border.set_context_flyout(&flyout);
    InputElement::set_is_hold_with_mouse_enabled(&border, true);
    let _root = root_with_child(&border);

    let context_requested = Rc::new(Cell::new(false));
    let context_canceled = Rc::new(Cell::new(false));

    let requested = context_requested.clone();
    let _opened = flyout.opened(move || requested.set(true));
    let canceled = context_canceled.clone();
    let _closed = flyout.closed(move || canceled.set(true));

    let mouse = MouseTestHelper::new();
    mouse.down(&border);
    let timers = Dispatcher::timers_for_unit_tests();
    assert_eq!(1, timers.len());
    Dispatcher::force_fire_timer_for_unit_tests(&timers[0]);
    mouse.move_(&border, Point::new(100.0, 100.0));

    assert!(context_requested.get());
    assert!(context_canceled.get());
}
