use super::raw::{IRawInputEventArgs, RawPointerEventArgs, RawPointerEventType};
use super::{
    CaptureSource, IInputDevice, IInputRoot, IPenDevice, IPointer, IPointerDevice, InputElement, IntermediatePoints,
    KeyModifiers, MouseButton, Pointer, PointerEventArgs, PointerPointProperties, PointerPressedEventArgs,
    PointerReleasedEventArgs, PointerType,
};
use crate::{Point, Rect, Ref, Size, Thickness};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

/// Represents a pen/stylus device.
pub struct PenDevice {
    pointers: RefCell<HashMap<i64, Rc<Pointer>>>,
    release_pointer_on_pen_up: bool,
    click_count: Cell<i32>,
    last_click_rect: Cell<Rect>,
    last_click_time: Cell<u64>,
    last_mouse_down_button: Cell<MouseButton>,
    disposed: Cell<bool>,
}

impl PenDevice {
    /// Creates a pen device. `release_pointer_on_pen_up` tells whether the
    /// pointer is released when the pen is lifted.
    pub fn new(release_pointer_on_pen_up: bool) -> Rc<PenDevice> {
        Rc::new(PenDevice {
            pointers: RefCell::new(HashMap::new()),
            release_pointer_on_pen_up,
            click_count: Cell::new(0),
            last_click_rect: Cell::new(Rect::default()),
            last_click_time: Cell::new(0),
            last_mouse_down_button: Cell::new(MouseButton::None),
            disposed: Cell::new(false),
        })
    }

    fn process_raw_pointer_event(&self, e: &RawPointerEventArgs) {
        let existing = self.pointers.borrow().get(&e.raw_pointer_id()).cloned();

        let pointer = match existing {
            Some(pointer) => pointer,
            None => {
                if e.type_() == RawPointerEventType::LeftButtonUp || e.type_() == RawPointerEventType::TouchEnd {
                    return;
                }

                let is_primary = self.pointers.borrow().is_empty();
                let pointer = Pointer::new(Pointer::get_next_free_id(), PointerType::Pen, is_primary);
                self.pointers.borrow_mut().insert(e.raw_pointer_id(), pointer.clone());
                pointer
            }
        };

        let point = e.point();
        let props = PointerPointProperties::with_pen_state_and_contact_rect(
            e.input_modifiers(),
            e.type_().to_update_kind(),
            point.twist,
            point.pressure,
            point.x_tilt,
            point.y_tilt,
            point.contact_rect(),
        );
        let key_modifiers = e.input_modifiers().to_key_modifiers();
        let hit_test = e.input_hit_test_result().1;

        /// Releases the pointer when the event is done, even on a panic.
        struct Release<'a> {
            device: &'a PenDevice,
            raw_pointer_id: i64,
            pointer: Rc<Pointer>,
            should_release: Cell<bool>,
        }

        impl Drop for Release<'_> {
            fn drop(&mut self) {
                if self.should_release.get() {
                    self.device.pointers.borrow_mut().remove(&self.raw_pointer_id);
                    self.pointer.dispose();
                }
            }
        }

        let release = Release {
            device: self,
            raw_pointer_id: e.raw_pointer_id(),
            pointer: pointer.clone(),
            should_release: Cell::new(false),
        };

        match e.type_() {
            RawPointerEventType::LeaveWindow => release.should_release.set(true),
            RawPointerEventType::CancelCapture => pointer.platform_capture_lost(),
            RawPointerEventType::LeftButtonDown
            | RawPointerEventType::RightButtonDown
            | RawPointerEventType::MiddleButtonDown
            | RawPointerEventType::XButton1Down
            | RawPointerEventType::XButton2Down => {
                let handled = self.pen_down(
                    &pointer,
                    e.timestamp(),
                    e.root(),
                    e.position(),
                    props,
                    key_modifiers,
                    hit_test,
                    e.platform_input_event_cookie(),
                );
                e.set_handled(handled);
            }
            RawPointerEventType::LeftButtonUp
            | RawPointerEventType::RightButtonUp
            | RawPointerEventType::MiddleButtonUp
            | RawPointerEventType::XButton1Up
            | RawPointerEventType::XButton2Up => {
                if self.release_pointer_on_pen_up {
                    release.should_release.set(true);
                }

                let handled =
                    self.pen_up(&pointer, e.timestamp(), e.root(), e.position(), props, key_modifiers, hit_test);
                e.set_handled(handled);
            }
            RawPointerEventType::Move => {
                let handled = Self::pen_move(
                    &pointer,
                    e.timestamp(),
                    e.root(),
                    e.position(),
                    props,
                    key_modifiers,
                    hit_test,
                    e.intermediate_points(),
                );
                e.set_handled(handled);
            }
            _ => {}
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn pen_down(
        &self,
        pointer: &Rc<Pointer>,
        timestamp: u64,
        root: &Rc<dyn IInputRoot>,
        p: Point,
        properties: PointerPointProperties,
        input_modifiers: KeyModifiers,
        hit_test: Option<Ref<InputElement>>,
        platform_input_event_cookie: Option<Rc<dyn Any>>,
    ) -> bool {
        let Some(source) = pointer.captured().or(hit_test) else { return false };

        pointer.capture_with_source(Some(&source), CaptureSource::Implicit);

        if let Some(settings) = source.get_platform_settings() {
            let double_click_time = settings.get_double_tap_time(PointerType::Pen).as_secs_f64() * 1000.0;
            let double_click_size = settings.get_double_tap_size(PointerType::Pen);

            if !self.last_click_rect.get().contains(p)
                || timestamp.wrapping_sub(self.last_click_time.get()) as f64 > double_click_time
            {
                self.click_count.set(0);
            }

            self.click_count.set(self.click_count.get() + 1);
            self.last_click_time.set(timestamp);
            self.last_click_rect.set(
                Rect::from_position_size(p, Size::default())
                    .inflate_thickness(Thickness::symmetric(double_click_size.width / 2.0, double_click_size.height / 2.0)),
            );
        }

        self.last_mouse_down_button.set(properties.pointer_update_kind.get_mouse_button());

        let e = PointerPressedEventArgs::new(
            &source,
            pointer.clone(),
            &root.root_element(),
            p,
            timestamp,
            properties,
            input_modifiers,
            self.click_count.get(),
        )
        .with_platform_input_event_cookie(platform_input_event_cookie);

        source.raise_event(&e);
        e.handled()
    }

    #[allow(clippy::too_many_arguments)]
    fn pen_move(
        pointer: &Rc<Pointer>,
        timestamp: u64,
        root: &Rc<dyn IInputRoot>,
        p: Point,
        properties: PointerPointProperties,
        input_modifiers: KeyModifiers,
        hit_test: Option<Ref<InputElement>>,
        intermediate_points: Option<IntermediatePoints>,
    ) -> bool {
        let gesture_recognizer = pointer.captured_gesture_recognizer();
        let source = gesture_recognizer
            .as_ref()
            .and_then(|recognizer| recognizer.target())
            .or_else(|| pointer.captured())
            .or(hit_test);
        let Some(source) = source else { return false };

        let e = PointerEventArgs::new(
            Some(InputElement::pointer_moved_event()),
            &source,
            pointer.clone(),
            Some(&root.root_element()),
            p,
            timestamp,
            properties,
            input_modifiers,
        )
        .with_previous_points(intermediate_points);

        match &gesture_recognizer {
            Some(gesture_recognizer) => gesture_recognizer.pointer_moved_internal(&e),
            None => source.raise_event(&e),
        }

        e.handled()
    }

    #[allow(clippy::too_many_arguments)]
    fn pen_up(
        &self,
        pointer: &Rc<Pointer>,
        timestamp: u64,
        root: &Rc<dyn IInputRoot>,
        p: Point,
        properties: PointerPointProperties,
        input_modifiers: KeyModifiers,
        hit_test: Option<Ref<InputElement>>,
    ) -> bool {
        let gesture_recognizer = pointer.captured_gesture_recognizer();
        let source = gesture_recognizer
            .as_ref()
            .and_then(|recognizer| recognizer.target())
            .or_else(|| pointer.captured())
            .or(hit_test);
        let Some(source) = source else { return false };

        let e = PointerReleasedEventArgs::new(
            &source,
            pointer.clone(),
            &root.root_element(),
            p,
            timestamp,
            properties,
            input_modifiers,
            self.last_mouse_down_button.get(),
        );

        /// Releases the implicit capture even if a handler panics.
        struct Finally<'a>(&'a PenDevice, &'a Pointer);

        impl Drop for Finally<'_> {
            fn drop(&mut self) {
                self.1.capture_lost(CaptureSource::Implicit);
                self.0.last_mouse_down_button.set(MouseButton::None);
            }
        }

        {
            let _finally = Finally(self, pointer);
            match &gesture_recognizer {
                Some(gesture_recognizer) => gesture_recognizer.pointer_released_internal(&e),
                None => source.raise_event(&e),
            }
        }

        e.handled()
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
}

impl IInputDevice for PenDevice {
    fn process_raw_event(&self, e: &dyn IRawInputEventArgs) {
        if !e.handled() {
            if let Some(margs) = e.downcast_ref::<RawPointerEventArgs>() {
                self.process_raw_pointer_event(margs);
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

impl IPointerDevice for PenDevice {
    fn try_get_pointer(&self, ev: &RawPointerEventArgs) -> Option<Rc<dyn IPointer>> {
        self.pointers.borrow().get(&ev.raw_pointer_id()).map(|pointer| pointer.clone() as Rc<dyn IPointer>)
    }
}

impl IPenDevice for PenDevice {}
