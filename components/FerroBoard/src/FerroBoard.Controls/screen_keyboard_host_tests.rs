use super::*;
use crate::{test_scheduler::ManualScheduler, view_fixture::bits};
use ferroboard_core::{
    engine::{KeyboardEngine, LanguageModels},
    layouts::KeyboardResources,
    settings::KeyboardSettings,
};
use ferroui_base::{
    Rect, Size,
    animation::{PlayState, TimeSpan},
    reactive::{IObservable, IObserver, LightweightSubject},
};
use ferroui_controls::{StackPanel, TextBox, Window};
use serde_json::{Value, json};
struct Clock {
    subject: LightweightSubject<TimeSpan>,
    state: Cell<PlayState>,
}
impl Clock {
    fn new() -> Rc<Self> {
        Rc::new(Self {
            subject: LightweightSubject::new(),
            state: Cell::new(PlayState::Run),
        })
    }
    fn step(&self, ticks: i64) {
        self.subject.on_next(TimeSpan::from_ticks(ticks));
    }
}
impl IObservable<TimeSpan> for Clock {
    fn subscribe(&self, observer: Rc<dyn IObserver<TimeSpan>>) -> Rc<dyn IDisposable> {
        self.subject.subscribe(observer)
    }
}
impl IClock for Clock {
    fn play_state(&self) -> PlayState {
        self.state.get()
    }
    fn set_play_state(&self, value: PlayState) {
        self.state.set(value);
    }
}
struct Fixture {
    host: Ref<ScreenKeyboardHost>,
    clock: Rc<Clock>,
    engine: Rc<KeyboardEngine>,
}
impl Fixture {
    fn new() -> Self {
        let settings = Rc::new(KeyboardSettings::default());
        settings.set_auto_capitalization(false);
        settings.set_haptic_feedback(false);
        let engine = KeyboardEngine::new_explicit(
            settings,
            Rc::new(KeyboardResources::create_default().unwrap()),
            LanguageModels::new(),
        )
        .unwrap();
        let keyboard =
            OnScreenKeyboard::with_scheduler(Some(engine.clone()), ManualScheduler::new()).unwrap();
        let clock = Clock::new();
        let host = ScreenKeyboardHost::with_keyboard(keyboard, Some(clock.clone())).unwrap();
        Self {
            host,
            clock,
            engine,
        }
    }
    fn show(&self, width: f64, height: f64) -> Ref<Window> {
        let window = Window::new();
        window.set_width(width);
        window.set_height(height);
        window.set_content(Some(Control::boxed(self.host.clone())));
        window.show();
        Dispatcher::current_dispatcher().run_jobs(None);
        window
    }
    fn state(&self, geometry_written: bool) -> Value {
        let h = &self.host;
        json!({"floating":h.is_floating(),"open":h.is_keyboard_open(),"keyboard":vis(h.keyboard.is_visible()),"layer":vis(h.float_layer.is_visible()),"parent":if h.float_grid.children().contains(&h.keyboard.clone().upcast::<Control>()){"floating"}else{"docked"},"width":geometry_written.then(||bits(h.keyboard.width())),"frameWidth":geometry_written.then(||bits(h.float_frame.width())),"x":geometry_written.then(||bits(Canvas::get_left(&h.float_frame))),"y":geometry_written.then(||bits(Canvas::get_top(&h.float_frame))),"translate":bits(h.translate.y()),"children":h.children().snapshot().iter().map(|c|if c==&h.keyboard.clone().upcast::<Control>(){"keyboard"}else if c==&h.content_host.clone().upcast::<Control>(){"content"}else{"float"}).collect::<Vec<_>>(),"floatCount":h.float_grid.children().count()})
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.host.release_native_subscriptions().unwrap();
        self.engine.dispose().unwrap();
    }
}
fn vis(value: bool) -> &'static str {
    if value { "Visible" } else { "Collapsed" }
}
fn float(value: &Value) -> f64 {
    f64::from_bits(u64::from_str_radix(value.as_str().unwrap(), 16).unwrap())
}
#[test]
fn source_host_visibility_floating_geometry_content_and_ancestry() {
    crate::test_support::run(|| {
        let f = Fixture::new();
        let h = &f.host;
        h.set_is_animated(false);
        h.set_use_layout_rounding(false);
        h.float_frame.set_use_layout_rounding(false);
        h.measure(Size::new(800.0, 600.0));
        h.arrange(Rect::new(0.0, 0.0, 800.0, 600.0));
        let mut written = false;
        for line in include_str!("../../tests/fixtures/host.jsonl").lines() {
            let row: Value = serde_json::from_str(line).unwrap();
            match row["op"].as_str().unwrap() {
                "visibility" => {
                    h.set_visibility_mode(KeyboardVisibilityMode(
                        row["mode"].as_i64().unwrap() as i32
                    ));
                    assert_eq!(f.state(written), row["state"], "{row}");
                }
                "open" => {
                    h.set_is_keyboard_open(row["open"].as_bool().unwrap());
                    assert_eq!(f.state(written), row["state"], "{row}");
                }
                "float" => {
                    let width = float(&row["width"]);
                    h.measure(Size::new(width, 600.0));
                    h.arrange(Rect::new(0.0, 0.0, width, 600.0));
                    h.float_frame.measure(Size::new(500.0, 280.0));
                    h.float_frame.arrange(Rect::new(0.0, 0.0, 500.0, 280.0));
                    h.keyboard
                        .settings()
                        .set_floating_width_ratio(float(&row["ratio"]));
                    h.keyboard.settings().set_floating(true);
                    h.apply_floating().unwrap();
                    written = true;
                    assert_eq!(f.state(written), row["state"], "{row}");
                }
                "position" => {
                    h.float_position
                        .set(Some(Point::new(float(&row["x"]), float(&row["y"]))));
                    h.position_floating_frame();
                    assert_eq!(f.state(written), row["state"], "{row}");
                    if row["x"] == json!(bits(23.125)) {
                        h.float_position.set(None);
                    }
                }
                "height-fallback" => {
                    h.float_frame.measure(Size::new(500.0, 0.0));
                    h.float_frame.arrange(Rect::new(0.0, 0.0, 500.0, 0.0));
                    h.position_floating_frame();
                    assert_eq!(f.state(written), row["state"]);
                }
                "zero-width" => {
                    h.measure(Size::new(0.0, 600.0));
                    h.arrange(Rect::new(0.0, 0.0, 0.0, 600.0));
                    h.position_floating_frame();
                    assert_eq!(f.state(written), row["state"]);
                }
                "dock" => {
                    h.keyboard.settings().set_floating(false);
                    h.apply_floating().unwrap();
                    assert_eq!(f.state(written), row["state"]);
                }
                "content-add" => {
                    h.set_content(Some(Grid::new().upcast()));
                    assert_eq!(
                        h.content_host.children().count(),
                        row["count"].as_u64().unwrap() as usize
                    );
                }
                "content-replace" => {
                    let next = Border::new();
                    h.set_content(Some(next.clone().upcast()));
                    assert_eq!(h.content_host.children().count(), 1);
                    assert_eq!(h.content_host.children().get(0), next.upcast::<Control>());
                    assert!(row["same"].as_bool().unwrap());
                }
                "content-remove" => {
                    h.set_content(None);
                    assert_eq!(h.content_host.children().count(), 0);
                }
                "animation" => {
                    h.animate(float(&row["from"]), float(&row["to"]), None);
                    f.clock.step(0);
                    f.clock.step(1_800_000);
                    assert_eq!(bits(h.translate.y()), row["to"]);
                    assert_eq!(
                        h.translate
                            .get_base_value(TranslateTransform::y_property())
                            .unwrap_or_default(),
                        0.0
                    );
                    h.stop_storyboard();
                    assert_eq!(h.translate.y(), 0.0);
                }
                "ancestry" => {
                    let parent = Grid::new();
                    let child = Border::new();
                    if row["related"].as_bool().unwrap() {
                        parent.children().add(child.clone());
                    }
                    assert_eq!(
                        ScreenKeyboardHost::is_descendant_of(&child, &parent),
                        row["result"].as_bool().unwrap()
                    );
                }
                "bring-before" => {
                    let fail = row["fail"].as_bool().unwrap();
                    let requests = Rc::new(RefCell::new(Vec::new()));
                    let captured = requests.clone();
                    h.set_bring_into_view_handler(Rc::new(move |request| {
                        captured.borrow_mut().push(json!({"animated":request.animation_desired,"ratio":bits(request.vertical_alignment_ratio)}));
                        if fail { Err(HostIntegrationError::BringIntoViewOptionsUnavailable) } else { Ok(()) }
                    }));
                    h.bring_into_view_later(Grid::new().upcast());
                    assert_eq!(
                        requests.borrow().len(),
                        row["count"].as_u64().unwrap() as usize
                    );
                    Dispatcher::current_dispatcher().run_jobs(None);
                    assert_eq!(
                        &*requests.borrow(),
                        &[json!({"animated":true,"ratio":bits(0.5)})]
                    );
                }
                "bring-after" => {
                    assert_eq!(row["trace"], json!([{"animated":true,"ratio":bits(0.5)}]));
                }
                "deferred-close" => {
                    h.set_is_animated(false);
                    h.set_visibility_mode(KeyboardVisibilityMode(
                        row["mode"].as_i64().unwrap() as i32
                    ));
                    h.set_is_keyboard_open(true);
                    h.on_is_open_changed(true).unwrap();
                    let field = Grid::new().upcast::<Visual>();
                    *h.focused_input.borrow_mut() = Some(field.clone());
                    h.keyboard
                        .attach(Rc::new(ferroboard_core::editor::TextBufferTarget::default()))
                        .unwrap();
                    h.on_lost_focus(Some(field));
                    h.pending_close.set(row["pending"].as_bool().unwrap());
                    assert_eq!(f.state(written), row["before"]);
                    Dispatcher::current_dispatcher().run_jobs(None);
                    assert_eq!(f.state(written), row["after"]);
                    assert_eq!(
                        json!(h.keyboard.engine().target().is_none()),
                        row["detached"]
                    );
                    assert_eq!(json!(h.focused_input.borrow().is_some()), row["focused"]);
                }
                op => panic!("{op}"),
            }
        }
    });
}
#[test]
fn native_host_focus_deferred_close_and_same_field_reopen() {
    crate::test_support::run(|| {
        use ferroui_base::input::{MouseButton, RawInputModifiers};
        use ferroui_headless::HeadlessWindowExtensions;
        let f = Fixture::new();
        let h = &f.host;
        h.set_is_animated(false);
        let first = TextBox::new();
        let second = TextBox::new();
        first.set_height(36.0);
        second.set_height(36.0);
        // The bare Headless Application has no TextBox theme. A real painted native template
        // supplies a hit surface; no input hit/route is synthesized for the reopen test.
        for field in [&first, &second] {
            field.set_template(Some(
                ferroui_controls::templates::FuncControlTemplate::for_type::<TextBox>(
                    |_, scope| {
                        use ferroui_controls::templates::FuncTemplateNameScopeExtensions;
                        let border = Border::new();
                        border.set_background(Some(Brushes::transparent().into()));
                        let presenter = ferroui_controls::presenters::TextPresenter::new();
                        presenter.set_name(Some("PART_TextPresenter".into()));
                        border.set_child(presenter.register_in_name_scope(&**scope));
                        border.upcast()
                    },
                ),
            ));
        }
        let outside = Border::new();
        outside.set_focusable(true);
        outside.set_height(30.0);
        let panel = StackPanel::new();
        panel.children().add(first.clone());
        panel.children().add(second.clone());
        panel.children().add(outside.clone());
        h.set_content(Some(panel.upcast()));
        let window = f.show(800.0, 600.0);
        let requests = Rc::new(RefCell::new(Vec::new()));
        let capture = requests.clone();
        h.set_bring_into_view_handler(Rc::new(move |request| {
            capture.borrow_mut().push((
                request.element.clone(),
                request.animation_desired,
                request.vertical_alignment_ratio,
            ));
            Ok(())
        }));
        assert!(first.focus());
        assert!(h.is_keyboard_open());
        assert_eq!(h.keyboard.target_element(), Some(first.clone().upcast()));
        assert!(second.focus());
        Dispatcher::current_dispatcher().run_jobs(None);
        assert!(h.is_keyboard_open());
        assert_eq!(h.keyboard.target_element(), Some(second.clone().upcast()));
        assert!(
            requests
                .borrow()
                .iter()
                .all(|(_, animated, ratio)| *animated && *ratio == 0.5)
        );
        h.set_is_keyboard_open(false);
        let point = second
            .translate_point(Point::new(5.0, 5.0), &window)
            .unwrap();
        window.mouse_down(point, MouseButton::Left, RawInputModifiers::NONE);
        window.mouse_up(point, MouseButton::Left, RawInputModifiers::NONE);
        assert!(h.is_keyboard_open());
        assert!(second.is_focused());
        outside.focus();
        assert!(h.is_keyboard_open());
        Dispatcher::current_dispatcher().run_jobs(None);
        assert!(!h.is_keyboard_open());
        assert!(f.engine.target().is_none());
        h.set_visibility_mode(KeyboardVisibilityMode::AlwaysVisible);
        assert!(h.is_keyboard_open());
        first.focus();
        outside.focus();
        Dispatcher::current_dispatcher().run_jobs(None);
        assert!(h.is_keyboard_open());
        assert!(f.engine.target().is_none());
        h.set_visibility_mode(KeyboardVisibilityMode::Manual);
        h.set_is_keyboard_open(false);
        first.focus();
        assert!(!h.is_keyboard_open());
        assert_eq!(h.keyboard.target_element(), Some(first.clone().upcast()));
        window.close();
        Dispatcher::current_dispatcher().run_jobs(None);
    });
}
#[test]
fn native_host_animation_hold_stop_reversal_and_event_order() {
    crate::test_support::run(|| {
        let f = Fixture::new();
        let h = &f.host;
        let window = f.show(800.0, 600.0);
        let events = Rc::new(RefCell::new(Vec::new()));
        let trace = events.clone();
        h.keyboard_open_changed().add(Rc::new(move |host| {
            trace
                .borrow_mut()
                .push((host.is_keyboard_open(), host.keyboard.is_visible()))
        }));
        h.set_is_keyboard_open(true);
        let height = h.keyboard.bounds().height;
        assert!(height > 0.0);
        f.clock.step(0);
        assert_eq!(h.translate.y(), height);
        f.clock.step(900_000);
        assert!((h.translate.y() - height * 0.125).abs() < 1e-10);
        f.clock.step(1_800_000);
        assert_eq!(h.translate.y(), 0.0);
        assert!(h.keyboard.is_visible());
        assert_eq!(&*events.borrow(), &[(true, true)]);
        h.set_is_keyboard_open(false);
        assert!(h.keyboard.is_visible());
        f.clock.step(2_000_000);
        f.clock.step(2_900_000);
        assert!(h.translate.y() > 0.0);
        assert!(h.keyboard.is_visible());
        h.set_is_keyboard_open(true);
        f.clock.step(3_000_000);
        f.clock.step(4_800_000);
        assert!(h.keyboard.is_visible());
        assert!(h.is_keyboard_open());
        assert_eq!(h.translate.y(), 0.0);
        h.set_is_keyboard_open(false);
        f.clock.step(5_000_000);
        f.clock.step(6_799_999);
        assert!(h.keyboard.is_visible());
        f.clock.step(6_800_000);
        assert!(!h.keyboard.is_visible());
        assert_eq!(h.translate.y(), height);
        assert_eq!(
            h.translate
                .get_base_value(TranslateTransform::y_property())
                .unwrap_or_default(),
            0.0
        );
        h.stop_storyboard();
        assert_eq!(h.translate.y(), 0.0);
        window.close();
        Dispatcher::current_dispatcher().run_jobs(None);
    });
}
#[test]
fn native_host_floating_drag_clamps_restores_parent_and_preserves_viewport() {
    crate::test_support::run(|| {
        use ferroui_base::input::{
            IPointer, KeyModifiers, MouseButton, Pointer, PointerPointProperties,
            PointerPressedEventArgs, PointerType, RawInputModifiers,
        };
        use ferroui_headless::HeadlessWindowExtensions;
        let f = Fixture::new();
        let h = &f.host;
        h.set_is_animated(false);
        let content = Border::new();
        h.set_content(Some(content.upcast()));
        let window = f.show(800.0, 600.0);
        h.set_is_keyboard_open(true);
        h.update_layout();
        assert!(h.content_host.bounds().height < 600.0);
        let docked_height = h.content_host.bounds().height;
        h.keyboard.settings().set_floating(true);
        h.update_layout();
        assert!(h.is_floating());
        assert_eq!(
            h.keyboard.visual_parent(),
            Some(h.float_grid.clone().upcast())
        );
        assert!(h.content_host.bounds().height > docked_height);
        assert_eq!(h.content_host.bounds().height, 600.0);
        assert!(h.float_layer.is_visible());
        let width = h.keyboard.width();
        assert_eq!(h.float_frame.width(), width + 2.0);
        let point = h
            .drag_handle
            .translate_point(Point::new(10.0, 10.0), &window)
            .unwrap();
        window.mouse_down(point, MouseButton::Left, RawInputModifiers::NONE);
        assert!(h.drag_pointer.get().is_some());
        window.mouse_move(
            Point::new(point.x + 2000.0, point.y + 2000.0),
            RawInputModifiers::LEFT_MOUSE_BUTTON,
        );
        assert!(h.float_position.get().unwrap().x > 800.0);
        assert_eq!(
            Canvas::get_left(&h.float_frame),
            source_max(0.0, h.bounds().width - width - 2.0)
        );
        window.mouse_up(
            Point::new(point.x + 2000.0, point.y + 2000.0),
            MouseButton::Left,
            RawInputModifiers::NONE,
        );
        assert!(h.drag_pointer.get().is_none());
        let desired = h.float_position.get();
        h.position_floating_frame();
        assert_eq!(h.float_position.get(), desired);
        h.set_is_keyboard_open(false);
        assert!(!h.float_layer.is_visible());
        assert!(h.keyboard.is_visible());
        h.keyboard.settings().set_floating(false);
        h.update_layout();
        assert!(!h.is_floating());
        assert_eq!(h.keyboard.visual_parent(), Some(h.to_ref().upcast()));
        assert!(h.keyboard.width().is_nan());
        assert!(!h.keyboard.is_visible());
        h.keyboard.settings().set_floating(true);
        h.set_is_keyboard_open(true);
        h.update_layout();
        assert_eq!(h.float_position.get(), desired);
        let pointer = Pointer::new(811, PointerType::Touch, true);
        let point = h
            .drag_handle
            .translate_point(Point::new(10.0, 10.0), &window)
            .unwrap();
        let args = PointerPressedEventArgs::new(
            &h.drag_handle,
            pointer.clone(),
            &window,
            point,
            0,
            PointerPointProperties::default(),
            KeyModifiers::NONE,
            1,
        );
        h.drag_handle.raise_event(&args);
        assert_eq!(h.drag_pointer.get(), Some(811));
        let other = h.to_ref().upcast::<InputElement>();
        pointer.capture(Some(&other));
        assert!(h.drag_pointer.get().is_none());
        assert_eq!(pointer.captured(), Some(other));
        pointer.capture(None);
        window.close();
        Dispatcher::current_dispatcher().run_jobs(None);
    });
}

#[test]
fn native_docked_resize_keeps_press_coordinate_space_during_live_layout() {
    crate::test_support::run(|| {
        use ferroui_base::{
            input::{MouseButton, RawInputModifiers},
            media::ScaleTransform,
        };
        use ferroui_headless::HeadlessWindowExtensions;
        for visual_scale in [1.0, 1.5] {
            let f = Fixture::new();
            f.host.set_is_animated(false);
            f.engine.settings().set_key_height(60.0);
            let window = f.show(1200.0, 900.0);
            f.host.set_render_transform(Some(
                ScaleTransform::with_scale(visual_scale, visual_scale).into(),
            ));
            f.host.set_is_keyboard_open(true);
            f.host.keyboard.show_resize_overlay().unwrap();
            f.host.update_layout();
            let root = Control::from_boxed(&f.host.keyboard.content().unwrap())
                .unwrap()
                .cast::<Grid>()
                .unwrap();
            let overlay = root.children().get(2).cast::<Grid>().unwrap();
            let point = overlay
                .translate_point(Point::new(overlay.bounds().width / 2.0, 12.0), &window)
                .unwrap();
            window.mouse_down(point, MouseButton::Left, RawInputModifiers::NONE);
            let expected = [
                1.03, 1.07, 1.10, 1.13, 1.17, 1.20, 1.23, 1.27, 1.30, 1.33, 1.37, 1.40,
            ];
            for (index, expected) in expected.into_iter().enumerate() {
                window.mouse_move(
                    Point::new(point.x, point.y - (index + 1) as f64 * 8.0 * visual_scale),
                    RawInputModifiers::LEFT_MOUSE_BUTTON,
                );
                f.host.update_layout();
                assert_eq!(
                    f.engine.settings().height_scale(),
                    expected,
                    "live docked resize step {index}, transform {visual_scale}"
                );
            }
            window.mouse_up(
                Point::new(point.x, point.y - 96.0 * visual_scale),
                MouseButton::Left,
                RawInputModifiers::NONE,
            );
            window.close();
            Dispatcher::current_dispatcher().run_jobs(None);
        }
    });
}
#[test]
fn native_resize_retains_press_units_when_ancestor_transform_changes() {
    crate::test_support::run(|| {
        use ferroui_base::{
            input::{MouseButton, RawInputModifiers},
            media::ScaleTransform,
        };
        use ferroui_headless::HeadlessWindowExtensions;
        let f = Fixture::new();
        f.host.set_is_animated(false);
        f.engine.settings().set_key_height(60.0);
        let window = f.show(1200.0, 900.0);
        f.host
            .set_render_transform(Some(ScaleTransform::with_scale(0.75, 0.75).into()));
        f.host.set_is_keyboard_open(true);
        f.host.keyboard.show_resize_overlay().unwrap();
        f.host.update_layout();
        let root = Control::from_boxed(&f.host.keyboard.content().unwrap())
            .unwrap()
            .cast::<Grid>()
            .unwrap();
        let overlay = root.children().get(2).cast::<Grid>().unwrap();
        let press = overlay
            .translate_point(Point::new(200.0, 12.0), &window)
            .unwrap();
        window.mouse_down(press, MouseButton::Left, RawInputModifiers::NONE);
        f.host.set_render_transform(None);
        f.host.update_layout();
        assert_eq!(f.engine.settings().height_scale(), 1.0);
        let moved = Point::new(press.x, press.y - 48.0);
        window.mouse_move(moved, RawInputModifiers::LEFT_MOUSE_BUTTON);
        f.host.update_layout();
        // 48 window DIPs / press scale 0.75 = 64 keyboard DIPs, not 48.
        assert_eq!(f.engine.settings().height_scale(), 1.27);
        window.mouse_up(moved, MouseButton::Left, RawInputModifiers::NONE);
        f.engine.settings().set_height_scale(1.0);
        f.host.update_layout();
        let fresh = overlay
            .translate_point(Point::new(200.0, 12.0), &window)
            .unwrap();
        window.mouse_down(fresh, MouseButton::Left, RawInputModifiers::NONE);
        let moved = Point::new(fresh.x, fresh.y - 48.0);
        window.mouse_move(moved, RawInputModifiers::LEFT_MOUSE_BUTTON);
        f.host.update_layout();
        // A new unscaled press must discard the previous transform basis.
        assert_eq!(f.engine.settings().height_scale(), 1.20);
        window.mouse_up(moved, MouseButton::Left, RawInputModifiers::NONE);
        window.close();
        Dispatcher::current_dispatcher().run_jobs(None);
    });
}
#[test]
fn native_host_scopes_target_to_content_and_releases_queued_work() {
    crate::test_support::run(|| {
        let f = Fixture::new();
        let h = &f.host;
        h.set_is_animated(false);
        h.set_height(500.0);
        let input = TextBox::new();
        input.set_height(36.0);
        h.set_content(Some(input.clone().upcast()));
        let sibling = TextBox::new();
        sibling.set_height(36.0);
        let outer = StackPanel::new();
        outer.children().add(sibling.clone());
        outer.children().add(h.clone());
        let window = Window::new();
        window.set_width(800.0);
        window.set_height(600.0);
        window.set_content(Some(Control::boxed(outer)));
        window.show();
        Dispatcher::current_dispatcher().run_jobs(None);
        let failures = Rc::new(Cell::new(0));
        let count = failures.clone();
        h.integration_failed().add(Rc::new(move |error| {
            assert_eq!(
                *error,
                HostIntegrationError::BringIntoViewOptionsUnavailable
            );
            count.set(count.get() + 1);
        }));
        assert!(input.focus());
        Dispatcher::current_dispatcher().run_jobs(None);
        assert_eq!(failures.get(), 0);
        assert!(h.is_keyboard_open());
        sibling.focus();
        Dispatcher::current_dispatcher().run_jobs(None);
        assert!(!h.is_keyboard_open());
        assert!(h.keyboard.target_element().is_none());
        input.focus();
        let other = Window::new();
        let other_input = TextBox::new();
        other.set_content(Some(Control::boxed(other_input.clone())));
        other.show();
        other_input.focus();
        Dispatcher::current_dispatcher().run_jobs(None);
        assert!(f.engine.target().is_none());
        assert!(!h.is_keyboard_open());
        other.close();
        window.set_content(None);
        window.close();
        Dispatcher::current_dispatcher().run_jobs(None);
        let weak = h.downgrade();
        h.bring_into_view_later(input.upcast());
        h.release_native_subscriptions().unwrap();
        h.release_native_subscriptions().unwrap();
        drop(f);
        assert!(
            weak.upgrade().is_none(),
            "queued scrolling must not own the host"
        );
        Dispatcher::current_dispatcher().run_jobs(None);
        assert!(weak.upgrade().is_none());
    });
}
#[test]
fn native_host_original_transform_identity_and_reentrant_stop() {
    crate::test_support::run(|| {
        let f = Fixture::new();
        let h = &f.host;
        let replacement = TranslateTransform::with_offset(0.0, 123.0);
        h.keyboard.set_render_transform(Some((&replacement).into()));
        h.animate(20.0, 40.0, None);
        f.clock.step(0);
        f.clock.step(900_000);
        assert_eq!(h.translate.y(), 37.5);
        assert_eq!(replacement.y(), 123.0);
        f.clock.step(1_800_000);
        assert_eq!(h.translate.y(), 40.0);
        assert_eq!(
            h.translate.get_base_value(TranslateTransform::y_property()),
            None
        );
        h.stop_storyboard();
        assert_eq!(h.translate.y(), 0.0);
        assert_eq!(replacement.y(), 123.0);
        let host = h.downgrade();
        let observed = Rc::new(Cell::new(false));
        let flag = observed.clone();
        let token = h.translate.property_changed(move |change| {
            if change.property() == TranslateTransform::y_property().as_property()
                && change.get_new_value::<f64>() == 10.0
                && !flag.replace(true)
            {
                host.upgrade().unwrap().stop_storyboard();
            }
        });
        let completed = Rc::new(Cell::new(false));
        let done = completed.clone();
        h.animate(0.0, 10.0, Some(Rc::new(move || done.set(true))));
        f.clock.step(2_000_000);
        f.clock.step(3_800_000);
        assert!(observed.get());
        assert!(!completed.get());
        assert_eq!(h.translate.y(), 0.0);
        token.dispose();
    });
}
