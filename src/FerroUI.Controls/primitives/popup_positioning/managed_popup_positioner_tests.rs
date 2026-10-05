use super::{
    IManagedPopupPositionerPopup, IPopupPositioner, ManagedPopupPositioner, ManagedPopupPositionerScreenInfo,
    PopupAnchor, PopupGravity, PopupPositionerConstraintAdjustment, PopupPositionerParameters,
};
use ferroui_base::{Point, Rect, Size};
use std::cell::Cell;
use std::rc::Rc;

struct MockManagedPopupPositionerPopup {
    last_position: Cell<Point>,
    #[allow(dead_code)] // Recorded as in the reference test double; no test reads it.
    last_size: Cell<Size>,
}

impl MockManagedPopupPositionerPopup {
    fn new() -> Rc<Self> {
        Rc::new(Self { last_position: Cell::new(Point::default()), last_size: Cell::new(Size::default()) })
    }
}

impl IManagedPopupPositionerPopup for MockManagedPopupPositionerPopup {
    // Four screens arranged in a 2x2 grid, meeting at (1000, 1000).
    fn screens(&self) -> Vec<ManagedPopupPositionerScreenInfo> {
        vec![
            ManagedPopupPositionerScreenInfo::new(
                Rect::new(0.0, 0.0, 1000.0, 1000.0),
                Rect::new(0.0, 0.0, 1000.0, 1000.0),
            ),
            ManagedPopupPositionerScreenInfo::new(
                Rect::new(1000.0, 0.0, 1000.0, 1000.0),
                Rect::new(1000.0, 0.0, 1000.0, 1000.0),
            ),
            ManagedPopupPositionerScreenInfo::new(
                Rect::new(0.0, 1000.0, 1000.0, 1000.0),
                Rect::new(0.0, 1000.0, 1000.0, 1000.0),
            ),
            ManagedPopupPositionerScreenInfo::new(
                Rect::new(1000.0, 1000.0, 1000.0, 1000.0),
                Rect::new(1000.0, 1000.0, 1000.0, 1000.0),
            ),
        ]
    }

    fn parent_client_area_screen_geometry(&self) -> Rect {
        Rect::new(0.0, 0.0, 1000.0, 1000.0)
    }

    fn scaling(&self) -> f64 {
        1.0
    }

    fn move_and_resize(&self, device_point: Point, virtual_size: Size) {
        self.last_position.set(device_point);
        self.last_size.set(virtual_size);
    }
}

// The anchor rectangle overlaps the 4 screens, so each corner lands on a different screen.
// With no gravity the popup is centered on the anchor point and is slid back into the screen containing that point.
fn uses_screen_containing_the_anchor_point(anchor: PopupAnchor, expected_x: f64, expected_y: f64) {
    let popup = MockManagedPopupPositionerPopup::new();
    let positioner = ManagedPopupPositioner::new(popup.clone());

    let mut parameters = PopupPositionerParameters::default();
    parameters.size = Size::new(400.0, 400.0);
    parameters.anchor_rectangle = Rect::new(900.0, 900.0, 200.0, 200.0);
    parameters.set_anchor(anchor);
    parameters.set_gravity(PopupGravity::NONE);
    parameters.constraint_adjustment = PopupPositionerConstraintAdjustment::ALL;

    positioner.update(parameters);

    assert_eq!(Point::new(expected_x, expected_y), popup.last_position.get());
}

#[test]
fn uses_screen_containing_the_anchor_point_top_left() {
    uses_screen_containing_the_anchor_point(PopupAnchor::TOP_LEFT, 600.0, 600.0);
}

#[test]
fn uses_screen_containing_the_anchor_point_top_right() {
    uses_screen_containing_the_anchor_point(PopupAnchor::TOP_RIGHT, 1000.0, 600.0);
}

#[test]
fn uses_screen_containing_the_anchor_point_bottom_left() {
    uses_screen_containing_the_anchor_point(PopupAnchor::BOTTOM_LEFT, 600.0, 1000.0);
}

#[test]
fn uses_screen_containing_the_anchor_point_bottom_right() {
    uses_screen_containing_the_anchor_point(PopupAnchor::BOTTOM_RIGHT, 1000.0, 1000.0);
}
