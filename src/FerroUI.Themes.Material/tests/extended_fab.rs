//! Arranged geometry, inherited Button behavior, and interruption/lifecycle conformance.
use crate::{
    MaterialExtendedFab, MaterialExtendedFabLayout, MaterialExtendedFabSize, MaterialMotionRole,
    MaterialMotionScheme, MaterialTheme, register_types,
};
use ferroui_base::animation::{IClock, IGlobalClock, PlayState, TimeSpan};
use ferroui_base::input::{ICommand, KeyModifiers, KeyboardDevice, NavigationMethod};
use ferroui_base::media::FlowDirection;
use ferroui_base::reactive::{Disposable, IDisposable, IObservable, IObserver, LightweightSubject};
use ferroui_base::{BoxedValue, ObjectType, Point, Ref, Visual};
use ferroui_controls::automation::peers::{
    AutomationControlType, ButtonAutomationPeer, ControlAutomationPeer,
};
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use ferroui_controls::{Border, Control, Viewbox, Window};
use std::cell::Cell;
use std::rc::Rc;

struct TestClock {
    subject: LightweightSubject<TimeSpan>,
    state: Cell<PlayState>,
}
impl IObservable<TimeSpan> for TestClock {
    fn subscribe(&self, observer: Rc<dyn IObserver<TimeSpan>>) -> Rc<dyn IDisposable> {
        self.subject.subscribe(observer)
    }
}
impl IClock for TestClock {
    fn play_state(&self) -> PlayState {
        self.state.get()
    }
    fn set_play_state(&self, value: PlayState) {
        self.state.set(value);
    }
}
impl IGlobalClock for TestClock {}

fn application() -> (UnitTestApplicationScope, Rc<TestClock>) {
    register_types();
    let clock = Rc::new(TestClock {
        subject: LightweightSubject::new(),
        state: Cell::new(PlayState::Run),
    });
    let app = UnitTestApplication::start(
        TestServices::styled_window()
            .with_keyboard_device(|| Some(KeyboardDevice::new()))
            .with_global_clock(clock.clone())
            .with_platform_settings(crate::motion::MotionTestSettings::new())
            .with_theme(|| MaterialTheme::new().as_style()),
    );
    (app, clock)
}

fn window(control: &Control) -> Ref<Window> {
    let window = Window::new();
    window.set_width(640.0);
    window.set_height(320.0);
    window.set_content(Some(Control::boxed(control.to_ref())));
    window.show();
    window.update_layout();
    window
}

fn named<T: ObjectType>(visual: &Visual, name: &str) -> Ref<T> {
    visual
        .get_visual_descendants()
        .filter(|node| node.name().as_deref() == Some(name))
        .find_map(|node| node.cast::<T>())
        .unwrap_or_else(|| panic!("missing FAB part {name}"))
}

fn button(size: MaterialExtendedFabSize, reduced: bool) -> Ref<MaterialExtendedFab> {
    let button = MaterialExtendedFab::new();
    button.set_size_variant(size);
    button.set_reduced_motion(reduced);
    let icon = Border::new();
    icon.set_width(24.0);
    icon.set_height(24.0);
    button.set_icon(Some(Control::boxed(&icon)));
    let label = Border::new();
    label.set_width(96.0);
    label.set_height(24.0);
    button.set_content(Some(Control::boxed(&label)));
    button
}

#[test]
fn expressive_variants_use_native_dimensions_and_corrected_icon_label_gaps() {
    let (_app, _) = application();
    for (variant, height, width, icon_size, start, gap, radius, font) in [
        (
            MaterialExtendedFabSize::Small,
            56.0,
            160.0,
            24.0,
            16.0,
            8.0,
            16.0,
            16.0,
        ),
        (
            MaterialExtendedFabSize::Medium,
            80.0,
            188.0,
            28.0,
            26.0,
            12.0,
            20.0,
            22.0,
        ),
        (
            MaterialExtendedFabSize::Large,
            96.0,
            204.0,
            36.0,
            28.0,
            16.0,
            28.0,
            24.0,
        ),
    ] {
        let button = button(variant, true);
        let window = window(&button);
        let layout: Ref<MaterialExtendedFabLayout> = named(&button, "PART_FabLayout");
        let icon: Ref<Viewbox> = named(&button, "PART_Icon");
        let label: Ref<ContentPresenter> = named(&button, "PART_ContentPresenter");
        assert_eq!(height, layout.bounds().height);
        assert_eq!(width, layout.bounds().width);
        assert_eq!(icon_size, icon.bounds().width);
        assert_eq!(start, icon.bounds().x);
        assert_eq!(gap, label.bounds().x - icon.bounds().right());
        assert_eq!(radius, button.corner_radius().top_left);
        assert_eq!(font, button.font_size());
        button.set_is_expanded(false);
        window.update_layout();
        assert_eq!(height, button.bounds().width);
        assert_eq!(height, button.bounds().height);
        assert!(!label.is_visible());
        button.set_is_expanded(true);
        window.update_layout();
        assert!(label.is_visible());
        assert_eq!(width, button.bounds().width);
        window.close();
    }
}

#[test]
fn initial_collapsed_markup_is_settled_without_starting_an_animation() {
    let (_app, clock) = application();
    let button = button(MaterialExtendedFabSize::Small, false);
    button.set_is_expanded(false);
    let _window = window(&button);
    let layout: Ref<MaterialExtendedFabLayout> = named(&button, "PART_FabLayout");
    let label: Ref<ContentPresenter> = named(&button, "PART_ContentPresenter");
    assert_eq!(0.0, layout.expansion_progress());
    assert_eq!(0.0, layout.label_opacity());
    assert_eq!(56.0, button.bounds().width);
    assert!(!label.is_visible());
    assert!(!clock.subject.has_observers());
}

#[test]
fn collapse_keeps_label_until_settled_and_uses_independent_native_springs() {
    let (_app, clock) = application();
    let button = button(MaterialExtendedFabSize::Small, false);
    let window = window(&button);
    let layout: Ref<MaterialExtendedFabLayout> = named(&button, "PART_FabLayout");
    let label: Ref<ContentPresenter> = named(&button, "PART_ContentPresenter");
    button.set_is_expanded(false);
    assert!(label.is_visible());
    assert!(clock.subject.has_observers());
    clock.subject.on_next(TimeSpan::from_seconds(0.0));
    clock.subject.on_next(TimeSpan::from_seconds(0.05));
    let scheme = MaterialMotionScheme::Expressive;
    let spatial = scheme
        .spring(MaterialMotionRole::FastSpatial)
        .sample(0.05, 1.0, 0.0, 0.0);
    let effects = scheme
        .spring(MaterialMotionRole::FastEffects)
        .sample(0.05, 1.0, 0.0, 0.0);
    assert!((spatial.value - layout.expansion_progress()).abs() < 1e-10);
    assert!((effects.value - label.opacity()).abs() < 1e-10);
    assert!(label.is_visible());
    window.update_layout();
    assert!(button.bounds().width > 56.0 && button.bounds().width < 160.0);
    clock.subject.on_next(TimeSpan::from_seconds(2.0));
    window.update_layout();
    assert!(!label.is_visible());
    assert_eq!(56.0, button.bounds().width);
    assert!(!clock.subject.has_observers());
}

#[test]
fn reversing_expansion_preserves_spatial_velocity_and_current_width() {
    let (_app, clock) = application();
    let button = button(MaterialExtendedFabSize::Small, false);
    let _window = window(&button);
    let layout: Ref<MaterialExtendedFabLayout> = named(&button, "PART_FabLayout");
    button.set_is_expanded(false);
    clock.subject.on_next(TimeSpan::from_seconds(0.0));
    clock.subject.on_next(TimeSpan::from_seconds(0.08));
    let spring = MaterialMotionScheme::Expressive.spring(MaterialMotionRole::FastSpatial);
    let interrupted = spring.sample(0.08, 1.0, 0.0, 0.0);
    let before = layout.expansion_progress();
    button.set_is_expanded(true);
    assert_eq!(
        before,
        layout.expansion_progress(),
        "retargeting must not jump to an endpoint"
    );
    clock.subject.on_next(TimeSpan::from_seconds(0.13));
    let expected = spring
        .sample(0.05, interrupted.value, 1.0, interrupted.velocity)
        .value;
    let discarded_velocity = spring.sample(0.05, interrupted.value, 1.0, 0.0).value;
    assert!((expected - layout.expansion_progress()).abs() < 1e-10);
    assert!((discarded_velocity - layout.expansion_progress()).abs() > 0.01);
}

#[test]
fn reduced_motion_snaps_an_active_expansion_and_removes_clock_work() {
    let (_app, clock) = application();
    let button = button(MaterialExtendedFabSize::Medium, false);
    let window = window(&button);
    button.set_is_expanded(false);
    clock.subject.on_next(TimeSpan::from_seconds(0.0));
    clock.subject.on_next(TimeSpan::from_seconds(0.03));
    button.set_reduced_motion(true);
    window.update_layout();
    let layout: Ref<MaterialExtendedFabLayout> = named(&button, "PART_FabLayout");
    let label: Ref<ContentPresenter> = named(&button, "PART_ContentPresenter");
    assert_eq!(0.0, layout.expansion_progress());
    assert_eq!(80.0, button.bounds().width);
    assert!(!label.is_visible());
    assert!(!clock.subject.has_observers());
    button.set_reduced_motion(false);
    assert!(
        !clock.subject.has_observers(),
        "re-enabling motion must not revive the cancelled trajectory"
    );
}

#[test]
fn hidden_and_detached_fab_layouts_stop_their_animation_subscriptions() {
    let (_app, clock) = application();
    let button = button(MaterialExtendedFabSize::Small, false);
    let window = window(&button);
    button.set_is_expanded(false);
    assert!(clock.subject.has_observers());
    button.set_is_visible(false);
    assert!(!clock.subject.has_observers());
    button.set_is_visible(true);
    assert!(clock.subject.has_observers());
    window.set_content(None);
    window.update_layout();
    assert!(!clock.subject.has_observers());
}

#[test]
fn label_updates_while_collapsed_are_measured_on_the_next_expansion() {
    let (_app, _) = application();
    let button = button(MaterialExtendedFabSize::Small, true);
    let window = window(&button);
    button.set_is_expanded(false);
    window.update_layout();
    let new_label = Border::new();
    new_label.set_width(180.0);
    new_label.set_height(24.0);
    button.set_content(Some(Control::boxed(&new_label)));
    window.update_layout();
    assert_eq!(56.0, button.bounds().width);
    button.set_is_expanded(true);
    window.update_layout();
    assert_eq!(244.0, button.bounds().width);
}

struct CountCommand {
    calls: Rc<Cell<usize>>,
}
impl ICommand for CountCommand {
    fn can_execute(&self, _parameter: Option<&BoxedValue>) -> bool {
        true
    }
    fn execute(&self, parameter: Option<&BoxedValue>) {
        assert!(parameter.is_some());
        self.calls.set(self.calls.get() + 1);
    }
    fn can_execute_changed(&self, _handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        Disposable::empty()
    }
}

#[test]
fn expanded_and_collapsed_fabs_preserve_button_command_click_and_invoke_semantics() {
    let (_app, _) = application();
    let button = button(MaterialExtendedFabSize::Small, true);
    button.set_content(Some(Rc::new("Compose".to_owned())));
    let calls = Rc::new(Cell::new(0));
    button.set_command(Some(Rc::new(CountCommand {
        calls: calls.clone(),
    })));
    button.set_command_parameter(Some(Rc::new("document".to_owned())));
    let clicks = Rc::new(Cell::new(0));
    let count = clicks.clone();
    button.click(move |_, _| count.set(count.get() + 1));
    let window = window(&button);
    let focus_ring: Ref<Border> = named(&button, "FocusRing");
    assert!(!focus_ring.is_visible());
    assert!(button.focus_with(NavigationMethod::Tab, KeyModifiers::NONE));
    window.update_layout();
    assert!(focus_ring.is_visible());
    assert_eq!(button.corner_radius(), focus_ring.corner_radius());
    let peer = ControlAutomationPeer::create_peer_for_element(&button);
    assert_eq!(
        AutomationControlType::Button,
        peer.get_automation_control_type()
    );
    assert_eq!("Compose", peer.get_name());
    let invoke = peer.cast::<ButtonAutomationPeer>().unwrap();
    invoke.invoke().unwrap();
    button.set_is_expanded(false);
    window.update_layout();
    assert_eq!(
        "Compose",
        peer.get_name(),
        "the accessible action name survives label collapse"
    );
    invoke.invoke().unwrap();
    assert_eq!(2, calls.get());
    assert_eq!(2, clicks.get());
    button.set_is_enabled(false);
    assert!(invoke.invoke().is_err());
    assert_eq!(2, calls.get());
}

#[test]
fn rtl_places_the_icon_at_logical_start_without_mirroring_label_text() {
    let (_app, _) = application();
    let button = button(MaterialExtendedFabSize::Small, true);
    let window = window(&button);
    let icon: Ref<Viewbox> = named(&button, "PART_Icon");
    let label: Ref<ContentPresenter> = named(&button, "PART_ContentPresenter");
    let ltr_icon = icon
        .translate_point(Point::new(12.0, 12.0), &window)
        .unwrap();
    let ltr_label = label
        .translate_point(Point::new(48.0, 12.0), &window)
        .unwrap();
    assert!(ltr_icon.x < ltr_label.x);
    button.set_flow_direction(FlowDirection::RightToLeft);
    window.update_layout();
    let rtl_icon = icon
        .translate_point(Point::new(12.0, 12.0), &window)
        .unwrap();
    let rtl_label = label
        .translate_point(Point::new(48.0, 12.0), &window)
        .unwrap();
    assert!(rtl_icon.x > rtl_label.x);
    assert_eq!(160.0, button.bounds().width);
}
