use super::*;
use ferroui_base::input::{Pointer, PointerType, PointerUpdateKind, RawInputModifiers};
use ferroui_base::layout::ILayoutManager;
use ferroui_base::threading::{
    Dispatcher, DispatcherImplEvent, IDispatcherImpl, IDispatcherSignal,
};
use ferroui_controls::{Border, Window};
use std::cell::Cell;
use std::sync::Arc;

#[derive(Default)]
struct Clock {
    now: Cell<i64>,
    due: Cell<Option<i64>>,
    timer: DispatcherImplEvent,
    signaled: DispatcherImplEvent,
}

struct Wake;
impl IDispatcherSignal for Wake {
    fn signal(&self) {}
}

impl IDispatcherImpl for Clock {
    fn current_thread_is_loop_thread(&self) -> bool {
        true
    }
    fn signal(&self) {}
    fn signal_handle(&self) -> Arc<dyn IDispatcherSignal> {
        Arc::new(Wake)
    }
    fn signaled(&self) -> &DispatcherImplEvent {
        &self.signaled
    }
    fn timer(&self) -> &DispatcherImplEvent {
        &self.timer
    }
    fn now(&self) -> i64 {
        self.now.get()
    }
    fn update_timer(&self, due: Option<i64>) {
        self.due.set(due);
    }
}

impl Clock {
    fn advance(&self, elapsed: i64) {
        self.now.set(self.now.get() + elapsed);
        if self.due.get().is_some_and(|due| due <= self.now.get()) {
            self.timer.raise();
        }
        Dispatcher::ui_thread().run_jobs(None);
    }
}

fn pointer_event(
    target: &Control,
    window: &Window,
    pointer: &Rc<dyn IPointer>,
    event: &RoutedEvent<PointerEventArgs>,
    local: Point,
    timestamp: u64,
) {
    let root = target.translate_point(local, window).unwrap();
    let args = PointerEventArgs::new(
        Some(event),
        target.to_ref(),
        pointer.clone(),
        Some(window),
        root,
        timestamp,
        PointerPointProperties::new(RawInputModifiers::NONE, PointerUpdateKind::Other),
        KeyModifiers::SHIFT,
    );
    target.raise_event(&args);
    assert!(!args.handled());
    assert_eq!(args.routed_event(), Some(event.as_routed_event()));
}

#[test]
fn text_area_hover_routes_at_the_original_point_and_restarts_after_motion() {
    let _scope = crate::tests::test_scope();
    let clock = Rc::new(Clock::default());
    Dispatcher::initialize_ui_thread_dispatcher(clock.clone());
    let editor = crate::tests::laid_out_editor("first\nsecond");
    let area = editor.text_area();
    let window = Window::new();
    window.set_width(300.0);
    window.set_height(140.0);
    window.set_content(Some(Control::boxed(editor)));
    window.show();
    window.layout_manager().execute_layout_pass();
    let pointer: Rc<dyn IPointer> =
        Pointer::new(Pointer::get_next_free_id(), PointerType::Mouse, true);

    let seen = Rc::new(RefCell::new(Vec::new()));
    let record = |kind, route| {
        let seen = seen.clone();
        let source = area.downgrade();
        move |_: &ferroui_base::interactivity::Interactive, args: &PointerEventArgs| {
            let source = source.upgrade().unwrap();
            assert!(args.is_source(&source));
            assert_eq!(args.route(), route);
            assert_eq!(args.key_modifiers(), KeyModifiers::SHIFT);
            seen.borrow_mut()
                .push((kind, args.get_position(Some(&source)), args.timestamp()));
        }
    };
    let _preview = window.add_disposable_handler_with(
        TextArea::preview_pointer_hover_event(),
        record("preview", RoutingStrategies::TUNNEL),
        RoutingStrategies::TUNNEL,
        false,
    );
    let _hover = area.add_disposable_handler(
        TextArea::pointer_hover_event(),
        record("hover", RoutingStrategies::BUBBLE),
    );
    let _preview_stop = window.add_disposable_handler_with(
        TextArea::preview_pointer_hover_stopped_event(),
        record("preview stop", RoutingStrategies::TUNNEL),
        RoutingStrategies::TUNNEL,
        false,
    );
    let _stop = area.add_disposable_handler(
        TextArea::pointer_hover_stopped_event(),
        record("stop", RoutingStrategies::BUBBLE),
    );

    let start = Point::new(20.0, 22.0);
    let next = Point::new(29.0, 27.0);
    pointer_event(
        &area,
        &window,
        &pointer,
        InputElement::pointer_entered_event(),
        start,
        1,
    );
    clock.advance(140);
    pointer_event(
        &area,
        &window,
        &pointer,
        InputElement::pointer_moved_event(),
        Point::new(22.0, 24.0),
        2,
    );
    clock.advance(259);
    assert!(seen.borrow().is_empty());
    clock.advance(1);
    assert_eq!(
        &*seen.borrow(),
        &[("preview", start, 1), ("hover", start, 1)]
    );
    pointer_event(
        &area,
        &window,
        &pointer,
        InputElement::pointer_moved_event(),
        next,
        3,
    );
    assert_eq!(
        &seen.borrow()[2..],
        &[("preview stop", start, 1), ("stop", start, 1)]
    );
    clock.advance(400);
    assert_eq!(
        &seen.borrow()[4..],
        &[("preview", next, 3), ("hover", next, 3)]
    );
    pointer_event(
        &area,
        &window,
        &pointer,
        InputElement::pointer_exited_event(),
        next,
        4,
    );
    clock.advance(800);
    assert_eq!(
        &seen.borrow()[6..],
        &[("preview stop", next, 3), ("stop", next, 3)]
    );

    drop((_preview, _hover, _preview_stop, _stop));
    let finished = Rc::new(Cell::new(false));
    let _finished = TextArea::pointer_hover_event().route_finished().subscribe({
        let finished = finished.clone();
        move |_| finished.set(true)
    });
    pointer_event(
        &area,
        &window,
        &pointer,
        InputElement::pointer_entered_event(),
        start,
        5,
    );
    clock.advance(400);
    assert!(finished.get());
    window.close();
}

#[test]
fn detach_and_dropping_a_standalone_attachment_cancel_pending_hover() {
    let _scope = crate::tests::test_scope();
    let clock = Rc::new(Clock::default());
    Dispatcher::initialize_ui_thread_dispatcher(clock.clone());
    let window = Window::new();
    let control = Border::new();
    window.set_content(Some(Control::boxed(control.clone())));
    window.show();
    window.layout_manager().execute_layout_pass();
    let calls = Rc::new(Cell::new(0));
    let seen = calls.clone();
    let tracker = PointerHoverLogic::attach(&control, move |_| seen.set(seen.get() + 1), |_| {});
    let pointer: Rc<dyn IPointer> =
        Pointer::new(Pointer::get_next_free_id(), PointerType::Mouse, true);
    let point = Point::new(4.0, 5.0);
    pointer_event(
        &control,
        &window,
        &pointer,
        InputElement::pointer_entered_event(),
        point,
        1,
    );
    window.set_content(None);
    clock.advance(400);
    assert_eq!(calls.get(), 0);

    window.set_content(Some(Control::boxed(control.clone())));
    window.layout_manager().execute_layout_pass();
    pointer_event(
        &control,
        &window,
        &pointer,
        InputElement::pointer_entered_event(),
        point,
        2,
    );
    clock.advance(400);
    assert_eq!(calls.get(), 1);
    pointer_event(
        &control,
        &window,
        &pointer,
        InputElement::pointer_entered_event(),
        point,
        3,
    );
    drop(tracker);
    clock.advance(800);
    assert_eq!(calls.get(), 1);
    window.close();
}
