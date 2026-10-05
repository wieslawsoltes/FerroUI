use super::raw_input_event_args::raw_input_event_args;
use super::RawInputEventArgs;
use crate::input::{IInputDevice, IInputRoot, InputElement, IntermediatePoints, RawInputModifiers};
use crate::{Point, Rect, Ref, Size};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The kinds of raw pointer events.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RawPointerEventType {
    LeaveWindow,
    LeftButtonDown,
    LeftButtonUp,
    RightButtonDown,
    RightButtonUp,
    MiddleButtonDown,
    MiddleButtonUp,
    XButton1Down,
    XButton1Up,
    XButton2Down,
    XButton2Up,
    Move,
    Wheel,
    NonClientLeftButtonDown,
    TouchBegin,
    TouchUpdate,
    TouchEnd,
    TouchCancel,
    Magnify,
    Rotate,
    Swipe,
    CancelCapture,
}

/// The result of input hit testing a raw pointer event: the element under
/// the pointer and its first enabled ancestor (or itself).
type InputHitTestResult = (Option<Ref<InputElement>>, Option<Ref<InputElement>>);

/// A raw mouse (or other pointer) event.
pub struct RawPointerEventArgs {
    base: RawInputEventArgs,
    point: Cell<RawPointerPoint>,
    raw_pointer_id: Cell<i64>,
    type_: Cell<RawPointerEventType>,
    input_modifiers: Cell<RawInputModifiers>,
    intermediate_points: RefCell<Option<IntermediatePoints>>,
    platform_input_event_cookie: RefCell<Option<Rc<dyn Any>>>,
    input_hit_test_result: RefCell<InputHitTestResult>,
}

raw_input_event_args!(RawPointerEventArgs: RawInputEventArgs);

impl RawPointerEventArgs {
    /// Creates raw pointer event args for a position in client coordinates.
    pub fn new(
        device: Rc<dyn IInputDevice>,
        timestamp: u64,
        root: Rc<dyn IInputRoot>,
        type_: RawPointerEventType,
        position: Point,
        input_modifiers: RawInputModifiers,
    ) -> Self {
        let point = RawPointerPoint { position, ..RawPointerPoint::new() };
        Self::with_point(device, timestamp, root, type_, point, input_modifiers)
    }

    /// Creates raw pointer event args for a raw pointer point.
    pub fn with_point(
        device: Rc<dyn IInputDevice>,
        timestamp: u64,
        root: Rc<dyn IInputRoot>,
        type_: RawPointerEventType,
        point: RawPointerPoint,
        input_modifiers: RawInputModifiers,
    ) -> Self {
        Self {
            base: RawInputEventArgs::new(device, timestamp, root),
            point: Cell::new(point),
            raw_pointer_id: Cell::new(0),
            type_: Cell::new(type_),
            input_modifiers: Cell::new(input_modifiers),
            intermediate_points: RefCell::new(None),
            platform_input_event_cookie: RefCell::new(None),
            input_hit_test_result: RefCell::new((None, None)),
        }
    }

    /// The raw pointer identifier.
    #[inline]
    pub fn raw_pointer_id(&self) -> i64 {
        self.raw_pointer_id.get()
    }

    pub fn set_raw_pointer_id(&self, value: i64) {
        self.raw_pointer_id.set(value)
    }

    /// The pointer properties and position, in client DIPs.
    #[inline]
    pub fn point(&self) -> RawPointerPoint {
        self.point.get()
    }

    pub fn set_point(&self, value: RawPointerPoint) {
        self.point.set(value)
    }

    /// The mouse position, in client DIPs.
    #[inline]
    pub fn position(&self) -> Point {
        self.point.get().position
    }

    pub fn set_position(&self, value: Point) {
        let mut point = self.point.get();
        point.position = value;
        self.point.set(point);
    }

    /// The type of the event.
    #[inline]
    pub fn type_(&self) -> RawPointerEventType {
        self.type_.get()
    }

    pub fn set_type(&self, value: RawPointerEventType) {
        self.type_.set(value)
    }

    /// The input modifiers.
    #[inline]
    pub fn input_modifiers(&self) -> RawInputModifiers {
        self.input_modifiers.get()
    }

    pub fn set_input_modifiers(&self, value: RawInputModifiers) {
        self.input_modifiers.set(value)
    }

    /// Points that were traversed by a pointer since the previous relevant
    /// event, only valid for Move and TouchUpdate.
    pub fn intermediate_points(&self) -> Option<IntermediatePoints> {
        self.intermediate_points.borrow().clone()
    }

    pub fn set_intermediate_points(&self, value: Option<IntermediatePoints>) {
        let old = self.intermediate_points.replace(value);
        drop(old);
    }

    /// The platform's opaque cookie for the input event.
    pub fn platform_input_event_cookie(&self) -> Option<Rc<dyn Any>> {
        self.platform_input_event_cookie.borrow().clone()
    }

    pub fn set_platform_input_event_cookie(&self, value: Option<Rc<dyn Any>>) {
        let old = self.platform_input_event_cookie.replace(value);
        drop(old);
    }

    /// The result of input hit testing the event: the element under the
    /// pointer and its first enabled ancestor. Set by the presentation
    /// source before the event is processed.
    pub fn input_hit_test_result(&self) -> (Option<Ref<InputElement>>, Option<Ref<InputElement>>) {
        self.input_hit_test_result.borrow().clone()
    }

    pub fn set_input_hit_test_result(
        &self,
        element: Option<Ref<InputElement>>,
        first_enabled_ancestor: Option<Ref<InputElement>>,
    ) {
        let old = self.input_hit_test_result.replace((element, first_enabled_ancestor));
        drop(old);
    }
}

/// The position and pen state of a raw pointer event.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RawPointerPoint {
    /// Pointer position, in client DIPs.
    pub position: Point,
    pub twist: f32,
    pub pressure: f32,
    pub x_tilt: f32,
    pub y_tilt: f32,
    contact_rect: Option<Rect>,
}

impl Default for RawPointerPoint {
    fn default() -> Self {
        Self::new()
    }
}

impl RawPointerPoint {
    /// Creates a raw pointer point at the origin with default pressure.
    pub fn new() -> Self {
        Self { position: Point::default(), twist: 0.0, pressure: 0.5, x_tilt: 0.0, y_tilt: 0.0, contact_rect: None }
    }

    /// The contact rectangle of the pointer; an empty rectangle at the
    /// pointer position unless set.
    pub fn contact_rect(&self) -> Rect {
        self.contact_rect.unwrap_or_else(|| Rect::from_position_size(self.position, Size::default()))
    }

    pub fn set_contact_rect(&mut self, value: Rect) {
        self.contact_rect = Some(value);
    }
}
