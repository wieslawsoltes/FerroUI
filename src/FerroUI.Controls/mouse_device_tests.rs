//! Port of the two tests of `Input/MouseDeviceTests.cs` (base unit tests) that close a presentation source: the
//! source of a top-level is in this crate. The other tests of the file are in `input/input_tests.rs` of
//! `ferroui-base`, among them `GetPosition_Should_Support_Cross_Tree_Requests` on the hosts of its tests.

use crate::pointer_tests_base::{
    create_input_root, create_pointer_device_mock, create_top_level_impl_mock, raw_pointer_moved, MockHitTester,
};
use crate::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use crate::{Border, Window};
use ferroui_base::input::{InputElement, InputManager, KeyModifiers, PointerEventArgs, PointerPointProperties};
use ferroui_base::{PixelPoint, Point, Ref};
use std::cell::Cell;
use std::rc::Rc;

/// What upstream's `SetupCrossTreePositionRequest` hands out: the pointer event raised on `element_a`, the two
/// elements, each in a top-level of its own, and the application.
struct CrossTreePositionRequest {
    pointer_event: Rc<PointerEventArgs>,
    element_a: Ref<Border>,
    element_b: Ref<Border>,
    root1: Ref<Window>,
    root2: Ref<Window>,
    _app: UnitTestApplicationScope,
}

fn setup_cross_tree_position_request(top_level_position: PixelPoint) -> CrossTreePositionRequest {
    let app = UnitTestApplication::start(TestServices::new().with_input_manager(Rc::new(InputManager::new())));

    let renderer = Rc::new(MockHitTester::default());
    let device = create_pointer_device_mock();
    let impl1 = create_top_level_impl_mock();
    // Mocked position: top_level_position
    impl1.position.set(top_level_position);

    let element_a = Border::new();
    let received = Rc::new(Cell::new(std::ptr::null::<PointerEventArgs>()));

    let recorded = received.clone();
    element_a.pointer_moved(move |_, e| recorded.set(e));
    let root1 = create_input_root(&impl1, &element_a, &renderer);

    // Upstream `SetMove`: the device raises a pointer moved event on the element when it processes a raw event.
    let pointer_event = Rc::new(PointerEventArgs::new(
        Some(InputElement::pointer_moved_event()),
        &element_a,
        device.pointer(),
        Some(&root1.input_root().root_element()),
        Point::default(),
        0,
        PointerPointProperties::NONE,
        KeyModifiers::NONE,
    ));
    let (raised, target) = (pointer_event.clone(), element_a.clone());
    device.setup_process_raw_event(move || target.raise_event(&*raised));
    raw_pointer_moved(&impl1, &device, &root1, Point::default());

    assert!(std::ptr::eq(received.get(), Rc::as_ptr(&pointer_event)));

    let impl2 = create_top_level_impl_mock();
    // Upstream sets `PointToClient` of the second mock up for the default point only ("mocked position:
    // top_level_position * 2"): for the point that is asked for, the mock answers as the mocks of the base class
    // do, a top-level at the origin of the screen. That is the position of this mock.

    let element_b = Border::new();
    let root2 = create_input_root(&impl2, &element_b, &renderer);

    // The assertion of `GetPosition_Should_Support_Cross_Tree_Requests`: while both sources are open the
    // position is translated between the trees, so the default the tests below expect is the answer for a
    // closed source and not the answer of a request that never worked.
    assert_eq!(top_level_position.to_point(1.0), pointer_event.get_position(Some(&element_b)));

    CrossTreePositionRequest { pointer_event, element_a, element_b, root1, root2, _app: app }
}

#[test]
fn get_position_should_return_default_when_cross_tree_source_closed() {
    let top_level_offset = PixelPoint::new(5, 0);
    let request = setup_cross_tree_position_request(top_level_offset);

    assert!(request.element_a.presentation_source().is_some());
    request.root1.presentation_source().dispose();

    assert_eq!(Point::default(), request.pointer_event.get_position(Some(&request.element_b)));
}

#[test]
fn get_position_should_return_default_when_cross_tree_target_closed() {
    let top_level_offset = PixelPoint::new(5, 0);
    let request = setup_cross_tree_position_request(top_level_offset);

    assert!(request.element_b.presentation_source().is_some());
    request.root2.presentation_source().dispose();

    assert_eq!(Point::default(), request.pointer_event.get_position(Some(&request.element_b)));
}
