//! Port of `MouseDeviceTests.cs` of the upstream unit test project of the
//! headless platform.

use super::popup_tests::get_popup_top_level;
use super::test_application::{ferro_fact, TestApplication};
use crate::HeadlessWindowExtensions;
use ferroui_base::input::{IPointer, InputElement, MouseButton, RawInputModifiers};
use ferroui_base::interactivity::Interactive;
use ferroui_base::media::Brushes;
use ferroui_base::threading::Dispatcher;
use ferroui_base::{Point, Ref};
use ferroui_controls::primitives::Popup;
use ferroui_controls::{Border, Control, Panel, Window};
use std::cell::RefCell;
use std::rc::Rc;

fn pointer_is_shared_between_windows_when_requested() {
    let (first_window, first_target) = create_window();
    let (second_window, second_target) = create_window();

    let first_pointer: Rc<RefCell<Option<Rc<dyn IPointer>>>> = Rc::new(RefCell::new(None));
    let second_pointer: Rc<RefCell<Option<Rc<dyn IPointer>>>> = Rc::new(RefCell::new(None));
    first_target.add_handler(InputElement::pointer_pressed_event(), {
        let first_pointer = first_pointer.clone();
        move |_, e| *first_pointer.borrow_mut() = Some(e.pointer().clone())
    });
    second_target.add_handler(InputElement::pointer_pressed_event(), {
        let second_pointer = second_pointer.clone();
        move |_, e| *second_pointer.borrow_mut() = Some(e.pointer().clone())
    });

    click(&first_window);
    click(&second_window);

    assert!(first_pointer.borrow().is_some());
    assert!(second_pointer.borrow().is_some());
    let first_pointer = first_pointer.borrow().clone().unwrap();
    let second_pointer = second_pointer.borrow().clone().unwrap();

    if TestApplication::uses_shared_mouse_device() {
        assert!(Rc::ptr_eq(&first_pointer, &second_pointer));
    } else {
        assert!(!Rc::ptr_eq(&first_pointer, &second_pointer));
    }

    first_window.close();
    second_window.close();
}
ferro_fact!(pointer_is_shared_between_windows_when_requested);

fn pointer_capture_crosses_top_levels_when_device_is_shared() {
    let popup_child = Border::new();
    popup_child.set_width(80.0);
    popup_child.set_height(30.0);
    popup_child.set_background(Some(Brushes::blue()));
    let target = Border::new();
    target.set_background(Some(Brushes::red()));
    let popup = Popup::new();
    popup.set_placement_target(target.clone());
    popup.set_child(popup_child.clone());
    let panel = Panel::new();
    panel.children().add(&target);
    panel.children().add(&popup);
    let window = Window::new();
    window.set_width(100.0);
    window.set_height(100.0);
    window.set_content(Some(Control::boxed(panel)));
    window.show();
    Dispatcher::ui_thread().run_jobs(None);

    popup.open();
    Dispatcher::ui_thread().run_jobs(None);

    let move_target: Rc<RefCell<Option<Ref<Interactive>>>> = Rc::new(RefCell::new(None));
    target.add_handler(InputElement::pointer_moved_event(), {
        let move_target = move_target.clone();
        move |s, _| *move_target.borrow_mut() = Some(s.to_ref())
    });
    popup_child.add_handler(InputElement::pointer_moved_event(), {
        let move_target = move_target.clone();
        move |s, _| *move_target.borrow_mut() = Some(s.to_ref())
    });

    // Pressing captures the pointer implicitly on the window's border.
    window.mouse_down(Point::new(50.0, 50.0), MouseButton::Left, RawInputModifiers::NONE);

    let popup_root = get_popup_top_level(&popup);
    assert!(popup_root.is_some());
    let popup_root = popup_root.unwrap();

    popup_root.mouse_move(Point::new(40.0, 15.0), RawInputModifiers::NONE);

    let expected = if TestApplication::uses_shared_mouse_device() { &target } else { &popup_child };
    assert!(move_target.borrow().as_ref().is_some_and(|move_target| expected.ptr_eq(move_target)));

    window.mouse_up(Point::new(50.0, 50.0), MouseButton::Left, RawInputModifiers::NONE);
    window.close();
}
ferro_fact!(pointer_capture_crosses_top_levels_when_device_is_shared);

fn create_window() -> (Ref<Window>, Ref<Border>) {
    let target = Border::new();
    target.set_background(Some(Brushes::red()));
    let window = Window::new();
    window.set_width(100.0);
    window.set_height(100.0);
    window.set_content(Some(Control::boxed(target.clone())));
    window.show();
    Dispatcher::ui_thread().run_jobs(None);
    (window, target)
}

fn click(window: &Window) {
    window.mouse_down(Point::new(50.0, 50.0), MouseButton::Left, RawInputModifiers::NONE);
    window.mouse_up(Point::new(50.0, 50.0), MouseButton::Left, RawInputModifiers::NONE);
}
