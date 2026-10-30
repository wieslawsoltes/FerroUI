//! Native chip anatomy, independent actions, accessibility and retained slot motion.
use super::support::start_themed_application;
use crate::{
    MaterialChip, MaterialChipDismissButton, MaterialChipKind, MaterialChipLayout,
    MaterialMotionRole, MaterialMotionScheme, MaterialTheme, register_types,
};
use ferroui_base::animation::{IClock, IGlobalClock, PlayState, TimeSpan};
use ferroui_base::controls::ResourceKey;
use ferroui_base::input::{
    ICommand, InputElement, Key, KeyEventArgs, KeyModifiers, KeyboardDevice, NavigationMethod,
};
use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
use ferroui_base::media::{BoxShadows, Color, EllipseGeometry, FlowDirection};
use ferroui_base::metadata::from_markup_value;
use ferroui_base::reactive::{
    AnonymousObserver, IDisposable, IObservable, IObserver, LightweightSubject,
};
use ferroui_base::styling::ControlTheme;
use ferroui_base::{AnyValue, BoxedValue, ObjectType, Point, Ref, Visual};
use ferroui_controls::automation::peers::{
    AutomationControlType, ButtonAutomationPeer, ControlAutomationPeer, ToggleButtonAutomationPeer,
};
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::primitives::ToggleButton;
use ferroui_controls::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use ferroui_controls::{Application, Border, Button, Control, Viewbox, Window};
use std::cell::Cell;
use std::rc::Rc;

fn theme(key: &str) -> Ref<ControlTheme> {
    from_markup_value(
        &Application::current()
            .unwrap()
            .try_get_resource(&ResourceKey::from(key), None)
            .flatten(),
    )
    .unwrap_or_else(|| panic!("missing chip theme {key}"))
}
fn color(control: &Control, key: &str) -> Color {
    from_markup_value(
        &control
            .try_find_resource(
                &ResourceKey::from(key),
                control.actual_theme_variant().as_ref(),
            )
            .flatten(),
    )
    .unwrap()
}
fn shadow(control: &Control, level: usize) -> BoxShadows {
    from_markup_value(
        &control
            .try_find_resource(
                &ResourceKey::from(format!("MaterialElevation{level}BoxShadow")),
                control.actual_theme_variant().as_ref(),
            )
            .flatten(),
    )
    .unwrap()
}
fn box_content(width: f64, height: f64) -> BoxedValue {
    let content = Border::new();
    content.set_width(width);
    content.set_height(height);
    Control::boxed(&content)
}
fn chip(key: &str) -> Ref<ToggleButton> {
    let chip = ToggleButton::new();
    chip.set_theme(theme(key));
    chip.set_content(Some(box_content(80.0, 20.0)));
    chip
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
        .filter(|v| v.name().as_deref() == Some(name))
        .find_map(|v| v.cast())
        .unwrap_or_else(|| {
            panic!(
                "missing chip part {name}; descendants: {:?}",
                visual
                    .get_visual_descendants()
                    .map(|v| v.name())
                    .collect::<Vec<_>>()
            )
        })
}
fn brush_color(presenter: &ContentPresenter) -> Color {
    presenter
        .foreground()
        .unwrap()
        .as_solid_color_brush()
        .unwrap()
        .color()
}
fn assert_shadow(control: &Control, surface: &Border, level: usize) {
    assert_eq!(
        shadow(control, level).iter().collect::<Vec<_>>(),
        surface.box_shadow().iter().collect::<Vec<_>>()
    );
}

#[test]
fn assist_and_suggestion_preserve_native_button_invoke_and_label_names() {
    let _app = start_themed_application();
    for key in [
        "MaterialAssistChip",
        "MaterialSuggestionChip",
        "MaterialElevatedAssistChip",
        "MaterialElevatedSuggestionChip",
    ] {
        let button = Button::new();
        button.set_theme(theme(key));
        button.set_content(Some(Rc::new("Add a task".to_owned())));
        let calls = Rc::new(Cell::new(0));
        let count = calls.clone();
        button.click(move |_, _| count.set(count.get() + 1));
        let window = host(&button);
        let peer = ControlAutomationPeer::create_peer_for_element(&button);
        assert_eq!(
            AutomationControlType::Button,
            peer.get_automation_control_type()
        );
        assert_eq!("Add a task", peer.get_name());
        peer.cast::<ButtonAutomationPeer>()
            .unwrap()
            .invoke()
            .unwrap();
        assert_eq!(1, calls.get());
        window.close();
    }
}

#[test]
fn compatibility_keys_get_native_label_insets_and_minimum_surface_and_touch_heights() {
    let _app = start_themed_application();
    for (key, inset, width) in [
        ("MaterialAssistChip", 16.0, 112.0),
        ("MaterialSuggestionChip", 16.0, 112.0),
        ("MaterialFilterChip", 16.0, 112.0),
        ("MaterialInputChip", 12.0, 104.0),
    ] {
        let chip = chip(key);
        let window = host(&chip);
        let label: Ref<ContentPresenter> = named(&chip, "PART_ContentPresenter");
        let surface: Ref<Border> = named(&chip, "ChipContainer");
        assert_eq!(inset, label.bounds().x, "{key}");
        assert_eq!(width, chip.bounds().width, "{key}");
        assert_eq!(48.0, chip.bounds().height);
        assert_eq!(32.0, surface.bounds().height);
        assert_eq!(8.0, surface.bounds().y);
        chip.set_content(Some(box_content(80.0, 52.0)));
        window.update_layout();
        assert_eq!(
            52.0,
            surface.bounds().height,
            "native chip height is a minimum, not a fixed clip"
        );
        window.close();
    }
}

#[test]
fn independent_eighteen_dp_icons_do_not_change_label_or_each_others_color_role() {
    let _app = start_themed_application();
    let chip = chip("MaterialAssistChip");
    MaterialChip::set_leading_icon(&chip, Some(box_content(24.0, 24.0)));
    MaterialChip::set_trailing_icon(&chip, Some(box_content(24.0, 24.0)));
    let _window = host(&chip);
    let layout: Ref<MaterialChipLayout> = named(&chip, "ChipLayout");
    let leading: Ref<Border> = named(&chip, "LeadingSlot");
    let trailing: Ref<Border> = named(&chip, "TrailingSlot");
    let label: Ref<ContentPresenter> = named(&chip, "PART_ContentPresenter");
    assert_eq!((18.0, 18.0), layout.slot_widths());
    assert_eq!(148.0, chip.bounds().width);
    assert_eq!(8.0, leading.bounds().x);
    assert_eq!(34.0, label.bounds().x);
    assert_eq!(122.0, trailing.bounds().x);
    assert_eq!(
        color(&chip, "MaterialPrimaryColor"),
        brush_color(&named::<ContentPresenter>(&chip, "LeadingIcon"))
    );
    assert_eq!(
        color(&chip, "MaterialPrimaryColor"),
        brush_color(&named::<ContentPresenter>(&chip, "TrailingIcon"))
    );
    assert_eq!(color(&chip, "MaterialOnSurfaceColor"), brush_color(&label));
}

#[test]
fn input_avatar_is_circular_twenty_four_dp_and_wins_over_leading_icon_and_selected_check() {
    let _app = start_themed_application();
    let chip = chip("MaterialInputChip");
    MaterialChip::set_leading_icon(&chip, Some(box_content(24.0, 24.0)));
    MaterialChip::set_avatar(&chip, Some(box_content(24.0, 24.0)));
    chip.set_is_checked(Some(true));
    let window = host(&chip);
    let layout: Ref<MaterialChipLayout> = named(&chip, "ChipLayout");
    let avatar: Ref<Border> = named(&chip, "AvatarFrame");
    let label: Ref<ContentPresenter> = named(&chip, "PART_ContentPresenter");
    let leading: Ref<Border> = named(&chip, "LeadingSlot");
    assert_eq!(24.0, avatar.bounds().width);
    assert_eq!(4.0, leading.bounds().x);
    assert_eq!(36.0, label.bounds().x);
    assert_eq!(128.0, chip.bounds().width);
    assert!(avatar.clip().unwrap().is::<EllipseGeometry>());
    assert!(layout.show_avatar());
    assert!(!layout.show_check());
    assert!(!layout.show_leading_icon());
    chip.set_is_enabled(false);
    assert_eq!(0.38, avatar.opacity());
    chip.set_is_enabled(true);
    MaterialChip::set_avatar(&chip, None);
    window.update_layout();
    assert!(layout.show_check());
    assert_eq!(18.0, layout.slot_widths().0);
    assert_eq!(34.0, label.bounds().x);
    MaterialChip::set_show_selected_icon(&chip, false);
    assert!(layout.show_leading_icon());
    MaterialChip::set_leading_icon(&chip, None);
    window.update_layout();
    assert_eq!(12.0, label.bounds().x);
}

#[test]
fn filter_and_input_keep_distinct_selected_icon_roles_and_native_checkbox_automation() {
    let _app = start_themed_application();
    for (key, selected_leading) in [
        ("MaterialFilterChip", "MaterialOnSecondaryContainerColor"),
        ("MaterialInputChip", "MaterialPrimaryColor"),
    ] {
        let chip = chip(key);
        chip.set_content(Some(Rc::new("Documents".to_owned())));
        MaterialChip::set_trailing_icon(&chip, Some(box_content(18.0, 18.0)));
        let _window = host(&chip);
        let peer = ControlAutomationPeer::create_peer_for_element(&chip);
        assert_eq!(
            AutomationControlType::CheckBox,
            peer.get_automation_control_type()
        );
        assert_eq!("Documents", peer.get_name());
        peer.cast::<ToggleButtonAutomationPeer>()
            .unwrap()
            .toggle()
            .unwrap();
        let layout: Ref<MaterialChipLayout> = named(&chip, "ChipLayout");
        assert!(layout.show_check());
        assert_eq!(
            color(&chip, selected_leading),
            brush_color(&named::<ContentPresenter>(&chip, "LeadingIcon"))
        );
        assert_eq!(
            color(&chip, "MaterialOnSecondaryContainerColor"),
            brush_color(&named::<ContentPresenter>(&chip, "TrailingIcon"))
        );
        chip.set_is_enabled(false);
        let surface: Ref<Border> = named(&chip, "ChipContainer");
        assert_eq!(0.12, surface.background().unwrap().opacity());
        assert_eq!(1.0, surface.opacity());
        assert_eq!(
            0.38,
            named::<ContentPresenter>(&chip, "LeadingIcon")
                .foreground()
                .unwrap()
                .opacity()
        );
        assert_eq!(
            0.38,
            named::<ContentPresenter>(&chip, "TrailingIcon")
                .foreground()
                .unwrap()
                .opacity()
        );
    }
}

#[test]
fn expressive_shapes_and_compact_gaps_follow_selection_and_press_then_restore() {
    let _app = start_themed_application();
    for key in [
        "MaterialExpressiveFilterChip",
        "MaterialExpressiveInputChip",
        "MaterialExpressiveElevatedFilterChip",
    ] {
        let chip = chip(key);
        let window = host(&chip);
        let surface: Ref<Border> = named(&chip, "ChipContainer");
        assert_eq!(12.0, surface.corner_radius().top_left);
        chip.set_is_checked(Some(true));
        window.update_layout();
        assert_eq!(16.0, surface.corner_radius().top_left);
        assert_eq!(
            30.0,
            named::<ContentPresenter>(&chip, "PART_ContentPresenter")
                .bounds()
                .x
        );
        chip.pseudo_classes().set(":pressed", true);
        assert_eq!(8.0, surface.corner_radius().top_left);
        chip.pseudo_classes().set(":pressed", false);
        assert_eq!(16.0, surface.corner_radius().top_left);
        MaterialChip::set_is_dismissible(&chip, true);
        window.update_layout();
        assert_eq!(48.0, surface.bounds().height);
        assert_eq!(
            24.0,
            surface.corner_radius().top_left,
            "full selected shape follows actual content height"
        );
        MaterialChip::set_is_dismissible(&chip, false);
        window.update_layout();
        assert_eq!(16.0, surface.corner_radius().top_left);
        chip.set_is_checked(Some(false));
        assert_eq!(12.0, surface.corner_radius().top_left);
        window.close();
    }
}

#[test]
fn elevated_and_flat_filter_elevations_follow_effective_native_interaction_defaults() {
    let _app = start_themed_application();
    for key in [
        "MaterialElevatedAssistChip",
        "MaterialElevatedSuggestionChip",
        "MaterialElevatedFilterChip",
    ] {
        let chip = chip(key);
        let window = host(&chip);
        let surface: Ref<Border> = named(&chip, "ChipContainer");
        assert_shadow(&chip, &surface, 1);
        chip.pseudo_classes().set(":pointerover", true);
        assert_shadow(&chip, &surface, 2);
        chip.pseudo_classes().set(":pressed", true);
        assert_shadow(&chip, &surface, 1);
        chip.pseudo_classes().set(":dragging", true);
        assert_shadow(&chip, &surface, 4);
        chip.set_is_enabled(false);
        assert_shadow(&chip, &surface, 0);
        assert_eq!(0.12, surface.background().unwrap().opacity());
        window.close();
    }
    let chip = chip("MaterialFilterChip");
    let _window = host(&chip);
    let surface: Ref<Border> = named(&chip, "ChipContainer");
    assert_shadow(&chip, &surface, 0);
    chip.pseudo_classes().set(":pointerover", true);
    assert_shadow(&chip, &surface, 1);
    chip.set_is_checked(Some(true));
    assert_shadow(&chip, &surface, 1);
    chip.pseudo_classes().set(":pressed", true);
    assert_shadow(&chip, &surface, 0);
}

struct Command {
    enabled: Cell<bool>,
    calls: Cell<usize>,
    changed: LightweightSubject<()>,
}
impl ICommand for Command {
    fn can_execute(&self, _: Option<&BoxedValue>) -> bool {
        self.enabled.get()
    }
    fn execute(&self, parameter: Option<&BoxedValue>) {
        let value: &dyn AnyValue = &**parameter.unwrap();
        assert_eq!(
            Some(&"document-42".to_owned()),
            value.downcast_ref::<String>()
        );
        self.calls.set(self.calls.get() + 1);
    }
    fn can_execute_changed(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        self.changed
            .subscribe(Rc::new(AnonymousObserver::new(move |_| handler())))
    }
}
#[test]
fn dismiss_is_an_independent_keyboard_and_automation_action_and_obeys_command_availability() {
    let _app = start_themed_application();
    let chip = chip("MaterialInputChip");
    chip.set_content(Some(Rc::new("Document".to_owned())));
    MaterialChip::set_is_dismissible(&chip, true);
    MaterialChip::set_dismiss_label(&chip, "Remove Document");
    let command = Rc::new(Command {
        enabled: Cell::new(true),
        calls: Cell::new(0),
        changed: LightweightSubject::new(),
    });
    MaterialChip::set_dismiss_command(&chip, Some(command.clone()));
    MaterialChip::set_dismiss_command_parameter(&chip, Some(Rc::new("document-42".to_owned())));
    let clicks = Rc::new(Cell::new(0));
    let count = clicks.clone();
    chip.click(move |_, _| count.set(count.get() + 1));
    let window = host(&chip);
    let action: Ref<MaterialChipDismissButton> = named(&chip, "PART_DismissButton");
    let initial_glyph: Ref<ferroui_controls::shapes::Path> = named(&action, "DismissGlyph");
    assert_eq!(1.0, initial_glyph.fill().unwrap().opacity());
    let glyph_transform = initial_glyph.transform_to_visual(&action).unwrap();
    near(18.0 / 24.0, glyph_transform.m11.abs());
    near(
        10.5,
        initial_glyph.rendered_geometry().unwrap().bounds().width * glyph_transform.m11.abs(),
    );
    assert!(
        !action
            .get_visual_descendants()
            .any(|v| v.is::<MaterialChipLayout>()),
        "a nested action must not inherit the chip-body template"
    );
    let surface: Ref<Border> = named(&chip, "ChipContainer");
    assert_eq!(48.0, surface.bounds().height);
    assert_eq!(48.0, action.bounds().width);
    assert_eq!(48.0, action.bounds().height);
    let peer = ControlAutomationPeer::create_peer_for_element(&action);
    assert_eq!(
        AutomationControlType::Button,
        peer.get_automation_control_type()
    );
    assert_eq!("Remove Document", peer.get_name());
    let invoke = peer.cast::<ButtonAutomationPeer>().unwrap();
    invoke.invoke().unwrap();
    assert_eq!(1, command.calls.get());
    assert_eq!(0, clicks.get());
    assert_ne!(Some(true), chip.is_checked());
    assert!(action.focus_with(NavigationMethod::Tab, KeyModifiers::NONE));
    for event in [InputElement::key_down_event(), InputElement::key_up_event()] {
        let mut key = KeyEventArgs::new();
        key.key = Key::Space;
        key.set_routed_event(Some(event));
        action.raise_event(&key);
        assert!(key.handled());
    }
    assert_eq!(2, command.calls.get());
    assert_eq!(0, clicks.get());
    command.enabled.set(false);
    command.changed.on_next(());
    assert!(!action.is_effectively_enabled());
    assert!(chip.is_effectively_enabled());
    window.update_layout();
    let glyph: Ref<ferroui_controls::shapes::Path> = named(&action, "DismissGlyph");
    assert_eq!(0.38, glyph.fill().unwrap().opacity());
    assert!(invoke.invoke().is_err());
    assert!(!MaterialChip::request_dismiss(&chip));
    assert_eq!(2, command.calls.get());
    ControlAutomationPeer::create_peer_for_element(&chip)
        .cast::<ToggleButtonAutomationPeer>()
        .unwrap()
        .toggle()
        .unwrap();
    assert_eq!(Some(true), chip.is_checked());
    assert_eq!(2, command.calls.get());
    assert_eq!(1, clicks.get());
    window.update_layout();
    assert!(
        chip.parent().is_some(),
        "the component never removes application data implicitly"
    );
}

#[test]
fn handled_dismiss_request_cancels_the_command_without_changing_selection() {
    let _app = start_themed_application();
    let chip = chip("MaterialInputChip");
    chip.set_is_checked(Some(true));
    MaterialChip::set_is_dismissible(&chip, true);
    let command = Rc::new(Command {
        enabled: Cell::new(true),
        calls: Cell::new(0),
        changed: LightweightSubject::new(),
    });
    MaterialChip::set_dismiss_command(&chip, Some(command.clone()));
    let requested = Rc::new(Cell::new(0));
    let count = requested.clone();
    MaterialChip::dismiss_requested(&chip, move |_, e| {
        count.set(count.get() + 1);
        e.set_handled(true);
    });
    let _window = host(&chip);
    let action: Ref<MaterialChipDismissButton> = named(&chip, "PART_DismissButton");
    ControlAutomationPeer::create_peer_for_element(&action)
        .cast::<ButtonAutomationPeer>()
        .unwrap()
        .invoke()
        .unwrap();
    assert_eq!(1, requested.get());
    assert_eq!(0, command.calls.get());
    assert_eq!(Some(true), chip.is_checked());
}

#[test]
fn rtl_mirrors_logical_slots_once_and_keeps_check_glyph_and_label_readable() {
    let _app = start_themed_application();
    let chip = chip("MaterialFilterChip");
    chip.set_is_checked(Some(true));
    MaterialChip::set_trailing_icon(&chip, Some(box_content(18.0, 18.0)));
    let window = host(&chip);
    let leading: Ref<Border> = named(&chip, "LeadingSlot");
    let trailing: Ref<Border> = named(&chip, "TrailingSlot");
    assert!(
        leading
            .translate_point(Point::new(9.0, 9.0), &window)
            .unwrap()
            .x
            < trailing
                .translate_point(Point::new(9.0, 9.0), &window)
                .unwrap()
                .x
    );
    let width = chip.bounds().width;
    chip.set_flow_direction(FlowDirection::RightToLeft);
    window.update_layout();
    assert!(
        leading
            .translate_point(Point::new(9.0, 9.0), &window)
            .unwrap()
            .x
            > trailing
                .translate_point(Point::new(9.0, 9.0), &window)
                .unwrap()
                .x
    );
    assert_eq!(width, chip.bounds().width);
    let check: Ref<Viewbox> = named(&chip, "CheckIcon");
    let glyph = check
        .get_visual_descendants()
        .find_map(|v| v.cast::<ferroui_controls::shapes::Path>())
        .unwrap();
    assert!(glyph.transform_to_visual(&window).unwrap().m11 > 0.0);
}

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
        self.state.set(value)
    }
}
impl IGlobalClock for TestClock {}
fn moving_application() -> (UnitTestApplicationScope, Rc<TestClock>) {
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
fn moving_layout(expressive: bool) -> Ref<MaterialChipLayout> {
    let layout = MaterialChipLayout::new();
    layout.set_kind(MaterialChipKind::Filter);
    layout.set_is_expressive(expressive);
    for _ in 0..4 {
        layout.children().add(Border::new().upcast::<Control>());
    }
    layout
}
fn near(expected: f64, actual: f64) {
    assert!(
        (expected - actual).abs() < 1e-9,
        "expected {expected}, got {actual}"
    );
}
#[test]
fn standard_slots_use_distinct_native_entry_and_exit_specs_and_retain_departing_content() {
    let (_app, clock) = moving_application();
    let layout = moving_layout(false);
    let _window = host(&layout);
    layout.set_leading_icon(Some(box_content(18.0, 18.0)));
    clock.subject.on_next(TimeSpan::from_seconds(0.0));
    clock.subject.on_next(TimeSpan::from_seconds(0.05));
    let scheme = MaterialMotionScheme::Expressive;
    let entry = scheme
        .spring(MaterialMotionRole::FastSpatial)
        .sample(0.05, 0.0, 18.0, 0.0);
    let alpha = scheme
        .spring(MaterialMotionRole::SlowEffects)
        .sample(0.05, 0.0, 1.0, 0.0);
    near(entry.value.max(0.0), layout.slot_widths().0);
    near(alpha.value, layout.slot_opacities().0);
    layout.set_leading_icon(None);
    assert!(layout.show_leading_icon());
    assert!(layout.displayed_leading_icon().is_some());
    clock.subject.on_next(TimeSpan::from_seconds(0.10));
    let exit = scheme.spring(MaterialMotionRole::DefaultEffects).sample(
        0.05,
        entry.value,
        0.0,
        entry.velocity,
    );
    let fade = scheme.spring(MaterialMotionRole::FastEffects).sample(
        0.05,
        alpha.value,
        0.0,
        alpha.velocity,
    );
    near(exit.value.max(0.0), layout.slot_widths().0);
    near(fade.value.clamp(0.0, 1.0), layout.slot_opacities().0);
    clock.subject.on_next(TimeSpan::from_seconds(2.0));
    assert_eq!((0.0, 0.0), layout.slot_widths());
    assert!(layout.displayed_leading_icon().is_none());
    assert!(!layout.show_leading_icon());
    assert!(!clock.subject.has_observers());
}
#[test]
fn expressive_slot_reversal_preserves_velocity_with_fast_spatial_and_default_effects() {
    let (_app, clock) = moving_application();
    let layout = moving_layout(true);
    let _window = host(&layout);
    layout.set_is_selected(true);
    clock.subject.on_next(TimeSpan::from_seconds(0.0));
    clock.subject.on_next(TimeSpan::from_seconds(0.05));
    let scheme = MaterialMotionScheme::Expressive;
    let width = scheme
        .spring(MaterialMotionRole::FastSpatial)
        .sample(0.05, 0.0, 18.0, 0.0);
    let alpha = scheme
        .spring(MaterialMotionRole::DefaultEffects)
        .sample(0.05, 0.0, 1.0, 0.0);
    near(width.value, layout.slot_widths().0);
    near(alpha.value, layout.slot_opacities().0);
    layout.set_is_selected(false);
    assert!(layout.show_check());
    clock.subject.on_next(TimeSpan::from_seconds(0.1));
    let reversed = scheme.spring(MaterialMotionRole::FastSpatial).sample(
        0.05,
        width.value,
        0.0,
        width.velocity,
    );
    near(reversed.value.max(0.0), layout.slot_widths().0);
    let faded = scheme.spring(MaterialMotionRole::DefaultEffects).sample(
        0.05,
        alpha.value,
        0.0,
        alpha.velocity,
    );
    near(faded.value, layout.slot_opacities().0);
}
#[test]
fn reduced_motion_can_be_reenabled_and_hidden_or_detached_slots_release_the_clock() {
    let (_app, clock) = moving_application();
    let layout = moving_layout(false);
    let window = host(&layout);
    layout.set_is_selected(true);
    assert!(clock.subject.has_observers());
    layout.set_is_visible(false);
    assert!(!clock.subject.has_observers());
    layout.set_is_visible(true);
    assert!(clock.subject.has_observers());
    layout.set_reduced_motion(true);
    assert_eq!(18.0, layout.slot_widths().0);
    assert_eq!(1.0, layout.slot_opacities().0);
    assert!(!clock.subject.has_observers());
    layout.set_reduced_motion(false);
    layout.set_is_selected(false);
    assert!(clock.subject.has_observers());
    assert_eq!(18.0, layout.slot_widths().0);
    window.set_content(None);
    assert!(!clock.subject.has_observers());
}
#[test]
fn initial_icons_are_settled_and_assist_slot_changes_do_not_animate() {
    let (_app, clock) = moving_application();
    let layout = moving_layout(false);
    layout.set_leading_icon(Some(box_content(18.0, 18.0)));
    let _window = host(&layout);
    assert_eq!((18.0, 0.0), layout.slot_widths());
    assert_eq!((1.0, 0.0), layout.slot_opacities());
    assert!(!clock.subject.has_observers());
    layout.set_kind(MaterialChipKind::Assist);
    layout.set_trailing_icon(Some(box_content(18.0, 18.0)));
    assert_eq!((18.0, 18.0), layout.slot_widths());
    assert!(!clock.subject.has_observers());
    layout.set_leading_icon(None);
    assert_eq!((0.0, 18.0), layout.slot_widths());
    assert!(layout.displayed_leading_icon().is_none());
}
