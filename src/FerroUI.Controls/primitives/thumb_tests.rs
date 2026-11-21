//! The reference implementation has no unit tests for `Thumb`; these tests
//! are specific to this port.

use crate::mouse_test_helper::MouseTestHelper;
use crate::primitives::Thumb;
use crate::test_support::{test_scope, TestRoot};
use ferroui_base::input::MouseButton;
use ferroui_base::Vector;
use std::cell::RefCell;
use std::rc::Rc;

#[test]
fn thumb_raises_drag_events_and_sets_pressed() {
    let _scope = test_scope();
    let thumb = Thumb::new();
    thumb.set_width(20.0);
    thumb.set_height(20.0);
    let root = TestRoot::with_child(thumb.clone());
    root.execute_initial_layout_pass();

    let events: Rc<RefCell<Vec<(&'static str, Vector)>>> = Rc::new(RefCell::new(Vec::new()));
    let recorded = events.clone();
    thumb.drag_started(move |_, e| recorded.borrow_mut().push(("started", e.vector)));
    let recorded = events.clone();
    thumb.drag_delta(move |_, e| recorded.borrow_mut().push(("delta", e.vector)));
    let recorded = events.clone();
    thumb.drag_completed(move |_, e| recorded.borrow_mut().push(("completed", e.vector)));

    let mouse = MouseTestHelper::new();
    let origin = thumb.bounds().position();
    let at = |x: f64, y: f64| origin + Vector::new(x, y);

    mouse.down_at(&thumb, MouseButton::Left, at(5.0, 5.0), 1);
    assert!(thumb.classes().contains(":pressed"));

    mouse.move_(&thumb, at(8.0, 9.0));
    mouse.up_at(&thumb, MouseButton::Left, at(8.0, 9.0));
    assert!(!thumb.classes().contains(":pressed"));

    assert_eq!(
        vec![
            ("started", Vector::new(5.0, 5.0)),
            ("delta", Vector::new(3.0, 4.0)),
            ("completed", Vector::new(8.0, 9.0)),
        ],
        *events.borrow()
    );
}

#[test]
fn secondary_press_keeps_existing_default_thumb_behavior() {
    let _scope = test_scope();
    let t = Thumb::new();
    t.set_width(20.);
    t.set_height(20.);
    let root = TestRoot::with_child(t.clone());
    root.execute_initial_layout_pass();
    let count = Rc::new(std::cell::Cell::new(0));
    let out = count.clone();
    t.drag_started(move |_, _| out.set(out.get() + 1));
    let mouse = MouseTestHelper::new();
    mouse.down_at(&t, MouseButton::Right, t.bounds().position(), 1);
    assert_eq!(count.get(), 1);
    assert!(t.classes().contains(":pressed"));
}
#[test]
fn primary_policy_filters_secondary_and_policy_change_preserves_active_drag() {
    let _scope = test_scope();
    let t = Thumb::new();
    t.set_width(20.);
    t.set_height(20.);
    t.set_drag_start_policy(crate::primitives::ThumbDragStartPolicy::PrimaryPressWhenIdle);
    let root = TestRoot::with_child(t.clone());
    root.execute_initial_layout_pass();
    let count = Rc::new(std::cell::Cell::new(0));
    let out = count.clone();
    t.drag_started(move |_, _| out.set(out.get() + 1));
    let mouse = MouseTestHelper::new();
    let point = t.bounds().position();
    mouse.down_at(&t, MouseButton::Right, point, 1);
    assert_eq!(count.get(), 0);
    assert!(!t.classes().contains(":pressed"));
    mouse.up_at(&t, MouseButton::Right, point);
    mouse.down_at(&t, MouseButton::Left, point, 1);
    assert_eq!(count.get(), 1);
    mouse.down_at(&t, MouseButton::Right, point, 1);
    assert_eq!(count.get(), 1);
    t.set_drag_start_policy(crate::primitives::ThumbDragStartPolicy::AnyPress);
    assert!(t.classes().contains(":pressed"));
    // Release the held secondary button before generating another physical press.
    mouse.up_at(&t, MouseButton::Right, point);
    mouse.up_at(&t, MouseButton::Left, point);
    mouse.down_at(&t, MouseButton::Right, point, 1);
    assert_eq!(count.get(), 2);
}
