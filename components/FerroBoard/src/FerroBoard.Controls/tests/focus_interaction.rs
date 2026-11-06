#[path = "support/headless.rs"]
mod headless;
use ferroui_base::{
    Point,
    input::{
        KeyModifiers, MouseButton, NavigationMethod, Pointer, PointerPointProperties,
        PointerPressedEventArgs, PointerReleasedEventArgs, PointerType,
    },
    threading::Dispatcher,
};
use ferroui_controls::{Control, Grid, StackPanel, TextBox, Window};
fn press(target: &Control, window: &Window, kind: PointerType) {
    let pointer = Pointer::new(41, kind, true);
    let position = target
        .translate_point(Point::new(5.0, 5.0), window)
        .unwrap();
    target.raise_event(&PointerPressedEventArgs::new(
        target.to_ref(),
        pointer.clone(),
        window,
        position,
        0,
        PointerPointProperties {
            is_left_button_pressed: true,
            ..Default::default()
        },
        KeyModifiers::empty(),
        1,
    ));
    target.raise_event(&PointerReleasedEventArgs::new(
        target.to_ref(),
        pointer.clone(),
        window,
        position,
        0,
        PointerPointProperties::default(),
        KeyModifiers::empty(),
        MouseButton::Left,
    ));
}
#[test]
fn source_framework_selection_policy_agrees_for_pointer_and_nonpointer_states() {
    headless::run(|| {
        let text = TextBox::new();
        let target = Grid::new();
        target.set_focusable(true);
        let panel = StackPanel::new();
        panel.children().add(text.clone());
        panel.children().add(target.clone());
        let window = Window::new();
        window.set_content(Some(Control::boxed(panel)));
        window.show();
        Dispatcher::current_dispatcher().run_jobs(None);
        let mut matched = 0;
        for line in include_str!("../../../tests/fixtures/focus-interaction.jsonl").lines() {
            let r: serde_json::Value = serde_json::from_str(line).unwrap();
            if r["tag"] != "policy" || r["kind"] != "framework" {
                continue;
            }
            target.set_allow_focus_on_interaction(r["allow"].as_bool().unwrap());
            assert!(text.focus());
            let method = if r["state"] == 1 {
                NavigationMethod::Pointer
            } else {
                NavigationMethod::Unspecified
            };
            assert_eq!(
                target.focus_with(method, KeyModifiers::empty()),
                r["value"].as_bool().unwrap()
            );
            matched += 1;
        }
        assert_eq!(matched, 12);
        window.close();
    });
}
#[test]
fn mouse_pen_and_touch_keep_the_editor_focused_without_disabling_programmatic_focus() {
    headless::run(|| {
        let text = TextBox::new();
        let target = Grid::new();
        target.set_focusable(true);
        target.set_width(80.0);
        target.set_height(40.0);
        target.set_allow_focus_on_interaction(false);
        let panel = StackPanel::new();
        panel.children().add(text.clone());
        panel.children().add(target.clone());
        let window = Window::new();
        window.set_content(Some(Control::boxed(panel)));
        window.show();
        Dispatcher::current_dispatcher().run_jobs(None);
        let losing = std::rc::Rc::new(std::cell::Cell::new(0));
        let getting = std::rc::Rc::new(std::cell::Cell::new(0));
        let count = losing.clone();
        text.losing_focus(move |_, _| count.set(count.get() + 1));
        let count = getting.clone();
        target.getting_focus(move |_, _| count.set(count.get() + 1));
        for kind in [PointerType::Mouse, PointerType::Pen, PointerType::Touch] {
            assert!(text.focus());
            losing.set(0);
            getting.set(0);
            press(&target, &window, kind);
            assert_eq!((losing.get(), getting.get()), (0, 0));
            assert!(text.is_focused());
            assert!(!target.is_focused());
            assert!(target.focus());
        }
        target.set_allow_focus_on_interaction(true);
        assert!(text.focus());
        press(&target, &window, PointerType::Mouse);
        assert!(target.is_focused());
        window.close();
    });
}
#[test]
fn new_touch_button_content_inherits_focus_neutrality_and_local_override_is_reversible() {
    headless::run(|| {
        use ferroboard_controls::primitives::TouchButton;
        let text = TextBox::new();
        let button = TouchButton::new();
        button.set_width(120.0);
        button.set_height(50.0);
        let child = Grid::new();
        child.set_focusable(true);
        child.set_width(80.0);
        child.set_height(40.0);
        button.children().add(child.clone());
        let panel = StackPanel::new();
        panel.children().add(text.clone());
        panel.children().add(button.clone());
        let window = Window::new();
        window.set_content(Some(Control::boxed(panel)));
        window.show();
        Dispatcher::current_dispatcher().run_jobs(None);
        assert!(!button.allow_focus_on_interaction());
        assert!(!child.allow_focus_on_interaction());
        assert!(text.focus());
        press(&child, &window, PointerType::Mouse);
        assert!(text.is_focused());
        assert!(child.focus());
        child.set_allow_focus_on_interaction(true);
        assert!(text.focus());
        press(&child, &window, PointerType::Mouse);
        assert!(child.is_focused());
        child.clear_value(ferroui_base::input::InputElement::allow_focus_on_interaction_property());
        assert!(!child.allow_focus_on_interaction());
        window.close();
    });
}
