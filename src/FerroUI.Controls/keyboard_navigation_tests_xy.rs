//! Port of the tests of `Input/KeyboardNavigationTests_XY.cs` (base unit tests) whose trees are windows, buttons
//! and a scroll viewer, so they live with the controls. The other tests of the file are in
//! `input/input_tests.rs` of `ferroui-base`.

use crate::testing::{TestServices, UnitTestApplication};
use crate::{Button, Canvas, Control, ScrollViewer, StackPanel, Window};
use ferroui_base::input::navigation::{XYFocus, XYFocusNavigationModes};
use ferroui_base::input::{
    FocusManager, InputElement, Key, KeyEventArgs, KeyboardNavigationHandler, NavigationDirection,
};
use ferroui_base::layout::Orientation;
use ferroui_base::{IntoRef, Ref};

fn el(control: &(impl IntoRef<InputElement> + Clone)) -> Ref<InputElement> {
    control.clone().into_ref()
}

fn button_with_height(height: f64) -> Ref<Button> {
    let button = Button::new();
    button.set_height(height);
    button
}

fn vertical_stack(current: &Ref<Button>, candidate: &Ref<Button>) -> Ref<StackPanel> {
    let parent = StackPanel::new();
    parent.set_orientation(Orientation::Vertical);
    parent.set_spacing(20.0);
    parent.children().add(current);
    parent.children().add(candidate);
    parent
}

fn window_with_content(content: impl IntoRef<Control>) -> Ref<Window> {
    let window = Window::new();
    XYFocus::set_navigation_modes(&window, XYFocusNavigationModes::ENABLED);
    window.set_content(Some(Control::boxed(content)));
    window
}

fn canvas_with_child(child: &Ref<Button>) -> Ref<Canvas> {
    let canvas = Canvas::new();
    canvas.children().add(child);
    canvas
}

#[test]
fn clipped_element_should_not_be_focused() {
    let _app = UnitTestApplication::start(TestServices::focusable_window());

    let current = button_with_height(20.0);
    let candidate = button_with_height(20.0);
    let parent = vertical_stack(&current, &candidate);
    let window = window_with_content(&parent);
    window.set_height(30.0);
    window.show();

    assert!(KeyboardNavigationHandler::get_next(&el(&current), NavigationDirection::Down).is_none());
}

#[test]
fn clipped_element_should_not_focused_if_inside_of_scroll_viewer() {
    let _app = UnitTestApplication::start(TestServices::focusable_window());

    let current = button_with_height(20.0);
    let candidate = button_with_height(20.0);
    let parent = vertical_stack(&current, &candidate);
    let scroll_viewer = ScrollViewer::new();
    scroll_viewer.set_content(Some(Control::boxed(&parent)));
    let window = window_with_content(&scroll_viewer);
    window.set_height(30.0);
    window.show();

    assert_eq!(Some(el(&candidate)), KeyboardNavigationHandler::get_next(&el(&current), NavigationDirection::Down));
}

fn arrow_key_should_not_be_handled_if_no_focus(key: Key) {
    let _app = UnitTestApplication::start(TestServices::focusable_window());

    let current = Button::new();
    let window = window_with_content(&canvas_with_child(&current));
    window.show();
    assert!(current.focus());

    let mut args = KeyEventArgs::new();
    args.set_routed_event(Some(InputElement::key_down_event()));
    args.key = key;
    args.set_source(&current);
    window.raise_event(&args);

    assert_eq!(Some(el(&current)), FocusManager::get_focus_manager(&current).unwrap().get_focused_element());
    assert!(!args.handled());
}

#[test]
fn arrow_key_should_not_be_handled_if_no_focus_left() {
    arrow_key_should_not_be_handled_if_no_focus(Key::Left);
}

#[test]
fn arrow_key_should_not_be_handled_if_no_focus_right() {
    arrow_key_should_not_be_handled_if_no_focus(Key::Right);
}

#[test]
fn arrow_key_should_not_be_handled_if_no_focus_up() {
    arrow_key_should_not_be_handled_if_no_focus(Key::Up);
}

#[test]
fn arrow_key_should_not_be_handled_if_no_focus_down() {
    arrow_key_should_not_be_handled_if_no_focus(Key::Down);
}

#[test]
fn can_focus_child_of_current_focused() {
    let _app = UnitTestApplication::start(TestServices::focusable_window());

    let candidate = button_with_height(20.0);
    candidate.set_width(20.0);
    let window = window_with_content(&candidate);
    window.set_height(30.0);
    window.show();

    assert!(KeyboardNavigationHandler::get_next(&el(&window), NavigationDirection::Down).is_none());
}

#[test]
fn can_focus_any_element_if_nothing_was_focused() {
    // In the future we might auto-focus any element, but for now XY algorithm should be aware of the specifics of
    // the framework.
    let _app = UnitTestApplication::start(TestServices::focusable_window());

    let candidate = Button::new();
    let window = window_with_content(&canvas_with_child(&candidate));
    window.show();

    assert!(FocusManager::get_focus_manager(&window).unwrap().get_focused_element().is_none());

    let mut args = KeyEventArgs::new();
    args.set_routed_event(Some(InputElement::key_down_event()));
    args.key = Key::Down;
    args.set_source(&window);
    window.raise_event(&args);

    assert_eq!(Some(el(&candidate)), FocusManager::get_focus_manager(&window).unwrap().get_focused_element());
}
