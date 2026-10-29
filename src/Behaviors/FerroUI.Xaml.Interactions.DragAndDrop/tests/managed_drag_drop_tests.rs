//! Not from the reference: the managed drag and drop, which runs inside the application, on a
//! headless window: the state and the events of the service, the drop targets it notifies, the
//! preview window and the drag behavior that drives it.

use crate::drag_drop_context_store::DragDropContextStore;
use crate::tests::support::{add_behavior, border, is_text, run_jobs, show_window, start_app, text, Mouse, TestDropHandler};
use crate::{DragPreviewService, ManagedContextDragBehavior, ManagedContextDropBehavior, ManagedDragDropService};
use ferroui_base::input::DragDropEffects;
use ferroui_base::{BoxedValue, Point, Ref};
use ferroui_controls::{StackPanel, TopLevel, Window};
use std::cell::RefCell;
use std::rc::Rc;

/// The window as its top level.
fn top_level(window: &Ref<Window>) -> Ref<TopLevel> {
    window.clone().upcast()
}

#[test]
fn the_service_holds_the_state_of_a_managed_drag_and_raises_its_events() {
    let _app = start_app();
    let window = show_window(border(10.0, 10.0), 100.0, 100.0);
    let service = ManagedDragDropService::instance();
    let events = Rc::new(RefCell::new(Vec::new()));
    let started = events.clone();
    let moved = events.clone();
    let ended = events.clone();
    let _started = service.drag_started(move || started.borrow_mut().push("started"));
    let _moved = service.drag_moved(move || moved.borrow_mut().push("moved"));
    let _ended = service.drag_ended(move || ended.borrow_mut().push("ended"));

    // Nothing moves or ends before a drag begins.
    service.move_(Point::new(1.0, 1.0));
    service.end();
    assert!(events.borrow().is_empty());

    let payload: BoxedValue = text("payload");
    service.begin(&top_level(&window), Some(payload.clone()), Some("Context".to_string()), DragDropEffects::COPY, Point::new(5.0, 5.0));

    assert!(service.is_dragging());
    assert!(service.payload().is_some_and(|value| Rc::ptr_eq(&value, &payload)));
    assert_eq!(Some("Context".to_string()), service.data_format());
    assert_eq!(DragDropEffects::COPY, service.effects());
    assert_eq!(Point::new(5.0, 5.0), service.current_client_position());
    // The payload is in the context store while it is dragged.
    let key = service.payload_key();
    assert!(DragDropContextStore::get(key.as_deref()).is_some_and(|value| Rc::ptr_eq(&value, &payload)));

    service.move_(Point::new(8.0, 9.0));
    assert_eq!(Point::new(8.0, 9.0), service.current_client_position());

    service.end();

    assert!(!service.is_dragging());
    assert!(service.payload().is_none());
    assert!(service.data_format().is_none());
    assert_eq!(DragDropEffects::NONE, service.effects());
    assert!(service.origin_top_level().is_none());
    assert!(DragDropContextStore::get(key.as_deref()).is_none());
    assert_eq!(vec!["started", "moved", "ended"], *events.borrow());
}

#[test]
fn a_disposed_subscription_of_the_service_is_not_notified() {
    let _app = start_app();
    let window = show_window(border(10.0, 10.0), 100.0, 100.0);
    let service = ManagedDragDropService::instance();
    let count = Rc::new(std::cell::Cell::new(0));
    let counted = count.clone();
    let subscription = service.drag_started(move || counted.set(counted.get() + 1));

    subscription.dispose();
    service.begin(&top_level(&window), Some(text("payload")), Some("Context".to_string()), DragDropEffects::MOVE, Point::default());
    service.end();

    assert_eq!(0, count.get());
}

#[test]
fn a_managed_drop_target_wants_the_drops_its_handler_validates() {
    let _app = start_app();
    let panel = StackPanel::new();
    let target = border(50.0, 50.0);
    let other_format_target = border(50.0, 50.0);
    panel.children().add(target.clone());
    panel.children().add(other_format_target.clone());
    let handler = Rc::new(TestDropHandler::default());
    let behavior = ManagedContextDropBehavior::new();
    behavior.set_handler(Some(handler.clone()));
    add_behavior(&target, &behavior);
    let other_format_behavior = ManagedContextDropBehavior::new();
    other_format_behavior.set_handler(Some(handler.clone()));
    other_format_behavior.set_accept_data_format("Other".to_string());
    add_behavior(&other_format_target, &other_format_behavior);
    let window = show_window(panel, 100.0, 200.0);
    let service = ManagedDragDropService::instance();

    service.begin(&top_level(&window), Some(text("payload")), Some("Context".to_string()), DragDropEffects::MOVE, Point::default());

    assert!(target.classes().contains("droptarget"));
    assert!(!other_format_target.classes().contains("droptarget"));

    service.end();

    assert!(!target.classes().contains("droptarget"));
    assert!(!target.classes().contains("dragover"));
}

#[test]
fn a_managed_drop_target_without_a_handler_does_not_want_drops() {
    let _app = start_app();
    let target = border(50.0, 50.0);
    add_behavior(&target, &ManagedContextDropBehavior::new());
    let window = show_window(target.clone(), 100.0, 100.0);
    let service = ManagedDragDropService::instance();

    service.begin(&top_level(&window), Some(text("payload")), Some("Context".to_string()), DragDropEffects::MOVE, Point::default());

    assert!(!target.classes().contains("droptarget"));

    service.end();
}

#[test]
fn a_detached_managed_drop_target_is_not_notified() {
    let _app = start_app();
    let target = border(50.0, 50.0);
    let handler = Rc::new(TestDropHandler::default());
    let behavior = ManagedContextDropBehavior::new();
    behavior.set_handler(Some(handler.clone()));
    add_behavior(&target, &behavior);
    let window = show_window(target.clone(), 100.0, 100.0);
    behavior.detach();
    let service = ManagedDragDropService::instance();

    service.begin(&top_level(&window), Some(text("payload")), Some("Context".to_string()), DragDropEffects::MOVE, Point::default());

    assert!(!target.classes().contains("droptarget"));

    service.end();
}

#[test]
fn the_preview_service_shows_one_preview_at_a_time() {
    let _app = start_app();
    let window = show_window(border(10.0, 10.0), 100.0, 100.0);

    DragPreviewService::show(Some(text("preview")), None, &window, Point::new(5.0, 5.0), Point::default(), 2.0);
    assert!(DragPreviewService::is_shown());

    DragPreviewService::move_(&window, Point::new(10.0, 10.0), Point::default());
    DragPreviewService::show(Some(text("preview")), None, &window, Point::new(5.0, 5.0), Point::default(), 0.5);
    assert!(DragPreviewService::is_shown());

    DragPreviewService::hide();
    assert!(!DragPreviewService::is_shown());
}

#[test]
fn the_managed_drag_behavior_drags_its_context_until_the_release() {
    let _app = start_app();
    let source = border(50.0, 50.0);
    let behavior = ManagedContextDragBehavior::new();
    behavior.set_context(Some(text("Item")));
    add_behavior(&source, &behavior);
    let _window = show_window(source.clone(), 200.0, 200.0);
    let service = ManagedDragDropService::instance();

    let mouse = Mouse::new();
    mouse.down(&source, Point::new(5.0, 5.0));
    mouse.move_to(&source, Point::new(6.0, 6.0));
    run_jobs();
    assert!(!service.is_dragging());

    mouse.move_to(&source, Point::new(30.0, 30.0));
    run_jobs();

    assert!(service.is_dragging());
    assert!(behavior.is_internal_dragging());
    assert!(is_text(service.payload().as_ref(), "Item"));
    assert_eq!(Some("Context".to_string()), service.data_format());
    assert!(DragPreviewService::is_shown());

    mouse.up(&source, Point::new(30.0, 30.0));
    run_jobs();

    assert!(!service.is_dragging());
    assert!(!behavior.is_internal_dragging());
    assert!(!DragPreviewService::is_shown());
}
