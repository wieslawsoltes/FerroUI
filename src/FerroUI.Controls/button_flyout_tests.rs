//! Tests of the flyout members of the button classes. The reference has no
//! unit tests for them (its flyout tests cover the flyout side only), so
//! these are written for the port.

use crate::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use crate::{Border, Button, Control, DropDownButton, Flyout, SplitButton, StackPanel, ToggleSplitButton, Window};
use ferroui_base::input::{IKeyboardDevice, InputElement, Key, KeyEventArgs, KeyboardDevice};
use ferroui_base::Ref;
use std::cell::Cell;
use std::rc::Rc;

const FLYOUT_OPEN: &str = ":flyout-open";

fn create_services() -> UnitTestApplicationScope {
    UnitTestApplication::start(
        TestServices::styled_window().with_keyboard_device(|| Some(KeyboardDevice::new() as Rc<dyn IKeyboardDevice>)),
    )
}

fn shown_window(content: Ref<Control>) -> Ref<Window> {
    let window = Window::new();
    window.set_content(Some(Control::boxed(content)));
    window.show();
    window
}

fn create_key_event(key: Key, down: bool) -> KeyEventArgs {
    let mut e = KeyEventArgs::new();
    e.key = key;
    if down {
        e.set_routed_event(Some(InputElement::key_down_event()));
    } else {
        e.set_routed_event(Some(InputElement::key_up_event()));
    }
    e
}

fn flyout_with_content() -> Ref<Flyout> {
    let flyout = Flyout::new();
    flyout.set_content(Some(Control::boxed(Border::new())));
    flyout
}

#[test]
fn click_opens_and_closes_the_flyout_of_a_button() {
    let _app = create_services();
    let flyout = flyout_with_content();
    let button = Button::new();
    button.set_flyout(&flyout);
    let _window = shown_window(button.clone().upcast());

    button.perform_click();

    assert!(flyout.is_open());
    assert_eq!(Some(button.clone().upcast::<Control>()), flyout.target());
    assert!(button.classes().contains(FLYOUT_OPEN));

    button.perform_click();

    assert!(!flyout.is_open());
    assert!(!button.classes().contains(FLYOUT_OPEN));
}

#[test]
fn click_opens_the_flyout_of_a_drop_down_button() {
    let _app = create_services();
    let flyout = flyout_with_content();
    let button = DropDownButton::new();
    button.set_flyout(&flyout);
    let _window = shown_window(button.clone().upcast());

    button.perform_click();

    assert!(flyout.is_open());
    assert!(button.classes().contains(FLYOUT_OPEN));
}

#[test]
fn shared_flyout_only_marks_the_button_it_is_shown_at() {
    let _app = create_services();
    let flyout = flyout_with_content();
    let button1 = Button::new();
    let button2 = Button::new();
    button1.set_flyout(&flyout);
    button2.set_flyout(&flyout);
    let panel = StackPanel::new();
    panel.children().add(button1.clone());
    panel.children().add(button2.clone());
    let _window = shown_window(panel.upcast());

    button2.perform_click();

    assert!(flyout.is_open());
    assert!(!button1.classes().contains(FLYOUT_OPEN));
    assert!(button2.classes().contains(FLYOUT_OPEN));

    flyout.hide();

    assert!(!button2.classes().contains(FLYOUT_OPEN));
}

#[test]
fn escape_key_closes_the_flyout_of_a_button() {
    let _app = create_services();
    let flyout = flyout_with_content();
    let button = Button::new();
    button.set_flyout(&flyout);
    let _window = shown_window(button.clone().upcast());

    button.perform_click();
    assert!(flyout.is_open());

    button.raise_event(&create_key_event(Key::Escape, true));

    assert!(!flyout.is_open());
    assert!(!button.classes().contains(FLYOUT_OPEN));
}

#[test]
fn replacing_the_flyout_of_a_button_closes_the_open_one() {
    let _app = create_services();
    let flyout = flyout_with_content();
    let button = Button::new();
    button.set_flyout(&flyout);
    let _window = shown_window(button.clone().upcast());

    button.perform_click();
    assert!(flyout.is_open());

    let other = flyout_with_content();
    button.set_flyout(&other);

    assert!(!flyout.is_open());
    assert!(!button.classes().contains(FLYOUT_OPEN));

    // The replaced flyout no longer opens at the button.
    flyout.set_is_open(true);
    assert!(!flyout.is_open());

    button.perform_click();
    assert!(other.is_open());
    assert!(button.classes().contains(FLYOUT_OPEN));
}

#[test]
fn secondary_click_opens_and_closes_the_flyout_of_a_split_button() {
    let _app = create_services();
    let flyout = flyout_with_content();
    let button = SplitButton::new();
    button.set_flyout(&flyout);
    let _window = shown_window(button.clone().upcast());

    let state_changed = Rc::new(Cell::new(0));
    let count = state_changed.clone();
    let _ = button.flyout_state_changed(move || count.set(count.get() + 1));

    button.on_click_secondary(None);

    assert!(flyout.is_open());
    assert!(button.is_flyout_open());
    assert!(button.classes().contains(FLYOUT_OPEN));
    assert_eq!(1, state_changed.get());

    button.on_click_secondary(None);

    assert!(!flyout.is_open());
    assert!(!button.is_flyout_open());
    assert!(!button.classes().contains(FLYOUT_OPEN));
    assert_eq!(2, state_changed.get());
}

#[test]
fn keys_open_and_close_the_flyout_of_a_split_button() {
    let _app = create_services();
    let flyout = flyout_with_content();
    let button = SplitButton::new();
    button.set_flyout(&flyout);
    let _window = shown_window(button.clone().upcast());

    let e = create_key_event(Key::F4, false);
    button.raise_event(&e);

    assert!(e.handled());
    assert!(flyout.is_open());

    let e = create_key_event(Key::Escape, false);
    button.raise_event(&e);

    assert!(e.handled());
    assert!(!flyout.is_open());
}

#[test]
fn is_open_opens_the_flyout_at_the_split_button_that_owns_it() {
    let _app = create_services();
    let flyout = flyout_with_content();
    let button = ToggleSplitButton::new();
    button.set_flyout(&flyout);
    let _window = shown_window(button.clone().upcast());

    flyout.set_is_open(true);

    assert!(flyout.is_open());
    assert_eq!(Some(button.clone().upcast::<Control>()), flyout.target());
    assert!(button.is_flyout_open());

    button.set_flyout(None);

    assert!(!flyout.is_open());
    assert!(!button.is_flyout_open());
}

/// A button owns its flyout and an open flyout refers back to the button
/// it is shown at; once the flyout is closed and the window is gone nothing
/// keeps the button, the flyout or its popup alive.
#[test]
fn closed_flyout_does_not_keep_its_button_alive() {
    let _app = create_services();

    let (weak_button, weak_flyout, weak_popup) = {
        let flyout = flyout_with_content();
        let button = Button::new();
        button.set_flyout(&flyout);
        let window = shown_window(button.clone().upcast());

        button.perform_click();
        assert!(flyout.is_open());
        button.perform_click();
        assert!(!flyout.is_open());
        assert!(flyout.target().is_none());

        window.close();

        (button.downgrade(), flyout.downgrade(), flyout.popup().downgrade())
    };
    ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);

    assert!(weak_button.upgrade().is_none());
    assert!(weak_flyout.upgrade().is_none());
    assert!(weak_popup.upgrade().is_none());
}

/// Closing the window closes a flyout that is open at one of its controls,
/// after which nothing keeps either alive.
#[test]
fn closing_window_closes_the_open_flyout_and_frees_it() {
    let _app = create_services();

    let (weak_button, weak_flyout) = {
        let flyout = flyout_with_content();
        let button = Button::new();
        button.set_flyout(&flyout);
        let window = shown_window(button.clone().upcast());

        button.perform_click();
        assert!(flyout.is_open());

        window.close();
        assert!(!flyout.is_open());
        assert!(!button.classes().contains(FLYOUT_OPEN));

        (button.downgrade(), flyout.downgrade())
    };
    ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);

    assert!(weak_button.upgrade().is_none());
    assert!(weak_flyout.upgrade().is_none());
}
