//! Pinned AndroidX TimePicker.kt geometry, valid-time/input separation, real input,
//! accessibility, and deterministic animation lifecycle regression coverage.
use super::support::start_themed_application;
use crate::time_picker_state::{
    DialGeometry, TimePickerMetrics, angle_for, parse_input, shortest_target,
};
use crate::{
    MaterialTheme, MaterialTimeDisplayMode, MaterialTimePicker, MaterialTimePickerDial,
    MaterialTimePickerField, MaterialTimeSelection, register_types,
};
use ferroui_base::animation::{IClock, IGlobalClock, PlayState, TimeSpan};
use ferroui_base::input::{
    IPointer, InputElement, Key, KeyEventArgs, KeyModifiers, KeyboardDevice, MouseButton,
    NavigationMethod, Pointer, PointerEventArgs, PointerPointProperties, PointerPressedEventArgs,
    PointerReleasedEventArgs, PointerType, PointerUpdateKind, RawInputModifiers,
    TextInputEventArgs,
};
use ferroui_base::layout::{HorizontalAlignment, Orientation, VerticalAlignment};
use ferroui_base::media::FlowDirection;
use ferroui_base::reactive::{IDisposable, IObservable, IObserver, LightweightSubject};
use ferroui_base::{ObjectType, Point, Ref, Size, Visual};
use ferroui_controls::automation::peers::{AutomationControlType, ControlAutomationPeer};
use ferroui_controls::automation::provider::{
    IInvokeProvider, ISelectionItemProvider, IValueProvider,
};
use ferroui_controls::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use ferroui_controls::{Border, Control, RadioButton, TextBlock, Window};
use std::cell::{Cell, RefCell};
use std::f64::consts::{FRAC_PI_2, PI, TAU};
use std::rc::Rc;

fn host(picker: &MaterialTimePicker) -> Ref<Window> {
    let frame = Border::new();
    frame.set_horizontal_alignment(HorizontalAlignment::Left);
    frame.set_vertical_alignment(VerticalAlignment::Top);
    frame.set_child(picker.to_ref());
    let window = Window::new();
    window.set_width(760.0);
    window.set_height(580.0);
    window.set_content(Some(Control::boxed(&frame)));
    window.show();
    window.update_layout();
    KeyboardDevice::instance().unwrap().set_focused_element(
        None,
        NavigationMethod::Unspecified,
        KeyModifiers::NONE,
    );
    window
}
fn named<T: ObjectType>(visual: &Visual, name: &str) -> Ref<T> {
    visual
        .get_visual_descendants()
        .filter(|v| v.name().as_deref() == Some(name))
        .find_map(|v| v.cast())
        .unwrap_or_else(|| panic!("missing {name}"))
}
fn key(control: &InputElement, key: Key, down: bool) -> bool {
    let mut e = KeyEventArgs::new();
    e.key = key;
    e.set_routed_event(Some(if down {
        InputElement::key_down_event()
    } else {
        InputElement::key_up_event()
    }));
    control.raise_event(&e);
    e.handled()
}
fn type_text(field: &MaterialTimePickerField, text: &str) {
    let mut e = TextInputEventArgs::new();
    e.text = Some(text.into());
    e.set_routed_event(Some(InputElement::text_input_event()));
    field.raise_event(&e);
}
fn dial_point(dial: &MaterialTimePickerDial, window: &Window, angle: f64, inner: bool) -> Point {
    dial.translate_point(
        DialGeometry::new(dial.bounds().size()).center_at(angle, inner),
        window,
    )
    .unwrap()
}
fn press(dial: &MaterialTimePickerDial, window: &Window, pointer: &Rc<Pointer>, point: Point) {
    let e = PointerPressedEventArgs::new(
        dial.to_ref(),
        pointer.clone(),
        window,
        point,
        0,
        PointerPointProperties::new(
            RawInputModifiers::LEFT_MOUSE_BUTTON,
            PointerUpdateKind::LeftButtonPressed,
        ),
        KeyModifiers::NONE,
        1,
    );
    dial.raise_event(&e);
    assert!(e.handled());
}
fn release(dial: &MaterialTimePickerDial, window: &Window, pointer: &Rc<Pointer>, point: Point) {
    let e = PointerReleasedEventArgs::new(
        dial.to_ref(),
        pointer.clone(),
        window,
        point,
        2,
        PointerPointProperties::new(
            RawInputModifiers::NONE,
            PointerUpdateKind::LeftButtonReleased,
        ),
        KeyModifiers::NONE,
        MouseButton::Left,
    );
    dial.raise_event(&e);
}
fn moved(dial: &MaterialTimePickerDial, window: &Window, pointer: &Rc<Pointer>, point: Point) {
    let e = PointerEventArgs::new(
        Some(InputElement::pointer_moved_event()),
        dial.to_ref(),
        pointer.clone(),
        Some(window),
        point,
        1,
        PointerPointProperties::new(
            RawInputModifiers::LEFT_MOUSE_BUTTON,
            PointerUpdateKind::Other,
        ),
        KeyModifiers::NONE,
    );
    dial.raise_event(&e);
}

#[test]
fn native_clock_centers_and_strict_24_hour_ring_boundary_are_scale_invariant() {
    for diameter in [200.0, 238.0, 256.0] {
        let g = DialGeometry::new(Size::new(diameter, diameter));
        assert!((g.center_at(-FRAC_PI_2, false).y - 27.0 * g.scale).abs() < 1e-9);
        assert!((g.center_at(-FRAC_PI_2, true).y - 59.0 * g.scale).abs() < 1e-9);
        let near = Point::new(g.center.x, g.center.y - 73.999 * g.scale);
        let edge = Point::new(g.center.x, g.center.y - 74.001 * g.scale);
        assert_eq!(
            12,
            g.value_at(near, MaterialTimeSelection::Hour, true, false, true)
                .0
        );
        assert_eq!(
            0,
            g.value_at(edge, MaterialTimeSelection::Hour, true, false, true)
                .0
        );
    }
}
#[test]
fn minute_taps_snap_to_five_while_drags_preserve_every_minute() {
    let g = DialGeometry::new(Size::new(256.0, 256.0));
    for minute in 0..60 {
        let p = g.center_at(TAU * minute as f64 / 60.0 - FRAC_PI_2, false);
        assert_eq!(
            minute,
            g.value_at(p, MaterialTimeSelection::Minute, false, false, false)
                .0
        );
        assert_eq!(
            ((minute as f64 / 5.0).round() as i32 * 5) % 60,
            g.value_at(p, MaterialTimeSelection::Minute, false, false, true)
                .0
        );
    }
}
#[test]
fn shortest_rotation_crosses_midnight_without_a_long_turn() {
    let from = angle_for(11, 0, MaterialTimeSelection::Hour);
    let to = angle_for(0, 0, MaterialTimeSelection::Hour);
    assert!((shortest_target(from, to) - from - TAU / 12.0).abs() < 1e-12);
    assert!((shortest_target(0.0, -PI) + PI).abs() < 1e-12);
}
#[test]
fn current_component_widths_and_landscape_breakpoints_override_stale_tokens() {
    let classic = TimePickerMetrics::new(
        false,
        true,
        MaterialTimeDisplayMode::Dial,
        Orientation::Vertical,
        900.0,
    );
    assert_eq!(96.0, classic.field_width);
    assert_eq!(Size::new(256.0, 396.0), classic.desired_size(true, false));
    let vibrant = TimePickerMetrics::new(
        true,
        true,
        MaterialTimeDisplayMode::Dial,
        Orientation::Vertical,
        900.0,
    );
    assert_eq!(132.0, vibrant.field_width);
    assert_eq!(Size::new(280.0, 388.0), vibrant.desired_size(true, true));
    for (height, side) in [
        (329.0, 200.0),
        (330.0, 238.0),
        (383.0, 238.0),
        (384.0, 256.0),
    ] {
        assert_eq!(
            side,
            TimePickerMetrics::new(
                false,
                false,
                MaterialTimeDisplayMode::Dial,
                Orientation::Horizontal,
                height
            )
            .dial_size
        );
    }
}
#[test]
fn input_parser_distinguishes_rejected_syntax_invalid_drafts_and_empty_values() {
    assert_eq!(
        Err(()),
        parse_input("1x", MaterialTimeSelection::Hour, true, false)
    );
    assert_eq!(
        Err(()),
        parse_input("123", MaterialTimeSelection::Hour, true, false)
    );
    assert_eq!(
        Ok(None),
        parse_input("24", MaterialTimeSelection::Hour, true, false)
    );
    assert_eq!(
        Ok(None),
        parse_input("60", MaterialTimeSelection::Minute, false, false)
    );
    assert_eq!(
        Ok(Some(0)),
        parse_input("12", MaterialTimeSelection::Hour, false, false)
    );
    assert_eq!(
        Ok(Some(12)),
        parse_input("", MaterialTimeSelection::Hour, false, true)
    );
}
#[test]
fn selected_time_updates_atomically_and_format_changes_preserve_the_value() {
    let _app = start_themed_application();
    let picker = MaterialTimePicker::new();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let observed = changes.clone();
    picker.add_handler(MaterialTimePicker::time_changed_event(), move |_, e| {
        observed.borrow_mut().push((e.old_time(), e.new_time()))
    });
    picker.set_time(23, 59);
    assert_eq!(1, changes.borrow().len());
    assert_eq!(TimeSpan::from_minutes(1439.0), picker.selected_time());
    assert_eq!("11", picker.hour_input());
    picker.set_is_24_hour(true);
    assert_eq!("23", picker.hour_input());
    picker.set_display_mode(MaterialTimeDisplayMode::Input);
    assert_eq!(1, changes.borrow().len());
    picker.set_selected_time(TimeSpan::from_minutes(65.0));
    assert_eq!((1, 5), (picker.hour(), picker.minute()));
    assert_eq!(2, changes.borrow().len());
}
#[test]
fn invalid_numeric_drafts_keep_the_last_valid_time_and_block_period_changes() {
    let _app = start_themed_application();
    let picker = MaterialTimePicker::new();
    picker.set_time(21, 42);
    assert!(picker.set_input(MaterialTimeSelection::Hour, "99"));
    assert!(!picker.is_input_valid());
    assert_eq!(21, picker.hour());
    assert_eq!("99", picker.hour_input());
    picker.set_period(false);
    assert_eq!(21, picker.hour());
    assert!(!picker.set_input(MaterialTimeSelection::Hour, "abc"));
    assert_eq!("99", picker.hour_input());
    assert!(picker.set_input(MaterialTimeSelection::Hour, "12"));
    assert!(picker.is_input_valid());
    assert_eq!(12, picker.hour());
    picker.set_period(false);
    assert_eq!(0, picker.hour());
    picker.set_input(MaterialTimeSelection::Minute, "");
    assert_eq!(0, picker.minute());
    assert_eq!("", picker.minute_input());
}
#[test]
fn classic_and_vibrant_actual_templates_have_native_geometry_and_type() {
    let _app = start_themed_application();
    let picker = MaterialTimePicker::new();
    let window = host(&picker);
    let hour: Ref<RadioButton> = named(&picker, "PART_Hour");
    let minute: Ref<RadioButton> = named(&picker, "PART_Minute");
    let am: Ref<RadioButton> = named(&picker, "PART_Am");
    let pm: Ref<RadioButton> = named(&picker, "PART_Pm");
    let dial: Ref<MaterialTimePickerDial> = named(&picker, "PART_Dial");
    assert_eq!(Size::new(96.0, 80.0), hour.bounds().size());
    assert_eq!(120.0, minute.bounds().x - hour.bounds().x);
    assert_eq!(Size::new(52.0, 38.0), am.bounds().size());
    assert_eq!(4.0, pm.bounds().y - am.bounds().bottom());
    assert_eq!(36.0, dial.bounds().y - hour.bounds().bottom());
    assert_eq!(Size::new(256.0, 256.0), dial.bounds().size());
    assert_eq!(57.0, hour.font_size());
    let numbers = dial.numbers();
    assert_eq!(12, numbers.iter().filter(|n| n.is_visible()).count());
    assert_eq!(16.0, numbers[0].font_size());
    picker.set_is_vibrant(true);
    window.update_layout();
    assert_eq!(Size::new(100.0, 120.0), hour.bounds().size());
    assert_eq!(12.0, dial.bounds().y - hour.bounds().bottom());
    picker.set_is_24_hour(true);
    window.update_layout();
    assert_eq!(132.0, hour.bounds().width);
    assert!(!am.is_visible());
    assert_eq!(24, numbers.iter().filter(|n| n.is_visible()).count());
}
#[test]
fn rtl_mirrors_outer_groups_and_retains_left_to_right_time_and_clock() {
    let _app = start_themed_application();
    let picker = MaterialTimePicker::new();
    picker.set_flow_direction(FlowDirection::RightToLeft);
    let window = host(&picker);
    let hour: Ref<RadioButton> = named(&picker, "PART_Hour");
    let minute: Ref<RadioButton> = named(&picker, "PART_Minute");
    let am: Ref<RadioButton> = named(&picker, "PART_Am");
    assert!(am.bounds().x < hour.bounds().x);
    assert!(hour.bounds().x < minute.bounds().x);
    picker.set_orientation(Orientation::Horizontal);
    window.update_layout();
    let dial: Ref<MaterialTimePickerDial> = named(&picker, "PART_Dial");
    assert!(dial.bounds().right() < hour.bounds().x);
    let numbers = dial.numbers();
    assert!(numbers[3].bounds().x > numbers[9].bounds().x);
}

#[test]
fn rtl_landscape_keeps_native_absolute_am_pm_order_in_classic_and_vibrant_layouts() {
    let _app = start_themed_application();
    for vibrant in [false, true] {
        let picker = MaterialTimePicker::new();
        picker.set_is_vibrant(vibrant);
        picker.set_orientation(Orientation::Horizontal);
        picker.set_flow_direction(FlowDirection::RightToLeft);
        let window = host(&picker);
        let hour: Ref<RadioButton> = named(&picker, "PART_Hour");
        let minute: Ref<RadioButton> = named(&picker, "PART_Minute");
        let am: Ref<RadioButton> = named(&picker, "PART_Am");
        let pm: Ref<RadioButton> = named(&picker, "PART_Pm");
        let dial: Ref<MaterialTimePickerDial> = named(&picker, "PART_Dial");
        assert!(
            dial.bounds().right() < hour.bounds().x,
            "outer groups still mirror"
        );
        assert!(
            hour.bounds().x < minute.bounds().x,
            "time remains left to right"
        );
        assert_eq!(hour.bounds().x, am.bounds().x);
        assert_eq!(am.bounds().y, pm.bounds().y);
        assert!(
            am.bounds().x < pm.bounds().x,
            "HorizontalPeriodToggle uses absolute place, not placeRelative"
        );
        assert_eq!(
            if vibrant { 16.0 } else { 4.0 },
            pm.bounds().x - am.bounds().right()
        );
        assert_eq!(
            if vibrant {
                Size::new(100.0, 40.0)
            } else {
                Size::new(106.0, 38.0)
            },
            am.bounds().size()
        );
        assert_eq!(am.bounds().size(), pm.bounds().size());
        window.close();
    }
}
#[test]
fn keyboard_input_template_preserves_drafts_and_native_editor_semantics() {
    let _app = start_themed_application();
    let picker = MaterialTimePicker::new();
    picker.set_display_mode(MaterialTimeDisplayMode::Input);
    picker.set_time(13, 7);
    let window = host(&picker);
    let hour: Ref<MaterialTimePickerField> = named(&picker, "PART_HourField");
    let minute: Ref<MaterialTimePickerField> = named(&picker, "PART_MinuteField");
    assert_eq!(Size::new(96.0, 72.0), hour.bounds().size());
    assert_eq!(45.0, hour.font_size());
    assert_eq!(Some("01".into()), hour.text());
    hour.focus();
    hour.select_all();
    type_text(&hour, "9");
    assert_eq!(21, picker.hour());
    assert_eq!(MaterialTimeSelection::Minute, picker.selection());
    assert!(minute.is_focused());
    minute.select_all();
    type_text(&minute, "99");
    assert!(!picker.is_minute_input_valid());
    assert_eq!(7, picker.minute());
    let label: Ref<TextBlock> = named(&picker, "PART_MinuteLabel");
    assert_eq!(Some(picker.minute_error_text()), label.text());
    type_text(&minute, "2");
    assert_eq!(2, picker.minute());
    assert_eq!(Some("2".into()), minute.text());
    picker.set_display_mode(MaterialTimeDisplayMode::Dial);
    picker.set_display_mode(MaterialTimeDisplayMode::Input);
    window.update_layout();
    assert_eq!(Some("2".into()), minute.text());
}
#[test]
fn two_digit_insertion_replaces_the_field_without_breaking_selection_replacement() {
    let _app = start_themed_application();
    let picker = MaterialTimePicker::new();
    picker.set_display_mode(MaterialTimeDisplayMode::Input);
    picker.set_is_24_hour(true);
    picker.set_auto_advance(false);
    picker.set_time(14, 0);
    let _window = host(&picker);
    let hour: Ref<MaterialTimePickerField> = named(&picker, "PART_HourField");
    hour.focus();
    hour.set_caret_index(1);
    hour.set_selection_start(1);
    hour.set_selection_end(1);
    type_text(&hour, "2");
    assert_eq!(2, picker.hour());
    assert_eq!(Some("2".into()), hour.text());
    hour.select_all();
    type_text(&hour, "23");
    assert_eq!(23, picker.hour());
    hour.select_all();
    type_text(&hour, "xx");
    assert_eq!(23, picker.hour());
    assert_eq!(Some("23".into()), hour.text());
}

#[test]
fn clipboard_ime_and_value_provider_edits_share_validation_without_losing_selection() {
    use ferroui_base::threading::Dispatcher;
    use ferroui_controls::PastingFromClipboardEventArgs;
    use ferroui_controls::presenters::TextPresenter;
    use ferroui_controls::testing::TestClipboardImpl;
    let _app = start_themed_application();
    let picker = MaterialTimePicker::new();
    picker.set_display_mode(MaterialTimeDisplayMode::Input);
    picker.set_is_24_hour(true);
    picker.set_auto_advance(false);
    picker.set_time(14, 23);
    let _window = host(&picker);
    let field: Ref<MaterialTimePickerField> = named(&picker, "PART_HourField");
    field.focus();
    field.set_selection_start(1);
    field.set_selection_end(2);
    let clipboard = TestClipboardImpl::new();
    clipboard.set_text(Some("123"));
    let args = PastingFromClipboardEventArgs::new(
        ferroui_controls::TextBox::pasting_from_clipboard_event(),
        Some(clipboard.clipboard()),
    );
    field.raise_event(&args);
    Dispatcher::ui_thread().run_jobs(None);
    assert!(args.handled());
    assert_eq!(Some("14".into()), field.text());
    assert_eq!((1, 2), (field.selection_start(), field.selection_end()));
    clipboard.set_text(Some("9"));
    let args = PastingFromClipboardEventArgs::new(
        ferroui_controls::TextBox::pasting_from_clipboard_event(),
        Some(clipboard.clipboard()),
    );
    field.raise_event(&args);
    Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(19, picker.hour());
    let presenter: Ref<TextPresenter> = named(&field, "PART_TextPresenter");
    presenter.set_preedit_text(Some("2"));
    assert_eq!(19, picker.hour());
    presenter.set_preedit_text(None);
    field.select_all();
    type_text(&field, "21");
    assert_eq!(21, picker.hour());
    let peer = ControlAutomationPeer::create_peer_for_element(&field);
    let value = peer.get_provider::<dyn IValueProvider>().unwrap();
    value.set_value(Some("99")).unwrap();
    assert_eq!(21, picker.hour());
    assert!(!picker.is_hour_input_valid());
    assert_eq!(Some("99".into()), field.text());
    value.set_value(Some("invalid")).unwrap();
    assert_eq!(Some("99".into()), field.text());
    picker.set_is_read_only(true);
    assert!(value.is_read_only());
    assert!(field.focus());
    field.select_all();
    assert_eq!((0, 2), (field.selection_start(), field.selection_end()));
    type_text(&field, "4");
    assert_eq!(Some("99".into()), field.text());
}

#[test]
fn independent_pickers_keep_hour_period_and_dial_groups_separate() {
    use ferroui_controls::StackPanel;
    let _app = start_themed_application();
    let first = MaterialTimePicker::new();
    let second = MaterialTimePicker::new();
    second.set_time(15, 0);
    let panel = StackPanel::new();
    panel.children().add(first.clone());
    panel.children().add(second.clone());
    let window = Window::new();
    window.set_content(Some(Control::boxed(&panel)));
    window.show();
    window.update_layout();
    let first_hour: Ref<RadioButton> = named(&first, "PART_Hour");
    let first_am: Ref<RadioButton> = named(&first, "PART_Am");
    let second_hour: Ref<RadioButton> = named(&second, "PART_Hour");
    let second_pm: Ref<RadioButton> = named(&second, "PART_Pm");
    assert_eq!(Some(true), first_hour.is_checked());
    assert_eq!(Some(true), first_am.is_checked());
    assert_eq!(Some(true), second_hour.is_checked());
    assert_eq!(Some(true), second_pm.is_checked());
    let first_dial: Ref<MaterialTimePickerDial> = named(&first, "PART_Dial");
    let second_dial: Ref<MaterialTimePickerDial> = named(&second, "PART_Dial");
    first_dial.numbers()[8].set_is_checked(Some(true));
    assert_eq!(8, first.hour());
    assert_eq!(15, second.hour());
    assert_eq!(Some(true), second_dial.numbers()[3].is_checked());
}
#[test]
fn clock_keyboard_arrows_only_move_focus_and_activation_does_not_auto_advance() {
    let (_app, clock) = animated();
    let picker = MaterialTimePicker::new();
    picker.set_is_24_hour(true);
    picker.set_time(11, 15);
    let _window = host(&picker);
    let dial: Ref<MaterialTimePickerDial> = named(&picker, "PART_Dial");
    let numbers = dial.numbers();
    let finished = Rc::new(Cell::new(0));
    let count = finished.clone();
    picker.add_handler(
        MaterialTimePicker::time_change_finished_event(),
        move |_, _| {
            count.set(count.get() + 1);
        },
    );
    numbers[11].focus();
    assert!(key(&numbers[11], Key::Right, true));
    assert!(numbers[12].is_focused());
    assert_eq!(11, picker.hour());
    key(&numbers[12], Key::Space, true);
    key(&numbers[12], Key::Space, false);
    assert_eq!(12, picker.hour());
    assert_eq!(MaterialTimeSelection::Hour, picker.selection());
    assert_eq!(0.0, dial.hand_velocity());
    assert!((dial.hand_angle().rem_euclid(TAU) - (-FRAC_PI_2).rem_euclid(TAU)).abs() < 1e-9);
    key(&numbers[12], Key::Space, true);
    key(&numbers[12], Key::Space, false);
    assert_eq!(
        2,
        finished.get(),
        "activating the selected number still completes an edit"
    );
    tick(&clock, 0.0);
    tick(&clock, 1.0);
    assert_eq!(MaterialTimeSelection::Hour, picker.selection());
    assert!(key(&numbers[12], Key::Tab, true));
    let minute: Ref<RadioButton> = named(&picker, "PART_Minute");
    assert!(minute.is_focused());
}

#[test]
fn clock_semantic_invoke_snaps_and_advances_even_when_the_number_was_selected() {
    let (_app, clock) = animated();
    let picker = MaterialTimePicker::new();
    picker.set_hour(3);
    let _window = host(&picker);
    let dial: Ref<MaterialTimePickerDial> = named(&picker, "PART_Dial");
    let numbers = dial.numbers();
    let peer = ControlAutomationPeer::create_peer_for_element(&numbers[3]);
    let invoke = peer
        .get_provider::<dyn IInvokeProvider>()
        .expect("native semantic click");
    let finished = Rc::new(Cell::new(0));
    let count = finished.clone();
    picker.add_handler(
        MaterialTimePicker::time_change_finished_event(),
        move |_, _| {
            count.set(count.get() + 1);
        },
    );
    assert_eq!(Some(true), numbers[3].is_checked());
    invoke.invoke().unwrap();
    assert_eq!(1, finished.get());
    assert_eq!(0.0, dial.hand_velocity());
    tick(&clock, 0.0);
    tick(&clock, 0.09);
    assert_eq!(MaterialTimeSelection::Hour, picker.selection());
    tick(&clock, 0.11);
    assert_eq!(MaterialTimeSelection::Minute, picker.selection());

    picker.set_auto_advance(false);
    picker.set_selection(MaterialTimeSelection::Hour);
    let peer = ControlAutomationPeer::create_peer_for_element(&numbers[9]);
    let invoke = peer.get_provider::<dyn IInvokeProvider>().unwrap();
    let select = peer.get_provider::<dyn ISelectionItemProvider>().unwrap();
    invoke.invoke().unwrap();
    assert_eq!(9, picker.hour());
    assert_eq!(0.0, dial.hand_velocity());
    assert!((dial.hand_angle().rem_euclid(TAU) - PI).abs() < 1e-9);
    tick(&clock, 1.0);
    assert_eq!(MaterialTimeSelection::Hour, picker.selection());
    assert_eq!(2, finished.get());
    picker.set_is_read_only(true);
    assert!(invoke.invoke().is_err());
    assert!(select.select().is_err());
    picker.set_is_read_only(false);
    picker.set_is_enabled(false);
    assert!(invoke.invoke().is_err());
    assert!(select.select().is_err());
    assert_eq!(2, finished.get());
}

#[test]
fn invalid_active_input_uses_error_text_and_inactive_selector_uses_on_error_container() {
    use ferroui_base::controls::ResourceKey;
    use ferroui_base::media::Color;
    use ferroui_base::metadata::from_markup_value;
    let _app = start_themed_application();
    for vibrant in [false, true] {
        let picker = MaterialTimePicker::new();
        picker.set_is_vibrant(vibrant);
        picker.set_display_mode(MaterialTimeDisplayMode::Input);
        let window = host(&picker);
        let field: Ref<MaterialTimePickerField> = named(&picker, "PART_HourField");
        field.focus();
        field.select_all();
        type_text(&field, "99");
        assert!(!picker.is_hour_input_valid());
        let color = |key: &str| -> Color {
            from_markup_value(
                &picker
                    .try_find_resource(
                        &ResourceKey::from(key),
                        picker.actual_theme_variant().as_ref(),
                    )
                    .flatten(),
            )
            .unwrap()
        };
        assert_eq!(
            color("MaterialErrorColor"),
            field
                .foreground()
                .unwrap()
                .as_solid_color_brush()
                .unwrap()
                .color()
        );
        picker.set_selection(MaterialTimeSelection::Minute);
        window.update_layout();
        let selector: Ref<RadioButton> = named(&picker, "PART_Hour");
        assert!(selector.is_visible());
        assert_eq!(
            color("MaterialOnErrorContainerColor"),
            selector
                .foreground()
                .unwrap()
                .as_solid_color_brush()
                .unwrap()
                .color()
        );
        window.close();
    }
}
#[test]
fn root_and_dial_automation_expose_the_real_value_and_selectable_clock_items() {
    let _app = start_themed_application();
    let picker = MaterialTimePicker::new();
    let _window = host(&picker);
    let peer = ControlAutomationPeer::create_peer_for_element(&picker);
    let value = peer.get_provider::<dyn IValueProvider>().unwrap();
    assert_eq!(Some("00:00".into()), value.value());
    value.set_value(Some("23:58")).unwrap();
    assert_eq!((23, 58), (picker.hour(), picker.minute()));
    assert!(value.set_value(Some("24:00")).is_err());
    let dial: Ref<MaterialTimePickerDial> = named(&picker, "PART_Dial");
    let numbers = dial.numbers();
    let peer = ControlAutomationPeer::create_peer_for_element(&numbers[3]);
    assert_eq!(
        AutomationControlType::RadioButton,
        peer.get_automation_control_type()
    );
    let item = peer.get_provider::<dyn ISelectionItemProvider>().unwrap();
    item.select().unwrap();
    assert_eq!(15, picker.hour());
    assert_eq!(MaterialTimeSelection::Hour, picker.selection());
    picker.set_is_read_only(true);
    assert!(value.is_read_only());
    assert!(value.set_value(Some("01:00")).is_err());
}
#[test]
fn pointer_tap_and_drag_use_different_minute_precision_and_release_capture() {
    let _app = start_themed_application();
    let picker = MaterialTimePicker::new();
    picker.set_selection(MaterialTimeSelection::Minute);
    let window = host(&picker);
    let dial: Ref<MaterialTimePickerDial> = named(&picker, "PART_Dial");
    let pointer = Pointer::new(Pointer::get_next_free_id(), PointerType::Mouse, true);
    let p = dial_point(&dial, &window, TAU * 7.0 / 60.0 - FRAC_PI_2, false);
    press(&dial, &window, &pointer, p);
    assert_eq!(0, picker.minute());
    release(&dial, &window, &pointer, p);
    assert_eq!(5, picker.minute());
    assert!(pointer.captured().is_none());
    let start = dial_point(&dial, &window, -FRAC_PI_2, false);
    press(&dial, &window, &pointer, start);
    moved(&dial, &window, &pointer, p);
    assert_eq!(7, picker.minute());
    release(&dial, &window, &pointer, p);
    assert_eq!(7, picker.minute());
    assert!(pointer.captured().is_none());
}
#[test]
fn dragging_hour_advances_only_after_release_and_can_be_disabled_for_touch_exploration() {
    let _app = start_themed_application();
    let picker = MaterialTimePicker::new();
    picker.set_is_24_hour(true);
    let window = host(&picker);
    let dial: Ref<MaterialTimePickerDial> = named(&picker, "PART_Dial");
    let pointer = Pointer::new(Pointer::get_next_free_id(), PointerType::Touch, true);
    let start = dial_point(&dial, &window, -FRAC_PI_2, false);
    let end = dial_point(&dial, &window, 0.0, true);
    press(&dial, &window, &pointer, start);
    moved(&dial, &window, &pointer, end);
    assert_eq!(15, picker.hour());
    assert_eq!(MaterialTimeSelection::Hour, picker.selection());
    release(&dial, &window, &pointer, end);
    assert_eq!(MaterialTimeSelection::Minute, picker.selection());
    picker.set_selection(MaterialTimeSelection::Hour);
    picker.set_auto_advance(false);
    press(&dial, &window, &pointer, start);
    moved(&dial, &window, &pointer, end);
    release(&dial, &window, &pointer, end);
    assert_eq!(MaterialTimeSelection::Hour, picker.selection());
}

struct Clock {
    subject: LightweightSubject<TimeSpan>,
    state: Cell<PlayState>,
}
impl IObservable<TimeSpan> for Clock {
    fn subscribe(&self, o: Rc<dyn IObserver<TimeSpan>>) -> Rc<dyn IDisposable> {
        self.subject.subscribe(o)
    }
}
impl IClock for Clock {
    fn play_state(&self) -> PlayState {
        self.state.get()
    }
    fn set_play_state(&self, p: PlayState) {
        self.state.set(p)
    }
}
impl IGlobalClock for Clock {}
fn animated() -> (UnitTestApplicationScope, Rc<Clock>) {
    register_types();
    let clock = Rc::new(Clock {
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
fn tick(clock: &Clock, time: f64) {
    clock.subject.on_next(TimeSpan::from_seconds(time));
}

#[test]
fn hand_interruption_preserves_velocity_and_reduced_motion_removes_clock_work() {
    let (_app, clock) = animated();
    let picker = MaterialTimePicker::new();
    let _window = host(&picker);
    let dial: Ref<MaterialTimePickerDial> = named(&picker, "PART_Dial");
    picker.set_hour(3);
    tick(&clock, 0.0);
    tick(&clock, 0.03);
    let velocity = dial.hand_velocity();
    assert!(velocity > 0.0);
    let position = dial.hand_angle();
    picker.set_hour(9);
    assert_eq!(position, dial.hand_angle());
    assert_eq!(velocity, dial.hand_velocity());
    picker.set_reduced_motion(true);
    assert!(dial.hand_velocity().abs() < 1e-12);
    assert!(!clock.subject.has_observers());
    assert!((dial.hand_angle().rem_euclid(TAU) - (PI).rem_euclid(TAU)).abs() < 1e-9);
}
#[test]
fn dial_mode_crossfades_and_hiding_during_animation_cancels_capture_and_observer() {
    let (_app, clock) = animated();
    let picker = MaterialTimePicker::new();
    let window = host(&picker);
    let dial: Ref<MaterialTimePickerDial> = named(&picker, "PART_Dial");
    picker.set_selection(MaterialTimeSelection::Minute);
    assert_eq!(0.0, dial.label_opacity());
    tick(&clock, 0.0);
    tick(&clock, 0.03);
    assert!((0.0..1.0).contains(&dial.label_opacity()));
    let pointer = Pointer::new(Pointer::get_next_free_id(), PointerType::Touch, true);
    let p = dial_point(&dial, &window, -FRAC_PI_2, false);
    press(&dial, &window, &pointer, p);
    assert!(pointer.captured().is_some());
    picker.set_display_mode(MaterialTimeDisplayMode::Input);
    assert!(pointer.captured().is_none());
    // AM/PM shape and editor focus animations may own independent subscriptions.
    assert!(!dial.has_clock_subscription());
}
#[test]
fn detaching_cancels_delayed_hour_advance_and_releases_weak_clock_subscription() {
    let (_app, clock) = animated();
    let picker = MaterialTimePicker::new();
    let window = host(&picker);
    let dial: Ref<MaterialTimePickerDial> = named(&picker, "PART_Dial");
    let pointer = Pointer::new(Pointer::get_next_free_id(), PointerType::Mouse, true);
    let p = dial_point(&dial, &window, 0.0, false);
    press(&dial, &window, &pointer, p);
    release(&dial, &window, &pointer, p);
    assert_eq!(3, picker.hour());
    assert!(dial.has_clock_subscription());
    window.set_content(None);
    window.update_layout();
    assert!(!dial.is_attached_to_visual_tree());
    assert!(!dial.has_clock_subscription());
    tick(&clock, 2.0);
    assert_eq!(MaterialTimeSelection::Hour, picker.selection());
    assert!(pointer.captured().is_none());
    let weak = dial.downgrade();
    drop(dial);
    drop(picker);
    window.close();
    // Complete framework-owned layout/render and TextBox notifications queued before
    // removal, just as the other detached-control leak tests do.
    ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
    assert!(
        weak.upgrade().is_none(),
        "the clock and number nodes must not retain a detached dial"
    );
}

#[test]
fn hour_tap_waits_one_hundred_milliseconds_after_the_hand_finishes() {
    let (_app, clock) = animated();
    let picker = MaterialTimePicker::new();
    let window = host(&picker);
    let dial: Ref<MaterialTimePickerDial> = named(&picker, "PART_Dial");
    let pointer = Pointer::new(Pointer::get_next_free_id(), PointerType::Touch, true);
    let point = dial_point(&dial, &window, 0.0, false);
    press(&dial, &window, &pointer, point);
    release(&dial, &window, &pointer, point);
    tick(&clock, 0.0);
    tick(&clock, 1.0);
    assert_eq!(
        MaterialTimeSelection::Hour,
        picker.selection(),
        "the frame that finishes the hand starts the native 100ms delay"
    );
    tick(&clock, 1.09);
    assert_eq!(MaterialTimeSelection::Hour, picker.selection());
    tick(&clock, 1.11);
    assert_eq!(MaterialTimeSelection::Minute, picker.selection());
}

#[test]
fn immediate_renderer_balances_dial_clips_and_crossfade_opacity() {
    use ferroui_base::media::DrawingContext;
    use ferroui_base::rendering::ImmediateRenderer;
    use ferroui_base::rendering::composition::drawing::RenderDataDrawingContext;

    let (_app, clock) = animated();
    let picker = MaterialTimePicker::new();
    let _window = host(&picker);
    let dial: Ref<MaterialTimePickerDial> = named(&picker, "PART_Dial");
    let render = || {
        let mut recorder = RenderDataDrawingContext::new(None);
        let mut context = DrawingContext::new(&mut recorder);
        let parent_opacity = context.push_opacity(0.8);
        // The real visual-tree renderer wraps each number in its own scopes.
        // A leaked selector clip or crossfade opacity must fail its ordered pop.
        ImmediateRenderer::render(&mut context, &dial.clone().upcast::<Visual>());
        context.pop(parent_opacity);
    };
    render();
    picker.set_selection(MaterialTimeSelection::Minute);
    tick(&clock, 0.0);
    tick(&clock, 0.03);
    assert!((0.0..1.0).contains(&dial.label_opacity()));
    render();
    picker.set_is_24_hour(true);
    picker.set_selection(MaterialTimeSelection::Hour);
    tick(&clock, 0.05);
    render();
    picker.set_reduced_motion(true);
    render();
}
