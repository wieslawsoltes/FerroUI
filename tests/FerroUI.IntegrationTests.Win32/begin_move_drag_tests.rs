use crate::window_extensions::when_loaded;
use crate::{TestCase, WindowGuard};
use ferroui_base::input::{
    KeyModifiers, Pointer, PointerPointProperties, PointerPressedEventArgs, PointerType, PointerUpdateKind,
    RawInputModifiers,
};
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::Point;
use ferroui_controls::Window;
use std::cell::Cell;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Rc;

pub fn tests(cases: &mut Vec<TestCase>) {
    cases.push(TestCase::new(
        "begin_move_drag_tests::begin_move_drag_with_non_primary_pointer_throws_synchronously".to_string(),
        begin_move_drag_with_non_primary_pointer_throws_synchronously,
    ));
}

fn begin_move_drag_with_non_primary_pointer_throws_synchronously() {
    let window = Window::new();
    window.set_width(200.0);
    window.set_height(200.0);
    let _guard = WindowGuard(window.clone());
    window.show();
    when_loaded(&window);

    let pointer = Pointer::new(Pointer::get_next_free_id(), PointerType::Touch, false);
    let e = PointerPressedEventArgs::new(
        window.clone(),
        pointer,
        &window,
        Point::default(),
        0,
        PointerPointProperties::new(RawInputModifiers::NONE, PointerUpdateKind::LeftButtonPressed),
        KeyModifiers::NONE,
        1,
    );
    let posted_exception = Rc::new(Cell::new(false));

    let subscription = {
        let posted_exception = posted_exception.clone();
        Dispatcher::ui_thread().unhandled_exception(move |args| {
            posted_exception.set(true);
            args.set_handled(true);
        })
    };

    // The failure of the reference is an exception thrown by the call; of
    // the port, a panic of the call.
    let thrown_exception = catch_unwind(AssertUnwindSafe(|| window.begin_move_drag(&e)));
    let _ = Dispatcher::ui_thread().invoke_local_with_priority(|| {}, DispatcherPriority::BACKGROUND);

    subscription.dispose();

    assert!(!posted_exception.get(), "nothing may fail later, in a callback of the dispatcher");
    let thrown_exception = thrown_exception.expect_err("the call fails itself");
    let message = thrown_exception
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| thrown_exception.downcast_ref::<&str>().copied());
    assert_eq!(message, Some("BeginMoveDrag Failed"));
}
