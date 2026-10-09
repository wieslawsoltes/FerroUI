//! Port of the tests of `Input/PointerOverTests.cs` (base unit tests) that need a top-level: its closing, the
//! scene invalidation of its renderer and a hit tester that answers what the test sets up. The other tests of the
//! file are in `input/input_tests.rs` of `ferroui-base`, on a tree that is hit-tested by geometry.
//!
//! The mocks of upstream's `PointerTestsBase` are in `pointer_tests_base.rs`. The device does nothing with a raw
//! event, as upstream's mock: no pointer moved event is raised.

use crate::pointer_tests_base::{
    create_input_root, create_pointer_device_mock, create_top_level_impl_mock, raw_pointer_moved, MockHitTester,
};
use crate::testing::{NullRenderer, TestServices, UnitTestApplication};
use crate::{Canvas, Panel, Window};
use ferroui_base::input::{InputElement, InputManager, PointerEventArgs};
use ferroui_base::interactivity::Interactive;
use ferroui_base::{Point, Rect, Ref, Visual};
use std::cell::RefCell;
use std::rc::Rc;

fn set_hit(renderer: &MockHitTester, hit: Option<&Ref<Canvas>>) {
    crate::pointer_tests_base::set_hit(renderer, hit.map(|hit| hit.clone().upcast::<Visual>()));
}

fn panel_with(children: &[&Ref<Canvas>]) -> Ref<Panel> {
    let panel = Panel::new();
    for child in children {
        panel.children().add(*child);
    }
    panel
}

type Events = Rc<RefCell<Vec<(Ref<Interactive>, String, Point)>>>;

fn add_entered_exited_handlers(result: &Events, controls: &[Ref<Interactive>]) {
    for control in controls {
        for event in [
            InputElement::pointer_entered_event(),
            InputElement::pointer_exited_event(),
            InputElement::pointer_moved_event(),
        ] {
            let result = result.clone();
            control.add_handler(event, move |sender: &Interactive, e: &PointerEventArgs| {
                result.borrow_mut().push((
                    sender.to_ref(),
                    e.routed_event().unwrap().name().to_string(),
                    e.get_position(None),
                ));
            });
        }
    }
}

fn raise_scene_invalidated(top_level: &Window) {
    let renderer = top_level.renderer();
    let renderer = renderer.as_any().downcast_ref::<NullRenderer>().expect("the renderer of the tests");
    renderer.raise_scene_invalidated(Rect::new(0.0, 0.0, 10000.0, 10000.0));
}

fn entered_exited(canvas: &Ref<Canvas>, root: &Ref<Window>, position: Point) -> Vec<(Ref<Interactive>, String, Point)> {
    vec![
        (canvas.clone().upcast(), "PointerEntered".to_string(), position),
        (root.clone().upcast(), "PointerEntered".to_string(), position),
        (canvas.clone().upcast(), "PointerExited".to_string(), position),
        (root.clone().upcast(), "PointerExited".to_string(), position),
    ]
}

#[test]
fn close_should_remove_pointer_over() {
    let _app = UnitTestApplication::start(TestServices::new().with_input_manager(Rc::new(InputManager::new())));

    let renderer = Rc::new(MockHitTester::default());
    let device = create_pointer_device_mock();
    let window_impl = create_top_level_impl_mock();

    let canvas = Canvas::new();
    let root = create_input_root(&window_impl, &panel_with(&[&canvas]), &renderer);

    set_hit(&renderer, Some(&canvas));
    raw_pointer_moved(&window_impl, &device, &root, Point::default());

    assert!(canvas.is_pointer_over());

    crate::platform::ITopLevelImpl::closed(&*window_impl).expect("the window handles its closing")();

    assert!(!canvas.is_pointer_over());
}

#[test]
fn pointer_entered_exited_should_set_correct_position() {
    let _app = UnitTestApplication::start(TestServices::new().with_input_manager(Rc::new(InputManager::new())));

    let expected_position = Point::new(15.0, 15.0);
    let renderer = Rc::new(MockHitTester::default());
    let device = create_pointer_device_mock();
    let window_impl = create_top_level_impl_mock();
    let result: Events = Rc::new(RefCell::new(Vec::new()));

    let canvas = Canvas::new();
    let root = create_input_root(&window_impl, &panel_with(&[&canvas]), &renderer);

    add_entered_exited_handlers(&result, &[root.clone().upcast(), canvas.clone().upcast()]);

    set_hit(&renderer, Some(&canvas));
    raw_pointer_moved(&window_impl, &device, &root, expected_position);

    set_hit(&renderer, None);
    raw_pointer_moved(&window_impl, &device, &root, expected_position);

    assert_eq!(entered_exited(&canvas, &root, expected_position), *result.borrow());
}

#[test]
fn render_invalidation_should_affect_pointer_over() {
    let _app = UnitTestApplication::start(TestServices::new().with_input_manager(Rc::new(InputManager::new())));

    let renderer = Rc::new(MockHitTester::default());
    let device = create_pointer_device_mock();
    let window_impl = create_top_level_impl_mock();

    let last_client_position = Point::new(1.0, 5.0);

    let result: Events = Rc::new(RefCell::new(Vec::new()));

    let canvas = Canvas::new();
    let root = create_input_root(&window_impl, &panel_with(&[&canvas]), &renderer);
    add_entered_exited_handlers(&result, &[root.clone().upcast(), canvas.clone().upcast()]);

    // Let input know about latest device.
    set_hit(&renderer, Some(&canvas));
    raw_pointer_moved(&window_impl, &device, &root, last_client_position);
    assert!(canvas.is_pointer_over());

    set_hit(&renderer, Some(&canvas));
    raise_scene_invalidated(&root);
    assert!(canvas.is_pointer_over());

    // Raise SceneInvalidated again, but now hide element from the hittest.
    set_hit(&renderer, None);
    raise_scene_invalidated(&root);
    assert!(!canvas.is_pointer_over());

    assert_eq!(entered_exited(&canvas, &root, last_client_position), *result.borrow());
}

#[test]
fn pointer_over_invalidation_should_use_previously_captured_element() {
    let _app = UnitTestApplication::start(TestServices::new().with_input_manager(Rc::new(InputManager::new())));

    let renderer = Rc::new(MockHitTester::default());
    let device = create_pointer_device_mock();
    let window_impl = create_top_level_impl_mock();

    let canvas1 = Canvas::new();
    let canvas2 = Canvas::new();

    let root = create_input_root(&window_impl, &panel_with(&[&canvas1, &canvas2]), &renderer);

    let captured = canvas1.clone();
    canvas1.pointer_moved(move |_, a| a.pointer().capture(Some(&captured.clone().upcast())));

    // Let input know about latest device.
    set_hit(&renderer, Some(&canvas1));
    raw_pointer_moved(&window_impl, &device, &root, Point::default());
    assert!(canvas1.is_pointer_over());
    assert!(!canvas2.is_pointer_over());

    set_hit(&renderer, Some(&canvas2));
    raise_scene_invalidated(&root);
    assert!(!canvas1.is_pointer_over());
    assert!(canvas2.is_pointer_over());
}
