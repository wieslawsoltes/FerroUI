//! The touch input tests of the text box.
//!
//! Positions are computed from the metrics of the test fonts (see
//! `ferroui_base::media::text_formatting::testing`): every glyph advances
//! half an em, so at the default size of 12 a character is 6 wide and the
//! point at x = 50 is nearest to the text position 8, as in the reference.

use crate::test_support::{test_scope, TestRoot};
use crate::text_box_tests::create_template;
use crate::TextBox;
use ferroui_base::input::{
    ContextRequestedEventArgs, IPointer, InputElement, KeyModifiers, MouseButton, Pointer, PointerEventArgs,
    PointerPointProperties, PointerPressedEventArgs, PointerReleasedEventArgs, PointerType, PointerUpdateKind,
    RawInputModifiers,
};
use ferroui_base::interactivity::Interactive;
use ferroui_base::platform::DefaultPlatformSettings;
use ferroui_base::threading::Dispatcher;
use ferroui_base::{Point, Ref};
use std::cell::Cell;
use std::rc::Rc;

/// Raises touch pointer events directly on elements.
pub(crate) struct TouchTestHelper {
    pointer: Rc<Pointer>,
    next_stamp: Cell<u64>,
    click_count: Cell<i32>,
}

impl TouchTestHelper {
    pub(crate) fn new() -> Self {
        Self {
            pointer: Pointer::new(Pointer::get_next_free_id(), PointerType::Touch, true),
            next_stamp: Cell::new(1),
            click_count: Cell::new(0),
        }
    }

    fn timestamp(&self) -> u64 {
        let stamp = self.next_stamp.get();
        self.next_stamp.set(stamp + 1);
        stamp
    }

    fn pointer(&self) -> Rc<dyn IPointer> {
        self.pointer.clone()
    }

    /// The element that captures the pointer.
    #[allow(dead_code)]
    pub(crate) fn captured(&self) -> Option<Ref<InputElement>> {
        self.pointer.captured()
    }

    /// Touches an element at a position in its coordinates.
    pub(crate) fn down(&self, target: &Interactive, position: Point) {
        let element: Option<Ref<InputElement>> = target.to_ref().cast::<InputElement>();
        self.pointer.capture(element.as_ref());
        self.click_count.set(self.click_count.get() + 1);
        target.raise_event(&PointerPressedEventArgs::new(
            target.to_ref(),
            self.pointer(),
            target,
            position,
            self.timestamp(),
            PointerPointProperties::new(RawInputModifiers::LEFT_MOUSE_BUTTON, PointerUpdateKind::LeftButtonPressed),
            KeyModifiers::NONE,
            self.click_count.get(),
        ));
    }

    /// Moves the contact to a position in the coordinates of the element.
    #[allow(dead_code)]
    pub(crate) fn move_(&self, target: &Interactive, position: Point) {
        let e = PointerEventArgs::new(
            Some(InputElement::pointer_moved_event()),
            target.to_ref(),
            self.pointer(),
            Some(target),
            position,
            self.timestamp(),
            PointerPointProperties::new(RawInputModifiers::LEFT_MOUSE_BUTTON, PointerUpdateKind::Other),
            KeyModifiers::NONE,
        );

        match self.pointer.captured_gesture_recognizer() {
            Some(recognizer) => recognizer.pointer_moved_for_testing(&e),
            None => target.raise_event(&e),
        }
    }

    /// Lifts the contact at a position in the coordinates of the element.
    pub(crate) fn up(&self, target: &Interactive, position: Point) {
        let e = PointerReleasedEventArgs::new(
            target.to_ref(),
            self.pointer(),
            target,
            position,
            self.timestamp(),
            PointerPointProperties::new(RawInputModifiers::NONE, PointerUpdateKind::LeftButtonReleased),
            KeyModifiers::NONE,
            MouseButton::Left,
        );

        match self.pointer.captured_gesture_recognizer() {
            Some(recognizer) => recognizer.pointer_released_for_testing(&e),
            None => target.raise_event(&e),
        }

        self.cancel();
    }

    pub(crate) fn cancel(&self) {
        self.pointer.platform_capture_lost();
    }
}

fn target_in_root() -> (Ref<TextBox>, Ref<TestRoot>) {
    let target = TextBox::new();
    target.set_template(create_template());
    target.set_text(Some("12 12345678"));

    let root = TestRoot::with_child(target.clone());

    // The gestures read the hold duration and the tap size from the platform settings of the root.
    root.set_platform_settings(Some(Rc::new(DefaultPlatformSettings::new())));

    (target, root)
}

/// Fires the only timer of the dispatcher: the hold timer of the gesture.
fn fire_single_timer() {
    let timers = Dispatcher::timers_for_unit_tests();
    assert_eq!(1, timers.len());
    Dispatcher::force_fire_timer_for_unit_tests(&timers[0]);
}

#[test]
fn touch_tap_moves_caret() {
    let _scope = test_scope();
    let (target, root) = target_in_root();

    target.apply_template();

    root.execute_initial_layout_pass();

    let touch = TouchTestHelper::new();

    assert_eq!(target.caret_index(), 0);

    // Move to index 8.
    touch.down(&target, Point::new(50.0, 0.0));
    touch.up(&target, Point::new(50.0, 0.0));

    assert_eq!(target.caret_index(), 8);
}

#[test]
fn touch_double_tap_selects_word() {
    let _scope = test_scope();
    let (target, root) = target_in_root();

    target.apply_template();

    root.execute_initial_layout_pass();

    let touch = TouchTestHelper::new();

    assert_eq!(target.caret_index(), 0);

    // Move to index 8.
    touch.down(&target, Point::new(50.0, 0.0));
    touch.up(&target, Point::new(50.0, 0.0));

    // Double tap.
    touch.down(&target, Point::new(50.0, 0.0));
    touch.up(&target, Point::new(50.0, 0.0));

    assert_eq!(target.selection_start(), 3);
    assert_eq!(target.selection_end(), 11);
    assert_eq!(target.caret_index(), 8);
}

#[test]
fn touch_hold_selects_word() {
    let _scope = test_scope();
    let (target, root) = target_in_root();

    target.apply_template();

    root.execute_initial_layout_pass();

    let touch = TouchTestHelper::new();

    assert_eq!(target.caret_index(), 0);

    // Move to index 8.
    touch.down(&target, Point::new(50.0, 0.0));

    fire_single_timer();
    touch.up(&target, Point::new(50.0, 0.0));

    assert_eq!(target.selection_start(), 3);
    assert_eq!(target.selection_end(), 11);
    assert_eq!(target.caret_index(), 8);
}

#[test]
fn touch_hold_on_selection_requests_context() {
    let _scope = test_scope();
    let (target, root) = target_in_root();

    target.apply_template();
    let requested = Rc::new(Cell::new(false));

    target.set_selection_start(3);
    target.set_selection_end(11);

    let flag = requested.clone();
    target.add_handler(InputElement::context_requested_event(), move |_, _: &ContextRequestedEventArgs| {
        flag.set(true)
    });

    root.execute_initial_layout_pass();

    let touch = TouchTestHelper::new();

    assert_eq!(target.caret_index(), 0);

    // Move to index 8.
    touch.down(&target, Point::new(50.0, 0.0));

    fire_single_timer();
    touch.up(&target, Point::new(50.0, 0.0));

    assert!(requested.get());
}
