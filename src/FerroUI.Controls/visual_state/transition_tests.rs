//! Tests of the transitions between visual states (#443, VSM-4), against WinUI's
//! `VisualStateManagerActuator` and `DynamicTransitionStoryboardGenerator`.

use super::transitions::find_transition;
use crate::animation::{DoubleAnimation, Storyboard};
use crate::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions};
use crate::test_support::test_scope;
use crate::{Border, ContentControl, VisualState, VisualStateGroup, VisualStateManager, VisualTransition};
use ferroui_base::animation::{IClock, IGlobalClock, PlayState, SignedDuration, TimelineDuration};
use ferroui_base::layout::Layoutable;
use ferroui_base::platform::IPlatformSettings;
use ferroui_base::reactive::{IDisposable, IObservable, IObserver, LightweightSubject};
use ferroui_base::styling::{Setter, TargetPropertyPath};
use ferroui_base::threading::Dispatcher;
use ferroui_base::{FerroLocator, Ref, Size, Visual};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

fn ms(value: f64) -> SignedDuration {
    SignedDuration::from_milliseconds(value)
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-6
}

/// A global clock that the test pulses.
struct TestClock {
    subject: LightweightSubject<SignedDuration>,
    play_state: Cell<PlayState>,
}

impl IObservable<SignedDuration> for TestClock {
    fn subscribe(&self, observer: Rc<dyn IObserver<SignedDuration>>) -> Rc<dyn IDisposable> {
        self.subject.subscribe(observer)
    }
}

impl IClock for TestClock {
    fn play_state(&self) -> PlayState {
        self.play_state.get()
    }

    fn set_play_state(&self, value: PlayState) {
        self.play_state.set(value)
    }
}

impl IGlobalClock for TestClock {}

fn clock() -> Rc<TestClock> {
    let clock = Rc::new(TestClock { subject: LightweightSubject::new(), play_state: Cell::new(PlayState::Run) });
    let service: Rc<dyn IGlobalClock> = clock.clone();
    FerroLocator::current_mutable().bind::<dyn IGlobalClock>().to_constant(service);
    // The first tick starts the time of the storyboards that begin before it.
    clock
}

fn tick(clock: &TestClock, time: f64) {
    clock.subject.on_next(ms(time));
}

/// A content control whose template is a border with a border `Inner` and a group
/// `CommonStates` with the states `Normal` and `Faded` (`Faded` sets the opacity of `Inner` to
/// 0.5 and its width to 50) and the transitions `transitions` gives.
struct Fixture {
    control: Ref<ContentControl>,
    inner: Ref<Border>,
    group: Ref<VisualStateGroup>,
}

fn fixture(transitions: impl Fn() -> Vec<Ref<VisualTransition>> + 'static) -> Fixture {
    let control = ContentControl::new();
    let slot: Rc<RefCell<Option<Ref<VisualStateGroup>>>> = Rc::default();
    let group_slot = slot.clone();
    control.set_template(Some(FuncControlTemplate::new(move |_, scope| {
        let inner = Border::new();
        inner.set_name(Some("Inner".to_string()));
        inner.set_width(100.0);
        let root = Border::new();
        root.set_child(inner.clone().register_in_name_scope(&**scope));
        let group = VisualStateGroup::with_name("CommonStates");
        group.states().add(VisualState::with_name("Normal"));
        let faded = VisualState::with_name("Faded");
        let opacity = Setter::new(Visual::opacity_property(), 0.5);
        opacity.set_target(Some(TargetPropertyPath::new(&inner, Visual::opacity_property().as_property())));
        faded.setters().add(opacity);
        let width = Setter::new(Layoutable::width_property(), 50.0);
        width.set_target(Some(TargetPropertyPath::new(&inner, Layoutable::width_property().as_property())));
        faded.setters().add(width);
        group.states().add(faded);
        for transition in transitions() {
            group.transitions().add(transition);
        }
        VisualStateManager::get_visual_state_groups(&root).add(group.clone());
        *group_slot.borrow_mut() = Some(group);
        root.upcast()
    })));
    control.apply_template();
    let root = control.visual_children().snapshot()[0].cast::<Border>().unwrap();
    let inner = root.child().unwrap().cast::<Border>().unwrap();
    let group = slot.borrow().clone().unwrap();
    Fixture { control, inner, group }
}

fn generated(duration: f64) -> Ref<VisualTransition> {
    let transition = VisualTransition::new();
    transition.set_generated_duration(ms(duration));
    transition
}

fn changed_count(group: &Ref<VisualStateGroup>) -> Rc<Cell<usize>> {
    let count = Rc::new(Cell::new(0));
    let counter = count.clone();
    group.current_state_changed(move |_, _| counter.set(counter.get() + 1)).detach();
    count
}

#[test]
fn the_transition_with_the_best_score_is_chosen() {
    let _scope = test_scope();
    let group = VisualStateGroup::new();
    let normal = VisualState::with_name("Normal");
    let pressed = VisualState::with_name("Pressed");
    let over = VisualState::with_name("PointerOver");
    let default = VisualTransition::new();
    let from_pressed = VisualTransition::new();
    from_pressed.set_from("Pressed");
    let to_normal = VisualTransition::new();
    to_normal.set_to("Normal");
    let pressed_to_normal = VisualTransition::new();
    pressed_to_normal.set_from("Pressed");
    pressed_to_normal.set_to("Normal");
    let to_over = VisualTransition::new();
    to_over.set_to("PointerOver");
    for transition in [&default, &from_pressed, &to_normal, &pressed_to_normal, &to_over] {
        group.transitions().add(transition.clone());
    }
    let transitions = group.transitions();
    assert_eq!(find_transition(&transitions, Some(&pressed), &normal), Some(pressed_to_normal.clone()));
    // To scores more than From.
    assert_eq!(find_transition(&transitions, Some(&over), &normal), Some(to_normal.clone()));
    assert_eq!(find_transition(&transitions, Some(&pressed), &over), Some(to_over.clone()));
    // Nothing names the states: the default transition.
    let disabled = VisualState::with_name("Disabled");
    assert_eq!(find_transition(&transitions, Some(&over), &disabled), Some(default.clone()));
    // A transition that names another state is out; the first of equal scores wins.
    let second_default = VisualTransition::new();
    group.transitions().add(second_default);
    assert_eq!(find_transition(&transitions, None, &disabled), Some(default));
    let only_named = VisualStateGroup::new();
    only_named.transitions().add(from_pressed);
    assert_eq!(find_transition(&only_named.transitions(), Some(&over), &disabled), None);
}

#[test]
fn a_generated_transition_animates_independent_values_and_applies_the_setters_at_the_end() {
    let _scope = test_scope();
    let clock = clock();
    let fixture = fixture(|| vec![generated(1000.0)]);
    let changed = changed_count(&fixture.group);
    assert!(VisualStateManager::go_to_state(&fixture.control, "Normal", false));
    Dispatcher::ui_thread().run_jobs(None);
    let before = changed.get();

    assert!(VisualStateManager::go_to_state(&fixture.control, "Faded", true));
    assert!(fixture.group.is_transitioning());
    // The group is in the new state while the transition runs.
    assert_eq!(fixture.group.current_state().unwrap().name(), "Faded");
    tick(&clock, 0.0);
    tick(&clock, 500.0);
    // The opacity is independent: it animates for the full duration.
    assert!(close(fixture.inner.opacity(), 0.75));
    // The width is not: it jumps at the start.
    assert!(close(fixture.inner.width(), 50.0));
    Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(changed.get(), before);
    tick(&clock, 1000.0);
    assert!(!fixture.group.is_transitioning());
    assert!(close(fixture.inner.opacity(), 0.5));
    assert!(close(fixture.inner.width(), 50.0));
    Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(changed.get(), before + 1);
    // The local value is untouched.
    assert_eq!(fixture.inner.get_base_value(Visual::opacity_property()), None);
}

#[test]
fn leaving_a_state_with_a_transition_animates_its_values_back() {
    let _scope = test_scope();
    let clock = clock();
    let fixture = fixture(|| vec![generated(1000.0)]);
    assert!(VisualStateManager::go_to_state(&fixture.control, "Faded", false));
    assert!(close(fixture.inner.opacity(), 0.5));
    assert!(VisualStateManager::go_to_state(&fixture.control, "Normal", true));
    tick(&clock, 0.0);
    tick(&clock, 500.0);
    assert!(close(fixture.inner.opacity(), 0.75));
    assert!(close(fixture.inner.width(), 100.0));
    tick(&clock, 1500.0);
    assert!(!fixture.group.is_transitioning());
    assert!(close(fixture.inner.opacity(), 1.0));
}

#[test]
fn without_transitions_or_with_a_zero_length_one_the_state_changes_at_once() {
    let _scope = test_scope();
    let _clock = clock();
    let fixture = fixture(|| vec![generated(0.0)]);
    assert!(VisualStateManager::go_to_state(&fixture.control, "Faded", true));
    assert!(!fixture.group.is_transitioning());
    assert!(close(fixture.inner.opacity(), 0.5));
    let timed = fixture_with_timed();
    assert!(VisualStateManager::go_to_state(&timed.control, "Faded", false));
    assert!(!timed.group.is_transitioning());
    assert!(close(timed.inner.opacity(), 0.5));
}

fn fixture_with_timed() -> Fixture {
    fixture(|| vec![generated(1000.0)])
}

#[test]
fn an_interrupted_transition_raises_no_changed_event() {
    let _scope = test_scope();
    let clock = clock();
    let fixture = fixture(|| vec![generated(1000.0)]);
    let changed = changed_count(&fixture.group);
    assert!(VisualStateManager::go_to_state(&fixture.control, "Faded", true));
    tick(&clock, 0.0);
    tick(&clock, 500.0);
    assert!(VisualStateManager::go_to_state(&fixture.control, "Normal", false));
    assert!(!fixture.group.is_transitioning());
    assert!(close(fixture.inner.opacity(), 1.0));
    assert!(close(fixture.inner.width(), 100.0));
    tick(&clock, 2000.0);
    Dispatcher::ui_thread().run_jobs(None);
    // Only the change to Normal completed.
    assert_eq!(changed.get(), 1);
    assert_eq!(fixture.group.current_state().unwrap().name(), "Normal");
}

#[test]
fn the_storyboard_of_a_transition_runs_and_its_targets_are_not_generated() {
    let _scope = test_scope();
    let clock = clock();
    let fixture = fixture(|| {
        let transition = generated(200.0);
        let animation = DoubleAnimation::new();
        animation.set_from(Some(0.0));
        animation.set_to(Some(0.2));
        animation.set_duration(TimelineDuration::Time(Duration::from_millis(1000)));
        Storyboard::set_target_name(&animation, "Inner");
        Storyboard::set_target_property(&animation, "Opacity");
        let storyboard = Storyboard::new();
        storyboard.children().add(animation.upcast());
        transition.set_storyboard(Some(storyboard));
        vec![transition]
    });
    assert!(VisualStateManager::go_to_state(&fixture.control, "Faded", true));
    tick(&clock, 0.0);
    tick(&clock, 500.0);
    // The transition's own animation of the opacity, not a generated one.
    assert!(close(fixture.inner.opacity(), 0.1));
    // The generated part ended; the transition waits for its storyboard.
    assert!(fixture.group.is_transitioning());
    tick(&clock, 1000.0);
    assert!(!fixture.group.is_transitioning());
    assert!(close(fixture.inner.opacity(), 0.5));
}

#[test]
fn with_system_animations_off_transitions_jump_to_their_end() {
    let _scope = test_scope();
    let _clock = clock();
    FerroLocator::current_mutable()
        .bind::<dyn IPlatformSettings>()
        .to_constant(Rc::new(ReducedMotion(Default::default())));
    let fixture = fixture(|| vec![generated(1000.0)]);
    assert!(VisualStateManager::go_to_state(&fixture.control, "Faded", true));
    assert!(!fixture.group.is_transitioning());
    assert!(close(fixture.inner.opacity(), 0.5));
}

#[test]
fn going_to_the_current_state_without_transitions_completes_the_transition() {
    let _scope = test_scope();
    let clock = clock();
    let fixture = fixture(|| vec![generated(1000.0)]);
    assert!(VisualStateManager::go_to_state(&fixture.control, "Faded", true));
    tick(&clock, 0.0);
    assert!(VisualStateManager::go_to_state(&fixture.control, "Faded", false));
    assert!(!fixture.group.is_transitioning());
    assert!(close(fixture.inner.opacity(), 0.5));
}

/// Platform settings with system animations off.
struct ReducedMotion(ferroui_base::platform::DefaultPlatformSettings);

impl IPlatformSettings for ReducedMotion {
    fn get_tap_size(&self, kind: ferroui_base::input::PointerType) -> Size {
        self.0.get_tap_size(kind)
    }
    fn get_double_tap_size(&self, kind: ferroui_base::input::PointerType) -> Size {
        self.0.get_double_tap_size(kind)
    }
    fn get_double_tap_time(&self, kind: ferroui_base::input::PointerType) -> Duration {
        self.0.get_double_tap_time(kind)
    }
    fn hold_wait_duration(&self) -> Duration {
        self.0.hold_wait_duration()
    }
    fn hotkey_configuration(&self) -> Rc<ferroui_base::input::platform::PlatformHotkeyConfiguration> {
        self.0.hotkey_configuration()
    }
    fn preferred_application_language(&self) -> String {
        self.0.preferred_application_language()
    }
    fn animations_enabled(&self) -> bool {
        false
    }
    fn get_color_values(&self) -> ferroui_base::platform::PlatformColorValues {
        self.0.get_color_values()
    }
    fn color_values_changed(
        &self,
        handler: Rc<dyn Fn(&ferroui_base::platform::PlatformColorValues)>,
    ) -> Rc<dyn ferroui_base::reactive::IDisposable> {
        self.0.color_values_changed(handler)
    }
    fn preferred_application_language_changed(
        &self,
        handler: Rc<dyn Fn()>,
    ) -> Rc<dyn ferroui_base::reactive::IDisposable> {
        self.0.preferred_application_language_changed(handler)
    }
}
