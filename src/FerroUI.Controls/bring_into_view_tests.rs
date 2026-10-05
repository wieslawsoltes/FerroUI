use crate::test_support::{test_scope, TestRoot};
use crate::{Border, Control};
use ferroui_base::Rect;
use std::cell::Cell;
use std::rc::Rc;

#[test]
fn bring_into_view_on_laid_out_control_raises_request_bring_into_view_synchronously() {
    let _scope = test_scope();

    let child = Border::new();
    child.set_width(50.0);
    child.set_height(50.0);
    let root = TestRoot::with_child(&child);

    root.layout_manager().execute_initial_layout_pass();

    let raised = Rc::new(Cell::new(false));
    let handler_raised = raised.clone();
    root.add_handler(Control::request_bring_into_view_event(), move |_, _| {
        handler_raised.set(true)
    });

    child.bring_into_view();

    assert!(raised.get());
}

#[test]
fn bring_into_view_before_layout_is_deferred_until_end_of_layout_pass() {
    let _scope = test_scope();

    let child = Border::new();
    child.set_width(50.0);
    child.set_height(50.0);
    let root = TestRoot::with_child(&child);

    let raised = Rc::new(Cell::new(0));
    let target_rect = Rc::new(Cell::new(Rect::default()));
    let handler_raised = raised.clone();
    let handler_target_rect = target_rect.clone();
    root.add_handler(Control::request_bring_into_view_event(), move |_, e| {
        handler_raised.set(handler_raised.get() + 1);
        handler_target_rect.set(e.target_rect());
    });

    child.bring_into_view();

    assert_eq!(raised.get(), 0);

    root.layout_manager().execute_initial_layout_pass();

    assert_eq!(raised.get(), 1);
    assert_eq!(target_rect.get(), Rect::new(0.0, 0.0, 50.0, 50.0));
}

#[test]
fn deferred_bring_into_view_is_abandoned_when_control_becomes_invisible() {
    let _scope = test_scope();

    let child = Border::new();
    child.set_width(50.0);
    child.set_height(50.0);
    let root = TestRoot::with_child(&child);

    let raised = Rc::new(Cell::new(false));
    let handler_raised = raised.clone();
    root.add_handler(Control::request_bring_into_view_event(), move |_, _| {
        handler_raised.set(true)
    });

    child.bring_into_view();
    child.set_is_visible(false);
    root.layout_manager().execute_initial_layout_pass();

    assert!(!raised.get());
}

#[test]
fn bring_into_view_on_invisible_control_is_ignored() {
    let _scope = test_scope();

    let child = Border::new();
    child.set_width(50.0);
    child.set_height(50.0);
    child.set_is_visible(false);
    let root = TestRoot::with_child(&child);

    root.layout_manager().execute_initial_layout_pass();

    let raised = Rc::new(Cell::new(false));
    let handler_raised = raised.clone();
    root.add_handler(Control::request_bring_into_view_event(), move |_, _| {
        handler_raised.set(true)
    });

    child.bring_into_view();
    root.layout_manager().execute_layout_pass();

    assert!(!raised.get());
}
