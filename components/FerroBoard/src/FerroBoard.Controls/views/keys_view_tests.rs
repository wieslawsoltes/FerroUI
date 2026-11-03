use super::*;
use ferroboard_core::input::ScheduledAction;
use ferroboard_core::{
    editor::{ITextInputTarget, TextBufferTarget},
    engine::LanguageModels,
    glide::GesturePoint,
    keys::KeyboardMode,
    layouts::KeyboardResources,
    settings::KeyboardSettings,
};
use ferroui_base::{
    input::{
        IntermediatePoints, KeyModifiers, MouseButton, Pointer, PointerPointProperties,
        PointerType, raw::RawPointerPoint,
    },
    threading::Dispatcher,
};
use ferroui_controls::{Control, Window};
use serde_json::{Value, json};
use std::time::Duration;
use std::{cell::LazyCell, rc::Weak};
#[path = "key_accessibility_tests.rs"]
mod native_accessibility;
struct Entry {
    due: i64,
    action: Rc<dyn Fn()>,
}
struct Clock {
    weak: Weak<Self>,
    now: Cell<i64>,
    next: Cell<u64>,
    entries: RefCell<Vec<(u64, Entry)>>,
}
impl Clock {
    fn new() -> Rc<Self> {
        Rc::new_cyclic(|weak| Self {
            weak: weak.clone(),
            now: Cell::new(0),
            next: Cell::new(0),
            entries: RefCell::new(Vec::new()),
        })
    }
    fn advance(&self, ticks: i64) {
        let end = self.now.get() + ticks;
        loop {
            let next = self
                .entries
                .borrow()
                .iter()
                .enumerate()
                .filter(|(_, (_, e))| e.due <= end)
                .min_by_key(|(_, (_, e))| e.due)
                .map(|(i, _)| i);
            let Some(i) = next else {
                break;
            };
            let (_, e) = self.entries.borrow_mut().remove(i);
            self.now.set(e.due);
            (e.action)();
        }
        self.now.set(end);
    }
}
impl IKeyboardScheduler for Clock {
    fn schedule(&self, delay: Duration, action: Rc<dyn Fn()>) -> ScheduledAction {
        let id = self.next.get();
        self.next.set(id + 1);
        self.entries.borrow_mut().push((
            id,
            Entry {
                due: self.now.get() + i64::try_from(delay.as_nanos() / 100).unwrap(),
                action,
            },
        ));
        let weak = self.weak.clone();
        ScheduledAction::new(move || {
            if let Some(this) = weak.upgrade() {
                this.entries.borrow_mut().retain(|(i, _)| *i != id);
            }
        })
    }
}
struct Fixture {
    engine: Rc<KeyboardEngine>,
    target: Rc<TextBufferTarget>,
    touch: Rc<KeyboardTouchProcessor>,
    clock: Rc<Clock>,
    view: Ref<KeysView>,
}
impl Fixture {
    fn new() -> Self {
        let settings = Rc::new(KeyboardSettings::default());
        settings.set_auto_capitalization(false);
        let engine = KeyboardEngine::new_explicit(
            settings,
            Rc::new(KeyboardResources::create_default().unwrap()),
            LanguageModels::new(),
        )
        .unwrap();
        let target = Rc::new(TextBufferTarget::default());
        engine.attach(target.clone()).unwrap();
        engine.update_layout(1000.0, 240.0, None).unwrap();
        let clock = Clock::new();
        let touch = KeyboardTouchProcessor::new(engine.clone(), clock.clone());
        let view = KeysView::new(engine.clone(), touch.clone(), clock.clone());
        Self {
            engine,
            target,
            touch,
            clock,
            view,
        }
    }
    fn key(&self, label: &str) -> Rc<ComputedKey> {
        self.engine
            .keyboard()
            .unwrap()
            .visible_keys()
            .map(|key| key.unwrap().unwrap())
            .find(|k| k.label() == Some(label.into()))
            .unwrap()
    }
    fn show(&self) -> Ref<Window> {
        self.view.set_width(1000.0);
        self.view.set_height(240.0);
        let window = Window::new();
        window.set_width(1000.0);
        window.set_height(240.0);
        window.set_content(Some(Control::boxed(self.view.clone())));
        window.show();
        Dispatcher::current_dispatcher().run_jobs(None);
        self.view.set_theme(KeyboardTheme::new()).unwrap();
        window
    }
    fn down(&self, w: &Ref<Window>, p: Rc<Pointer>, point: Point) {
        let pos = self.view.translate_point(point, w).unwrap();
        let args = PointerPressedEventArgs::new(
            &self.view,
            p,
            w,
            pos,
            0,
            PointerPointProperties {
                is_left_button_pressed: true,
                ..Default::default()
            },
            KeyModifiers::empty(),
            1,
        );
        self.view.raise_event(&args);
        assert!(args.handled());
    }
    fn up(&self, w: &Ref<Window>, p: Rc<Pointer>, point: Point) {
        let pos = self.view.translate_point(point, w).unwrap();
        let args = PointerReleasedEventArgs::new(
            &self.view,
            p,
            w,
            pos,
            0,
            PointerPointProperties::default(),
            KeyModifiers::empty(),
            MouseButton::Left,
        );
        self.view.raise_event(&args);
        assert!(args.handled());
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.touch.cancel_all().unwrap();
        self.view.release_subscriptions();
        self.clock.entries.borrow_mut().clear();
        self.engine.dispose().unwrap();
    }
}
fn center(key: &ComputedKey) -> Point {
    let b = key.touch_bounds();
    Point::new(b.center_x(), b.center_y())
}
fn state(f: &Fixture) -> Value {
    let points = f.view.trail.points().unwrap();
    json!({"opacity":format!("{:016x}",f.view.trail.opacity().to_bits()),"visible":f.view.trail.is_visible(),"points":points.iter().map(|p|[format!("{:016x}",p.x.to_bits()),format!("{:016x}",p.y.to_bits())]).collect::<Vec<_>>(),"now":f.clock.now.get(),"pending":f.clock.entries.borrow().len()})
}
#[test]
fn frozen_unchanged_source_trail_and_timer_boundaries() {
    crate::test_support::run(|| {
        let f = Fixture::new();
        for line in include_str!("../../../tests/fixtures/keys-view.jsonl").lines() {
            f.clock.now.set(0);
            f.touch.clear_glide_trail();
            let source: Value = serde_json::from_str(line).unwrap();
            let duration = source["duration"].as_i64().unwrap() as i32;
            let count = source["count"].as_u64().unwrap() as usize;
            f.engine.settings().set_glide_trail_duration(duration);
            for i in 0..count {
                f.touch.glide_trail().borrow_mut().push(GesturePoint {
                    x: i as f64 + 0.25,
                    y: -(i as f64) - 0.5,
                });
            }
            f.view.update_trail();
            assert_eq!(state(&f), source["states"][0]);
            let old_points = f.view.trail.points().unwrap();
            f.view.schedule_trail_fade();
            assert_eq!(state(&f), source["states"][1]);
            let interval = i64::from((duration / 6).max(1)) * 10_000;
            for i in 0..6 {
                let previous = state(&f);
                f.clock.advance(interval - 1);
                let mut expected = previous;
                expected["now"] = json!(f.clock.now.get());
                assert_eq!(state(&f), expected);
                f.clock.advance(1);
                assert_eq!(
                    state(&f),
                    source["states"][i + 2],
                    "duration={duration}, count={count}, step={i}"
                );
            }
            assert_eq!(old_points.len(), count.min(80));
            assert_eq!(f.view.trail.points().unwrap().len(), 0);
        }
    });
}
#[test]
fn native_capture_release_cancellation_and_visual_event_order() {
    crate::test_support::run(|| {
        let f = Fixture::new();
        let w = f.show();
        let key = f.key("a");
        let point = center(&key);
        let events = Rc::new(RefCell::new(Vec::new()));
        let e = events.clone();
        let token = f.view.key_pressed_visual(move |v, key| {
            assert!(
                v.views
                    .borrow()
                    .iter()
                    .find(|(k, _)| Rc::ptr_eq(&k.0, key))
                    .unwrap()
                    .1
                    .is_pressed()
            );
            e.borrow_mut().push("press");
        });
        let e = events.clone();
        let token2 = f.view.key_released_visual(move |v, key| {
            assert!(
                !v.views
                    .borrow()
                    .iter()
                    .find(|(k, _)| Rc::ptr_eq(&k.0, key))
                    .unwrap()
                    .1
                    .is_pressed()
            );
            e.borrow_mut().push("release");
        });
        for (id, kind) in [
            (11, PointerType::Mouse),
            (12, PointerType::Pen),
            (13, PointerType::Touch),
        ] {
            let p = Pointer::new(id, kind, true);
            f.down(&w, p.clone(), point);
            assert_eq!(p.captured(), Some(f.view.clone().upcast::<InputElement>()));
            assert!(f.target.text().len() < 3);
            f.up(&w, p.clone(), point);
            assert!(p.captured().is_none());
            assert!(!f.view.captured_pointers.borrow().contains(&id));
        }
        assert_eq!(f.target.text(), "aaa".into());
        assert_eq!(
            &*events.borrow(),
            &["press", "release", "press", "release", "press", "release"]
        );
        let p = Pointer::new(14, PointerType::Touch, true);
        f.down(&w, p.clone(), point);
        p.capture(None);
        assert_eq!(f.target.text(), "aaa".into());
        assert!(f.touch.pressed_keys().iter().next().is_none());
        assert!(!f.view.captured_pointers.borrow().contains(&14));
        f.view.remove_key_pressed_visual(token);
        f.view.remove_key_released_visual(token2);
        w.close();
    });
}
#[test]
fn refresh_reference_identity_versions_and_geometry_are_source_driven() {
    crate::test_support::run(|| {
        let f = Fixture::new();
        f.view.refresh(false).unwrap();
        let old = f.view.views.borrow().values().next().unwrap().clone();
        let key = old.key();
        let old_size = label_size(&old);
        f.view.set_font_scale(1.6);
        f.view.refresh(false).unwrap();
        assert_eq!(label_size(&old), old_size);
        f.view.refresh(true).unwrap();
        assert_eq!(f.view.views.borrow().values().next().unwrap().clone(), old);
        assert_eq!(
            label_size(&old),
            (old_size * 1.6).min((key.visible_bounds().height * 0.52).max(8.0))
        );
        let options = Rc::new(KeyboardGeometryOptions::from_values(
            ferroboard_core::layouts::KeyboardGeometryOptionsValues {
                split_gap: 50.5,
                ..Default::default()
            },
        ));
        f.view.set_geometry(options.clone());
        assert!(Rc::ptr_eq(&f.view.geometry(), &options));
        let w = f.show();
        f.view.relayout().unwrap();
        assert_eq!(old.width(), key.visible_bounds().width.max(0.0));
        f.engine.set_mode(KeyboardMode::Numeric).unwrap();
        f.view.refresh(false).unwrap();
        assert_ne!(f.view.views.borrow().values().next().unwrap().clone(), old);
        assert!(
            !f.view
                .views
                .borrow()
                .iter()
                .any(|(k, _)| Rc::ptr_eq(&k.0, &key))
        );
        w.close();
    });
}
#[test]
fn coalesced_moves_follow_native_chronology_and_hover_is_unhandled() {
    crate::test_support::run(|| {
        let f = Fixture::new();
        let w = f.show();
        let p = Pointer::new(51, PointerType::Touch, true);
        let a = center(&f.key("a"));
        let s = center(&f.key("s"));
        let d = center(&f.key("d"));
        let q = center(&f.key("q"));
        f.down(&w, p.clone(), a);
        let previous: Vec<_> = [s, d]
            .into_iter()
            .map(|point| {
                let mut raw = RawPointerPoint::new();
                raw.position = f.view.translate_point(point, &w).unwrap();
                raw
            })
            .collect();
        let lazy: IntermediatePoints = Rc::new(LazyCell::new(Box::new(move || Some(previous))));
        let args = PointerEventArgs::new(
            Some(InputElement::pointer_moved_event()),
            &f.view,
            p.clone(),
            Some(&w),
            f.view.translate_point(q, &w).unwrap(),
            0,
            PointerPointProperties {
                is_left_button_pressed: true,
                ..Default::default()
            },
            KeyModifiers::empty(),
        )
        .with_previous_points(Some(lazy));
        f.view.raise_event(&args);
        assert!(args.handled());
        let trail = f.touch.glide_trail();
        let trail = trail.borrow();
        assert!(trail.len() >= 4);
        let last = &trail[trail.len() - 3..];
        assert_eq!(
            last.iter()
                .map(|p| Point::new(p.x, p.y))
                .collect::<Vec<_>>(),
            vec![s, d, q]
        );
        drop(trail);
        p.capture(None);
        let hover = PointerEventArgs::new(
            Some(InputElement::pointer_moved_event()),
            &f.view,
            p,
            Some(&w),
            Point::new(20.0, 20.0),
            0,
            PointerPointProperties::default(),
            KeyModifiers::empty(),
        );
        f.view.raise_event(&hover);
        assert!(!hover.handled());
        w.close();
    });
}
#[test]
fn fade_cancellation_duration_snapshot_and_explicit_subscription_cleanup() {
    crate::test_support::run(|| {
        let f = Fixture::new();
        f.engine.settings().set_glide_trail_duration(300);
        f.view.schedule_trail_fade();
        f.clock.advance(500_000);
        assert_eq!(f.view.trail.opacity(), 0.625);
        f.engine.settings().set_glide_trail_duration(0);
        f.clock.advance(500_000);
        assert_eq!(f.view.trail.opacity(), 0.5);
        let w = f.show();
        let p = Pointer::new(99, PointerType::Touch, true);
        let a = center(&f.key("a"));
        f.down(&w, p.clone(), a);
        let opacity = f.view.trail.opacity();
        f.clock.advance(500_000);
        assert_eq!(f.view.trail.opacity(), opacity);
        p.capture(None);
        assert!(f.touch.key_pressed().has_handlers());
        let view = f.view.clone();
        let weak = view.downgrade();
        view.release_subscriptions();
        view.release_subscriptions();
        assert!(!f.touch.key_pressed().has_handlers());
        assert!(!f.touch.glide_ended().has_handlers());
        drop(view);
        assert!(weak.upgrade().is_some());
        w.close();
    });
}

fn label_size(view: &KeyView) -> f64 {
    let face = view.children().get(1);
    let face = face.downcast_ref::<ferroui_controls::Border>().unwrap();
    let grid = face.child().unwrap();
    let grid = grid.downcast_ref::<Grid>().unwrap();
    let label = grid.children().get(0);
    label
        .downcast_ref::<ferroui_controls::TextBlock>()
        .unwrap()
        .font_size()
}

#[test]
fn raw_scaled_input_preserves_native_editor_focus_and_multitouch_rollover() {
    crate::test_support::run(|| {
        use crate::input::TextBoxInputTarget;
        use ferroui_base::{input::RawInputModifiers, media::ScaleTransform};
        use ferroui_controls::{StackPanel, TextBox};
        use ferroui_headless::HeadlessWindowExtensions;
        let f = Fixture::new();
        let text = TextBox::new();
        text.set_height(40.0);
        let target = TextBoxInputTarget::new(text.clone(), None).unwrap();
        f.engine.attach(target.clone()).unwrap();
        f.view.set_width(1000.0);
        f.view.set_height(240.0);
        f.view
            .set_render_transform(Some(ScaleTransform::with_scale(0.75, 0.75).into()));
        let panel = StackPanel::new();
        panel.children().add(text.clone());
        panel.children().add(f.view.clone());
        let w = Window::new();
        w.set_width(1000.0);
        w.set_height(300.0);
        w.set_content(Some(Control::boxed(panel)));
        w.show();
        Dispatcher::current_dispatcher().run_jobs(None);
        f.view.set_theme(KeyboardThemes::floris_day()).unwrap();
        assert!(text.focus());
        let point = |label| f.view.translate_point(center(&f.key(label)), &w).unwrap();
        let modifiers = RawInputModifiers::empty();
        w.mouse_down(point("a"), MouseButton::Left, modifiers);
        assert!(text.is_focused());
        w.mouse_up(point("a"), MouseButton::Left, modifiers);
        assert_eq!(text.text().as_deref(), Some("a"));
        assert!(text.is_focused());
        let first = w.touch_begin(point("s"), modifiers);
        let second = w.touch_begin(point("d"), modifiers);
        assert_eq!(text.text().as_deref(), Some("as"));
        w.touch_end(first.as_ref(), point("s"), modifiers);
        w.touch_end(second.as_ref(), point("d"), modifiers);
        assert_eq!(text.text().as_deref(), Some("asd"));
        assert!(text.is_focused());
        let canceled = w.touch_begin(point("e"), modifiers);
        assert!(!f.view.captured_pointers.borrow().is_empty());
        canceled.dispose();
        assert!(f.view.captured_pointers.borrow().is_empty());
        assert_eq!(text.text().as_deref(), Some("asd"));
        f.clock.advance(10_000_000);
        assert!(f.touch.popup().is_none());
        assert!(text.is_focused());
        w.close();
        f.engine.detach().unwrap();
        target.detach();
    });
}

#[test]
fn native_weak_subscribers_release_view_without_explicit_shutdown() {
    crate::test_support::run(|| {
        let engine = KeyboardEngine::new_explicit(
            Rc::new(KeyboardSettings::default()),
            Rc::new(KeyboardResources::create_default().unwrap()),
            LanguageModels::new(),
        )
        .unwrap();
        let clock = Clock::new();
        let touch = KeyboardTouchProcessor::new(engine.clone(), clock);
        let view = KeysView::new(engine.clone(), touch.clone(), Clock::new());
        let weak = view.downgrade();
        drop(view);
        assert!(weak.upgrade().is_none());
        assert!(!touch.key_pressed().has_handlers());
        assert!(!touch.key_released().has_handlers());
        assert!(!touch.glide_trail_changed().has_handlers());
        assert!(!touch.glide_ended().has_handlers());
        engine.dispose().unwrap();
    });
}

#[test]
fn duplicate_keys_unknown_visual_events_and_failed_refresh_keep_source_partial_state() {
    crate::test_support::run(|| {
        let f = Fixture::new();
        let keyboard = f.engine.keyboard().unwrap();
        let rows = keyboard.rows();
        let first_row = rows.get(0).unwrap();
        let key = first_row.get(0).unwrap();
        first_row.push(key.clone());
        let count = keyboard
            .keys()
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .len();
        f.view.refresh(false).unwrap();
        assert_eq!(f.view.keys_layer.children().count(), count);
        assert_eq!(f.view.views.borrow().len(), count - 1);
        let abandoned = f.view.keys_layer.children().get(0);
        assert!(abandoned.width().is_nan());
        let replacement = f
            .view
            .views
            .borrow()
            .get(&KeyIdentity(key))
            .unwrap()
            .clone();
        assert!(!replacement.width().is_nan());
        let foreign = ferroboard_core::layouts::LayoutComputer::new(Rc::new(
            KeyboardResources::create_default().unwrap(),
        ))
        .compute(
            KeyboardMode::Numeric,
            ferroboard_core::layouts::Subtype::shared_default(),
            false,
        )
        .unwrap()
        .keys()
        .map(|key| key.unwrap().unwrap())
        .next()
        .unwrap();
        let observed = Rc::new(RefCell::new(None));
        let result = observed.clone();
        let subscription = f
            .view
            .key_pressed_visual(move |_, k| *result.borrow_mut() = Some(k.clone()));
        f.view.set_pressed(&foreign, true).unwrap();
        assert!(Rc::ptr_eq(observed.borrow().as_ref().unwrap(), &foreign));
        f.view.remove_key_pressed_visual(subscription);
        let broken = KeyboardTheme::new();
        broken.set_key_rules(None);
        assert!(matches!(
            f.view.set_theme(broken),
            Err(KeysViewError::Key(KeyViewError::Theme(_)))
        ));
        assert_eq!(f.view.rendered_version.get(), keyboard.version());
        // The unchanged source writes renderedVersion before Update; the next non-forced
        // refresh skips even though the preceding update threw. Preserve that partial state.
        f.view.refresh(false).unwrap();
        assert!(f.view.refresh(true).is_err());
    });
}

#[test]
fn reentrant_native_property_callback_invalidates_source_view_enumeration() {
    crate::test_support::run(|| {
        let f = Fixture::new();
        f.view.refresh(false).unwrap();
        let first = f.view.views.borrow().values().next().unwrap().clone();
        let weak = f.view.downgrade();
        let engine = f.engine.clone();
        let once = Rc::new(Cell::new(false));
        let changed = once.clone();
        let subscription = first.property_changed(move |e| {
            if e.property() == ferroui_base::layout::Layoutable::width_property().as_property()
                && !changed.replace(true)
            {
                engine.set_mode(KeyboardMode::Numeric).unwrap();
                weak.upgrade().unwrap().refresh(true).unwrap();
            }
        });
        f.engine.update_layout(777.25, 240.0, None).unwrap();
        assert!(matches!(
            f.view.refresh(true),
            Err(KeysViewError::ViewsCollectionModified)
        ));
        assert!(once.get());
        assert_eq!(f.engine.mode(), KeyboardMode::Numeric);
        subscription.dispose();
    });
}

#[test]
fn keyboard_touch_stays_out_of_ancestor_scroll_recognition() {
    crate::test_support::run(|| {
        use ferroui_base::{
            input::{RawInputModifiers, gesture_recognizers::ScrollGestureRecognizer},
            media::ScaleTransform,
        };
        use ferroui_controls::{Border, StackPanel};
        use ferroui_headless::HeadlessWindowExtensions;
        let f = Fixture::new();
        f.view.set_width(1000.0);
        f.view.set_height(240.0);
        f.view
            .set_render_transform(Some(ScaleTransform::with_scale(0.75, 0.75).into()));
        let outside = Border::new();
        outside.set_height(80.0);
        outside.set_background(Some(Brushes::transparent().into()));
        let panel = StackPanel::new();
        panel.children().add(f.view.clone());
        panel.children().add(outside.clone());
        let recognizer = ScrollGestureRecognizer::new();
        recognizer.set_can_horizontally_scroll(true);
        recognizer.set_can_vertically_scroll(true);
        recognizer.set_scroll_start_distance(5);
        panel.gesture_recognizers().add(recognizer);
        let scrolls = Rc::new(Cell::new(0));
        let observed = scrolls.clone();
        panel.add_handler(InputElement::scroll_gesture_event(), move |_, e| {
            observed.set(observed.get() + 1);
            e.set_handled(true);
        });
        let w = Window::new();
        w.set_width(1000.0);
        w.set_height(360.0);
        w.set_content(Some(Control::boxed(panel)));
        w.show();
        Dispatcher::current_dispatcher().run_jobs(None);
        f.view.set_theme(KeyboardThemes::floris_day()).unwrap();
        let point = |label| f.view.translate_point(center(&f.key(label)), &w).unwrap();
        let modifiers = RawInputModifiers::empty();
        let contact = w.touch_begin(point("a"), modifiers);
        w.touch_move(contact.as_ref(), point("s"), modifiers);
        w.touch_move(contact.as_ref(), point("d"), modifiers);
        assert_eq!(
            scrolls.get(),
            0,
            "ancestor must not recognize keyboard touches"
        );
        assert_eq!(f.view.captured_pointers.borrow().len(), 1);
        let last = *f.touch.glide_trail().borrow().last().unwrap();
        assert_eq!(Point::new(last.x, last.y), center(&f.key("d")));
        w.touch_end(contact.as_ref(), point("d"), modifiers);
        assert!(f.view.captured_pointers.borrow().is_empty());
        assert_eq!(scrolls.get(), 0);
        let canceled = w.touch_begin(point("e"), modifiers);
        w.touch_move(canceled.as_ref(), point("r"), modifiers);
        canceled.dispose();
        assert!(f.view.captured_pointers.borrow().is_empty());
        assert_eq!(scrolls.get(), 0);
        // A following contact outside the keyboard must still scroll normally.
        let start = outside
            .translate_point(Point::new(200.0, 20.0), &w)
            .unwrap();
        let end = outside
            .translate_point(Point::new(250.0, 50.0), &w)
            .unwrap();
        let contact = w.touch_begin(start, modifiers);
        w.touch_move(contact.as_ref(), end, modifiers);
        w.touch_end(contact.as_ref(), end, modifiers);
        assert!(
            scrolls.get() > 0,
            "ancestor recognizer remains enabled outside keyboard"
        );
        w.close();
    });
}
