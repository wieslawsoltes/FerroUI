//! Ported from `tests/Xaml.Behaviors.Interactions.UnitTests/Custom/FocusNextElementActionTests.cs`
//! of Xaml.Behaviors.
//!
//! Adapted: the window is shown and laid out without a rendered frame; the key down handler of
//! the window is a routed event handler.

use super::support::{run_jobs, start_app};
use crate::part_a::FocusNavigationHelper;
use crate::FocusNextElementAction;
use ferroui_base::input::{InputElement, KeyEventArgs, KeyboardNavigation, KeyboardNavigationMode, NavigationDirection};
use ferroui_base::interactivity::Interactive;
use ferroui_base::{BoxedValue, FerroObject, Ref};
use ferroui_controls::{Button, Control, StackPanel, Window};
use std::cell::Cell;
use std::rc::Rc;

/// `CreateButton`.
fn create_button(content: &str) -> Ref<Button> {
    let button = Button::new();
    button.set_content(Some(Rc::new(content.to_string())));
    button.set_name(Some(content.to_string()));
    button.set_width(120.0);
    button.set_height(32.0);
    button.set_focusable(true);
    button
}

/// `new StackPanel { Children = { .. } }`.
fn stack_panel(children: &[Ref<Control>]) -> Ref<StackPanel> {
    let panel = StackPanel::new();
    for child in children {
        panel.children().add(child);
    }
    panel
}

fn control<T: ferroui_base::ObjectType + ferroui_base::Upcast<Control>>(value: &Ref<T>) -> Ref<Control> {
    value.clone().upcast()
}

/// `new Window { Width, Height, Content }`, shown and laid out.
fn show_window(width: f64, height: f64, content: &Ref<StackPanel>) -> Ref<Window> {
    let window = Window::new();
    window.set_width(width);
    window.set_height(height);
    window.set_content(Some(Control::boxed(content)));
    window.show();
    run_jobs();
    window.update_layout();
    run_jobs();
    window
}

/// The sender of the action: the control as an untyped value.
fn sender(value: &Ref<Button>) -> BoxedValue {
    let object: Ref<FerroObject> = value.clone().upcast();
    Rc::new(object)
}

/// The content of the focused button (`(window.FocusManager?.GetFocusedElement() as Button)?.Content`).
fn focused_content(window: &Window) -> Option<String> {
    let focused = window.focus_manager().get_focused_element()?;
    let button = focused.cast::<Button>()?;
    button.content().and_then(|content| content.downcast_ref::<String>().cloned())
}

/// The content of a button the helper found.
fn content_of(element: Option<Ref<InputElement>>) -> Option<String> {
    let button = element?.cast::<Button>()?;
    button.content().and_then(|content| content.downcast_ref::<String>().cloned())
}

fn execute(action: &FocusNextElementAction, source: &Ref<Button>) {
    action.execute(Some(&sender(source)), None);
}

#[test]
fn execute_uses_framework_tab_navigation_rules() {
    let _app = start_app();
    let before = create_button("Before");
    let cycle_first = create_button("CycleFirst");
    let cycle_second = create_button("CycleSecond");
    let after = create_button("After");

    let cycle_container = stack_panel(&[control(&cycle_first), control(&cycle_second)]);

    KeyboardNavigation::set_tab_navigation(&cycle_container, KeyboardNavigationMode::Cycle);

    let window = show_window(240.0, 160.0, &stack_panel(&[control(&before), control(&cycle_container), control(&after)]));

    assert!(cycle_second.focus());
    let current: &InputElement = &cycle_second;
    assert_eq!(
        Some("CycleFirst".to_string()),
        content_of(FocusNavigationHelper::find_adjacent(
            &cycle_container,
            Some(current),
            NavigationDirection::Next,
            false
        ))
    );

    let action = FocusNextElementAction::new();
    execute(&action, &cycle_second);

    run_jobs();

    assert_eq!(Some("CycleFirst".to_string()), focused_content(&window));
}

#[test]
fn execute_does_not_raise_tab_key_down_handlers() {
    let _app = start_app();
    let first = create_button("First");
    let second = create_button("Second");
    let tab_key_down_count = Rc::new(Cell::new(0));

    let window = show_window(240.0, 120.0, &stack_panel(&[control(&first), control(&second)]));

    let count = tab_key_down_count.clone();
    window.add_handler(InputElement::key_down_event(), move |_sender: &Interactive, e: &KeyEventArgs| {
        if e.key == ferroui_base::input::Key::Tab {
            count.set(count.get() + 1);
        }
    });

    assert!(first.focus());

    let action = FocusNextElementAction::new();
    execute(&action, &first);

    run_jobs();

    assert_eq!(Some("Second".to_string()), focused_content(&window));
    assert_eq!(0, tab_key_down_count.get());
}

#[test]
fn execute_skips_none_tab_navigation_children() {
    let _app = start_app();
    let before = create_button("Before");
    let skipped_child = create_button("SkippedChild");
    let after = create_button("After");

    let none_container = stack_panel(&[control(&skipped_child)]);

    KeyboardNavigation::set_tab_navigation(&none_container, KeyboardNavigationMode::None);

    let window = show_window(240.0, 160.0, &stack_panel(&[control(&before), control(&none_container), control(&after)]));

    assert!(before.focus());

    let action = FocusNextElementAction::new();
    execute(&action, &before);

    run_jobs();

    assert_eq!(Some("After".to_string()), focused_content(&window));
}

#[test]
fn execute_uses_tab_once_active_element_for_once_containers() {
    let _app = start_app();
    let before = create_button("Before");
    let once_first = create_button("OnceFirst");
    let once_second = create_button("OnceSecond");
    let after = create_button("After");

    let once_container = stack_panel(&[control(&once_first), control(&once_second)]);

    KeyboardNavigation::set_tab_navigation(&once_container, KeyboardNavigationMode::Once);
    KeyboardNavigation::set_tab_once_active_element(&once_container, Some(once_second.clone().upcast()));

    let window = show_window(240.0, 160.0, &stack_panel(&[control(&before), control(&once_container), control(&after)]));

    let action = FocusNextElementAction::new();

    assert!(before.focus());
    execute(&action, &before);
    run_jobs();
    assert_eq!(Some("OnceSecond".to_string()), focused_content(&window));

    execute(&action, &once_second);
    run_jobs();
    assert_eq!(Some("After".to_string()), focused_content(&window));
}

#[test]
fn execute_respects_local_tab_navigation_subtree_order() {
    let _app = start_app();
    let local_first = create_button("LocalFirst");
    local_first.set_tab_index(10);

    let local_second = create_button("LocalSecond");
    local_second.set_tab_index(0);

    let after = create_button("After");
    after.set_tab_index(1);

    let local_container = stack_panel(&[control(&local_first), control(&local_second)]);

    KeyboardNavigation::set_tab_navigation(&local_container, KeyboardNavigationMode::Local);

    let window = show_window(240.0, 160.0, &stack_panel(&[control(&local_container), control(&after)]));

    assert!(local_second.focus());

    let action = FocusNextElementAction::new();
    execute(&action, &local_second);

    run_jobs();

    assert_eq!(Some("LocalFirst".to_string()), focused_content(&window));
}
