//! Raises pointer events on elements the way a mouse would, for the tests
//! of this crate.

#![allow(dead_code)]

use ferroui_base::input::{
    CaptureSource, IPointer, InputElement, KeyModifiers, MouseButton, Pointer, PointerEventArgs, PointerPointProperties,
    PointerPressedEventArgs, PointerReleasedEventArgs, PointerType, PointerUpdateKind, RawInputModifiers,
};
use ferroui_base::interactivity::Interactive;
use ferroui_base::{Point, Ref, Visual};
use std::cell::Cell;
use std::rc::Rc;

pub struct MouseTestHelper {
    pointer: Rc<Pointer>,
    next_stamp: Cell<u64>,
    pressed_buttons: Cell<RawInputModifiers>,
    pressed_button: Cell<MouseButton>,
}

impl MouseTestHelper {
    pub fn new() -> Self {
        Self::with_pointer_type(PointerType::Mouse)
    }

    pub fn with_pointer_type(pointer_type: PointerType) -> Self {
        Self {
            pointer: Pointer::new(Pointer::get_next_free_id(), pointer_type, true),
            next_stamp: Cell::new(1),
            pressed_buttons: Cell::new(RawInputModifiers::NONE),
            pressed_button: Cell::new(MouseButton::None),
        }
    }

    fn timestamp(&self) -> u64 {
        let stamp = self.next_stamp.get();
        self.next_stamp.set(stamp + 1);
        stamp
    }

    /// The element that captures the pointer.
    pub fn captured(&self) -> Option<Ref<InputElement>> {
        self.pointer.captured()
    }

    fn convert(mouse_button: MouseButton) -> RawInputModifiers {
        match mouse_button {
            MouseButton::Left => RawInputModifiers::LEFT_MOUSE_BUTTON,
            MouseButton::Right => RawInputModifiers::RIGHT_MOUSE_BUTTON,
            MouseButton::Middle => RawInputModifiers::MIDDLE_MOUSE_BUTTON,
            _ => RawInputModifiers::NONE,
        }
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
        rv
    }

    fn pointer(&self) -> Rc<dyn IPointer> {
        self.pointer.clone()
    }

    pub fn down(&self, target: &Interactive) {
        self.down_with(target, target, MouseButton::Left, None, KeyModifiers::NONE, 1);
    }

    pub fn down_at(&self, target: &Interactive, mouse_button: MouseButton, position: Point, click_count: i32) {
        self.down_with(target, target, mouse_button, Some(position), KeyModifiers::NONE, click_count);
    }

    pub fn down_with(
        &self,
        target: &Interactive,
        source: &Interactive,
        mouse_button: MouseButton,
        position: Option<Point>,
        modifiers: KeyModifiers,
        click_count: i32,
    ) {
        self.pressed_buttons.set(self.pressed_buttons.get() | Self::convert(mouse_button));
        let props = PointerPointProperties::new(
            self.pressed_buttons.get(),
            match mouse_button {
                MouseButton::Left => PointerUpdateKind::LeftButtonPressed,
                MouseButton::Middle => PointerUpdateKind::MiddleButtonPressed,
                MouseButton::Right => PointerUpdateKind::RightButtonPressed,
                _ => PointerUpdateKind::Other,
            },
        );
        if Self::button_count(&props) > 1 {
            self.move_with(target, source, position.unwrap_or_default(), modifiers);
        } else {
            self.pressed_button.set(mouse_button);
            let target_element: Option<Ref<InputElement>> = target.to_ref().cast::<InputElement>();
            self.pointer.capture(target_element.as_ref());
            let root = Self::get_root(target);
            source.raise_event(&PointerPressedEventArgs::new(
                source.to_ref(),
                self.pointer(),
                &root,
                position.unwrap_or_else(|| Self::midpoint_relative_to_root(target)),
                self.timestamp(),
                props,
                modifiers,
                click_count,
            ));
        }
    }

    pub fn move_(&self, target: &Interactive, position: Point) {
        self.move_with(target, target, position, KeyModifiers::NONE);
    }

    pub fn move_with(&self, target: &Interactive, source: &Interactive, position: Point, modifiers: KeyModifiers) {
        let root = Self::get_root(target);
        let e = PointerEventArgs::new(
            Some(InputElement::pointer_moved_event()),
            source.to_ref(),
            self.pointer(),
            Some(&*root),
            position,
            self.timestamp(),
            PointerPointProperties::new(self.pressed_buttons.get(), PointerUpdateKind::Other),
            modifiers,
        );

        match self.pointer.captured_gesture_recognizer() {
            Some(recognizer) => recognizer.pointer_moved_for_testing(&e),
            None => target.raise_event(&e),
        }
    }

    pub fn up(&self, target: &Interactive) {
        self.up_with(target, target, MouseButton::Left, None, KeyModifiers::NONE);
    }

    pub fn up_at(&self, target: &Interactive, mouse_button: MouseButton, position: Point) {
        self.up_with(target, target, mouse_button, Some(position), KeyModifiers::NONE);
    }

    pub fn up_with(
        &self,
        target: &Interactive,
        source: &Interactive,
        mouse_button: MouseButton,
        position: Option<Point>,
        modifiers: KeyModifiers,
    ) {
        let conv = Self::convert(mouse_button);
        self.pressed_buttons.set((self.pressed_buttons.get() | conv) ^ conv);
        let props = PointerPointProperties::new(
            self.pressed_buttons.get(),
            match mouse_button {
                MouseButton::Left => PointerUpdateKind::LeftButtonReleased,
                MouseButton::Middle => PointerUpdateKind::MiddleButtonReleased,
                MouseButton::Right => PointerUpdateKind::RightButtonReleased,
                _ => PointerUpdateKind::Other,
            },
        );
        if Self::button_count(&props) == 0 {
            let root = Self::get_root(target);
            let e = PointerReleasedEventArgs::new(
                source.to_ref(),
                self.pointer(),
                &root,
                position.unwrap_or_else(|| Self::midpoint_relative_to_root(target)),
                self.timestamp(),
                props,
                modifiers,
                self.pressed_button.get(),
            );

            match self.pointer.captured_gesture_recognizer() {
                Some(recognizer) => recognizer.pointer_released_for_testing(&e),
                None => target.raise_event(&e),
            }

            // Ends every capture the pointer holds, as the capture loss of
            // the pointer does.
            self.pointer.capture_with_source(None, CaptureSource::Explicit);
            self.pointer.set_is_gesture_recognition_skipped(false);
        } else {
            self.move_with(target, source, position.unwrap_or_default(), modifiers);
        }
    }

    pub fn click(&self, target: &Interactive) {
        self.click_with(target, target, MouseButton::Left, None, KeyModifiers::NONE);
    }

    pub fn click_with(
        &self,
        target: &Interactive,
        source: &Interactive,
        button: MouseButton,
        position: Option<Point>,
        modifiers: KeyModifiers,
    ) {
        self.down_with(target, source, button, position, modifiers, 1);
        let captured: Ref<Interactive> = match self.pointer.captured() {
            Some(captured) => captured.upcast(),
            None => source.to_ref(),
        };
        self.up_with(&captured, &captured, button, position, modifiers);
    }

    pub fn double_click(&self, target: &Interactive) {
        self.double_click_with(target, target, MouseButton::Left, None, KeyModifiers::NONE);
    }

    pub fn double_click_with(
        &self,
        target: &Interactive,
        source: &Interactive,
        button: MouseButton,
        position: Option<Point>,
        modifiers: KeyModifiers,
    ) {
        self.down_with(target, source, button, position, modifiers, 1);
        let captured: Ref<Interactive> = match self.pointer.captured() {
            Some(captured) => captured.upcast(),
            None => source.to_ref(),
        };
        self.up_with(&captured, &captured, button, position, modifiers);
        self.down_with(target, source, button, position, modifiers, 2);
    }

    pub fn enter(&self, target: &Interactive) {
        target.raise_event(&PointerEventArgs::new(
            Some(InputElement::pointer_entered_event()),
            target.to_ref(),
            self.pointer(),
            Some(target),
            Point::default(),
            self.timestamp(),
            PointerPointProperties::new(self.pressed_buttons.get(), PointerUpdateKind::Other),
            KeyModifiers::NONE,
        ));
    }

    pub fn leave(&self, target: &Interactive) {
        target.raise_event(&PointerEventArgs::new(
            Some(InputElement::pointer_exited_event()),
            target.to_ref(),
            self.pointer(),
            Some(target),
            Point::default(),
            self.timestamp(),
            PointerPointProperties::new(self.pressed_buttons.get(), PointerUpdateKind::Other),
            KeyModifiers::NONE,
        ));
    }

    fn get_root(source: &Interactive) -> Ref<Visual> {
        source.visual_root().unwrap_or_else(|| source.to_ref().upcast())
    }

    fn midpoint_relative_to_root(element: &Interactive) -> Point {
        let root = Self::get_root(element);
        let bounds = element.bounds();
        element.translate_point(Point::new(bounds.width / 2.0, bounds.height / 2.0), &root).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::MouseTestHelper;
    use crate::test_support::{test_scope, TestRoot};
    use crate::Border;
    use ferroui_base::input::gesture_recognizers::SwipeGestureRecognizer;
    use ferroui_base::input::{
        InputElement, MouseButton, PointerEventArgs, PointerReleasedEventArgs, SwipeGestureEndedEventArgs,
        SwipeGestureEventArgs,
    };
    use ferroui_base::interactivity::RoutingStrategies;
    use ferroui_base::{Point, Size};
    use std::cell::Cell;
    use std::rc::Rc;

    /// Once a gesture recognizer has captured the pointer, moves and the release go to the recognizer
    /// only, as the input devices deliver them: they are not raised on the element.
    #[test]
    fn move_and_up_go_to_the_capturing_gesture_recognizer() {
        let _scope = test_scope();

        let border = Border::new();
        border.set_width(400.0);
        border.set_height(300.0);
        let recognizer = SwipeGestureRecognizer::new();
        recognizer.set_can_horizontally_swipe(true);
        recognizer.set_is_mouse_enabled(true);
        border.gesture_recognizers().add(recognizer);

        let root = TestRoot::new();
        root.set_client_size(Size::new(400.0, 300.0));
        root.set_child(border.clone());
        root.execute_initial_layout_pass();

        let moved = Rc::new(Cell::new(0));
        let released = Rc::new(Cell::new(0));
        let swiped = Rc::new(Cell::new(0));
        let ended = Rc::new(Cell::new(0));
        let all = RoutingStrategies::DIRECT | RoutingStrategies::TUNNEL | RoutingStrategies::BUBBLE;
        let count = moved.clone();
        border.add_handler_with(
            InputElement::pointer_moved_event(),
            move |_, _: &PointerEventArgs| count.set(count.get() + 1),
            all,
            true,
        );
        let count = released.clone();
        border.add_handler_with(
            InputElement::pointer_released_event(),
            move |_, _: &PointerReleasedEventArgs| count.set(count.get() + 1),
            all,
            true,
        );
        let count = swiped.clone();
        border.add_handler(InputElement::swipe_gesture_event(), move |_, _: &SwipeGestureEventArgs| {
            count.set(count.get() + 1)
        });
        let count = ended.clone();
        border.add_handler(InputElement::swipe_gesture_ended_event(), move |_, _: &SwipeGestureEndedEventArgs| {
            count.set(count.get() + 1)
        });

        let mouse = MouseTestHelper::new();
        mouse.down_at(&border, MouseButton::Left, Point::new(200.0, 100.0), 1);

        // Nothing has captured the pointer yet: the move is raised on the element, where the
        // recognizer sees it, starts the swipe and captures the pointer.
        mouse.move_(&border, Point::new(100.0, 100.0));
        assert_eq!(1, swiped.get());
        let moved_before_capture = moved.get();
        assert!(moved_before_capture > 0);
        assert!(mouse.pointer.captured_gesture_recognizer().is_some());

        mouse.move_(&border, Point::new(40.0, 100.0));
        assert_eq!(moved_before_capture, moved.get());
        assert_eq!(2, swiped.get());

        mouse.up_at(&border, MouseButton::Left, Point::new(40.0, 100.0));
        assert_eq!(0, released.get());
        assert_eq!(1, ended.get());
        assert!(mouse.pointer.captured_gesture_recognizer().is_none());
    }
}
