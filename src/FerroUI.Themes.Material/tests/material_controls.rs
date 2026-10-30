//! Behavioral/layout regressions for the Android component templates. These tests
//! deliberately check the arranged controls and template bindings, not XAML text.

use crate::{register_types, MaterialTheme};
use ferroui_base::animation::{IClock, IGlobalClock, PlayState, TimeSpan};
use ferroui_base::layout::{HorizontalAlignment, Orientation, VerticalAlignment};
use ferroui_base::reactive::{IDisposable, IObservable, IObserver, LightweightSubject};
use ferroui_base::{ObjectType, Point, Ref, StaticType, Visual};
use ferroui_controls::automation::peers::{AutomationControlType, ControlAutomationPeer};
use ferroui_controls::automation::provider::IRangeValueProvider;
use ferroui_controls::presenters::TextPresenter;
use ferroui_controls::primitives::{TemplatedControl, Track};
use ferroui_controls::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use ferroui_controls::{
    Border, Button, Canvas, CheckBox, Control, DataValidationErrors, Panel, RepeatButton, Slider,
    TextBlock, TextBox, ToggleSwitch, Window,
};
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

fn application() -> UnitTestApplicationScope {
    register_types();
    UnitTestApplication::start(
        TestServices::styled_window()
            .with_global_clock(Rc::new(TestClock {
                subject: LightweightSubject::new(),
                state: Cell::new(PlayState::Run),
            }))
            .with_theme(|| {
                let theme = MaterialTheme::new();
                theme.set_reduced_motion(true);
                theme.as_style()
            }),
    )
}

fn host(control: &Control) -> Ref<Window> {
    let frame = Border::new();
    frame.set_horizontal_alignment(HorizontalAlignment::Left);
    frame.set_vertical_alignment(VerticalAlignment::Top);
    frame.set_child(control.to_ref());
    let window = Window::new();
    window.set_width(480.0);
    window.set_height(320.0);
    window.set_content(Some(Control::boxed(&frame)));
    window.show();
    window.update_layout();
    window
}

fn named<T: ObjectType>(visual: &Visual, name: &str) -> Ref<T> {
    visual
        .get_visual_descendants()
        .filter(|node| node.name().as_deref() == Some(name))
        .find_map(|node| node.cast::<T>())
        .unwrap_or_else(|| panic!("Material template does not contain {name}"))
}

#[test]
fn material_button_preserves_48dp_target_around_40dp_surface() {
    let _app = application();
    let button = Button::new();
    button.set_content(Some(Rc::new("Save".to_string())));
    let _window = host(&button);
    let surface: Ref<Border> = named(&button, "MaterialContainer");
    assert!(button.bounds().height >= 48.0);
    assert_eq!(40.0, surface.bounds().height);
    assert_eq!(20.0, surface.corner_radius().top_left);
}

#[test]
fn checkbox_container_and_feedback_are_distinct_from_touch_target() {
    let _app = application();
    let check = CheckBox::new();
    let _window = host(&check);
    let outline: Ref<Border> = named(&check, "SelectionOutline");
    let layer: Ref<Border> = named(&check, "MaterialStateLayer");
    assert_eq!(18.0, outline.bounds().width);
    assert_eq!(18.0, outline.bounds().height);
    assert_eq!(40.0, layer.bounds().width);
    assert!(check.bounds().height >= 48.0);
}

#[test]
fn switch_retains_core_knob_travel_with_android_geometry() {
    let _app = application();
    let switch = ToggleSwitch::new();
    let window = host(&switch);
    let track: Ref<Border> = named(&switch, "MaterialSwitchTrack");
    let travel: Ref<Panel> = named(&switch, "PART_SwitchKnob");
    let moving: Ref<Panel> = named(&switch, "PART_MovingKnobs");
    let thumb: Ref<Border> = named(&switch, "MaterialSwitchThumb");
    assert_eq!(52.0, track.bounds().width);
    assert_eq!(32.0, track.bounds().height);
    assert_eq!(20.0, travel.bounds().width);
    assert_eq!(16.0, thumb.bounds().width);
    switch.set_knob_transitions(None);
    moving.set_transitions(None);
    thumb.set_transitions(None);
    switch.set_is_checked(Some(true));
    window.update_layout();
    assert_eq!(20.0, Canvas::get_left(&moving));
    assert_eq!(24.0, thumb.bounds().width);
}

#[test]
fn slider_keeps_value_binding_when_orientation_changes() {
    let _app = application();
    let slider = Slider::new();
    slider.set_width(240.0);
    slider.set_range_value(30.0);
    let window = host(&slider);
    let track: Ref<Track> = named(&slider, "PART_Track");
    let handle: Ref<Border> = named(&slider, "MaterialHandle");
    assert_eq!(30.0, track.value());
    assert_eq!(4.0, handle.bounds().width);
    assert_eq!(44.0, handle.bounds().height);
    slider.set_orientation(Orientation::Vertical);
    slider.set_width(48.0);
    slider.set_height(240.0);
    window.update_layout();
    let track: Ref<Track> = named(&slider, "PART_Track");
    let handle: Ref<Border> = named(&slider, "MaterialHandle");
    assert_eq!(30.0, track.value());
    assert_eq!(44.0, handle.bounds().width);
    assert_eq!(4.0, handle.bounds().height);
    slider.set_range_value(70.0);
    assert_eq!(70.0, track.value());
}

#[test]
fn disabled_slider_resolves_both_track_colors_and_the_stop_indicator() {
    let _app = application();
    let slider = Slider::new();
    slider.set_width(240.0);
    slider.set_range_value(30.0);
    slider.set_is_enabled(false);
    let _window = host(&slider);
    let inactive: Ref<RepeatButton> = named(&slider, "PART_IncreaseButton");
    let track = inactive
        .background()
        .expect("disabled inactive track brush");
    assert_eq!(0.12, track.opacity());
    assert_eq!(255, track.as_solid_color_brush().unwrap().color().a);
    assert_eq!(0.38, inactive.foreground().unwrap().opacity());
    let active: Ref<RepeatButton> = named(&slider, "PART_DecreaseButton");
    assert_eq!(0.38, active.opacity());
    assert_eq!(
        255,
        active
            .background()
            .unwrap()
            .as_solid_color_brush()
            .unwrap()
            .color()
            .a
    );
}

#[test]
fn material_text_field_keeps_editing_and_selection_template_contracts() {
    let _app = application();
    let field = TextBox::new();
    field.set_text(Some("Material"));
    let _window = host(&field);
    let surface: Ref<Border> = named(&field, "BorderElement");
    let presenter: Ref<TextPresenter> = named(&field, "PART_TextPresenter");
    assert_eq!(56.0, surface.bounds().height);
    assert_eq!(Some("Material".to_string()), presenter.text());
    field.set_text(Some("Edited"));
    assert_eq!(Some("Edited".to_string()), presenter.text());
    let templated: &TemplatedControl = &field;
    assert!(templated.template().is_some());
}

#[test]
fn outlined_floating_label_opens_a_real_notch_and_keeps_border_state_selectors() {
    let _app = application();
    let field = TextBox::new();
    field.classes().add("outlined");
    field.classes().add("floatingLabel");
    field.set_placeholder_text(Some("Account"));
    field.set_text(Some("ready"));
    let window = host(&field);
    let surface: Ref<crate::MaterialTextFieldBorder> = named(&field, "BorderElement");
    let validation: Ref<DataValidationErrors> = named(&field, "MaterialValidationRoot");
    let label: Ref<TextBlock> = named(&field, "MaterialFloatingLabel");
    assert_eq!(<Border as StaticType>::TYPE, surface.style_key());
    assert!(!field.clip_to_bounds());
    assert!(!validation.clip_to_bounds());
    assert_eq!(56.0, surface.bounds().height);
    assert_eq!(1.0, surface.notch_open());
    assert_eq!(label.bounds().width, surface.notch_width());
    assert_eq!(16.0, label.line_height());
    assert_eq!(surface.bounds().y - 8.0, label.bounds().y);
    field.pseudo_classes().set(":focus", true);
    window.update_layout();
    assert_eq!(2.0, surface.border_thickness().top);
    field.pseudo_classes().set(":focus", false);
    field.set_text(None);
    window.update_layout();
    assert_eq!(0.0, surface.notch_open());
    assert_eq!(1.0, surface.border_thickness().top);
    assert!(label.bounds().y >= 0.0);
}

#[test]
fn button_variants_keep_disabled_colors_and_flat_hover_surfaces() {
    let _app = application();
    for variant in ["text", "outlined", "tonal", "elevated", "accent"] {
        let button = Button::new();
        button.classes().add(variant);
        button.set_content(Some(Rc::new("Action".to_string())));
        let window = host(&button);
        button.pseudo_classes().set(":pointerover", true);
        window.update_layout();
        let surface: Ref<Border> = named(&button, "MaterialContainer");
        if variant == "text" || variant == "outlined" {
            assert_eq!(
                0,
                surface.box_shadow().count(),
                "{variant} must stay flat on hover"
            );
        }
        button.set_is_enabled(false);
        window.update_layout();
        assert_eq!(
            0.38,
            button.foreground().unwrap().opacity(),
            "disabled {variant} label opacity"
        );
        assert_eq!(
            0,
            surface.box_shadow().count(),
            "disabled {variant} elevation"
        );
        if variant == "outlined" {
            let outline = button.border_brush().unwrap();
            assert_eq!(0.1, outline.opacity());
            assert_eq!(255, outline.as_solid_color_brush().unwrap().color().a);
        }
    }
}

#[test]
fn loading_indicator_preserves_range_and_percentage_semantics() {
    let _app = application();
    let indicator = crate::MaterialLoadingIndicator::new();
    indicator.set_is_indeterminate(false);
    indicator.set_minimum(20.0);
    indicator.set_maximum(120.0);
    let _window = host(&indicator);
    indicator.set_percentage(50.0);
    assert_eq!(70.0, indicator.value());
    assert_eq!(50.0, indicator.percentage());
    assert_eq!(48.0, indicator.bounds().width);
    assert_eq!(48.0, indicator.bounds().height);
    indicator.set_range_value(100.0);
    assert_eq!(80.0, indicator.percentage());
    let peer = ControlAutomationPeer::create_peer_for_element(&indicator);
    assert_eq!(
        AutomationControlType::ProgressBar,
        peer.get_automation_control_type()
    );
    let range = peer
        .get_provider::<dyn IRangeValueProvider>()
        .expect("loading range automation");
    assert!(range.is_read_only());
    assert_eq!(20.0, range.minimum());
    assert_eq!(120.0, range.maximum());
    assert_eq!(100.0, range.value());
    assert!(!indicator.is_hit_test_visible());
    assert!(!indicator.focusable());
    let mark = crate::MaterialSelectionMark::new();
    let peer = ControlAutomationPeer::create_peer_for_element(&mark);
    assert!(
        !peer.is_control_element(),
        "selection decoration must not create an extra accessible control"
    );
    assert!(!mark.is_hit_test_visible());
}

fn animation_application() -> (
    UnitTestApplicationScope,
    Rc<TestClock>,
    Rc<crate::motion::MotionTestSettings>,
) {
    register_types();
    let clock = Rc::new(TestClock {
        subject: LightweightSubject::new(),
        state: Cell::new(PlayState::Run),
    });
    let settings = crate::motion::MotionTestSettings::new();
    let app = UnitTestApplication::start(
        TestServices::styled_window()
            .with_global_clock(clock.clone())
            .with_platform_settings(settings.clone()),
    );
    (app, clock, settings)
}

#[test]
fn progress_animation_releases_clock_on_hide_reduced_motion_and_detach() {
    let (_app, clock, _) = animation_application();
    let indicator = crate::MaterialProgressIndicator::new();
    indicator.set_is_indeterminate(true);
    let window = host(&indicator);
    assert!(clock.subject.has_observers());
    indicator.set_is_visible(false);
    assert!(
        !clock.subject.has_observers(),
        "hidden progress must stop continuous clock work"
    );
    indicator.set_is_visible(true);
    assert!(clock.subject.has_observers());
    indicator.set_reduced_motion(true);
    assert!(!clock.subject.has_observers());
    indicator.set_reduced_motion(false);
    assert!(clock.subject.has_observers());
    window.set_content(None);
    window.update_layout();
    assert!(
        !clock.subject.has_observers(),
        "detached progress must release its clock observer"
    );
}

#[test]
fn loading_animation_honors_live_platform_preference_and_detachment() {
    let (_app, clock, settings) = animation_application();
    let indicator = crate::MaterialLoadingIndicator::new();
    let window = host(&indicator);
    assert!(clock.subject.has_observers());
    settings.animations_enabled.set(false);
    clock.subject.on_next(TimeSpan::from_seconds(0.0));
    assert!(
        !clock.subject.has_observers(),
        "platform motion preference cancels an active loading cycle"
    );
    settings.animations_enabled.set(true);
    indicator.set_is_indeterminate(false);
    indicator.set_is_indeterminate(true);
    assert!(clock.subject.has_observers());
    indicator.set_is_visible(false);
    assert!(!clock.subject.has_observers());
    indicator.set_is_visible(true);
    assert!(clock.subject.has_observers());
    window.set_content(None);
    window.update_layout();
    assert!(!clock.subject.has_observers());
}

#[test]
fn selection_mark_releases_clock_when_settled_and_when_detached() {
    let (_app, clock, _) = animation_application();
    let mark = crate::MaterialSelectionMark::new();
    let window = host(&mark);
    assert!(!clock.subject.has_observers());
    mark.set_is_checked(Some(true));
    assert!(clock.subject.has_observers());
    clock.subject.on_next(TimeSpan::from_seconds(0.0));
    clock.subject.on_next(TimeSpan::from_seconds(1.0));
    assert!(
        !clock.subject.has_observers(),
        "a settled check should not keep requesting frames"
    );
    mark.set_is_checked(None);
    assert!(clock.subject.has_observers());
    mark.set_is_visible(false);
    assert!(!clock.subject.has_observers());
    mark.set_is_visible(true);
    assert!(clock.subject.has_observers());
    window.set_content(None);
    window.update_layout();
    assert!(!clock.subject.has_observers());
}

#[test]
fn ripple_cancels_indications_when_hidden_and_detached() {
    let (_app, _, _) = animation_application();
    let ripple = crate::MaterialRipple::new();
    ripple.set_width(48.0);
    ripple.set_height(48.0);
    let window = host(&ripple);
    ripple.begin_press(Point::new(12.0, 12.0));
    assert_eq!(1, ripple.active_ripple_count());
    ripple.set_is_visible(false);
    assert_eq!(0, ripple.active_ripple_count());
    ripple.begin_press(Point::new(12.0, 12.0));
    assert_eq!(
        0,
        ripple.active_ripple_count(),
        "hidden ripple cannot restart"
    );
    ripple.set_is_visible(true);
    ripple.begin_press(Point::new(12.0, 12.0));
    assert_eq!(1, ripple.active_ripple_count());
    window.set_content(None);
    window.update_layout();
    assert_eq!(0, ripple.active_ripple_count());
}
