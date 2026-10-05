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
