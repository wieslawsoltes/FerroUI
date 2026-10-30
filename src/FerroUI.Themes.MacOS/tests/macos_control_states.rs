//! Interaction regressions for the macOS templates. Exercise instantiated controls,
//! actual selectors and layout rather than searching their XAML source text.

use crate::MacOSTheme;
use ferroui_base::animation::{IClock, IGlobalClock, PlayState, TimeSpan, TransitionBase};
use ferroui_base::controls::ResourceKey;
use ferroui_base::input::{IKeyboardDevice, IKeyboardNavigationHandler, InputManager, KeyboardDevice, KeyboardNavigationHandler, KeyModifiers, NavigationMethod};
use ferroui_base::media::{Color, IBrush};
use ferroui_base::metadata::from_markup_value;
use ferroui_base::reactive::{IDisposable, IObservable, IObserver, LightweightSubject};
use ferroui_base::styling::ThemeVariant;
use ferroui_base::threading::Dispatcher;
use ferroui_base::{BoxedValue, CornerRadius, Ref, Thickness};
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::primitives::AdornerLayer;
use ferroui_controls::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use ferroui_controls::shapes::Path;
use ferroui_controls::{Application, Border, Button, CalendarDatePicker, Canvas, ComboBox, ComboBoxItem, ContentControl, Control, Grid, StackPanel, TextBox, ToggleSwitch, Window};
use std::cell::Cell;
use std::rc::Rc;

struct PulseClock {
    ticks: LightweightSubject<TimeSpan>,
    elapsed: Cell<f64>,
    state: Cell<PlayState>,
}

impl PulseClock {
    fn new() -> Self {
        Self { ticks: LightweightSubject::new(), elapsed: Cell::new(0.0), state: Cell::new(PlayState::Run) }
    }

    fn finish_transitions(&self) {
        // A newly subscribed child clock establishes its origin on the first tick.
        // The second passes all theme transition durations without wall-clock sleeps.
        for _ in 0..2 {
            self.advance(1.0);
        }
    }

    fn advance(&self, seconds: f64) {
        self.elapsed.set(self.elapsed.get() + seconds);
        self.ticks.on_next(TimeSpan::from_seconds(self.elapsed.get()));
    }
}

impl IObservable<TimeSpan> for PulseClock {
    fn subscribe(&self, observer: Rc<dyn IObserver<TimeSpan>>) -> Rc<dyn IDisposable> {
        self.ticks.subscribe(observer)
    }
}
impl IClock for PulseClock {
    fn play_state(&self) -> PlayState { self.state.get() }
    fn set_play_state(&self, value: PlayState) { self.state.set(value); }
}
impl IGlobalClock for PulseClock {}

struct Scene {
    theme: Ref<MacOSTheme>,
    clock: Rc<PulseClock>,
    _application: UnitTestApplicationScope,
}

impl Scene {
    fn new() -> Self {
        crate::register_types();
        let clock = Rc::new(PulseClock::new());
        // styled_window supplies rendering/layout, but deliberately no keyboard.
        // Keep its test font services and add the real input/focus services instead
        // of simulating :focus pseudo-classes or weakening the focus assertions.
        let services = TestServices::styled_window()
            .with_keyboard_device(|| Some(KeyboardDevice::new() as Rc<dyn IKeyboardDevice>))
            .with_keyboard_navigation(|| Some(KeyboardNavigationHandler::new() as Rc<dyn IKeyboardNavigationHandler>))
            .with_input_manager(Rc::new(InputManager::new()))
            .with_global_clock(clock.clone())
            .with_theme(|| MacOSTheme::new().as_style());
        let application = UnitTestApplication::start(services);
        let style = Application::current().unwrap().styles().get(0);
        let theme = style.as_object().unwrap().to_ref().cast::<MacOSTheme>().expect("the installed macOS theme");
        Self { theme, clock, _application: application }
    }

    fn show(&self, control: &Control) -> Ref<Window> {
        let window = Window::new();
        window.set_requested_theme_variant(Some(ThemeVariant::light()));
        window.set_width(400.0);
        window.set_height(240.0);
        window.set_content(Some(Control::boxed(control.to_ref())));
        window.show();
        self.settle(&window);
        window.focus_manager().focus(None, NavigationMethod::Unspecified, KeyModifiers::NONE);
        self.settle(&window);
        window
    }

    fn settle(&self, window: &Window) {
        window.update_layout();
        Dispatcher::ui_thread().run_jobs(None);
        self.clock.finish_transitions();
        window.update_layout();
        // Template creation and layout may queue Loaded/resource work. Drain it
        // before callers start measuring an interaction's animation clock.
        Dispatcher::ui_thread().run_jobs(None);
    }

    fn resource_color(&self, key: &str) -> Color {
        let value = self.theme.try_get_resource(&ResourceKey::from(key), Some(&ThemeVariant::light()))
            .unwrap_or_else(|| panic!("missing semantic resource {key}"));
        // Some inherited aliases intentionally resolve to a Color. Brush-valued
        // styled properties convert those; a raw resource lookup does not.
        if let Some(color) = from_markup_value::<Color>(&value) {
            return color;
        }
        let brush = from_markup_value::<Rc<dyn IBrush>>(&value)
            .unwrap_or_else(|| panic!("{key} is neither a color nor a brush"));
        solid(Some(brush))
    }
}

fn text(value: &str) -> Option<BoxedValue> { Some(Rc::new(value.to_owned())) }

fn part(control: &Control, name: &str) -> Ref<Control> {
    control.get_visual_descendants().into_iter().filter_map(|v| v.cast::<Control>())
        .find(|c| c.name().as_deref() == Some(name)).unwrap_or_else(|| panic!("missing {name}"))
}

fn solid(brush: Option<Rc<dyn IBrush>>) -> Color {
    brush.expect("a brush").as_solid_color_brush().expect("a semantic solid brush").color()
}

#[test]
fn switch_knob_travels_between_track_insets_and_captions_remain_optional() {
    let scene = Scene::new();
    let switch = ToggleSwitch::new();
    switch.set_is_checked(Some(false));
    let window = scene.show(&switch);
    let track = part(&switch, "SwitchTrack").cast::<Border>().unwrap();
    let travel = part(&switch, "PART_SwitchKnob").cast::<Canvas>().unwrap();
    let moving = part(&switch, "PART_MovingKnobs").cast::<Grid>().unwrap();
    let labels = part(&switch, "SwitchLabels").cast::<StackPanel>().unwrap();
    assert!(switch.off_content().is_none() && switch.on_content().is_none());
    assert!(!labels.is_visible(), "a caption-free switch must not reserve an invisible label gap");
    assert_eq!(0.0, Canvas::get_left(&moving));
    assert_eq!(track.bounds().width, moving.bounds().width + travel.bounds().width);
    switch.set_is_checked(Some(true));
    scene.settle(&window);
    assert_eq!(travel.bounds().width, Canvas::get_left(&moving));
    assert_eq!(scene.resource_color("ToggleSwitchFillOn"), solid(track.background()));
    switch.set_off_content(text("Off label"));
    switch.set_on_content(text("On label"));
    scene.settle(&window);
    assert!(labels.is_visible());
    assert!(switch.on_content_presenter().unwrap().is_visible());
    assert!(!switch.off_content_presenter().unwrap().is_visible());
    switch.set_is_checked(Some(false));
    scene.settle(&window);
    assert_eq!(0.0, Canvas::get_left(&moving));
    assert!(switch.off_content_presenter().unwrap().is_visible());
    assert!(!switch.on_content_presenter().unwrap().is_visible());
    assert_eq!(scene.resource_color("ToggleSwitchFillOff"), solid(track.background()));
    window.close();
}

#[test]
fn switch_reverses_an_in_flight_transition_from_its_visible_position() {
    let scene = Scene::new();
    let switch = ToggleSwitch::new();
    switch.set_is_checked(Some(false));
    let window = scene.show(&switch);
    let moving = part(&switch, "PART_MovingKnobs").cast::<Grid>().unwrap();
    let travel = part(&switch, "PART_SwitchKnob").cast::<Canvas>().unwrap();
    let transitions = moving.transitions().expect("Loaded must attach the switch knob transitions");
    let transition = transitions.get(0).as_object().and_then(|o| o.cast::<TransitionBase>())
        .expect("the knob's DoubleTransition");
    assert_eq!(TimeSpan::from_milliseconds(180.0), transition.duration(), "the knob transition must resolve its live motion token");
    switch.set_is_checked(Some(true));
    scene.clock.advance(0.01);
    scene.clock.advance(0.09);
    let middle = Canvas::get_left(&moving);
    assert!(middle > 0.0 && middle < travel.bounds().width,
        "the knob must move through intermediate positions: position {middle}, travel {}", travel.bounds().width);
    switch.set_is_checked(Some(false));
    assert_eq!(middle, Canvas::get_left(&moving), "reversing direction must preserve the visible position");
    scene.settle(&window);
    assert_eq!(0.0, Canvas::get_left(&moving));
    window.close();
}

#[test]
fn default_button_uses_primary_prominence_and_disabled_state_wins_over_hover_and_press() {
    let scene = Scene::new();
    let button = Button::new();
    button.set_content(text("Save"));
    button.set_is_default(true);
    let window = scene.show(&button);
    let presenter = part(&button, "PART_ContentPresenter").cast::<ContentPresenter>().unwrap();
    assert_eq!(scene.resource_color("AccentButtonBackground"), solid(presenter.background()));
    assert_eq!(scene.resource_color("AccentButtonForeground"), solid(presenter.foreground()));
    button.pseudo_classes().set(":pointerover", true);
    button.pseudo_classes().set(":pressed", true);
    button.set_is_enabled(false);
    scene.settle(&window);
    assert_eq!(scene.resource_color("AccentButtonBackgroundDisabled"), solid(presenter.background()));
    assert_eq!(scene.resource_color("AccentButtonForegroundDisabled"), solid(presenter.foreground()));
    button.pseudo_classes().set(":pointerover", false);
    button.pseudo_classes().set(":pressed", false);
    button.set_is_enabled(true);
    button.set_is_default(false);
    scene.settle(&window);
    assert_eq!(scene.resource_color("ButtonBackground"), solid(presenter.background()));
    window.close();
}

#[test]
fn per_control_size_classes_update_both_control_and_focus_ring_geometry() {
    let scene = Scene::new();
    let button = Button::new();
    button.set_content(text("Action"));
    button.classes().add("small");
    let window = scene.show(&button);
    assert_eq!(22.0, button.min_height());
    assert_eq!(11.0, button.font_size());
    assert_eq!(CornerRadius::uniform(5.0), button.corner_radius());
    let ring = button.focus_adorner().unwrap().build_typed().unwrap().cast::<Border>().unwrap();
    assert_eq!(button.corner_radius(), ring.corner_radius());
    button.classes().remove("small");
    button.classes().add("large");
    scene.settle(&window);
    assert_eq!(34.0, button.min_height());
    assert_eq!(15.0, button.font_size());
    assert!(button.corner_radius().top_left >= button.min_height() / 2.0);
    let ring = button.focus_adorner().unwrap().build_typed().unwrap().cast::<Border>().unwrap();
    assert_eq!(button.corner_radius(), ring.corner_radius());
    window.close();
}

#[test]
fn focused_text_input_keeps_a_complete_border_without_shifting_the_text() {
    let scene = Scene::new();
    let field = TextBox::new();
    field.set_text(Some("Value"));
    let window = scene.show(&field);
    let border = part(&field, "BorderElement").cast::<Border>().unwrap();
    let before = border.border_thickness();
    assert!(before.left > 0.0);
    assert_eq!(Thickness::uniform(before.left), before, "a full outline, without a bottom-only underline");
    assert!(field.focus_with(NavigationMethod::Tab, KeyModifiers::NONE));
    scene.settle(&window);
    assert_eq!(before, border.border_thickness(), "focus must not alter the content inset");
    let ring = window.get_visual_descendants().into_iter().filter_map(|v| v.cast::<Border>())
        .find(|b| AdornerLayer::get_adorned_element(b).is_some()).expect("a visible keyboard focus ring");
    let thickness = ring.border_thickness();
    assert!(thickness.left >= 2.0);
    assert_eq!(Thickness::uniform(thickness.left), thickness);
    assert_eq!(scene.resource_color("MacOSFocusBrush"), solid(ring.border_brush()));
    let pointer_ring = part(&field, "MacOSPointerFocusRing").cast::<Border>().unwrap();
    assert_eq!(0.0, pointer_ring.opacity(), "keyboard focus must not draw two rings");
    window.focus_manager().focus(None, NavigationMethod::Unspecified, KeyModifiers::NONE);
    assert!(field.focus_with(NavigationMethod::Pointer, KeyModifiers::NONE));
    scene.settle(&window);
    assert_eq!(1.0, pointer_ring.opacity(), "pointer focus also needs a full field outline");
    assert_eq!(before, border.border_thickness());
    assert!(!pointer_ring.is_hit_test_visible());
    window.close();
}

#[test]
fn selected_combobox_item_keeps_its_checkmark_and_label_in_the_same_foreground_state() {
    let scene = Scene::new();
    let item = ComboBoxItem::new();
    item.set_content(text("Selected item"));
    item.set_is_selected(true);
    let window = scene.show(&item);
    let presenter = part(&item, "PART_ContentPresenter").cast::<ContentPresenter>().unwrap();
    let checkmark = part(&item, "MacOSSelectionCheck").cast::<Path>().unwrap();
    let background = part(&item, "LayoutRoot").cast::<Border>().unwrap();
    assert!(checkmark.is_visible());
    assert_eq!(scene.resource_color("ComboBoxItemForegroundSelected"), solid(presenter.foreground()));
    assert_eq!(solid(presenter.foreground()), solid(checkmark.fill()));
    item.pseudo_classes().set(":pointerover", true);
    scene.settle(&window);
    assert_eq!(scene.resource_color("ComboBoxItemForegroundSelectedPointerOver"), solid(presenter.foreground()));
    assert_eq!(solid(presenter.foreground()), solid(checkmark.fill()));
    item.pseudo_classes().set(":pressed", true);
    item.set_is_enabled(false);
    scene.settle(&window);
    assert_eq!(scene.resource_color("ComboBoxItemForegroundDisabled"), solid(presenter.foreground()));
    assert_eq!(solid(presenter.foreground()), solid(checkmark.fill()));
    assert_eq!(scene.resource_color("ComboBoxItemBackgroundDisabled"), solid(background.background()));
    window.close();
}

#[test]
fn calendar_date_picker_focus_surrounds_the_whole_field_when_its_editor_has_focus() {
    let scene = Scene::new();
    let picker = CalendarDatePicker::new();
    let window = scene.show(&picker);
    let editor = part(&picker, "PART_TextBox").cast::<TextBox>().unwrap();
    let border = part(&picker, "Background").cast::<Border>().unwrap();
    let ring = part(&picker, "MacOSInputFocusRing").cast::<Border>().unwrap();
    let thickness = border.border_thickness();
    assert_eq!(0.0, ring.opacity());
    assert!(editor.focus_with(NavigationMethod::Tab, KeyModifiers::NONE));
    scene.settle(&window);
    assert_eq!(1.0, ring.opacity());
    assert_eq!(Thickness::uniform(3.0), ring.border_thickness());
    assert_eq!(thickness, border.border_thickness());
    assert!(ring.bounds().width >= border.bounds().width, "the calendar button belongs inside the focused field");
    picker.set_is_enabled(false);
    scene.settle(&window);
    assert_eq!(0.0, ring.opacity());
    window.close();
}

#[test]
fn combobox_selection_and_editable_text_use_the_actual_foreground() {
    let scene = Scene::new();
    let combo = ComboBox::new();
    combo.items().add(text("First"));
    combo.items().add(text("Second"));
    combo.set_selected_index(1);
    let window = scene.show(&combo);
    let selected = part(&combo, "ContentPresenter").cast::<ContentControl>().unwrap();
    assert_eq!(scene.resource_color("ComboBoxForeground"), solid(selected.foreground()));
    combo.set_is_editable(true);
    scene.settle(&window);
    let editor = part(&combo, "PART_EditableTextBox").cast::<TextBox>().unwrap();
    assert_eq!(solid(combo.foreground()), solid(editor.foreground()), "entered text must not take the placeholder brush");
    combo.pseudo_classes().set(":pointerover", true);
    combo.set_is_enabled(false);
    scene.settle(&window);
    let background = part(&combo, "Background").cast::<Border>().unwrap();
    assert_eq!(scene.resource_color("ComboBoxBackgroundDisabled"), solid(background.background()));
    assert_eq!(scene.resource_color("ComboBoxForegroundDisabled"), solid(selected.foreground()));
    window.close();
}

#[test]
fn increase_contrast_updates_and_restores_existing_window_backgrounds() {
    let scene = Scene::new();
    let window = scene.show(&Border::new());
    for (variant, expected) in [
        (ThemeVariant::light(), Color::from_rgb(255, 255, 255)),
        (ThemeVariant::dark(), Color::from_rgb(28, 28, 30)),
    ] {
        window.set_requested_theme_variant(Some(variant));
        scene.settle(&window);
        let normal = solid(window.background());
        scene.theme.set_increase_contrast(true);
        scene.settle(&window);
        assert_eq!(expected, solid(window.background()));
        assert_eq!(expected, solid(window.transparency_background_fallback()));
        scene.theme.set_increase_contrast(false);
        scene.settle(&window);
        assert_eq!(normal, solid(window.background()));
        assert_eq!(normal, solid(window.transparency_background_fallback()));
    }
    window.close();
}
