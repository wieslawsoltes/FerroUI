//! Motion events of the system to raw pointer events of the framework.
//!
//! The reference reads a `MotionEvent`; here the Java view copies what the
//! reference reads of it ([`MotionEventData`]), because the event is only
//! valid while the view dispatches it, and this helper translates the copy.

use ferroui_base::input::raw::{
    IRawInputEventArgs, RawMouseWheelEventArgs, RawPointerEventType, RawPointerPoint, RawTouchEventArgs,
};
use ferroui_base::input::{
    IInputDevice, IInputRoot, IntermediatePoints, MouseDevice, PenDevice, RawInputModifiers, TouchDevice,
};
use ferroui_base::{Point, Vector};
use std::cell::{Cell, LazyCell};
use std::rc::{Rc, Weak};

/// `MotionEvent.ACTION_*`, masked.
#[allow(dead_code)]
pub(crate) mod motion_event_actions {
    pub const DOWN: i32 = 0;
    pub const UP: i32 = 1;
    pub const MOVE: i32 = 2;
    pub const CANCEL: i32 = 3;
    pub const OUTSIDE: i32 = 4;
    pub const POINTER_DOWN: i32 = 5;
    pub const POINTER_UP: i32 = 6;
    pub const HOVER_MOVE: i32 = 7;
    pub const SCROLL: i32 = 8;
    pub const HOVER_ENTER: i32 = 9;
    pub const HOVER_EXIT: i32 = 10;
    pub const BUTTON_PRESS: i32 = 11;
    pub const BUTTON_RELEASE: i32 = 12;
}

/// `MotionEvent.TOOL_TYPE_*`.
#[allow(dead_code)]
pub(crate) mod motion_event_tool_type {
    pub const UNKNOWN: i32 = 0;
    pub const FINGER: i32 = 1;
    pub const STYLUS: i32 = 2;
    pub const MOUSE: i32 = 3;
    pub const ERASER: i32 = 4;
}

/// `MotionEvent.BUTTON_*`.
pub(crate) mod motion_event_button_state {
    pub const PRIMARY: i32 = 1;
    pub const SECONDARY: i32 = 2;
    pub const TERTIARY: i32 = 4;
    pub const BACK: i32 = 8;
    pub const FORWARD: i32 = 16;
    pub const STYLUS_PRIMARY: i32 = 32;
    pub const STYLUS_SECONDARY: i32 = 64;
}

/// `KeyEvent.META_*_ON`.
pub(crate) mod meta_key_states {
    pub const SHIFT_ON: i32 = 0x1;
    pub const ALT_ON: i32 = 0x2;
    pub const CTRL_ON: i32 = 0x1000;
    pub const META_ON: i32 = 0x10000;
}

/// What the reference reads of one pointer at one time: the position in
/// pixels of the view, the pressure and the orientation in radians.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct MotionSample {
    pub x: f32,
    pub y: f32,
    pub pressure: f32,
    pub orientation: f32,
}

/// A pointer of a motion event.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct MotionPointer {
    pub id: i32,
    pub tool_type: i32,
    pub current: MotionSample,
    /// The earlier positions of the pointer since the last event, oldest
    /// first; only a move has them.
    pub history: Vec<MotionSample>,
}

/// The copy of a `MotionEvent`.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct MotionEventData {
    /// `getEventTime`: milliseconds of the uptime clock.
    pub event_time: i64,
    pub action_masked: i32,
    pub action_index: i32,
    pub meta_state: i32,
    pub button_state: i32,
    pub action_button: i32,
    pub pointers: Vec<MotionPointer>,
    /// The values of the two scroll axes.
    pub hscroll: f32,
    pub vscroll: f32,
}

impl MotionEventData {
    /// Reads the arrays the Java view fills: per pointer its id and tool
    /// type in `pointers`, and in `values` four numbers per pointer (x, y,
    /// pressure, orientation) followed by four numbers per pointer and
    /// position of the history. `None` when the arrays do not have that
    /// shape.
    #[allow(clippy::too_many_arguments)]
    pub fn from_arrays(
        event_time: i64,
        action_masked: i32,
        action_index: i32,
        meta_state: i32,
        button_state: i32,
        action_button: i32,
        history_size: i32,
        pointers: &[i32],
        values: &[f32],
        hscroll: f32,
        vscroll: f32,
    ) -> Option<MotionEventData> {
        if pointers.len() % 2 != 0 || history_size < 0 {
            return None;
        }
        let pointer_count = pointers.len() / 2;
        let history_size = history_size as usize;
        if values.len() != pointer_count * 4 * (1 + history_size) {
            return None;
        }

        let sample = |at: usize| MotionSample {
            x: values[at],
            y: values[at + 1],
            pressure: values[at + 2],
            orientation: values[at + 3],
        };
        let history_start = pointer_count * 4;
        let pointers = (0..pointer_count)
            .map(|index| MotionPointer {
                id: pointers[index * 2],
                tool_type: pointers[index * 2 + 1],
                current: sample(index * 4),
                history: (0..history_size)
                    .map(|pos| sample(history_start + (index * history_size + pos) * 4))
                    .collect(),
            })
            .collect();

        Some(MotionEventData {
            event_time,
            action_masked,
            action_index,
            meta_state,
            button_state,
            action_button,
            pointers,
            hscroll,
            vscroll,
        })
    }
}

/// What the helper needs of the top-level it serves.
pub(crate) trait IMotionEventsTopLevel {
    /// The input root the events are for; `None` before the top-level has
    /// one.
    fn motion_input_root(&self) -> Option<Rc<dyn IInputRoot>>;

    /// Delivers a raw input event (`Input?.Invoke`).
    fn dispatch_input(&self, args: Rc<dyn IRawInputEventArgs>);

    /// `RenderScaling` of the top-level.
    fn motion_render_scaling(&self) -> f64;
}

pub(crate) struct AndroidMotionEventsHelper {
    touch_device: Rc<TouchDevice>,
    mouse_device: Rc<MouseDevice>,
    pen_device: Rc<PenDevice>,
    view: Weak<dyn IMotionEventsTopLevel>,
    disposed: Cell<bool>,
}

// As the reference has it, which multiplies where a conversion divides: the twist of a pointer is
// its orientation in radians times this.
const RADIANS_TO_DEGREE: f32 = (180.0 * std::f64::consts::PI) as f32;

impl AndroidMotionEventsHelper {
    pub fn new(view: Weak<dyn IMotionEventsTopLevel>) -> Self {
        Self {
            touch_device: TouchDevice::new(),
            pen_device: PenDevice::new(false),
            mouse_device: MouseDevice::new(),
            view,
            disposed: Cell::new(false),
        }
    }

    /// Translates a motion event. The result is `None` when the event was
    /// not looked at; `call_base` says whether the base class dispatches
    /// the event too.
    pub fn dispatch_motion_event(&self, e: Option<&MotionEventData>, call_base: &mut bool) -> Option<bool> {
        *call_base = true;
        let e = match e {
            Some(e) if !self.disposed.get() => e,
            _ => return None,
        };
        let view = self.view.upgrade()?;

        let event_time = e.event_time as u64;
        let Some(input_root) = view.motion_input_root() else {
            return Some(false); // too early to handle events.
        };

        let action_masked = e.action_masked;
        let mut modifiers = Self::get_modifiers(e.meta_state, e.button_state);

        if action_masked == motion_event_actions::MOVE {
            for pointer in &e.pointers {
                let tool_type = pointer.tool_type;
                let device = self.get_device(tool_type);
                let event_type = if tool_type == motion_event_tool_type::FINGER {
                    RawPointerEventType::TouchUpdate
                } else {
                    RawPointerEventType::Move
                };
                let scaling = view.motion_render_scaling();
                let point = Self::create_point(&pointer.current, scaling);
                modifiers |= Self::get_tool_modifiers(tool_type);

                // ButtonState reports only mouse buttons, but not touch or stylus pointer.
                if tool_type != motion_event_tool_type::MOUSE {
                    modifiers |= RawInputModifiers::LEFT_MOUSE_BUTTON;
                }

                let args = RawTouchEventArgs::with_point(
                    device,
                    event_time,
                    input_root.clone(),
                    event_type,
                    point,
                    modifiers,
                    i64::from(pointer.id),
                );
                let history = pointer.history.clone();
                let create: Box<dyn FnOnce() -> Option<Vec<RawPointerPoint>>> =
                    Box::new(move || Some(history.iter().map(|sample| Self::create_point(sample, scaling)).collect()));
                let intermediate_points: IntermediatePoints = Rc::new(LazyCell::new(create));
                args.set_intermediate_points(Some(intermediate_points));
                view.dispatch_input(Rc::new(args));
            }
        } else {
            let Some(pointer) = usize::try_from(e.action_index).ok().and_then(|index| e.pointers.get(index)) else {
                return Some(true);
            };
            let tool_type = pointer.tool_type;
            let device = self.get_device(tool_type);
            modifiers |= Self::get_tool_modifiers(tool_type);
            let point = Self::create_point(&pointer.current, view.motion_render_scaling());

            if action_masked == motion_event_actions::SCROLL && tool_type == motion_event_tool_type::MOUSE {
                let delta = Vector::new(f64::from(e.hscroll), f64::from(e.vscroll));
                let args = RawMouseWheelEventArgs::new(
                    device,
                    event_time,
                    input_root,
                    point.position,
                    delta,
                    RawInputModifiers::NONE,
                );
                view.dispatch_input(Rc::new(args));
            } else if let Some(event_type) = Self::get_action_type(e.action_button, action_masked, tool_type) {
                let args = RawTouchEventArgs::with_point(
                    device,
                    event_time,
                    input_root,
                    event_type,
                    point,
                    modifiers,
                    i64::from(pointer.id),
                );
                view.dispatch_input(Rc::new(args));
            }
        }

        Some(true)
    }

    pub(crate) fn get_modifiers(meta_state: i32, button_state: i32) -> RawInputModifiers {
        let mut modifiers = RawInputModifiers::NONE;
        if meta_state & meta_key_states::SHIFT_ON != 0 {
            modifiers |= RawInputModifiers::SHIFT;
        }
        if meta_state & meta_key_states::CTRL_ON != 0 {
            modifiers |= RawInputModifiers::CONTROL;
        }
        if meta_state & meta_key_states::ALT_ON != 0 {
            modifiers |= RawInputModifiers::ALT;
        }
        if meta_state & meta_key_states::META_ON != 0 {
            modifiers |= RawInputModifiers::META;
        }
        if button_state & motion_event_button_state::PRIMARY != 0 {
            modifiers |= RawInputModifiers::LEFT_MOUSE_BUTTON;
        }
        if button_state & motion_event_button_state::SECONDARY != 0 {
            modifiers |= RawInputModifiers::RIGHT_MOUSE_BUTTON;
        }
        if button_state & motion_event_button_state::TERTIARY != 0 {
            modifiers |= RawInputModifiers::MIDDLE_MOUSE_BUTTON;
        }
        if button_state & motion_event_button_state::BACK != 0 {
            modifiers |= RawInputModifiers::X_BUTTON_1_MOUSE_BUTTON;
        }
        if button_state & motion_event_button_state::FORWARD != 0 {
            modifiers |= RawInputModifiers::X_BUTTON_2_MOUSE_BUTTON;
        }
        if button_state & motion_event_button_state::STYLUS_PRIMARY != 0 {
            modifiers |= RawInputModifiers::PEN_BARREL_BUTTON;
        }
        modifiers
    }

    /// The raw event of an action; `None` for an action that raises none.
    pub(crate) fn get_action_type(
        action_button: i32,
        action_masked: i32,
        tool_type: i32,
    ) -> Option<RawPointerEventType> {
        use motion_event_actions as actions;
        use motion_event_button_state as buttons;

        let is_touch = tool_type == motion_event_tool_type::FINGER;
        let is_mouse = tool_type == motion_event_tool_type::MOUSE;
        match action_masked {
            // DOWN
            actions::DOWN | actions::POINTER_DOWN if !is_mouse => {
                Some(if is_touch { RawPointerEventType::TouchBegin } else { RawPointerEventType::LeftButtonDown })
            }
            actions::BUTTON_PRESS => Some(match action_button {
                buttons::BACK => RawPointerEventType::XButton1Down,
                buttons::FORWARD => RawPointerEventType::XButton2Down,
                buttons::PRIMARY => RawPointerEventType::LeftButtonDown,
                buttons::SECONDARY => RawPointerEventType::RightButtonDown,
                buttons::STYLUS_PRIMARY => RawPointerEventType::LeftButtonDown,
                buttons::STYLUS_SECONDARY => RawPointerEventType::RightButtonDown,
                buttons::TERTIARY => RawPointerEventType::MiddleButtonDown,
                _ => RawPointerEventType::LeftButtonDown,
            }),
            // UP
            actions::UP | actions::POINTER_UP if !is_mouse => {
                Some(if is_touch { RawPointerEventType::TouchEnd } else { RawPointerEventType::LeftButtonUp })
            }
            actions::BUTTON_RELEASE => Some(match action_button {
                buttons::BACK => RawPointerEventType::XButton1Up,
                buttons::FORWARD => RawPointerEventType::XButton2Up,
                buttons::PRIMARY => RawPointerEventType::LeftButtonUp,
                buttons::SECONDARY => RawPointerEventType::RightButtonUp,
                buttons::STYLUS_PRIMARY => RawPointerEventType::LeftButtonUp,
                buttons::STYLUS_SECONDARY => RawPointerEventType::RightButtonUp,
                buttons::TERTIARY => RawPointerEventType::MiddleButtonUp,
                _ => RawPointerEventType::LeftButtonUp,
            }),
            // MOVE
            actions::OUTSIDE | actions::HOVER_MOVE | actions::MOVE => {
                Some(if is_touch { RawPointerEventType::TouchUpdate } else { RawPointerEventType::Move })
            }
            // CANCEL
            actions::CANCEL => {
                Some(if is_touch { RawPointerEventType::TouchCancel } else { RawPointerEventType::LeaveWindow })
            }
            _ => None,
        }
    }

    fn get_device(&self, tool_type: i32) -> Rc<dyn IInputDevice> {
        match tool_type {
            motion_event_tool_type::MOUSE => self.mouse_device.clone(),
            motion_event_tool_type::STYLUS => self.pen_device.clone(),
            motion_event_tool_type::ERASER => self.pen_device.clone(),
            motion_event_tool_type::FINGER => self.touch_device.clone(),
            _ => self.touch_device.clone(),
        }
    }

    pub(crate) fn create_point(sample: &MotionSample, render_scaling: f64) -> RawPointerPoint {
        let mut point = RawPointerPoint::new();
        point.position = Point::new(f64::from(sample.x) / render_scaling, f64::from(sample.y) / render_scaling);
        // android pressure can depend on the device, can be mixed up with "GetSize", may be larger than 1.0f on some devices
        point.pressure = sample.pressure.min(1.0);
        point.twist = sample.orientation * RADIANS_TO_DEGREE;
        point
    }

    pub(crate) fn get_tool_modifiers(tool_type: i32) -> RawInputModifiers {
        // Android "Eraser" indicates Inverted pen OR actual Eraser. So we have to go both here.
        if tool_type == motion_event_tool_type::ERASER {
            RawInputModifiers::PEN_INVERTED | RawInputModifiers::PEN_ERASER
        } else {
            RawInputModifiers::NONE
        }
    }

    pub fn dispose(&self) {
        self.disposed.set(true);
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests of the helper.
    use super::*;
    use ferroui_base::input::raw::RawPointerEventArgs;
    use ferroui_base::input::{FocusManager, InputElement};
    use ferroui_base::Ref;
    use std::cell::RefCell;

    struct TestRoot {
        element: Ref<InputElement>,
    }

    impl IInputRoot for TestRoot {
        fn focus_manager(&self) -> Option<Rc<FocusManager>> {
            None
        }

        fn pointer_over_element(&self) -> Option<Ref<InputElement>> {
            None
        }

        fn set_pointer_over_element(&self, _value: Option<Ref<InputElement>>) {}

        fn cursor_element(&self) -> Option<Ref<InputElement>> {
            None
        }

        fn set_cursor_element(&self, _value: Option<Ref<InputElement>>) {}

        fn root_element(&self) -> Ref<InputElement> {
            self.element.clone()
        }

        fn focus_root(&self) -> Ref<InputElement> {
            self.element.clone()
        }

        fn pointer_over_invalidated(&self) {}
    }

    /// What the helper dispatched: the kind of event, the id of the pointer, the point,
    /// the modifiers, the time, the intermediate points and the wheel delta.
    #[derive(Debug, PartialEq)]
    struct Seen {
        type_: RawPointerEventType,
        id: i64,
        position: Point,
        pressure: f32,
        modifiers: RawInputModifiers,
        timestamp: u64,
        intermediate: Option<Vec<Point>>,
        delta: Option<Vector>,
    }

    struct TopLevel {
        root: RefCell<Option<Rc<dyn IInputRoot>>>,
        scaling: f64,
        seen: RefCell<Vec<Seen>>,
    }

    impl IMotionEventsTopLevel for TopLevel {
        fn motion_input_root(&self) -> Option<Rc<dyn IInputRoot>> {
            self.root.borrow().clone()
        }

        fn dispatch_input(&self, args: Rc<dyn IRawInputEventArgs>) {
            let pointer = args.downcast_ref::<RawPointerEventArgs>().expect("a pointer event");
            let intermediate = pointer
                .intermediate_points()
                .and_then(|points| (**points).clone())
                .map(|points| points.iter().map(|point| point.position).collect());
            self.seen.borrow_mut().push(Seen {
                type_: pointer.type_(),
                id: pointer.raw_pointer_id(),
                position: pointer.position(),
                pressure: pointer.point().pressure,
                modifiers: pointer.input_modifiers(),
                timestamp: pointer.timestamp(),
                intermediate,
                delta: args.downcast_ref::<RawMouseWheelEventArgs>().map(RawMouseWheelEventArgs::delta),
            });
        }

        fn motion_render_scaling(&self) -> f64 {
            self.scaling
        }
    }

    fn fixture(with_root: bool) -> (Rc<TopLevel>, AndroidMotionEventsHelper) {
        let root: Option<Rc<dyn IInputRoot>> =
            with_root.then(|| Rc::new(TestRoot { element: InputElement::new() }) as Rc<dyn IInputRoot>);
        let top_level = Rc::new(TopLevel { root: RefCell::new(root), scaling: 2.0, seen: RefCell::new(Vec::new()) });
        let as_trait: Rc<dyn IMotionEventsTopLevel> = top_level.clone();
        (top_level, AndroidMotionEventsHelper::new(Rc::downgrade(&as_trait)))
    }

    fn sample(x: f32, y: f32) -> MotionSample {
        MotionSample { x, y, pressure: 0.75, orientation: 0.0 }
    }

    fn event(action_masked: i32, action_index: i32, pointers: Vec<MotionPointer>) -> MotionEventData {
        MotionEventData {
            event_time: 1234,
            action_masked,
            action_index,
            meta_state: 0,
            button_state: 0,
            action_button: 0,
            pointers,
            hscroll: 0.0,
            vscroll: 0.0,
        }
    }

    fn finger(id: i32, x: f32, y: f32) -> MotionPointer {
        MotionPointer { id, tool_type: motion_event_tool_type::FINGER, current: sample(x, y), history: Vec::new() }
    }

    #[test]
    fn the_arrays_of_the_view_are_read_per_pointer_with_their_history() {
        let values = [
            // Two pointers now.
            10.0, 20.0, 0.5, 0.1, 30.0, 40.0, 0.6, 0.2, //
            // The history of the first, two positions; then of the second.
            1.0, 2.0, 0.3, 0.0, 3.0, 4.0, 0.4, 0.0, //
            5.0, 6.0, 0.3, 0.0, 7.0, 8.0, 0.4, 0.0,
        ];
        let event = MotionEventData::from_arrays(99, 2, 0, 1, 2, 0, 2, &[7, 1, 9, 2], &values, 0.5, -1.0).unwrap();

        assert_eq!(event.event_time, 99);
        assert_eq!(event.pointers.len(), 2);
        assert_eq!((event.pointers[0].id, event.pointers[0].tool_type), (7, 1));
        assert_eq!(event.pointers[0].current, MotionSample { x: 10.0, y: 20.0, pressure: 0.5, orientation: 0.1 });
        assert_eq!(event.pointers[0].history.iter().map(|s| (s.x, s.y)).collect::<Vec<_>>(), [(1.0, 2.0), (3.0, 4.0)]);
        assert_eq!((event.pointers[1].id, event.pointers[1].tool_type), (9, 2));
        assert_eq!(event.pointers[1].history.iter().map(|s| (s.x, s.y)).collect::<Vec<_>>(), [(5.0, 6.0), (7.0, 8.0)]);
        assert_eq!((event.hscroll, event.vscroll), (0.5, -1.0));

        // Arrays of another shape are no event.
        assert!(MotionEventData::from_arrays(0, 0, 0, 0, 0, 0, 0, &[1], &[], 0.0, 0.0).is_none());
        assert!(MotionEventData::from_arrays(0, 0, 0, 0, 0, 0, 1, &[1, 1], &[0.0; 4], 0.0, 0.0).is_none());
        assert!(MotionEventData::from_arrays(0, 0, 0, 0, 0, 0, -1, &[], &[], 0.0, 0.0).is_none());
    }

    #[test]
    fn an_event_is_not_looked_at_without_an_event_or_after_dispose() {
        let (top_level, helper) = fixture(true);
        let mut call_base = false;

        assert_eq!(helper.dispatch_motion_event(None, &mut call_base), None);
        assert!(call_base);

        helper.dispose();
        let down = event(motion_event_actions::DOWN, 0, vec![finger(0, 10.0, 10.0)]);
        assert_eq!(helper.dispatch_motion_event(Some(&down), &mut call_base), None);
        assert!(top_level.seen.borrow().is_empty());
    }

    #[test]
    fn an_event_before_the_input_root_exists_is_not_handled() {
        let (top_level, helper) = fixture(false);
        let mut call_base = false;
        let down = event(motion_event_actions::DOWN, 0, vec![finger(0, 10.0, 10.0)]);

        assert_eq!(helper.dispatch_motion_event(Some(&down), &mut call_base), Some(false));
        assert!(call_base);
        assert!(top_level.seen.borrow().is_empty());
    }

    #[test]
    fn a_finger_goes_down_moves_and_up_as_a_touch_with_its_pointer_id() {
        let (top_level, helper) = fixture(true);
        let mut call_base = false;

        let down = event(motion_event_actions::DOWN, 0, vec![finger(5, 100.0, 200.0)]);
        assert_eq!(helper.dispatch_motion_event(Some(&down), &mut call_base), Some(true));

        let mut moving = finger(5, 120.0, 260.0);
        moving.history = vec![sample(104.0, 210.0), sample(110.0, 240.0)];
        let moved = event(motion_event_actions::MOVE, 0, vec![moving]);
        assert_eq!(helper.dispatch_motion_event(Some(&moved), &mut call_base), Some(true));

        let up = event(motion_event_actions::UP, 0, vec![finger(5, 120.0, 260.0)]);
        assert_eq!(helper.dispatch_motion_event(Some(&up), &mut call_base), Some(true));

        let seen = top_level.seen.borrow();
        assert_eq!(seen.len(), 3);
        // Positions are divided by the scaling of the top-level.
        assert_eq!(
            (seen[0].type_, seen[0].id, seen[0].position),
            (RawPointerEventType::TouchBegin, 5, Point::new(50.0, 100.0))
        );
        assert_eq!(seen[0].modifiers, RawInputModifiers::NONE);
        assert_eq!(seen[0].timestamp, 1234);
        assert_eq!(seen[0].pressure, 0.75);
        assert_eq!(
            (seen[1].type_, seen[1].id, seen[1].position),
            (RawPointerEventType::TouchUpdate, 5, Point::new(60.0, 130.0))
        );
        // A move of a finger counts as the left button held.
        assert_eq!(seen[1].modifiers, RawInputModifiers::LEFT_MOUSE_BUTTON);
        assert_eq!(seen[1].intermediate, Some(vec![Point::new(52.0, 105.0), Point::new(55.0, 120.0)]));
        assert_eq!((seen[2].type_, seen[2].id), (RawPointerEventType::TouchEnd, 5));
    }

    #[test]
    fn a_second_finger_is_a_second_pointer() {
        let (top_level, helper) = fixture(true);
        let mut call_base = false;

        let first = event(motion_event_actions::DOWN, 0, vec![finger(0, 10.0, 10.0)]);
        helper.dispatch_motion_event(Some(&first), &mut call_base);
        let second = event(motion_event_actions::POINTER_DOWN, 1, vec![finger(0, 10.0, 10.0), finger(1, 300.0, 400.0)]);
        helper.dispatch_motion_event(Some(&second), &mut call_base);
        // A move reports every pointer.
        let moved = event(motion_event_actions::MOVE, 0, vec![finger(0, 12.0, 10.0), finger(1, 310.0, 400.0)]);
        helper.dispatch_motion_event(Some(&moved), &mut call_base);
        let up = event(motion_event_actions::POINTER_UP, 1, vec![finger(0, 12.0, 10.0), finger(1, 310.0, 400.0)]);
        helper.dispatch_motion_event(Some(&up), &mut call_base);

        let seen = top_level.seen.borrow();
        let summary: Vec<_> = seen.iter().map(|seen| (seen.type_, seen.id)).collect();
        assert_eq!(
            summary,
            [
                (RawPointerEventType::TouchBegin, 0),
                (RawPointerEventType::TouchBegin, 1),
                (RawPointerEventType::TouchUpdate, 0),
                (RawPointerEventType::TouchUpdate, 1),
                (RawPointerEventType::TouchEnd, 1),
            ]
        );
        assert_eq!(seen[1].position, Point::new(150.0, 200.0));
    }

    #[test]
    fn the_wheel_of_a_mouse_scrolls_by_the_two_scroll_axes() {
        let (top_level, helper) = fixture(true);
        let mut call_base = false;
        let mouse = MotionPointer {
            id: 0,
            tool_type: motion_event_tool_type::MOUSE,
            current: sample(40.0, 60.0),
            history: Vec::new(),
        };
        let mut scroll = event(motion_event_actions::SCROLL, 0, vec![mouse]);
        scroll.hscroll = 0.5;
        scroll.vscroll = -1.0;
        scroll.meta_state = meta_key_states::SHIFT_ON;

        assert_eq!(helper.dispatch_motion_event(Some(&scroll), &mut call_base), Some(true));

        let seen = top_level.seen.borrow();
        assert_eq!(seen.len(), 1);
        assert_eq!(seen[0].type_, RawPointerEventType::Wheel);
        assert_eq!(seen[0].position, Point::new(20.0, 30.0));
        assert_eq!(seen[0].delta, Some(Vector::new(0.5, -1.0)));
        // As the reference, the wheel carries no modifiers.
        assert_eq!(seen[0].modifiers, RawInputModifiers::NONE);
    }

    #[test]
    fn the_actions_of_each_tool_are_the_raw_events_of_the_reference() {
        use motion_event_actions as actions;
        use motion_event_button_state as buttons;
        use motion_event_tool_type::{ERASER, FINGER, MOUSE, STYLUS};
        let action = AndroidMotionEventsHelper::get_action_type;

        // A mouse presses and releases through its buttons, not through down and up.
        assert_eq!(action(0, actions::DOWN, MOUSE), None);
        assert_eq!(action(0, actions::UP, MOUSE), None);
        assert_eq!(action(buttons::PRIMARY, actions::BUTTON_PRESS, MOUSE), Some(RawPointerEventType::LeftButtonDown));
        assert_eq!(
            action(buttons::SECONDARY, actions::BUTTON_PRESS, MOUSE),
            Some(RawPointerEventType::RightButtonDown)
        );
        assert_eq!(
            action(buttons::TERTIARY, actions::BUTTON_RELEASE, MOUSE),
            Some(RawPointerEventType::MiddleButtonUp)
        );
        assert_eq!(action(buttons::BACK, actions::BUTTON_PRESS, MOUSE), Some(RawPointerEventType::XButton1Down));
        assert_eq!(action(buttons::FORWARD, actions::BUTTON_RELEASE, MOUSE), Some(RawPointerEventType::XButton2Up));
        assert_eq!(action(0, actions::HOVER_MOVE, MOUSE), Some(RawPointerEventType::Move));
        assert_eq!(action(0, actions::CANCEL, MOUSE), Some(RawPointerEventType::LeaveWindow));

        // A pen is a left button.
        assert_eq!(action(0, actions::DOWN, STYLUS), Some(RawPointerEventType::LeftButtonDown));
        assert_eq!(action(0, actions::UP, ERASER), Some(RawPointerEventType::LeftButtonUp));
        assert_eq!(
            action(buttons::STYLUS_SECONDARY, actions::BUTTON_PRESS, STYLUS),
            Some(RawPointerEventType::RightButtonDown)
        );
        assert_eq!(action(0, actions::MOVE, STYLUS), Some(RawPointerEventType::Move));

        // A finger is a touch.
        assert_eq!(action(0, actions::POINTER_DOWN, FINGER), Some(RawPointerEventType::TouchBegin));
        assert_eq!(action(0, actions::POINTER_UP, FINGER), Some(RawPointerEventType::TouchEnd));
        assert_eq!(action(0, actions::OUTSIDE, FINGER), Some(RawPointerEventType::TouchUpdate));
        assert_eq!(action(0, actions::CANCEL, FINGER), Some(RawPointerEventType::TouchCancel));

        // Hover enter and exit, and a scroll of another tool than the mouse, raise nothing.
        assert_eq!(action(0, actions::HOVER_ENTER, MOUSE), None);
        assert_eq!(action(0, actions::HOVER_EXIT, FINGER), None);
        assert_eq!(action(0, actions::SCROLL, FINGER), None);
    }

    #[test]
    fn the_meta_state_and_the_buttons_are_the_modifiers() {
        use meta_key_states::{ALT_ON, CTRL_ON, META_ON, SHIFT_ON};
        use motion_event_button_state as buttons;
        let modifiers = AndroidMotionEventsHelper::get_modifiers;

        assert_eq!(modifiers(0, 0), RawInputModifiers::NONE);
        assert_eq!(
            modifiers(SHIFT_ON | CTRL_ON | ALT_ON | META_ON, 0),
            RawInputModifiers::SHIFT | RawInputModifiers::CONTROL | RawInputModifiers::ALT | RawInputModifiers::META
        );
        assert_eq!(
            modifiers(0, buttons::PRIMARY | buttons::SECONDARY | buttons::TERTIARY),
            RawInputModifiers::LEFT_MOUSE_BUTTON
                | RawInputModifiers::RIGHT_MOUSE_BUTTON
                | RawInputModifiers::MIDDLE_MOUSE_BUTTON
        );
        assert_eq!(
            modifiers(0, buttons::BACK | buttons::FORWARD | buttons::STYLUS_PRIMARY),
            RawInputModifiers::X_BUTTON_1_MOUSE_BUTTON
                | RawInputModifiers::X_BUTTON_2_MOUSE_BUTTON
                | RawInputModifiers::PEN_BARREL_BUTTON
        );
        assert_eq!(
            AndroidMotionEventsHelper::get_tool_modifiers(motion_event_tool_type::ERASER),
            RawInputModifiers::PEN_INVERTED | RawInputModifiers::PEN_ERASER
        );
        assert_eq!(
            AndroidMotionEventsHelper::get_tool_modifiers(motion_event_tool_type::STYLUS),
            RawInputModifiers::NONE
        );
    }

    #[test]
    fn a_point_has_its_pressure_capped_and_the_twist_of_the_reference() {
        let point = AndroidMotionEventsHelper::create_point(
            &MotionSample { x: 30.0, y: 60.0, pressure: 1.5, orientation: 0.5 },
            3.0,
        );

        assert_eq!(point.position, Point::new(10.0, 20.0));
        assert_eq!(point.pressure, 1.0);
        // The orientation in radians times 180 * pi, as the reference computes it.
        assert_eq!(point.twist, 0.5 * (180.0 * std::f64::consts::PI) as f32);
    }
}
