use super::*;
use ferroui_base::input::{
    Pointer, PointerPointProperties, PointerType, PointerUpdateKind, RawInputModifiers,
};
use ferroui_base::{Size, Vector};
use ferroui_controls::Window;
use ferroui_controls::testing::{TestServices, UnitTestApplication};

fn key(lane: &TimelineLane, key: Key, modifiers: KeyModifiers) -> KeyEventArgs {
    let mut event = KeyEventArgs::new();
    event.key = key;
    event.key_modifiers = modifiers;
    event.set_routed_event(Some(InputElement::key_down_event()));
    lane.raise_event(&event);
    event
}

struct PointerDriver {
    window: Ref<Window>,
    lane: Ref<TimelineLane>,
    pointer: Rc<Pointer>,
}
impl PointerDriver {
    fn new(lane: Ref<TimelineLane>) -> Self {
        let window = Window::new();
        window.set_width(424.0);
        window.set_height(80.0);
        window.set_content(Some(Control::boxed(&lane)));
        window.show();
        window.update_layout();
        assert!(lane.bounds().width > 24.0);
        Self {
            window,
            lane,
            pointer: Pointer::new(Pointer::get_next_free_id(), PointerType::Mouse, true),
        }
    }
    fn position(&self, time: f64) -> Point {
        self.lane
            .translate_point(Point::new(self.lane.x(time), 17.0), &self.window)
            .unwrap()
    }
    fn press(&self, time: f64, count: i32) {
        self.lane.raise_event(&PointerPressedEventArgs::new(
            self.lane.to_ref(),
            self.pointer.clone(),
            &self.window,
            self.position(time),
            1,
            PointerPointProperties::new(
                RawInputModifiers::LEFT_MOUSE_BUTTON,
                PointerUpdateKind::LeftButtonPressed,
            ),
            KeyModifiers::NONE,
            count,
        ));
    }
    fn move_to(&self, time: f64) {
        self.lane.raise_event(&PointerEventArgs::new(
            Some(InputElement::pointer_moved_event()),
            self.lane.to_ref(),
            self.pointer.clone(),
            Some(&self.window),
            self.position(time),
            2,
            PointerPointProperties::new(
                RawInputModifiers::LEFT_MOUSE_BUTTON,
                PointerUpdateKind::Other,
            ),
            KeyModifiers::NONE,
        ));
    }
    fn release(&self, time: f64) {
        self.lane.raise_event(&PointerReleasedEventArgs::new(
            self.lane.to_ref(),
            self.pointer.clone(),
            &self.window,
            self.position(time),
            3,
            PointerPointProperties::new(
                RawInputModifiers::NONE,
                PointerUpdateKind::LeftButtonReleased,
            ),
            KeyModifiers::NONE,
            MouseButton::Left,
        ));
    }
}
impl Drop for PointerDriver {
    fn drop(&mut self) {
        self.pointer.capture(None);
        self.window.close();
    }
}

#[test]
fn ruler_and_tracks_use_the_same_zoomed_axis_and_precision() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let lane = TimelineLane::new();
    let ruler = TimelineLane::ruler();
    for control in [&lane, &ruler] {
        control.configure(10.0, vec![f64::NAN, 12.0, -1.0, 2.5, 2.5, 0.0]);
        control.set_view(2.0, 4.0);
        control.measure(Size::new(424.0, 34.0));
        control.arrange(Rect::new(0.0, 0.0, 424.0, 34.0));
    }
    assert_eq!(*lane.keys.borrow(), [0.0, 2.5]);
    for time in [2.0, 2.5, 2.73, 4.0] {
        assert_eq!(lane.x(time), ruler.x(time));
        assert!((ruler.time(lane.x(time), KeyModifiers::ALT) - time).abs() < 1e-10);
    }
    assert_eq!(lane.key_at(lane.x(2.5)), Some(2.5));
    assert_eq!(
        lane.key_at(lane.x(0.0)),
        None,
        "Keys outside the viewport are not selectable"
    );
    assert!((lane.time(lane.x(2.73), KeyModifiers::CONTROL) - 2.73).abs() < 1e-10);
    assert!((lane.time(lane.x(2.73), KeyModifiers::NONE) - 2.7).abs() < 1e-10);
    lane.configure(0.001, vec![]);
    assert!((lane.time(lane.x(0.0006), KeyModifiers::NONE) - 0.0006).abs() < 1e-10);
    lane.set_view(f64::NAN, f64::INFINITY);
    assert_eq!(lane.view_start.get(), 0.0);
    assert_eq!(lane.view_end.get(), 0.001);
    for span in [f64::MIN_POSITIVE, 0.00001, 4.0, 1e100, f64::MAX] {
        let interval = tick_interval(span, 424.0);
        assert!(interval.is_finite() && interval > 0.0, "{span}: {interval}");
    }
    assert_eq!(format_time(0.001, 0.001), "0.001s");
    assert_eq!(format_time(0.002, 0.001), "0.002s");
    assert_eq!(format_time(60.001, 0.001), "1:00.001");
}

#[test]
fn timeline_keyframe_drag_commits_once_and_capture_loss_or_escape_cancels() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let lane = TimelineLane::new();
    lane.configure(4.0, vec![1.0]);
    let moves = Rc::new(RefCell::new(Vec::new()));
    let observed = moves.clone();
    lane.on_move(move |from, to| observed.borrow_mut().push((from, to)));
    let driver = PointerDriver::new(lane);
    driver.press(1.0, 1);
    assert!(driver.pointer.captured().is_some());
    driver.move_to(2.0);
    assert!(
        moves.borrow().is_empty(),
        "Dragging changes only the ghost marker"
    );
    driver.release(2.0);
    assert_eq!(*moves.borrow(), [(1.0, 2.0)]);
    assert!(driver.pointer.captured().is_none());
    driver.press(1.0, 1);
    driver.move_to(3.0);
    driver.pointer.platform_capture_lost();
    driver.release(3.0);
    assert_eq!(moves.borrow().len(), 1);
    driver.press(1.0, 1);
    driver.move_to(3.0);
    assert!(key(&driver.lane, Key::Escape, KeyModifiers::NONE).handled());
    assert!(
        driver.pointer.captured().is_none(),
        "Escape releases pointer capture immediately"
    );
    driver.release(3.0);
    assert_eq!(moves.borrow().len(), 1);
}

#[test]
fn timeline_keyboard_nudging_deletion_scrubbing_and_zoom_are_host_actions() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let lane = TimelineLane::new();
    lane.configure(4.0, vec![1.0]);
    lane.set_selected(Some(1.0));
    let moves = Rc::new(RefCell::new(Vec::new()));
    let observed = moves.clone();
    lane.on_move(move |from, to| observed.borrow_mut().push((from, to)));
    let removals = Rc::new(RefCell::new(Vec::new()));
    let observed = removals.clone();
    lane.on_remove(move |time| observed.borrow_mut().push(time));
    assert!(key(&lane, Key::Right, KeyModifiers::CONTROL).handled());
    assert!(key(&lane, Key::Left, KeyModifiers::SHIFT).handled());
    assert_eq!(*moves.borrow(), [(1.0, 1.01), (1.0, 0.0)]);
    assert!(key(&lane, Key::Delete, KeyModifiers::NONE).handled());
    assert_eq!(*removals.borrow(), [1.0]);
    let ruler = TimelineLane::ruler();
    ruler.configure(4.0, vec![]);
    ruler.set_playhead(1.0);
    assert!(
        key(&ruler, Key::Delete, KeyModifiers::NONE).handled(),
        "Timeline focus must not route Delete to the artboard selection"
    );
    let seeks = Rc::new(RefCell::new(Vec::new()));
    let observed = seeks.clone();
    ruler.on_activate(move |time| observed.borrow_mut().push(time));
    key(&ruler, Key::Home, KeyModifiers::NONE);
    key(&ruler, Key::End, KeyModifiers::NONE);
    key(&ruler, Key::Right, KeyModifiers::CONTROL);
    assert_eq!(*seeks.borrow(), [0.0, 4.0, 1.01]);
    let zooms = Rc::new(RefCell::new(Vec::new()));
    let observed = zooms.clone();
    ruler.on_zoom(move |factor, pivot| observed.borrow_mut().push((factor, pivot)));
    let pans = Rc::new(RefCell::new(Vec::new()));
    let observed = pans.clone();
    ruler.on_pan(move |seconds| observed.borrow_mut().push(seconds));
    assert!(key(&ruler, Key::PageDown, KeyModifiers::NONE).handled());
    assert_eq!(*pans.borrow(), [3.2]);
    let driver = PointerDriver::new(ruler);
    let wheel = PointerWheelEventArgs::new(
        driver.lane.to_ref(),
        driver.pointer.clone(),
        &driver.window,
        driver.position(3.0),
        4,
        PointerPointProperties::new(RawInputModifiers::NONE, PointerUpdateKind::Other),
        KeyModifiers::CONTROL,
        Vector::new(0.0, 1.0),
    );
    driver.lane.raise_event(&wheel);
    assert!(wheel.handled());
    assert_eq!(*zooms.borrow(), [(1.25, 3.0)]);
}

#[test]
fn ruler_scrubs_continuously_and_double_clicking_a_track_requests_a_keyframe() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let ruler = TimelineLane::ruler();
    ruler.configure(4.0, vec![]);
    let seeks = Rc::new(RefCell::new(Vec::new()));
    let observed = seeks.clone();
    ruler.on_activate(move |time| observed.borrow_mut().push(time));
    let driver = PointerDriver::new(ruler);
    driver.press(1.0, 1);
    driver.move_to(2.0);
    driver.release(3.0);
    assert_eq!(*seeks.borrow(), [1.0, 2.0, 3.0]);
    drop(driver);
    let lane = TimelineLane::new();
    lane.configure(4.0, vec![]);
    let inserts = Rc::new(RefCell::new(Vec::new()));
    let observed = inserts.clone();
    lane.on_insert(move |time| observed.borrow_mut().push(time));
    let driver = PointerDriver::new(lane);
    driver.press(2.0, 2);
    assert_eq!(*inserts.borrow(), [2.0]);
    assert!(driver.pointer.captured().is_none());
}
