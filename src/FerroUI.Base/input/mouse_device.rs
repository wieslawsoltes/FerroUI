use super::raw::{
    IRawInputEventArgs, RawMouseWheelEventArgs, RawPointerEventArgs, RawPointerEventType, RawPointerGestureEventArgs,
};
use super::{
    CaptureSource, IInputDevice, IInputRoot, IMouseDevice, IPointer, IPointerDevice, InputElement, IntermediatePoints,
    KeyModifiers, MouseButton, Pointer, PointerDeltaEventArgs, PointerEventArgs, PointerPointProperties,
    PointerPressedEventArgs, PointerReleasedEventArgs, PointerType, PointerWheelEventArgs,
};
use crate::interactivity::RoutedEvent;
use crate::{Point, Rect, Ref, Size, Thickness, Vector};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

thread_local! {
    static PRIMARY: RefCell<Option<Rc<MouseDevice>>> = const { RefCell::new(None) };
}

/// Represents a mouse device: turns raw pointer events into routed pointer
/// events on the element under the pointer or capturing it.
pub struct MouseDevice {
    click_count: Cell<i32>,
    last_click_rect: Cell<Rect>,
    last_click_time: Cell<u64>,
    pointer: Rc<Pointer>,
    disposed: Cell<bool>,
    last_mouse_down_button: Cell<MouseButton>,
}

impl MouseDevice {
    /// Creates a mouse device with a pointer of its own.
    pub fn new() -> Rc<MouseDevice> {
        Self::with_pointer(Pointer::new(Pointer::get_next_free_id(), PointerType::Mouse, true))
    }

    /// Creates a mouse device for an existing pointer.
    pub fn with_pointer(pointer: Rc<Pointer>) -> Rc<MouseDevice> {
        Rc::new(MouseDevice {
            click_count: Cell::new(0),
            last_click_rect: Cell::new(Rect::default()),
            last_click_time: Cell::new(0),
            pointer,
            disposed: Cell::new(false),
            last_mouse_down_button: Cell::new(MouseButton::None),
        })
    }

    /// The primary mouse device of the current thread, created on first
    /// use.
    pub fn primary() -> Rc<MouseDevice> {
        if let Some(primary) = PRIMARY.with(|primary| primary.borrow().clone()) {
            return primary;
        }
        let primary = MouseDevice::new();
        PRIMARY.with(|slot| *slot.borrow_mut() = Some(primary.clone()));
        primary
    }

    /// Forgets the primary mouse device, so that tests start with a fresh
    /// one.
    pub fn reset_primary_for_unit_tests() {
        let old = PRIMARY.with(|primary| primary.take());
        drop(old);
    }

    /// The pointer of the mouse.
    pub fn pointer(&self) -> &Rc<Pointer> {
        &self.pointer
    }

    fn button_count(props: &PointerPointProperties) -> i32 {
        let mut rv = 0;
        if props.is_left_button_pressed {
            rv += 1;
        }
        if props.is_middle_button_pressed {
            rv += 1;
        }
        if props.is_right_button_pressed {
            rv += 1;
        }
        if props.is_x_button_1_pressed {
            rv += 1;
        }
        if props.is_x_button_2_pressed {
            rv += 1;
        }
        rv
    }

    fn process_raw_pointer_event(&self, raw: &dyn IRawInputEventArgs, e: &RawPointerEventArgs) {
        if let Some(mouse) = e.device().as_any().downcast_ref::<MouseDevice>() {
            if mouse.disposed.get() {
                return;
            }
        }

        let props = Self::create_properties(e);
        let key_modifiers = e.input_modifiers().to_key_modifiers();
        let root = e.root();
        let hit_test = e.input_hit_test_result().1;

        match e.type_() {
            RawPointerEventType::LeaveWindow | RawPointerEventType::NonClientLeftButtonDown => self.leave_window(),
            RawPointerEventType::LeftButtonDown
            | RawPointerEventType::RightButtonDown
            | RawPointerEventType::MiddleButtonDown
            | RawPointerEventType::XButton1Down
            | RawPointerEventType::XButton2Down => {
                let handled = if Self::button_count(&props) > 1 {
                    self.mouse_move(
                        e.timestamp(),
                        root,
                        e.position(),
                        props,
                        key_modifiers,
                        e.intermediate_points(),
                        hit_test,
                    )
                } else {
                    self.mouse_down(
                        e.timestamp(),
                        root,
                        e.position(),
                        props,
                        key_modifiers,
                        e.platform_input_event_cookie(),
                    )
                };
                e.set_handled(handled);
            }
            RawPointerEventType::LeftButtonUp
            | RawPointerEventType::RightButtonUp
            | RawPointerEventType::MiddleButtonUp
            | RawPointerEventType::XButton1Up
            | RawPointerEventType::XButton2Up => {
                let handled = if Self::button_count(&props) != 0 {
                    self.mouse_move(
                        e.timestamp(),
                        root,
                        e.position(),
                        props,
                        key_modifiers,
                        e.intermediate_points(),
                        hit_test,
                    )
                } else {
                    self.mouse_up(e.timestamp(), root, e.position(), props, key_modifiers, hit_test)
                };
                e.set_handled(handled);
            }
            RawPointerEventType::Move => {
                let handled = self.mouse_move(
                    e.timestamp(),
                    root,
                    e.position(),
                    props,
                    key_modifiers,
                    e.intermediate_points(),
                    hit_test,
                );
                e.set_handled(handled);
            }
            RawPointerEventType::Wheel => {
                let delta = raw
                    .downcast_ref::<RawMouseWheelEventArgs>()
                    .expect("a raw wheel event must be a RawMouseWheelEventArgs")
                    .delta();
                let handled = self.mouse_wheel(e.timestamp(), root, e.position(), props, delta, key_modifiers, hit_test);
                e.set_handled(handled);
            }
            RawPointerEventType::Magnify | RawPointerEventType::Rotate | RawPointerEventType::Swipe => {
                let delta = raw
                    .downcast_ref::<RawPointerGestureEventArgs>()
                    .expect("a raw gesture event must be a RawPointerGestureEventArgs")
                    .delta();
                let routed_event = match e.type_() {
                    RawPointerEventType::Magnify => InputElement::pointer_touch_pad_gesture_magnify_event(),
                    RawPointerEventType::Rotate => InputElement::pointer_touch_pad_gesture_rotate_event(),
                    _ => InputElement::pointer_touch_pad_gesture_swipe_event(),
                };
                let handled =
                    self.gesture(routed_event, e.timestamp(), root, e.position(), props, delta, key_modifiers, hit_test);
                e.set_handled(handled);
            }
            RawPointerEventType::CancelCapture => self.platform_capture_lost(),
            RawPointerEventType::TouchBegin
            | RawPointerEventType::TouchUpdate
            | RawPointerEventType::TouchEnd
            | RawPointerEventType::TouchCancel => {}
        }
    }

    fn leave_window(&self) {}

    fn create_properties(args: &RawPointerEventArgs) -> PointerPointProperties {
        PointerPointProperties::new(args.input_modifiers(), args.type_().to_update_kind())
    }

    fn mouse_down(
        &self,
        timestamp: u64,
        root: &Rc<dyn IInputRoot>,
        p: Point,
        properties: PointerPointProperties,
        input_modifiers: KeyModifiers,
        platform_input_event_cookie: Option<Rc<dyn Any>>,
    ) -> bool {
        let root_element = root.root_element();
        let source = self.pointer.captured().or_else(|| root_element.input_hit_test(p));

        let Some(source) = source else { return false };

        self.pointer.capture_with_source(Some(&source), CaptureSource::Implicit);

        if let Some(settings) = source.get_platform_settings() {
            let double_click_time = settings.get_double_tap_time(PointerType::Mouse).as_secs_f64() * 1000.0;
            let double_click_size = settings.get_double_tap_size(PointerType::Mouse);

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
            self.pointer.clone(),
            &root_element,
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
    fn mouse_move(
        &self,
        timestamp: u64,
        root: &Rc<dyn IInputRoot>,
        p: Point,
        properties: PointerPointProperties,
        input_modifiers: KeyModifiers,
        intermediate_points: Option<IntermediatePoints>,
        hit_test: Option<Ref<InputElement>>,
    ) -> bool {
        let gesture_recognizer = self.pointer.captured_gesture_recognizer();
        let source = gesture_recognizer
            .as_ref()
            .and_then(|recognizer| recognizer.target())
            .or_else(|| self.pointer.captured())
            .or(hit_test);
        let Some(source) = source else { return false };
        let root_element = root.root_element();

        let e = PointerEventArgs::new(
            Some(InputElement::pointer_moved_event()),
            &source,
            self.pointer.clone(),
            Some(&root_element),
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

    fn mouse_up(
        &self,
        timestamp: u64,
        root: &Rc<dyn IInputRoot>,
        p: Point,
        props: PointerPointProperties,
        input_modifiers: KeyModifiers,
        hit_test: Option<Ref<InputElement>>,
    ) -> bool {
        let gesture_recognizer = self.pointer.captured_gesture_recognizer();
        let source = gesture_recognizer
            .as_ref()
            .and_then(|recognizer| recognizer.target())
            .or_else(|| self.pointer.captured())
            .or(hit_test);
        let Some(source) = source else { return false };
        let root_element = root.root_element();

        let e = PointerReleasedEventArgs::new(
            &source,
            self.pointer.clone(),
            &root_element,
            p,
            timestamp,
            props,
            input_modifiers,
            self.last_mouse_down_button.get(),
        );

        /// Releases the implicit capture even if a handler panics.
        struct Finally<'a>(&'a MouseDevice);

        impl Drop for Finally<'_> {
            fn drop(&mut self) {
                self.0.pointer.capture_lost(CaptureSource::Implicit);
                self.0.last_mouse_down_button.set(MouseButton::None);
            }
        }

        {
            let _finally = Finally(self);
            match &gesture_recognizer {
                Some(gesture_recognizer) => gesture_recognizer.pointer_released_internal(&e),
                None => source.raise_event(&e),
            }
        }

        e.handled()
    }

    #[allow(clippy::too_many_arguments)]
    fn mouse_wheel(
        &self,
        timestamp: u64,
        root: &Rc<dyn IInputRoot>,
        p: Point,
        props: PointerPointProperties,
        delta: Vector,
        input_modifiers: KeyModifiers,
        hit_test: Option<Ref<InputElement>>,
    ) -> bool {
        let Some(source) = self.pointer.captured().or(hit_test) else { return false };
        let root_element = root.root_element();

        let e = PointerWheelEventArgs::new(
            &source,
            self.pointer.clone(),
            &root_element,
            p,
            timestamp,
            props,
            input_modifiers,
            delta,
        );

        source.raise_event(&e);
        e.handled()
    }

    /// Raises one of the touchpad gesture events (magnify, rotate, swipe).
    #[allow(clippy::too_many_arguments)]
    fn gesture(
        &self,
        routed_event: &RoutedEvent<PointerDeltaEventArgs>,
        timestamp: u64,
        root: &Rc<dyn IInputRoot>,
        p: Point,
        props: PointerPointProperties,
        delta: Vector,
        input_modifiers: KeyModifiers,
        hit_test: Option<Ref<InputElement>>,
    ) -> bool {
        let Some(source) = self.pointer.captured().or(hit_test) else { return false };
        let root_element = root.root_element();

        let e = PointerDeltaEventArgs::new(
            Some(routed_event),
            &source,
            self.pointer.clone(),
            &root_element,
            p,
            timestamp,
            props,
            input_modifiers,
            delta,
        );

        source.raise_event(&e);
        e.handled()
    }

    /// Releases the mouse device and its pointer.
    pub fn dispose(&self) {
        self.disposed.set(true);
        self.pointer.dispose();
    }

    /// Notifies the device that the capture of its pointer was lost in the
    /// platform.
    pub fn platform_capture_lost(&self) {
        self.pointer.platform_capture_lost();
    }
}

impl IInputDevice for MouseDevice {
    fn process_raw_event(&self, e: &dyn IRawInputEventArgs) {
        if !e.handled() {
            if let Some(margs) = e.downcast_ref::<RawPointerEventArgs>() {
                self.process_raw_pointer_event(e, margs);
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

impl IPointerDevice for MouseDevice {
    fn try_get_pointer(&self, _ev: &RawPointerEventArgs) -> Option<Rc<dyn IPointer>> {
        Some(self.pointer.clone())
    }
}

impl IMouseDevice for MouseDevice {}
