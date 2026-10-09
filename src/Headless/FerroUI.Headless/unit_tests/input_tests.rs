//! Port of `InputTests.cs` of the upstream unit test project of the
//! headless platform.
//!
//! The constructor and `Dispose` of the test class are `InputTests::new`
//! and its `Drop`, which each test holds for its duration.

use super::test_application::ferro_fact;
use crate::HeadlessWindowExtensions;
use ferroui_base::input::{InputElement, MouseButton, PointerType, RawInputModifiers};
use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
use ferroui_base::media::Brushes;
use ferroui_base::threading::Dispatcher;
use ferroui_base::{BoxedValue, PixelPoint, Point, Ref};
use ferroui_controls::{Application, Border, Button, Control, Window};
use std::cell::{Cell, RefCell};
use std::collections::HashSet;
use std::rc::Rc;

struct InputTests {
    window: Ref<Window>,
    setup_app: Option<Ref<Application>>,
}

fn same(expected: &Option<Ref<Application>>, actual: &Option<Ref<Application>>) -> bool {
    match (expected, actual) {
        (Some(expected), Some(actual)) => expected.ptr_eq(actual),
        (None, None) => true,
        _ => false,
    }
}

impl InputTests {
    fn new() -> Self {
        let setup_app = Application::current();
        Dispatcher::ui_thread().verify_access();
        let window = Window::new();
        window.set_width(100.0);
        window.set_height(100.0);
        Self { window, setup_app }
    }
}

impl Drop for InputTests {
    fn drop(&mut self) {
        // A failed test is unwinding: its failure is the one to report.
        if !std::thread::panicking() {
            assert!(same(&self.setup_app, &Application::current()));

            Dispatcher::ui_thread().verify_access();
        }
        self.window.close();
    }
}

fn should_click_button_on_window() {
    let this = InputTests::new();
    assert!(same(&this.setup_app, &Application::current()));
    let button_clicked = Rc::new(Cell::new(false));
    let button = Button::new();
    button.set_horizontal_alignment(HorizontalAlignment::Stretch);
    button.set_vertical_alignment(VerticalAlignment::Stretch);

    button.click({
        let button_clicked = button_clicked.clone();
        move |_, _| button_clicked.set(true)
    });

    this.window.set_content(Some(Control::boxed(button.clone())));
    this.window.show();

    this.window.mouse_down(Point::new(50.0, 50.0), MouseButton::Left, RawInputModifiers::NONE);
    this.window.mouse_up(Point::new(50.0, 50.0), MouseButton::Left, RawInputModifiers::NONE);

    assert!(button_clicked.get());
}
ferro_fact!(should_click_button_on_window);

fn change_window_position() {
    let this = InputTests::new();
    let new_window_position = PixelPoint::new(100, 150);
    this.window.set_position(new_window_position);
    this.window.show();
    assert_eq!(new_window_position, this.window.position());
}
ferro_fact!(change_window_position);

fn should_click_button_after_explicit_run_jobs() {
    let this = InputTests::new();
    // Regression test for the upstream issue 20309
    // Ensure that calling Dispatcher.UIThread.RunJobs() before MouseDown does not throw
    let button = Button::new();
    let content: BoxedValue = Rc::new("Test content".to_string());
    button.set_content(Some(content));
    this.window.set_content(Some(Control::boxed(button.clone())));
    this.window.show();

    Dispatcher::ui_thread().run_jobs(None);

    let click_count = Rc::new(Cell::new(0));
    button.click({
        let click_count = click_count.clone();
        move |_, _| click_count.set(click_count.get() + 1)
    });

    let point = Point::new(button.bounds().width / 2.0, button.bounds().height / 2.0);
    let translate_point = button.translate_point(point, &this.window);

    // Move
    this.window.mouse_move(translate_point.unwrap(), RawInputModifiers::NONE);

    // Click
    this.window.mouse_down(translate_point.unwrap(), MouseButton::Left, RawInputModifiers::NONE);
    this.window.mouse_up(translate_point.unwrap(), MouseButton::Left, RawInputModifiers::NONE);

    assert_eq!(1, click_count.get());
}
ferro_fact!(should_click_button_after_explicit_run_jobs);

fn touch_contact_raises_touch_pointer_events() {
    let this = InputTests::new();
    let pressed_count = Rc::new(Cell::new(0));
    let moved_count = Rc::new(Cell::new(0));
    let released_count = Rc::new(Cell::new(0));
    let pressed_pointer_type: Rc<Cell<Option<PointerType>>> = Rc::new(Cell::new(None));

    let border = Border::new();
    border.set_background(Some(Brushes::red()));
    border.add_handler(InputElement::pointer_pressed_event(), {
        let pressed_count = pressed_count.clone();
        let pressed_pointer_type = pressed_pointer_type.clone();
        move |_, e| {
            pressed_count.set(pressed_count.get() + 1);
            pressed_pointer_type.set(Some(e.pointer().type_()));
        }
    });
    border.add_handler(InputElement::pointer_moved_event(), {
        let moved_count = moved_count.clone();
        move |_, _| moved_count.set(moved_count.get() + 1)
    });
    border.add_handler(InputElement::pointer_released_event(), {
        let released_count = released_count.clone();
        move |_, _| released_count.set(released_count.get() + 1)
    });

    this.window.set_content(Some(Control::boxed(border.clone())));
    this.window.show();

    let touch = this.window.touch_begin(Point::new(50.0, 50.0), RawInputModifiers::NONE);
    this.window.touch_move(&*touch, Point::new(60.0, 60.0), RawInputModifiers::NONE);
    this.window.touch_end(&*touch, Point::new(60.0, 60.0), RawInputModifiers::NONE);

    assert_eq!(1, pressed_count.get());
    assert_eq!(1, moved_count.get());
    assert_eq!(1, released_count.get());
    assert_eq!(Some(PointerType::Touch), pressed_pointer_type.get());
}
ferro_fact!(touch_contact_raises_touch_pointer_events);

fn multiple_touch_contacts_are_distinct_pointers() {
    let this = InputTests::new();
    let pointer_ids = Rc::new(RefCell::new(HashSet::new()));

    let border = Border::new();
    border.set_background(Some(Brushes::red()));
    border.add_handler(InputElement::pointer_pressed_event(), {
        let pointer_ids = pointer_ids.clone();
        move |_, e| {
            pointer_ids.borrow_mut().insert(e.pointer().id());
        }
    });

    this.window.set_content(Some(Control::boxed(border.clone())));
    this.window.show();

    let touch1 = this.window.touch_begin(Point::new(30.0, 30.0), RawInputModifiers::NONE);
    let touch2 = this.window.touch_begin(Point::new(70.0, 70.0), RawInputModifiers::NONE);
    this.window.touch_end(&*touch1, Point::new(30.0, 30.0), RawInputModifiers::NONE);
    this.window.touch_end(&*touch2, Point::new(70.0, 70.0), RawInputModifiers::NONE);

    assert_eq!(2, pointer_ids.borrow().len());
}
ferro_fact!(multiple_touch_contacts_are_distinct_pointers);

fn disposing_touch_pointer_cancels_contact() {
    let this = InputTests::new();
    let capture_lost_count = Rc::new(Cell::new(0));
    let released_count = Rc::new(Cell::new(0));

    let border = Border::new();
    border.set_background(Some(Brushes::red()));
    border.add_handler(InputElement::pointer_capture_lost_event(), {
        let capture_lost_count = capture_lost_count.clone();
        move |_, _| capture_lost_count.set(capture_lost_count.get() + 1)
    });
    border.add_handler(InputElement::pointer_released_event(), {
        let released_count = released_count.clone();
        move |_, _| released_count.set(released_count.get() + 1)
    });

    this.window.set_content(Some(Control::boxed(border.clone())));
    this.window.show();

    {
        let touch = this.window.touch_begin(Point::new(50.0, 50.0), RawInputModifiers::NONE);
        touch.dispose();
    }

    assert_eq!(1, capture_lost_count.get());
    assert_eq!(0, released_count.get());
}
ferro_fact!(disposing_touch_pointer_cancels_contact);
