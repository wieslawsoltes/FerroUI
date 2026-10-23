//! Native WinUI progress state, range, animation lifetime and automation contracts.

use ferroui_controls::automation::peers::{ProgressRingAutomationPeer};
use ferroui_base::animation::{IClock, PlayState, TimeSpan};
use ferroui_base::data::BindingPriority;
use ferroui_base::layout::ILayoutManager;
use ferroui_base::media::{Brushes, IBrush, PenLineCap};
use ferroui_base::reactive::{Disposable, IDisposable, IObservable, IObserver, LightweightSubject};
use ferroui_base::{Rect, Ref, Size, Thickness};
use ferroui_controls::automation::peers::{AutomationControlType, ControlAutomationPeer};
use ferroui_controls::automation::provider::IRangeValueProvider;
use ferroui_controls::automation::{AccessibilityView, AutomationProperties};
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::shapes::Arc;
use ferroui_controls::templates::FuncControlTemplate;
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_controls::{
    ProgressRing,
};
use ferroui_controls::{Border, Control, Grid, ProgressBar, Window};
use std::cell::Cell;
use std::rc::Rc;

fn start() -> ferroui_controls::testing::UnitTestApplicationScope {
    ferroui_controls::register_types();
    UnitTestApplication::start(TestServices::styled_window())
}

#[derive(Default)]
struct TestClock {
    subject: LightweightSubject<TimeSpan>,
    subscriptions: Rc<Cell<usize>>,
}
impl TestClock {
    fn tick(&self, seconds: f64) {
        self.subject.on_next(TimeSpan::from_seconds(seconds));
    }
    fn active(&self) -> usize {
        self.subscriptions.get()
    }
}
impl IObservable<TimeSpan> for TestClock {
    fn subscribe(&self, observer: Rc<dyn IObserver<TimeSpan>>) -> Rc<dyn IDisposable> {
        self.subscriptions.set(self.active() + 1);
        let subscription = self.subject.subscribe(observer);
        let count = self.subscriptions.clone();
        Disposable::create(move || {
            subscription.dispose();
            count.set(count.get() - 1);
        })
    }
}
impl IClock for TestClock {
    fn play_state(&self) -> PlayState {
        PlayState::Run
    }
    fn set_play_state(&self, _: PlayState) {}
}

fn part<T: ferroui_base::ObjectType>(ring: &ProgressRing, name: &str) -> Ref<T> {
    ring.get_template_descendants()
        .iter()
        .find(|element| element.name().as_deref() == Some(name))
        .unwrap_or_else(|| panic!("missing ProgressRing part {name}"))
        .cast::<T>()
        .unwrap()
}

fn show(ring: &ProgressRing) -> Ref<Window> {
    let window = Window::new();
    window.set_width(400.0);
    window.set_height(300.0);
    window.set_content(Some(Control::boxed(ring.to_ref())));
    window.show();
    window.layout_manager().execute_layout_pass();
    window
}

#[test]
fn defaults_are_winui_specific_and_leave_existing_progress_bar_unchanged() {
    let _app = start();
    let ring = ProgressRing::new();
    assert!(ring.is_active() && ring.is_indeterminate());
    assert_eq!(
        (ring.minimum(), ring.maximum(), ring.value()),
        (0.0, 100.0, 0.0)
    );
    assert_eq!(
        (
            ring.width(),
            ring.height(),
            ring.min_width(),
            ring.min_height()
        ),
        (32.0, 32.0, 16.0, 16.0)
    );
    assert!(!ring.focusable() && !ring.is_hit_test_visible());
    assert_eq!(
        AutomationProperties::get_accessibility_view(&ring),
        AccessibilityView::Content
    );
    assert!(!ProgressBar::new().is_indeterminate());
    assert_ne!(ProgressRing::TYPE, ProgressBar::TYPE);
}

#[test]
fn range_coercion_gives_the_last_set_bound_precedence() {
    let _app = start();
    let ring = ProgressRing::new();
    ring.set_range_value(150.0);
    assert_eq!(ring.value(), 100.0);
    ring.set_range_value(-10.0);
    assert_eq!(ring.value(), 0.0);
    ring.set_minimum(200.0);
    assert_eq!(
        (ring.minimum(), ring.maximum(), ring.value()),
        (200.0, 200.0, 200.0)
    );
    ring.set_maximum(-20.0);
    assert_eq!(
        (ring.minimum(), ring.maximum(), ring.value()),
        (-20.0, -20.0, -20.0)
    );
    ring.set_maximum(80.0);
    ring.set_range_value(30.0);
    assert_eq!(ring.normalized_value(), 0.5);
}

#[test]
fn markup_property_writes_follow_the_same_range_coercion() {
    let _app = start();
    let ring = ProgressRing::new();
    ring.set_value(ProgressRing::minimum_property(), 10.0);
    ring.set_value(ProgressRing::maximum_property(), 20.0);
    ring.set_value(ProgressRing::value_property(), 500.0);
    assert_eq!(ring.value(), 20.0);
    ring.set_value(ProgressRing::is_indeterminate_property(), false);
    ring.apply_template();
    assert_eq!(part::<Arc>(&ring, "PART_Indicator").sweep_angle(), 360.0);
}

#[test]
fn nan_and_collapsed_ranges_preserve_public_values_but_emit_finite_geometry() {
    let _app = start();
    let ring = ProgressRing::new();
    ring.set_is_indeterminate(false);
    ring.apply_template();
    ring.set_range_value(f64::NAN);
    assert!(ring.value().is_nan());
    assert_eq!(ring.normalized_value(), 0.0);
    assert_eq!(part::<Arc>(&ring, "PART_Indicator").sweep_angle(), 0.0);
    ring.set_minimum(100.0);
    ring.set_range_value(100.0);
    assert_eq!(ring.normalized_value(), 0.0);
    assert_eq!(part::<Arc>(&ring, "PART_Indicator").sweep_angle(), 0.0);
    ring.set_minimum(f64::NEG_INFINITY);
    ring.set_maximum(f64::INFINITY);
    ring.set_range_value(50.0);
    assert_eq!(ring.normalized_value(), 0.0);
}

#[test]
fn inactive_ring_retains_layout_while_removing_its_automation_content() {
    let _app = start();
    let ring = ProgressRing::new();
    let window = show(&ring);
    let desired = ring.desired_size();
    assert_eq!(desired, Size::new(32.0, 32.0));
    ring.set_is_active(false);
    window.layout_manager().execute_layout_pass();
    assert_eq!(ring.desired_size(), desired);
    assert!(ring.is_visible());
    assert_eq!(part::<Grid>(&ring, "LayoutRoot").opacity(), 0.0);
    assert_eq!(
        AutomationProperties::get_accessibility_view(&ring),
        AccessibilityView::Raw
    );
    ring.set_is_active(true);
    assert_eq!(part::<Grid>(&ring, "LayoutRoot").opacity(), 1.0);
    assert_eq!(
        AutomationProperties::get_accessibility_view(&ring),
        AccessibilityView::Content
    );
}

#[test]
fn legacy_template_settings_use_actual_width_and_the_forty_pixel_boundary() {
    let _app = start();
    let ring = ProgressRing::new();
    let other = ProgressRing::new();
    assert_ne!(ring.template_settings(), other.template_settings());
    let settings = ring.template_settings();
    for (width, expected_diameter) in [(32.0, 4.2), (40.0, 5.0), (60.0, 6.0)] {
        ring.set_width(width);
        ring.measure(Size::new(200.0, 200.0));
        ring.arrange(Rect::new(0.0, 0.0, width, 32.0));
        assert_eq!(settings.max_side_length(), width);
        assert!((settings.ellipse_diameter() - expected_diameter).abs() < 1e-12);
        assert_eq!(
            settings.ellipse_offset(),
            Thickness::new(0.0, width / 2.0 - expected_diameter, 0.0, 0.0)
        );
    }
    assert_eq!(other.template_settings().max_side_length(), 0.0);
}

#[test]
fn native_arc_template_uses_live_brushes_and_normalized_progress() {
    let _app = start();
    let ring = ProgressRing::new();
    ring.set_is_indeterminate(false);
    ring.set_minimum(20.0);
    ring.set_maximum(60.0);
    ring.set_range_value(30.0);
    ring.apply_template();
    let indicator = part::<Arc>(&ring, "PART_Indicator");
    let track = part::<Arc>(&ring, "PART_Track");
    assert_eq!(indicator.start_angle(), -90.0);
    assert_eq!(indicator.sweep_angle(), 90.0);
    assert_eq!(indicator.stroke_line_cap(), PenLineCap::Round);
    assert_eq!(track.sweep_angle(), 360.0);
    ring.set_foreground(Some(Brushes::red()));
    ring.set_background(Some(Brushes::blue()));
    assert_eq!(indicator.stroke(), Some(Brushes::red() as Rc<dyn IBrush>));
    assert_eq!(track.stroke(), Some(Brushes::blue() as Rc<dyn IBrush>));
    ring.set_range_value(40.0);
    assert_eq!(indicator.sweep_angle(), 180.0);
}

#[test]
fn foreground_uses_effective_animation_and_restores_the_underlying_brush() {
    let _app = start();
    let ring = ProgressRing::new();
    ring.set_foreground(Some(Brushes::red()));
    ring.apply_template();
    let indicator = part::<Arc>(&ring, "PART_Indicator");
    let animation = ring
        .set_value_with_priority(
            TemplatedControl::foreground_property(),
            Some(Brushes::blue()),
            BindingPriority::Animation,
        )
        .unwrap();
    assert_eq!(indicator.stroke(), Some(Brushes::blue() as Rc<dyn IBrush>));
    ring.set_foreground(Some(Brushes::green()));
    assert_eq!(indicator.stroke(), Some(Brushes::blue() as Rc<dyn IBrush>));
    animation.dispose();
    assert_eq!(indicator.stroke(), Some(Brushes::green() as Rc<dyn IBrush>));
}

#[test]
fn indeterminate_clock_subscription_follows_activation_and_effective_visibility() {
    let _app = start();
    let clock = Rc::new(TestClock::default());
    let ring = ProgressRing::new();
    ring.set_clock(Some(clock.clone()));
    ring.apply_template();
    assert_eq!(clock.active(), 0);
    let window = show(&ring);
    let indicator = part::<Arc>(&ring, "PART_Indicator");
    assert_eq!(clock.active(), 1);
    clock.tick(0.0);
    let initial = indicator.start_angle();
    clock.tick(0.5);
    assert_ne!(indicator.start_angle(), initial);
    ring.set_is_active(false);
    assert_eq!(clock.active(), 0);
    let stopped = indicator.start_angle();
    clock.tick(1.0);
    assert_eq!(indicator.start_angle(), stopped);
    ring.set_is_active(true);
    assert_eq!(clock.active(), 1);
    window.set_is_visible(false);
    assert_eq!(clock.active(), 0);
    window.set_is_visible(true);
    assert_eq!(clock.active(), 1);
    ring.set_is_visible(false);
    assert_eq!(clock.active(), 0);
    ring.set_is_visible(true);
    assert_eq!(clock.active(), 1);
}

#[test]
fn numeric_updates_do_not_restart_indeterminate_motion() {
    let _app = start();
    let clock = Rc::new(TestClock::default());
    let ring = ProgressRing::new();
    ring.set_clock(Some(clock.clone()));
    let _window = show(&ring);
    let indicator = part::<Arc>(&ring, "PART_Indicator");
    clock.tick(0.0);
    clock.tick(0.5);
    assert_eq!(indicator.start_angle(), 135.0);
    ring.set_range_value(80.0);
    clock.tick(1.0);
    assert_eq!(indicator.start_angle(), 360.0);
    assert_eq!(indicator.sweep_angle(), 180.0);
    assert_eq!(clock.active(), 1);
}

#[test]
fn hidden_local_activation_changes_do_not_restart_motion_and_apply_after_animation() {
    let _app = start();
    let clock = Rc::new(TestClock::default());
    let ring = ProgressRing::new();
    ring.set_clock(Some(clock.clone()));
    let _window = show(&ring);
    let indicator = part::<Arc>(&ring, "PART_Indicator");
    let animation = ring
        .set_value_with_priority(
            ProgressRing::is_active_property(),
            true,
            BindingPriority::Animation,
        )
        .unwrap();
    clock.tick(0.0);
    clock.tick(0.5);
    assert_eq!(indicator.start_angle(), 135.0);

    ring.set_is_active(false);
    assert!(ring.is_active());
    assert_eq!(clock.active(), 1);
    clock.tick(1.0);
    assert_eq!(indicator.start_angle(), 360.0);

    animation.dispose();
    assert!(!ring.is_active());
    assert_eq!(clock.active(), 0);
    assert_eq!(part::<Grid>(&ring, "LayoutRoot").opacity(), 0.0);
}

#[test]
fn determinate_increases_animate_then_unsubscribe_and_decreases_are_immediate() {
    let _app = start();
    let clock = Rc::new(TestClock::default());
    let ring = ProgressRing::new();
    ring.set_clock(Some(clock.clone()));
    ring.set_is_indeterminate(false);
    let _window = show(&ring);
    let indicator = part::<Arc>(&ring, "PART_Indicator");
    assert_eq!(clock.active(), 0);
    ring.set_range_value(50.0);
    assert_eq!(clock.active(), 1);
    clock.tick(0.0);
    assert_eq!(indicator.sweep_angle(), 0.0);
    clock.tick(0.5);
    assert!(indicator.sweep_angle() > 0.0 && indicator.sweep_angle() < 180.0);
    clock.tick(1.0);
    assert_eq!(indicator.sweep_angle(), 180.0);
    assert_eq!(clock.active(), 0);
    ring.set_range_value(25.0);
    assert_eq!(indicator.sweep_angle(), 90.0);
    assert_eq!(clock.active(), 0);
}

#[test]
fn changing_the_inherited_clock_disposes_old_source_before_subscribing_new() {
    let _app = start();
    let first = Rc::new(TestClock::default());
    let second = Rc::new(TestClock::default());
    let ring = ProgressRing::new();
    let window = Window::new();
    window.set_clock(Some(first.clone()));
    window.set_content(Some(Control::boxed(ring.clone())));
    window.show();
    window.layout_manager().execute_layout_pass();
    assert_eq!(first.active(), 1);
    window.set_clock(Some(second.clone()));
    assert_eq!(first.active(), 0);
    assert_eq!(second.active(), 1);
    first.tick(0.0);
    second.tick(0.0);
    second.tick(1.0);
    assert_eq!(part::<Arc>(&ring, "PART_Indicator").start_angle(), 360.0);
}

#[test]
fn retemplating_and_detaching_release_the_animation_without_retaining_the_owner() {
    let _app = start();
    let clock = Rc::new(TestClock::default());
    let ring = ProgressRing::new();
    ring.set_clock(Some(clock.clone()));
    let window = show(&ring);
    let old = part::<Arc>(&ring, "PART_Indicator");
    let template = ring.template();
    ring.set_template(Some(FuncControlTemplate::for_type::<ProgressRing>(
        |_, _| Border::new().upcast(),
    )));
    ring.apply_template();
    assert_eq!(clock.active(), 0);
    let old_angle = old.start_angle();
    ring.set_template(template);
    ring.apply_template();
    assert_eq!(clock.active(), 1);
    clock.tick(0.0);
    clock.tick(0.5);
    assert_eq!(old.start_angle(), old_angle);
    window.set_content(None);
    assert!(!ring.is_attached_to_visual_tree());
    assert_eq!(clock.active(), 0);
    // Retemplating queues strong Layoutable references in the window's layout
    // manager. Detachment stops the clock immediately; draining that existing
    // work releases the framework's temporary references before checking the
    // control's own subscriptions. Keep the window and clock alive for the check.
    window.layout_manager().execute_layout_pass();
    let weak = ring.downgrade();
    drop(old);
    drop(ring);
    assert!(
        weak.upgrade().is_none(),
        "ProgressRing retained after detach and queued layout completion"
    );
    clock.tick(1.0);
    assert_eq!(clock.active(), 0);
}

#[test]
fn range_automation_is_available_only_when_determinate() {
    let _app = start();
    let ring = ProgressRing::new();
    let peer = ControlAutomationPeer::create_peer_for_element(&ring);
    assert!(peer.cast::<ProgressRingAutomationPeer>().is_some());
    assert_eq!(peer.get_class_name(), "ProgressRing");
    assert_eq!(
        peer.get_automation_control_type(),
        AutomationControlType::ProgressBar
    );
    assert!(peer.get_provider::<dyn IRangeValueProvider>().is_none());
    ring.set_is_indeterminate(false);
    let provider = peer.get_provider::<dyn IRangeValueProvider>().unwrap();
    assert!(provider.is_read_only());
    assert!(provider.small_change().is_nan() && provider.large_change().is_nan());
    ring.set_minimum(20.0);
    ring.set_maximum(80.0);
    provider.set_value(150.0).unwrap();
    assert_eq!(
        (provider.minimum(), provider.maximum(), provider.value()),
        (20.0, 80.0, 80.0)
    );
    ring.set_is_indeterminate(true);
    assert!(peer.get_provider::<dyn IRangeValueProvider>().is_none());
}

#[test]
fn accessible_name_reports_busy_only_for_active_indeterminate_progress() {
    let _app = start();
    let ring = ProgressRing::new();
    AutomationProperties::set_name(&ring, Some("Downloading"));
    let peer = ProgressRingAutomationPeer::new(&ring);
    assert_eq!(peer.get_name(), "Busy Downloading");
    ring.set_is_active(false);
    assert_eq!(peer.get_name(), "Downloading");
    ring.set_is_active(true);
    ring.set_is_indeterminate(false);
    assert_eq!(peer.get_name(), "Downloading");
    assert_eq!(peer.get_localized_control_type(), "progress ring");
}
