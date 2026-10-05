use super::raw::RawPointerPoint;
use super::{IPointer, MouseButton, RawInputModifiers};
use crate::{Point, Rect};
use std::rc::Rc;

/// Provides basic properties for the input pointer associated with a single
/// mouse, pen/stylus, or touch contact.
#[derive(Clone)]
pub struct PointerPoint {
    /// The pointer associated with this point.
    pub pointer: Rc<dyn IPointer>,
    /// Extended information about the input pointer.
    pub properties: PointerPointProperties,
    /// The location of the pointer input in client coordinates.
    pub position: Point,
}

impl PointerPoint {
    /// Creates a pointer point.
    pub fn new(pointer: Rc<dyn IPointer>, position: Point, properties: PointerPointProperties) -> Self {
        Self { pointer, properties, position }
    }
}

impl PartialEq for PointerPoint {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(Rc::as_ptr(&self.pointer), Rc::as_ptr(&other.pointer))
            && self.properties == other.properties
            && self.position == other.position
    }
}

/// Provides extended properties for a pointer point.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PointerPointProperties {
    /// The bounding rectangle of the contact area (typically from touch
    /// input).
    pub contact_rect: Rect,
    /// Whether the pointer input was triggered by the primary action mode of
    /// an input device.
    pub is_left_button_pressed: bool,
    /// Whether the pointer input was triggered by the tertiary action mode
    /// of an input device.
    pub is_middle_button_pressed: bool,
    /// Whether the pointer input was triggered by the secondary action mode
    /// (if supported) of an input device.
    pub is_right_button_pressed: bool,
    /// Whether the pointer input was triggered by the first extended mouse
    /// button (XButton1).
    pub is_x_button_1_pressed: bool,
    /// Whether the pointer input was triggered by the second extended mouse
    /// button (XButton2).
    pub is_x_button_2_pressed: bool,
    /// Whether the barrel button of the pen/stylus device is pressed.
    pub is_barrel_button_pressed: bool,
    /// Whether the input is from a pen eraser.
    pub is_eraser: bool,
    /// Whether the digitizer input is from an inverted pen.
    pub is_inverted: bool,
    /// The clockwise rotation in degrees of a pen device around its own
    /// major axis (such as when the user spins the pen in their fingers).
    /// A value between 0.0 and 359.0 in degrees of rotation; the default
    /// value is 0.0.
    pub twist: f32,
    /// A value that indicates the force that the pointer device (typically
    /// a pen/stylus) exerts on the surface of the digitizer. A value from 0
    /// to 1.0; the default value is 0.5.
    pub pressure: f32,
    /// The plane angle between the Y-Z plane and the plane that contains
    /// the Y axis and the axis of the input device (typically a
    /// pen/stylus). The value is 0.0 when the finger or pen is
    /// perpendicular to the digitizer surface, between 0.0 and 90.0 when
    /// tilted to the right of perpendicular, and between 0.0 and -90.0 when
    /// tilted to the left of perpendicular. The default value is 0.0.
    pub x_tilt: f32,
    /// The plane angle between the X-Z plane and the plane that contains
    /// the X axis and the axis of the input device (typically a
    /// pen/stylus). The value is 0.0 when the finger or pen is
    /// perpendicular to the digitizer surface, between 0.0 and 90.0 when
    /// tilted towards the user, and between 0.0 and -90.0 when tilted away
    /// from the user. The default value is 0.0.
    pub y_tilt: f32,
    /// The kind of pointer state change.
    pub pointer_update_kind: PointerUpdateKind,
}

impl Default for PointerPointProperties {
    fn default() -> Self {
        Self::NONE
    }
}

impl PointerPointProperties {
    /// The default properties: nothing pressed.
    pub const NONE: PointerPointProperties = PointerPointProperties {
        contact_rect: Rect::new(0.0, 0.0, 0.0, 0.0),
        is_left_button_pressed: false,
        is_middle_button_pressed: false,
        is_right_button_pressed: false,
        is_x_button_1_pressed: false,
        is_x_button_2_pressed: false,
        is_barrel_button_pressed: false,
        is_eraser: false,
        is_inverted: false,
        twist: 0.0,
        pressure: 0.5,
        x_tilt: 0.0,
        y_tilt: 0.0,
        pointer_update_kind: PointerUpdateKind::LeftButtonPressed,
    };

    /// Creates properties from raw input modifiers and the kind of state
    /// change.
    pub fn new(modifiers: RawInputModifiers, kind: PointerUpdateKind) -> Self {
        let mut result = Self {
            pointer_update_kind: kind,
            is_left_button_pressed: modifiers.contains(RawInputModifiers::LEFT_MOUSE_BUTTON),
            is_middle_button_pressed: modifiers.contains(RawInputModifiers::MIDDLE_MOUSE_BUTTON),
            is_right_button_pressed: modifiers.contains(RawInputModifiers::RIGHT_MOUSE_BUTTON),
            is_x_button_1_pressed: modifiers.contains(RawInputModifiers::X_BUTTON_1_MOUSE_BUTTON),
            is_x_button_2_pressed: modifiers.contains(RawInputModifiers::X_BUTTON_2_MOUSE_BUTTON),
            is_inverted: modifiers.contains(RawInputModifiers::PEN_INVERTED),
            is_eraser: modifiers.contains(RawInputModifiers::PEN_ERASER),
            is_barrel_button_pressed: modifiers.contains(RawInputModifiers::PEN_BARREL_BUTTON),
            ..Self::NONE
        };

        // The underlying input source might be reporting the previous state,
        // so make sure that we reflect the current state.
        match kind {
            PointerUpdateKind::LeftButtonPressed => result.is_left_button_pressed = true,
            PointerUpdateKind::LeftButtonReleased => result.is_left_button_pressed = false,
            PointerUpdateKind::MiddleButtonPressed => result.is_middle_button_pressed = true,
            PointerUpdateKind::MiddleButtonReleased => result.is_middle_button_pressed = false,
            PointerUpdateKind::RightButtonPressed => result.is_right_button_pressed = true,
            PointerUpdateKind::RightButtonReleased => result.is_right_button_pressed = false,
            PointerUpdateKind::XButton1Pressed => result.is_x_button_1_pressed = true,
            PointerUpdateKind::XButton1Released => result.is_x_button_1_pressed = false,
            PointerUpdateKind::XButton2Pressed => result.is_x_button_2_pressed = true,
            PointerUpdateKind::XButton2Released => result.is_x_button_2_pressed = false,
            PointerUpdateKind::Other => {}
        }

        result
    }

    /// Creates properties with pen state.
    pub fn with_pen_state(
        modifiers: RawInputModifiers,
        kind: PointerUpdateKind,
        twist: f32,
        pressure: f32,
        x_tilt: f32,
        y_tilt: f32,
    ) -> Self {
        Self::with_pen_state_and_contact_rect(modifiers, kind, twist, pressure, x_tilt, y_tilt, Rect::default())
    }

    /// Creates properties with pen state and a contact rectangle.
    pub fn with_pen_state_and_contact_rect(
        modifiers: RawInputModifiers,
        kind: PointerUpdateKind,
        twist: f32,
        pressure: f32,
        x_tilt: f32,
        y_tilt: f32,
        contact_rect: Rect,
    ) -> Self {
        Self { twist, pressure, x_tilt, y_tilt, contact_rect, ..Self::new(modifiers, kind) }
    }

    /// Creates properties from a raw pointer point.
    pub fn from_raw_point(modifiers: RawInputModifiers, kind: PointerUpdateKind, raw_point: RawPointerPoint) -> Self {
        Self::with_pen_state_and_contact_rect(
            modifiers,
            kind,
            raw_point.twist,
            raw_point.pressure,
            raw_point.x_tilt,
            raw_point.y_tilt,
            raw_point.contact_rect(),
        )
    }

    /// Creates the properties of an intermediate point: the button state of
    /// `based_on` with the pen state of a raw pointer point.
    pub fn based_on(based_on: PointerPointProperties, raw_point: RawPointerPoint) -> Self {
        Self {
            is_left_button_pressed: based_on.is_left_button_pressed,
            is_middle_button_pressed: based_on.is_middle_button_pressed,
            is_right_button_pressed: based_on.is_right_button_pressed,
            is_x_button_1_pressed: based_on.is_x_button_1_pressed,
            is_x_button_2_pressed: based_on.is_x_button_2_pressed,
            is_inverted: based_on.is_inverted,
            is_eraser: based_on.is_eraser,
            is_barrel_button_pressed: based_on.is_barrel_button_pressed,
            twist: raw_point.twist,
            pressure: raw_point.pressure,
            x_tilt: raw_point.x_tilt,
            y_tilt: raw_point.y_tilt,
            ..Self::NONE
        }
    }
}

/// The kind of state change of a pointer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum PointerUpdateKind {
    #[default]
    LeftButtonPressed,
    MiddleButtonPressed,
    RightButtonPressed,
    XButton1Pressed,
    XButton2Pressed,
    LeftButtonReleased,
    MiddleButtonReleased,
    RightButtonReleased,
    XButton1Released,
    XButton2Released,
    Other,
}

impl PointerUpdateKind {
    /// The mouse button whose state the update changed.
    pub fn get_mouse_button(self) -> MouseButton {
        match self {
            PointerUpdateKind::LeftButtonPressed | PointerUpdateKind::LeftButtonReleased => MouseButton::Left,
            PointerUpdateKind::MiddleButtonPressed | PointerUpdateKind::MiddleButtonReleased => MouseButton::Middle,
            PointerUpdateKind::RightButtonPressed | PointerUpdateKind::RightButtonReleased => MouseButton::Right,
            PointerUpdateKind::XButton1Pressed | PointerUpdateKind::XButton1Released => MouseButton::XButton1,
            PointerUpdateKind::XButton2Pressed | PointerUpdateKind::XButton2Released => MouseButton::XButton2,
            PointerUpdateKind::Other => MouseButton::None,
        }
    }
}
