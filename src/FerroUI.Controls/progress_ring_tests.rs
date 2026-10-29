//! Tests of `ProgressRing` (WinUI; docs/porting/winui-controls.md, section 10.2), adapted from the
//! tests of #286. The control has no template of its own (the WinUI theme has it,
//! `src/FerroUI.Themes.WinUI/Controls/ProgressRing.xaml`), so a test gives the ring the part the
//! control moves (the arc `PART_Indicator`); the looks of the states are the theme's.

use crate::automation::peers::{AutomationControlType, ControlAutomationPeer, ProgressRingAutomationPeer};
use crate::automation::provider::IRangeValueProvider;
use crate::automation::{AccessibilityView, AutomationProperties};
use crate::shapes::Arc;
use crate::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions};
use crate::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use crate::{Border, Control, Grid, ProgressBar, ProgressRing, Window};
use ferroui_base::animation::{IClock, PlayState, TimeSpan};
use ferroui_base::layout::ILayoutManager;
use ferroui_base::reactive::{Disposable, IDisposable, IObservable, IObserver, LightweightSubject};
use ferroui_base::{FerroObject, Rect, Ref, Size, Thickness};
use std::cell::Cell;
use std::rc::Rc;

fn start() -> UnitTestApplicationScope {
    UnitTestApplication::start(TestServices::styled_window())
}

/// A clock the test advances, which counts its subscriptions.
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

/// Gives `ring` the parts of the template of the WinUI theme: the root, the track and the arc of
/// the indicator.
fn apply_parts(ring: &ProgressRing) {
    ring.set_template(Some(parts_template()));
    ring.apply_template();
}

fn parts_template() -> Rc<FuncControlTemplate> {
    FuncControlTemplate::new(|_, scope| {
        let root = Grid::new();
        root.set_name(Some("LayoutRoot".to_string()));
        let track = Arc::new();
        track.set_start_angle(-90.0);
        track.set_sweep_angle(360.0);
        root.children().add(track);
        let indicator = Arc::new();
        indicator.set_name(Some("PART_Indicator".to_string()));
        indicator.set_start_angle(-90.0);
        root.children().add(indicator.register_in_name_scope(&**scope));
        root.upcast()
    })
}

fn indicator(ring: &ProgressRing) -> Ref<Arc> {
    ring.get_template_descendants()
        .into_iter()
        .find(|element| element.name().as_deref() == Some("PART_Indicator"))
        .and_then(|element| element.cast::<Arc>())
        .expect("PART_Indicator")
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

fn has(ring: &ProgressRing, class: &str) -> bool {
    ring.classes().contains(class)
}

#[test]
fn the_defaults_are_those_of_winui_and_the_progress_bar_keeps_its_own() {
    let _app = start();
    let ring = ProgressRing::new();
    assert!(ring.is_active() && ring.is_indeterminate());
    assert_eq!((0.0, 100.0, 0.0), (ring.minimum(), ring.maximum(), ring.value()));
    assert!(has(&ring, ":active") && !has(&ring, ":determinate-active") && !has(&ring, ":inactive"));
    assert_eq!(AccessibilityView::Content, AutomationProperties::get_accessibility_view(&ring));
    assert!(!ProgressBar::new().is_indeterminate());
    assert!(!std::ptr::eq(ProgressRing::TYPE, ProgressBar::TYPE));
}

#[test]
fn the_bound_set_last_wins_and_the_value_moves_into_the_range() {
    let _app = start();
    let ring = ProgressRing::new();
    ring.set_range_value(150.0);
    assert_eq!(100.0, ring.value());
    ring.set_range_value(-10.0);
    assert_eq!(0.0, ring.value());
    ring.set_minimum(200.0);
    assert_eq!((200.0, 200.0, 200.0), (ring.minimum(), ring.maximum(), ring.value()));
    ring.set_maximum(-20.0);
    assert_eq!((-20.0, -20.0, -20.0), (ring.minimum(), ring.maximum(), ring.value()));
    ring.set_maximum(80.0);
    ring.set_range_value(30.0);
    assert_eq!(0.5, ring.normalized_value());
}

#[test]
fn values_set_through_the_properties_are_coerced_too() {
    let _app = start();
    let ring = ProgressRing::new();
    FerroObject::set_value(&ring, ProgressRing::minimum_property(), 10.0);
    FerroObject::set_value(&ring, ProgressRing::maximum_property(), 20.0);
    FerroObject::set_value(&ring, ProgressRing::value_property(), 500.0);
    assert_eq!(20.0, ring.value());
    FerroObject::set_value(&ring, ProgressRing::is_indeterminate_property(), false);
    apply_parts(&ring);
    assert_eq!(360.0, indicator(&ring).sweep_angle());
}

#[test]
fn a_value_that_is_not_a_number_and_an_empty_range_show_no_progress() {
    let _app = start();
    let ring = ProgressRing::new();
    ring.set_is_indeterminate(false);
    apply_parts(&ring);
    ring.set_range_value(f64::NAN);
    assert!(ring.value().is_nan());
    assert_eq!(0.0, ring.normalized_value());
    assert_eq!(0.0, indicator(&ring).sweep_angle());
    ring.set_minimum(100.0);
    ring.set_range_value(100.0);
    assert_eq!(0.0, ring.normalized_value());
    assert_eq!(0.0, indicator(&ring).sweep_angle());
    ring.set_minimum(f64::NEG_INFINITY);
    ring.set_maximum(f64::INFINITY);
    ring.set_range_value(50.0);
    assert_eq!(0.0, ring.normalized_value());
}

#[test]
fn an_inactive_ring_has_the_inactive_state_and_no_automation_content() {
    let _app = start();
    let ring = ProgressRing::new();
    ring.set_is_active(false);
    assert!(has(&ring, ":inactive") && !has(&ring, ":active"));
    assert_eq!(AccessibilityView::Raw, AutomationProperties::get_accessibility_view(&ring));
    ring.set_is_active(true);
    ring.set_is_indeterminate(false);
    assert!(has(&ring, ":determinate-active") && !has(&ring, ":inactive"));
    assert_eq!(AccessibilityView::Content, AutomationProperties::get_accessibility_view(&ring));
}

#[test]
fn the_template_settings_follow_the_actual_width() {
    let _app = start();
    let ring = ProgressRing::new();
    let other = ProgressRing::new();
    assert_ne!(ring.template_settings(), other.template_settings());
    let settings = ring.template_settings();
    for (width, diameter) in [(32.0, 4.2), (40.0, 5.0), (60.0, 6.0)] {
        ring.set_width(width);
        ring.measure(Size::new(200.0, 200.0));
        ring.arrange(Rect::new(0.0, 0.0, width, 32.0));
        assert_eq!(width, settings.max_side_length());
        assert!((settings.ellipse_diameter() - diameter).abs() < 1e-12);
        assert_eq!(Thickness::new(0.0, width / 2.0 - diameter, 0.0, 0.0), settings.ellipse_offset());
    }
    assert_eq!(0.0, other.template_settings().max_side_length());
}

#[test]
fn a_determinate_ring_shows_the_part_of_its_range() {
    let _app = start();
    let ring = ProgressRing::new();
    ring.set_is_indeterminate(false);
    ring.set_minimum(20.0);
    ring.set_maximum(60.0);
    ring.set_range_value(30.0);
    apply_parts(&ring);
    let indicator = indicator(&ring);
    assert_eq!(-90.0, indicator.start_angle());
    assert_eq!(90.0, indicator.sweep_angle());
    ring.set_range_value(40.0);
    assert_eq!(180.0, indicator.sweep_angle());
}

#[test]
fn the_indeterminate_motion_follows_activation_and_visibility() {
    let _app = start();
    let clock = Rc::new(TestClock::default());
    let ring = ProgressRing::new();
    ring.set_clock(Some(clock.clone()));
    apply_parts(&ring);
    assert_eq!(0, clock.active());
    let window = show(&ring);
    let indicator = indicator(&ring);
    assert_eq!(1, clock.active());
    clock.tick(0.0);
    let initial = indicator.start_angle();
    clock.tick(0.5);
    assert_ne!(initial, indicator.start_angle());
    ring.set_is_active(false);
    assert_eq!(0, clock.active());
    let stopped = indicator.start_angle();
    clock.tick(1.0);
    assert_eq!(stopped, indicator.start_angle());
    ring.set_is_active(true);
    assert_eq!(1, clock.active());
    window.set_is_visible(false);
    assert_eq!(0, clock.active());
    window.set_is_visible(true);
    assert_eq!(1, clock.active());
    ring.set_is_visible(false);
    assert_eq!(0, clock.active());
    ring.set_is_visible(true);
    assert_eq!(1, clock.active());
}

#[test]
fn the_indeterminate_motion_is_that_of_the_composition_and_a_value_does_not_restart_it() {
    let _app = start();
    let clock = Rc::new(TestClock::default());
    let ring = ProgressRing::new();
    ring.set_clock(Some(clock.clone()));
    apply_parts(&ring);
    let _window = show(&ring);
    let indicator = indicator(&ring);
    clock.tick(0.0);
    clock.tick(0.5);
    // A quarter of the timeline: 225 of the 900 degrees.
    assert_eq!(135.0, indicator.start_angle());
    ring.set_range_value(80.0);
    clock.tick(1.0);
    // Half the timeline: the arc is half the ring.
    assert_eq!(360.0, indicator.start_angle());
    assert_eq!(180.0, indicator.sweep_angle());
    assert_eq!(1, clock.active());
}

#[test]
fn a_determinate_increase_plays_and_a_decrease_is_immediate() {
    let _app = start();
    let clock = Rc::new(TestClock::default());
    let ring = ProgressRing::new();
    ring.set_clock(Some(clock.clone()));
    ring.set_is_indeterminate(false);
    apply_parts(&ring);
    let _window = show(&ring);
    let indicator = indicator(&ring);
    assert_eq!(0, clock.active());
    ring.set_range_value(50.0);
    assert_eq!(1, clock.active());
    clock.tick(0.0);
    assert_eq!(0.0, indicator.sweep_angle());
    clock.tick(0.5);
    assert!(indicator.sweep_angle() > 0.0 && indicator.sweep_angle() < 180.0);
    clock.tick(1.0);
    assert_eq!(180.0, indicator.sweep_angle());
    assert_eq!(0, clock.active());
    ring.set_range_value(25.0);
    assert_eq!(90.0, indicator.sweep_angle());
    assert_eq!(0, clock.active());
}

#[test]
fn a_new_inherited_clock_replaces_the_old_one() {
    let _app = start();
    let first = Rc::new(TestClock::default());
    let second = Rc::new(TestClock::default());
    let ring = ProgressRing::new();
    apply_parts(&ring);
    let window = Window::new();
    window.set_clock(Some(first.clone()));
    window.set_content(Some(Control::boxed(ring.clone())));
    window.show();
    window.layout_manager().execute_layout_pass();
    assert_eq!(1, first.active());
    window.set_clock(Some(second.clone()));
    assert_eq!(0, first.active());
    assert_eq!(1, second.active());
    first.tick(0.0);
    second.tick(0.0);
    second.tick(1.0);
    assert_eq!(360.0, indicator(&ring).start_angle());
}

#[test]
fn a_new_template_and_the_detachment_release_the_motion_and_the_ring() {
    let _app = start();
    let clock = Rc::new(TestClock::default());
    let ring = ProgressRing::new();
    ring.set_clock(Some(clock.clone()));
    apply_parts(&ring);
    let window = show(&ring);
    let old = indicator(&ring);
    ring.set_template(Some(FuncControlTemplate::new(|_, _| Border::new().upcast())));
    ring.apply_template();
    assert_eq!(0, clock.active());
    let old_angle = old.start_angle();
    apply_parts(&ring);
    assert_eq!(1, clock.active());
    clock.tick(0.0);
    clock.tick(0.5);
    assert_eq!(old_angle, old.start_angle());
    window.set_content(None);
    assert!(!ring.is_attached_to_visual_tree());
    assert_eq!(0, clock.active());
    // The layout manager of the window holds the elements it has queued until its next pass.
    window.layout_manager().execute_layout_pass();
    let weak = ring.downgrade();
    drop(old);
    drop(ring);
    assert!(weak.upgrade().is_none(), "the ring is kept after it was detached");
    clock.tick(1.0);
    assert_eq!(0, clock.active());
}

#[test]
fn the_range_value_pattern_is_there_only_while_determinate() {
    let _app = start();
    let ring = ProgressRing::new();
    let peer = ControlAutomationPeer::create_peer_for_element(&ring);
    assert!(peer.cast::<ProgressRingAutomationPeer>().is_some());
    assert_eq!("ProgressRing", peer.get_class_name());
    assert_eq!(AutomationControlType::ProgressBar, peer.get_automation_control_type());
    assert_eq!("ProgressRing", peer.get_localized_control_type());
    assert!(peer.get_provider::<dyn IRangeValueProvider>().is_none());
    ring.set_is_indeterminate(false);
    let provider = peer.get_provider::<dyn IRangeValueProvider>().expect("the range value pattern");
    assert!(provider.is_read_only());
    assert!(provider.small_change().is_nan() && provider.large_change().is_nan());
    ring.set_minimum(20.0);
    ring.set_maximum(80.0);
    assert!(provider.set_value(150.0).is_ok());
    assert_eq!((20.0, 80.0, 80.0), (provider.minimum(), provider.maximum(), provider.value()));
    ring.set_is_indeterminate(true);
    assert!(peer.get_provider::<dyn IRangeValueProvider>().is_none());
}

#[test]
fn the_name_of_an_active_indeterminate_ring_says_busy() {
    let _app = start();
    let ring = ProgressRing::new();
    AutomationProperties::set_name(&ring, Some("Downloading"));
    let peer = ProgressRingAutomationPeer::new(&ring);
    assert_eq!("Busy Downloading", peer.get_name());
    ring.set_is_active(false);
    assert_eq!("Downloading", peer.get_name());
    ring.set_is_active(true);
    ring.set_is_indeterminate(false);
    assert_eq!("Downloading", peer.get_name());
}

