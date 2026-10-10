//! The input of a view: touches of fingers, of a pencil and of an
//! indirect pointer become raw pointer events of the top-level.
//!
//! Stage 2 of `docs/porting/ios-platform.md` adds the rest of the
//! reference's handler: key presses, the swipe gestures of a remote, and
//! the scroll wheel with its inertia.

use ferroui_base::input::raw::RawPointerEventType;
use ferroui_base::input::RawInputModifiers;

/// The device a touch belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TouchDeviceKind {
    /// A finger on the screen.
    Touch,
    /// A pencil.
    Pen,
    /// A pointer that is not on the screen (a mouse or a trackpad).
    Mouse,
}

/// `UITouchTypeDirect` and the other values of `UITouchType`.
pub mod touch_type {
    pub const DIRECT: isize = 0;
    pub const INDIRECT: isize = 1;
    pub const STYLUS: isize = 2;
    pub const INDIRECT_POINTER: isize = 3;
}

/// `UITouchPhaseBegan` and the other values of `UITouchPhase`.
pub mod touch_phase {
    pub const BEGAN: isize = 0;
    pub const MOVED: isize = 1;
    pub const STATIONARY: isize = 2;
    pub const ENDED: isize = 3;
    pub const CANCELLED: isize = 4;
}

/// The bits of `UIKeyModifierFlags`.
pub mod key_modifier_flags {
    pub const SHIFT: isize = 1 << 17;
    pub const CONTROL: isize = 1 << 18;
    pub const ALTERNATE: isize = 1 << 19;
    pub const COMMAND: isize = 1 << 20;
}

/// The device of a touch of the given `UITouchType`; none for an indirect
/// touch (the trackpad of a remote), which is handled with gestures.
pub fn device_kind(touch_type: isize) -> Option<TouchDeviceKind> {
    match touch_type {
        touch_type::INDIRECT => None,
        touch_type::STYLUS => Some(TouchDeviceKind::Pen),
        touch_type::INDIRECT_POINTER => Some(TouchDeviceKind::Mouse),
        _ => Some(TouchDeviceKind::Touch),
    }
}

/// The raw event of a touch of a device in the given `UITouchPhase`.
/// `is_right_click` tells whether the secondary button is in the button
/// mask of the event.
pub fn pointer_event_type(device: TouchDeviceKind, phase: isize, is_right_click: bool) -> RawPointerEventType {
    match (device, phase) {
        (TouchDeviceKind::Touch, touch_phase::BEGAN) => RawPointerEventType::TouchBegin,
        (TouchDeviceKind::Touch, touch_phase::ENDED) => RawPointerEventType::TouchEnd,
        (TouchDeviceKind::Touch, touch_phase::CANCELLED) => RawPointerEventType::TouchCancel,
        (TouchDeviceKind::Touch, _) => RawPointerEventType::TouchUpdate,

        (_, touch_phase::BEGAN) if is_right_click => RawPointerEventType::RightButtonDown,
        (_, touch_phase::BEGAN) => RawPointerEventType::LeftButtonDown,
        (_, touch_phase::ENDED) if is_right_click => RawPointerEventType::RightButtonUp,
        (_, touch_phase::ENDED) => RawPointerEventType::LeftButtonUp,
        (_, touch_phase::CANCELLED) => RawPointerEventType::LeaveWindow,
        (_, _) => RawPointerEventType::Move,
    }
}

/// The pressure of a touch. On iOS "1.0 represents the force of an average
/// touch", where the framework expects 0.5 for "average". A touch whose
/// maximum possible force is 0 has no force and gets the average.
pub fn pressure(force: f64, maximum_possible_force: f64) -> f32 {
    if maximum_possible_force == 0.0 {
        0.5
    } else {
        force as f32 / 2.0
    }
}

/// The modifiers of `UIKeyModifierFlags`.
pub fn convert_modifier_keys(ui_modifier: Option<isize>) -> RawInputModifiers {
    let mut modifier = RawInputModifiers::NONE;
    if let Some(flags) = ui_modifier {
        if flags & key_modifier_flags::SHIFT != 0 {
            modifier |= RawInputModifiers::SHIFT;
        }
        if flags & key_modifier_flags::ALTERNATE != 0 {
            modifier |= RawInputModifiers::ALT;
        }
        if flags & key_modifier_flags::CONTROL != 0 {
            modifier |= RawInputModifiers::CONTROL;
        }
        if flags & key_modifier_flags::COMMAND != 0 {
            modifier |= RawInputModifiers::META;
        }
    }

    modifier
}

/// The timestamp of an event (seconds since the system started) in
/// milliseconds; 0 without an event.
pub fn ts(event_timestamp: Option<f64>) -> u64 {
    match event_timestamp {
        None => 0,
        Some(timestamp) => (timestamp * 1000.0) as u64,
    }
}

/// Whether a touch in the given phase is over.
pub fn is_final_phase(phase: isize) -> bool {
    phase == touch_phase::CANCELLED || phase == touch_phase::ENDED
}

#[cfg(target_os = "ios")]
pub(crate) use uikit::InputHandler;

#[cfg(target_os = "ios")]
mod uikit {
    use super::*;
    use crate::extensions::to_point;
    use crate::ferro_view::{FerroView, TopLevelImpl};
    use ferroui_base::input::raw::{IRawInputEventArgs, RawPointerPoint, RawTouchEventArgs};
    use ferroui_base::input::{IInputDevice, IntermediatePoints, MouseDevice, PenDevice, TouchDevice};
    use ferroui_controls::platform::ITopLevelImpl;
    use objc2::rc::{Retained, Weak};
    use objc2::Message;
    use objc2_foundation::NSSet;
    use objc2_ui_kit::{UIEvent, UIEventButtonMask, UITouch};
    use std::cell::{Cell, LazyCell, RefCell};
    use std::collections::HashMap;
    use std::rc::Rc;

    thread_local! {
        /// The identifier of the next touch, across the views of the
        /// application.
        static NEXT_TOUCH_POINT_ID: Cell<i64> = const { Cell::new(1) };
    }

    const _: () = {
        use objc2_ui_kit::{UIKeyModifierFlags, UITouchPhase, UITouchType};
        // The constants of this file are those of UIKit.
        assert!(touch_type::DIRECT == UITouchType::Direct.0);
        assert!(touch_type::INDIRECT == UITouchType::Indirect.0);
        assert!(touch_type::STYLUS == UITouchType::Pencil.0);
        assert!(touch_type::INDIRECT_POINTER == UITouchType::IndirectPointer.0);
        assert!(touch_phase::BEGAN == UITouchPhase::Began.0);
        assert!(touch_phase::MOVED == UITouchPhase::Moved.0);
        assert!(touch_phase::STATIONARY == UITouchPhase::Stationary.0);
        assert!(touch_phase::ENDED == UITouchPhase::Ended.0);
        assert!(touch_phase::CANCELLED == UITouchPhase::Cancelled.0);
        assert!(key_modifier_flags::SHIFT == UIKeyModifierFlags::Shift.0);
        assert!(key_modifier_flags::CONTROL == UIKeyModifierFlags::Control.0);
        assert!(key_modifier_flags::ALTERNATE == UIKeyModifierFlags::Alternate.0);
        assert!(key_modifier_flags::COMMAND == UIKeyModifierFlags::Command.0);
    };

    /// Translates the touches of a view to the input of its top-level.
    pub(crate) struct InputHandler {
        view: Weak<FerroView>,
        tl: Rc<TopLevelImpl>,
        touch_device: Rc<TouchDevice>,
        mouse_device: Rc<MouseDevice>,
        pen_device: Rc<PenDevice>,
        /// The identifiers of the touches that are down, by the address of
        /// the touch object, which UIKit keeps for the life of a touch.
        known_touches: RefCell<HashMap<usize, i64>>,
    }

    impl InputHandler {
        pub(crate) fn new(view: Weak<FerroView>, tl: Rc<TopLevelImpl>) -> Self {
            Self {
                view,
                tl,
                touch_device: TouchDevice::new(),
                mouse_device: MouseDevice::new(),
                pen_device: PenDevice::new(true),
                known_touches: RefCell::new(HashMap::new()),
            }
        }

        /// Handles the touches of a `touchesBegan:withEvent:` and of the
        /// three methods that follow it.
        pub(crate) fn handle(&self, touches: &NSSet<UITouch>, evt: Option<&UIEvent>) {
            let Some(view) = self.view.load() else {
                return;
            };

            for t in touches.iter() {
                let Some(kind) = device_kind(t.r#type().0) else {
                    // Ignore indirect input, like the trackpad of a remote
                    // controller, which is handled with gestures.
                    continue;
                };

                let key = Retained::as_ptr(&t) as usize;
                let id = *self.known_touches.borrow_mut().entry(key).or_insert_with(|| {
                    NEXT_TOUCH_POINT_ID.with(|next| {
                        let id = next.get();
                        next.set(id + 1);
                        id
                    })
                });

                let device: Rc<dyn IInputDevice> = match kind {
                    TouchDeviceKind::Pen => self.pen_device.clone(),
                    TouchDeviceKind::Mouse => self.mouse_device.clone(),
                    TouchDeviceKind::Touch => self.touch_device.clone(),
                };

                let modifiers = convert_modifier_keys(evt.map(|evt| evt.modifierFlags().0));
                let is_right_click = evt.is_some_and(|evt| evt.buttonMask().contains(UIEventButtonMask::Secondary));
                let phase = t.phase().0;

                let to_pointer_point = {
                    let view = view.clone();
                    // The pressure of every point is the one of the touch
                    // that is handled, as in the reference.
                    let pressure = pressure(t.force(), t.maximumPossibleForce());
                    move |touch: &UITouch| {
                        let mut point = RawPointerPoint::new();
                        point.position = to_point(touch.locationInView(Some(&view)));
                        point.pressure = pressure;
                        point
                    }
                };

                let ev = RawTouchEventArgs::with_point(
                    device,
                    ts(evt.map(|evt| evt.timestamp())),
                    view.input_root(),
                    pointer_event_type(kind, phase, is_right_click),
                    to_pointer_point(&t),
                    modifiers,
                    id,
                );

                if let Some(this_event) = evt {
                    let this_event = this_event.retain();
                    let touch = t.clone();
                    let intermediate_points: IntermediatePoints = Rc::new(LazyCell::new(Box::new(move || {
                        let coalesced = this_event.coalescedTouchesForTouch(&touch);
                        let coalesced: Vec<Retained<UITouch>> =
                            coalesced.map(|coalesced| coalesced.iter().collect()).unwrap_or_default();
                        // Skip the last one, as it is the point that was
                        // processed already.
                        let count = coalesced.len().saturating_sub(1);
                        Some(coalesced[..count].iter().map(|touch| to_pointer_point(touch)).collect())
                    })));
                    ev.set_intermediate_points(Some(intermediate_points));
                }

                let args: Rc<dyn IRawInputEventArgs> = Rc::new(ev);
                if let Some(input) = self.tl.input() {
                    input(args);
                }

                if is_final_phase(phase) {
                    self.known_touches.borrow_mut().remove(&key);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests of this file.
    use super::*;

    #[test]
    fn fingers_pencils_and_pointers_have_their_devices() {
        assert_eq!(Some(TouchDeviceKind::Touch), device_kind(touch_type::DIRECT));
        assert_eq!(None, device_kind(touch_type::INDIRECT));
        assert_eq!(Some(TouchDeviceKind::Pen), device_kind(touch_type::STYLUS));
        assert_eq!(Some(TouchDeviceKind::Mouse), device_kind(touch_type::INDIRECT_POINTER));
        // A type this version does not know is a touch.
        assert_eq!(Some(TouchDeviceKind::Touch), device_kind(17));
    }

    #[test]
    fn the_phases_of_a_finger_are_the_touch_events() {
        let event = |phase| pointer_event_type(TouchDeviceKind::Touch, phase, false);
        assert_eq!(RawPointerEventType::TouchBegin, event(touch_phase::BEGAN));
        assert_eq!(RawPointerEventType::TouchUpdate, event(touch_phase::MOVED));
        assert_eq!(RawPointerEventType::TouchUpdate, event(touch_phase::STATIONARY));
        assert_eq!(RawPointerEventType::TouchEnd, event(touch_phase::ENDED));
        assert_eq!(RawPointerEventType::TouchCancel, event(touch_phase::CANCELLED));
        // The secondary button means nothing for a finger.
        assert_eq!(RawPointerEventType::TouchBegin, pointer_event_type(TouchDeviceKind::Touch, touch_phase::BEGAN, true));
    }

    #[test]
    fn the_phases_of_a_pencil_and_of_a_pointer_are_button_events() {
        for device in [TouchDeviceKind::Pen, TouchDeviceKind::Mouse] {
            let event = |phase, right| pointer_event_type(device, phase, right);
            assert_eq!(RawPointerEventType::LeftButtonDown, event(touch_phase::BEGAN, false));
            assert_eq!(RawPointerEventType::RightButtonDown, event(touch_phase::BEGAN, true));
            assert_eq!(RawPointerEventType::Move, event(touch_phase::MOVED, false));
            assert_eq!(RawPointerEventType::Move, event(touch_phase::STATIONARY, true));
            assert_eq!(RawPointerEventType::LeftButtonUp, event(touch_phase::ENDED, false));
            assert_eq!(RawPointerEventType::RightButtonUp, event(touch_phase::ENDED, true));
            assert_eq!(RawPointerEventType::LeaveWindow, event(touch_phase::CANCELLED, false));
        }
    }

    #[test]
    fn the_average_force_is_half_the_pressure_range() {
        assert_eq!(0.5, pressure(1.0, 6.67));
        assert_eq!(1.0, pressure(2.0, 6.67));
        assert_eq!(0.0, pressure(0.0, 6.67));
        // A screen that measures no force.
        assert_eq!(0.5, pressure(0.0, 0.0));
    }

    #[test]
    fn the_modifier_flags_are_the_raw_modifiers() {
        assert_eq!(RawInputModifiers::NONE, convert_modifier_keys(None));
        assert_eq!(RawInputModifiers::NONE, convert_modifier_keys(Some(0)));
        assert_eq!(RawInputModifiers::SHIFT, convert_modifier_keys(Some(key_modifier_flags::SHIFT)));
        assert_eq!(RawInputModifiers::ALT, convert_modifier_keys(Some(key_modifier_flags::ALTERNATE)));
        assert_eq!(RawInputModifiers::CONTROL, convert_modifier_keys(Some(key_modifier_flags::CONTROL)));
        assert_eq!(RawInputModifiers::META, convert_modifier_keys(Some(key_modifier_flags::COMMAND)));
        assert_eq!(
            RawInputModifiers::SHIFT | RawInputModifiers::META,
            convert_modifier_keys(Some(key_modifier_flags::SHIFT | key_modifier_flags::COMMAND | 1 << 16))
        );
    }

    #[test]
    fn the_timestamp_is_in_milliseconds() {
        assert_eq!(0, ts(None));
        assert_eq!(1500, ts(Some(1.5)));
        assert_eq!(86_400_123, ts(Some(86_400.1234)));
    }

    #[test]
    fn a_touch_is_over_when_it_ended_or_was_cancelled() {
        assert!(is_final_phase(touch_phase::ENDED));
        assert!(is_final_phase(touch_phase::CANCELLED));
        assert!(!is_final_phase(touch_phase::BEGAN));
        assert!(!is_final_phase(touch_phase::MOVED));
        assert!(!is_final_phase(touch_phase::STATIONARY));
    }
}
