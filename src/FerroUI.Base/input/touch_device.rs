use super::raw::{IRawInputEventArgs, RawPointerEventArgs, RawPointerEventType};
use super::{
    CaptureSource, IInputDevice, IPointer, IPointerDevice, InputElement, MouseButton, Pointer, PointerEventArgs,
    PointerPointProperties, PointerPressedEventArgs, PointerReleasedEventArgs, PointerType, RawInputModifiers,
};
use crate::{Rect, Size, Thickness};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

/// Handles raw touch events.
///
/// This class is supposed to be used on per-toplevel basis; don't use a
/// shared one.
pub struct TouchDevice {
    pointers: RefCell<HashMap<i64, Rc<Pointer>>>,
    disposed: Cell<bool>,
    click_count: Cell<i32>,
    last_click_rect: Cell<Rect>,
    last_click_time: Cell<u64>,
}

impl TouchDevice {
    /// Creates a touch device.
    pub fn new() -> Rc<TouchDevice> {
        Rc::new(TouchDevice {
            pointers: RefCell::new(HashMap::new()),
            disposed: Cell::new(false),
            click_count: Cell::new(0),
            last_click_rect: Cell::new(Rect::default()),
            last_click_time: Cell::new(0),
        })
    }

    fn get_modifiers(modifiers: RawInputModifiers, is_left_button_down: bool) -> RawInputModifiers {
        let mut rv = modifiers & RawInputModifiers::KEYBOARD_MASK;
        if is_left_button_down {
            rv |= RawInputModifiers::LEFT_MOUSE_BUTTON;
        }
        rv
    }

    /// Releases the device and its pointers.
    pub fn dispose(&self) {
        if self.disposed.get() {
            return;
        }

        let values: Vec<_> = self.pointers.borrow_mut().drain().map(|(_, pointer)| pointer).collect();
        self.disposed.set(true);

        for p in values {
            p.dispose();
        }
    }

    /// Notifies the device that the capture of its pointers was lost in
    /// the platform.
    pub fn platform_capture_lost(&self) {
        let pointers: Vec<_> = self.pointers.borrow().values().cloned().collect();
        for pointer in pointers {
            pointer.platform_capture_lost();
        }
    }
}

impl IInputDevice for TouchDevice {
    fn process_raw_event(&self, ev: &dyn IRawInputEventArgs) {
        if ev.handled() || self.disposed.get() {
            return;
        }

        let args = ev.downcast_ref::<RawPointerEventArgs>().expect("a touch device processes raw pointer events");
        let hit = args.input_hit_test_result().1;
        let existing = self.pointers.borrow().get(&args.raw_pointer_id()).cloned();

        let pointer = match existing {
            Some(pointer) => pointer,
            None => {
                if args.type_() == RawPointerEventType::TouchEnd {
                    return;
                }

                let is_primary = self.pointers.borrow().is_empty();
                let pointer = Pointer::new(Pointer::get_next_free_id(), PointerType::Touch, is_primary);
                self.pointers.borrow_mut().insert(args.raw_pointer_id(), pointer.clone());
                pointer.capture_with_source(hit.as_ref(), CaptureSource::Implicit);
                pointer
            }
        };

        let root_element = args.root().root_element();
        let mut target = pointer.captured().or(hit).unwrap_or_else(|| root_element.clone());
        let gesture_recognizer = pointer.captured_gesture_recognizer();
        let gesture_target = gesture_recognizer.as_ref().and_then(|recognizer| recognizer.target());
        let update_kind = args.type_().to_update_kind();
        let key_modifier = args.input_modifiers().to_key_modifiers();

        if args.type_() == RawPointerEventType::TouchBegin {
            if self.pointers.borrow().len() > 1 {
                self.click_count.set(1);
                self.last_click_time.set(0);
                self.last_click_rect.set(Rect::default());
            } else if let Some(settings) = target.get_platform_settings() {
                let double_click_time = settings.get_double_tap_time(PointerType::Touch).as_secs_f64() * 1000.0;
                let double_click_size = settings.get_double_tap_size(PointerType::Touch);

                if !self.last_click_rect.get().contains(args.position())
                    || ev.timestamp().wrapping_sub(self.last_click_time.get()) as f64 > double_click_time
                {
                    self.click_count.set(0);
                }

                self.click_count.set(self.click_count.get() + 1);
                self.last_click_time.set(ev.timestamp());
                self.last_click_rect.set(Rect::from_position_size(args.position(), Size::default()).inflate_thickness(
                    Thickness::symmetric(double_click_size.width / 2.0, double_click_size.height / 2.0),
                ));
            }

            target.raise_event(
                &PointerPressedEventArgs::new(
                    &target,
                    pointer.clone(),
                    &root_element,
                    args.position(),
                    ev.timestamp(),
                    PointerPointProperties::from_raw_point(
                        Self::get_modifiers(args.input_modifiers(), true),
                        update_kind,
                        args.point(),
                    ),
                    key_modifier,
                    self.click_count.get(),
                )
                .with_platform_input_event_cookie(args.platform_input_event_cookie()),
            );
        }

        if args.type_() == RawPointerEventType::TouchEnd {
            self.pointers.borrow_mut().remove(&args.raw_pointer_id());

            target = gesture_target.clone().unwrap_or(target);

            let e = PointerReleasedEventArgs::new(
                &target,
                pointer.clone(),
                &root_element,
                args.position(),
                ev.timestamp(),
                PointerPointProperties::from_raw_point(
                    Self::get_modifiers(args.input_modifiers(), false),
                    update_kind,
                    args.point(),
                ),
                key_modifier,
                MouseButton::Left,
            );

            match (&gesture_target, pointer.captured_gesture_recognizer()) {
                (Some(_), Some(recognizer)) => recognizer.pointer_released_internal(&e),
                (Some(_), None) => {}
                (None, _) => target.raise_event(&e),
            }

            pointer.capture_lost(CaptureSource::Implicit);
            pointer.dispose();
        }

        if args.type_() == RawPointerEventType::TouchCancel {
            self.pointers.borrow_mut().remove(&args.raw_pointer_id());
            pointer.dispose();
        }

        if args.type_() == RawPointerEventType::TouchUpdate {
            target = gesture_target.clone().unwrap_or(target);

            let e = PointerEventArgs::new(
                Some(InputElement::pointer_moved_event()),
                &target,
                pointer.clone(),
                Some(&root_element),
                args.position(),
                ev.timestamp(),
                PointerPointProperties::from_raw_point(
                    Self::get_modifiers(args.input_modifiers(), true),
                    update_kind,
                    args.point(),
                ),
                key_modifier,
            )
            .with_previous_points(args.intermediate_points());

            match (&gesture_target, pointer.captured_gesture_recognizer()) {
                (Some(_), Some(recognizer)) => recognizer.pointer_moved_internal(&e),
                (Some(_), None) => {}
                (None, _) => target.raise_event(&e),
            }
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_pointer_device(&self) -> Option<&dyn IPointerDevice> {
        Some(self)
    }
}

impl IPointerDevice for TouchDevice {
    fn try_get_pointer(&self, ev: &RawPointerEventArgs) -> Option<Rc<dyn IPointer>> {
        self.pointers.borrow().get(&ev.raw_pointer_id()).map(|pointer| pointer.clone() as Rc<dyn IPointer>)
    }
}
