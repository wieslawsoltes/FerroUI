use super::*;
use ferroui_base::input::{
    MouseButton, Pointer, PointerPointProperties, PointerType, PointerUpdateKind, RawInputModifiers,
};
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_controls::Window;

#[test]
fn pointer_capture_edits_overshoot_and_restores_the_draft_on_cancel() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let graph = EasingPreview::new();
    let initial = Easing::CubicBezier {
        x1: 0.125,
        y1: 0.25,
        x2: 0.75,
        y2: 1.0,
    };
    graph.set_easing(Some(initial.clone()));
    let draft = Rc::new(RefCell::new(initial.clone()));
    let changed = draft.clone();
    graph.edit_handles(move |easing| {
        changed.replace(easing);
    });
    let window = Window::new();
    window.set_width(250.0);
    window.set_height(120.0);
    window.set_content(Some(Control::boxed(&graph)));
    window.show();
    window.update_layout();
    let pointer = Pointer::new(Pointer::get_next_free_id(), PointerType::Mouse, true);
    let plot = graph.plot();
    let window_point = |point| graph.translate_point(plot.point(point), &window).unwrap();
    let start = window_point(Point::new(0.125, 0.25));
    let end = window_point(Point::new(-0.1, 1.5));
    let press = || {
        graph.raise_event(&PointerPressedEventArgs::new(
            graph.to_ref(),
            pointer.clone(),
            &window,
            start,
            1,
            PointerPointProperties::new(
                RawInputModifiers::LEFT_MOUSE_BUTTON,
                PointerUpdateKind::LeftButtonPressed,
            ),
            KeyModifiers::NONE,
            1,
        ));
    };
    let move_handle = || {
        graph.raise_event(&PointerEventArgs::new(
            Some(InputElement::pointer_moved_event()),
            graph.to_ref(),
            pointer.clone(),
            Some(&window),
            end,
            2,
            PointerPointProperties::new(
                RawInputModifiers::LEFT_MOUSE_BUTTON,
                PointerUpdateKind::Other,
            ),
            KeyModifiers::NONE,
        ));
    };
    press();
    move_handle();
    let overshoot = Easing::CubicBezier {
        x1: 0.0,
        y1: 1.5,
        x2: 0.75,
        y2: 1.0,
    };
    assert_eq!(*draft.borrow(), overshoot);
    pointer.capture(None);
    assert_eq!(*draft.borrow(), initial);
    assert_eq!(graph.easing(), Some(initial));

    press();
    move_handle();
    graph.raise_event(&PointerReleasedEventArgs::new(
        graph.to_ref(),
        pointer.clone(),
        &window,
        end,
        3,
        PointerPointProperties::new(
            RawInputModifiers::NONE,
            PointerUpdateKind::LeftButtonReleased,
        ),
        KeyModifiers::NONE,
        MouseButton::Left,
    ));
    assert!(pointer.captured().is_none());
    assert_eq!(*draft.borrow(), overshoot);
    assert_eq!(graph.easing(), Some(overshoot));
}
