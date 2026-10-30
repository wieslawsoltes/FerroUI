//! Ported from `tests/Xaml.Behaviors.Interactions.UnitTests/Custom/FocusTrapBehaviorTests.cs` of
//! Xaml.Behaviors.
//!
//! Adapted: the key down event of the Tab key is raised on the button directly, as the reference
//! does; the window is laid out instead of rendered (`support.rs`).

use super::support::{add_behavior, run_jobs, show_window, start_app};
use crate::FocusTrapBehavior;
use ferroui_base::input::{
    InputElement, Key, KeyDeviceType, KeyEventArgs, KeyModifiers, KeyboardNavigation, KeyboardNavigationMode,
    PhysicalKey,
};
use ferroui_base::{BoxedValue, Ref};
use ferroui_controls::{Button, StackPanel};
use std::rc::Rc;

#[test]
fn tab_inside_cycle_group_uses_framework_navigation() {
    let _app = start_app();

    let trap_before = create_button("TrapBefore");
    let cycle_first = create_button("CycleFirst");
    let cycle_second = create_button("CycleSecond");
    let outside = create_button("Outside");

    let cycle_container = StackPanel::new();
    cycle_container.children().add(&cycle_first);
    cycle_container.children().add(&cycle_second);

    KeyboardNavigation::set_tab_navigation(&cycle_container, KeyboardNavigationMode::Cycle);

    let trap = StackPanel::new();
    trap.children().add(&trap_before);
    trap.children().add(&cycle_container);

    add_behavior(&trap, &FocusTrapBehavior::new());

    let content = StackPanel::new();
    content.children().add(&trap);
    content.children().add(&outside);

    let window = show_window(&content, 320.0, 160.0);

    assert!(cycle_second.focus());

    let mut e = KeyEventArgs::new();
    e.key = Key::Tab;
    e.physical_key = PhysicalKey::Tab;
    e.key_modifiers = KeyModifiers::NONE;
    e.key_device_type = KeyDeviceType::Keyboard;
    e.set_routed_event(Some(InputElement::key_down_event()));
    e.set_source(cycle_second.to_ref());
    cycle_second.raise_event(&e);

    run_jobs();

    let focused_content = window
        .focus_manager()
        .get_focused_element()
        .and_then(|element| element.cast::<Button>())
        .and_then(|button| button.content())
        .and_then(|content| content.downcast_ref::<String>().cloned());
    assert_eq!(Some("CycleFirst".to_string()), focused_content);
}

fn create_button(content: &str) -> Ref<Button> {
    let button = Button::new();
    button.set_content(Some(Rc::new(content.to_string()) as BoxedValue));
    button.set_name(Some(content.to_string()));
    button.set_width(120.0);
    button.set_height(32.0);
    button.set_focusable(true);
    button
}
