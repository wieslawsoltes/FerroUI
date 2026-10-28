//! Source-state, actual rendered calendar/input, automation and lifecycle conformance.
use crate::{
    MaterialDateCell, MaterialDateInputFormat, MaterialDateMonthView, MaterialDateMonthsPanel,
    MaterialDatePicker, MaterialDatePickerMode, MaterialDatePickerState, MaterialDateSelectionMode,
    MaterialTheme, register_types,
};
use ferroui_base::animation::{IClock, IGlobalClock, PlayState, TimeSpan};
use ferroui_base::input::{
    InputElement, Key, KeyEventArgs, KeyModifiers, KeyboardDevice, NavigationMethod,
};
use ferroui_base::media::{FlowDirection, Stretch};
use ferroui_base::reactive::{IDisposable, IObservable, IObserver, LightweightSubject};
use ferroui_base::threading::Dispatcher;
use ferroui_base::utilities::{CultureInfo, DateTime, DayOfWeek, TestCultureDataProvider};
use ferroui_base::{ObjectType, Point, Rect, Ref, Size, Vector, Visual};
use ferroui_controls::automation::provider::{
    IInvokeProvider, ISelectionItemProvider, ISelectionProvider,
};
use ferroui_controls::automation::{AutomationProperties, peers::ControlAutomationPeer};
use ferroui_controls::shapes::Path;
use ferroui_controls::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use ferroui_controls::{
    Border, Button, CalendarDateRange, Control, Grid, Panel, PathIcon, ScrollViewer, TextBlock,
    TextBox, Window,
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
    TestCultureDataProvider::register();
    (app, clock)
}
fn date(y: i32, m: i32, d: i32) -> DateTime {
    DateTime::new(y, m, d)
}
fn picker() -> Ref<MaterialDatePicker> {
    let p = MaterialDatePicker::new();
    p.set_culture(Some(CultureInfo::get_culture_info("en-US")));
    p.set_today(date(2026, 7, 15));
    p.set_displayed_month(date(2026, 7, 1));
    p.set_reduced_motion(true);
    p
}
fn window(p: &MaterialDatePicker) -> Ref<Window> {
    let w = Window::new();
    w.set_width(600.0);
    w.set_height(900.0);
    w.set_content(Some(Control::boxed(p.to_ref())));
    w.show();
    w.update_layout();
    Dispatcher::ui_thread().run_jobs(None);
    w.update_layout();
    w
}
fn named<T: ObjectType>(v: &Visual, name: &str) -> Ref<T> {
    v.get_visual_descendants()
        .filter(|c| c.name().as_deref() == Some(name))
        .find_map(|c| c.cast::<T>())
        .unwrap_or_else(|| panic!("missing picker part {name}"))
}
fn invoke(button: &Button) {
    ControlAutomationPeer::create_peer_for_element(button)
        .get_provider::<dyn IInvokeProvider>()
        .unwrap()
        .invoke()
        .unwrap();
}
fn key(cell: &MaterialDateCell, key: Key) {
    let mut e = KeyEventArgs::new();
    e.key = key;
    e.set_routed_event(Some(InputElement::key_down_event()));
    cell.raise_event(&e);
}

#[test]
fn state_matches_native_canonical_range_restart_and_invalid_selection_rules() {
    let mut s = MaterialDatePickerState::default();
    s.selection_mode = MaterialDateSelectionMode::Range;
    s.select_date(date(2026, 7, 15));
    assert_eq!(
        (Some(date(2026, 7, 15)), None),
        (s.selected_start, s.selected_end)
    );
    s.select_date(date(2026, 7, 13));
    assert_eq!(
        (Some(date(2026, 7, 13)), None),
        (s.selected_start, s.selected_end)
    );
    s.select_date(date(2026, 7, 13));
    assert_eq!(s.selected_start, s.selected_end);
    assert!(s.is_complete());
    s.select_date(date(2026, 8, 1));
    assert_eq!(
        (Some(date(2026, 8, 1)), None),
        (s.selected_start, s.selected_end)
    );
    s.set_selection(Some(date(2026, 8, 3)), Some(date(2026, 8, 2)));
    assert_eq!((None, None), (s.selected_start, s.selected_end));
    s.set_selection(None, Some(date(2026, 8, 2)));
    assert_eq!((None, None), (s.selected_start, s.selected_end));
    s.set_selection(Some(date(1800, 8, 2)), None);
    assert_eq!(None, s.selected_start);
    s.set_selection(Some(DateTime::new_with_time(2026, 7, 12, 18, 30, 0)), None);
    assert_eq!(Some(date(2026, 7, 12)), s.selected_start);
    s.set_mode(MaterialDatePickerMode::Input);
    assert_eq!(date(2026, 7, 1), s.displayed_month);
    s.set_displayed_month(date(1800, 1, 1));
    assert_eq!(date(2026, 7, 1), s.displayed_month);
}

#[test]
fn numeric_input_uses_locale_order_two_digit_parts_and_real_gregorian_validation() {
    let (_app, _) = application();
    for (culture, pattern, digits, formatted) in [
        ("en-US", "MM/DD/YYYY", "02292024", "02/29/2024"),
        ("en-GB", "DD/MM/YYYY", "29022024", "29/02/2024"),
    ] {
        let c = CultureInfo::get_culture_info(culture);
        let f = MaterialDateInputFormat::for_culture(&c.date_time_format());
        assert_eq!(pattern, f.pattern());
        assert_eq!(Some(date(2024, 2, 29)), f.parse_digits(digits));
        assert_eq!(formatted, f.format_digits(digits));
        assert_eq!(digits, f.digits(date(2024, 2, 29)));
        assert_eq!(None, f.extract_digits("1x"));
        assert_eq!(None, f.extract_digits("123456789"));
        assert_eq!(None, f.parse_digits("123"));
    }
    let f = MaterialDateInputFormat::for_culture(
        &CultureInfo::get_culture_info("en-US").date_time_format(),
    );
    assert_eq!(None, f.parse_digits("02292023"));
    assert_eq!("02/", f.format_digits("02"));
    assert_eq!("02/29/", f.format_digits("0229"));
}

#[test]
fn single_picker_has_native_header_toolbar_grid_and_day_surface_geometry() {
    let (_app, _) = application();
    let p = picker();
    let _w = window(&p);
    let header: Ref<Grid> = named(&p, "PART_Header");
    let body: Ref<Panel> = named(&p, "PART_Body");
    let view: Ref<MaterialDateMonthView> = named(&p, "PART_Month");
    assert_eq!(120.0, header.bounds().height);
    assert_eq!(392.0, body.bounds().height);
    assert_eq!(336.0, view.bounds().width);
    assert_eq!(288.0, view.bounds().height);
    assert_eq!(31, view.children().count());
    let first: Ref<MaterialDateCell> = named(&view, "Date_2026_07_01");
    let paint: Ref<Border> = named(&first, "DateSurface");
    assert_eq!(
        (144.0, 0.0, 48.0, 48.0),
        (
            first.bounds().x,
            first.bounds().y,
            first.bounds().width,
            first.bounds().height
        )
    );
    assert_eq!((40.0, 40.0), (paint.bounds().width, paint.bounds().height));
    assert_eq!(20.0, paint.corner_radius().top_left);
    let today: Ref<MaterialDateCell> = named(&view, "Date_2026_07_15");
    assert!(today.is_today());
    assert_eq!(1.0, today.border_thickness().left);
    assert!(
        AutomationProperties::get_name(&today)
            .unwrap()
            .contains("Today")
    );
}

#[test]
fn source_icons_preserve_their_twenty_four_dp_viewport_and_small_ink_bounds() {
    let (_app, _) = application();
    let p = picker();
    let w = window(&p);
    // material3/internal/Icons.kt at e4d933c98df7baa7ea2b83eea6e086915961e0ff:
    // paths occupy only part of the 24dp viewport; fitting their tight bounds enlarges them.
    for (name, expected) in [
        ("PART_YearArrow", Rect::new(7.0, 10.0, 10.0, 5.0)),
        ("PART_PreviousIcon", Rect::new(8.0, 6.0, 7.41, 12.0)),
        ("PART_NextIcon", Rect::new(8.59, 6.0, 7.41, 12.0)),
    ] {
        let icon: Ref<PathIcon> = named(&p, name);
        let path: Ref<Path> = named(&icon, "SymbolPath");
        assert_eq!(Size::new(24.0, 24.0), icon.bounds().size());
        assert_eq!(Rect::new(0.0, 0.0, 24.0, 24.0), path.bounds());
        assert_eq!(Stretch::None, path.stretch());
        let ink = path.rendered_geometry().unwrap().bounds();
        for (actual, expected) in [
            (ink.x, expected.x),
            (ink.y, expected.y),
            (ink.width, expected.width),
            (ink.height, expected.height),
        ] {
            assert!((actual - expected).abs() < 1e-9, "{name}: {ink:?}");
        }
        let left = path.translate_point(Point::new(ink.x, ink.y), &w).unwrap();
        let right = path
            .translate_point(Point::new(ink.right(), ink.y), &w)
            .unwrap();
        assert!((right.x - left.x - ink.width).abs() < 1e-9);
    }
    p.set_display_mode(MaterialDatePickerMode::Input);
    w.update_layout();
    let calendar: Ref<PathIcon> = named(&p, "PART_CalendarIcon");
    let calendar_path: Ref<Path> = named(&calendar, "SymbolPath");
    assert_eq!(Size::new(24.0, 24.0), calendar.bounds().size());
    assert_eq!(
        Rect::new(3.0, 2.0, 18.0, 20.0),
        calendar_path.rendered_geometry().unwrap().bounds()
    );
}

#[test]
fn symbol_viewport_scales_with_requested_size_and_only_navigation_arrows_mirror() {
    let (_app, _) = application();
    let p = picker();
    let w = window(&p);
    let arrow: Ref<PathIcon> = named(&p, "PART_YearArrow");
    arrow.set_width(48.0);
    arrow.set_height(48.0);
    w.update_layout();
    let path: Ref<Path> = named(&arrow, "SymbolPath");
    let a = path.translate_point(Point::new(7.0, 10.0), &w).unwrap();
    let b = path.translate_point(Point::new(17.0, 10.0), &w).unwrap();
    assert!((b.x - a.x - 20.0).abs() < 1e-9);
    assert_eq!(Size::new(24.0, 24.0), path.bounds().size());

    p.set_flow_direction(FlowDirection::RightToLeft);
    w.update_layout();
    for (name, expected_direction) in [("PART_PreviousIcon", -1.0), ("PART_EditIcon", 1.0)] {
        let icon: Ref<PathIcon> = named(&p, name);
        let path: Ref<Path> = named(&icon, "SymbolPath");
        let a = path.translate_point(Point::new(0.0, 12.0), &w).unwrap();
        let b = path.translate_point(Point::new(24.0, 12.0), &w).unwrap();
        assert!((b.x - a.x - expected_direction * 24.0).abs() < 1e-9);
    }
}

#[test]
fn clicking_real_day_changes_selection_and_selection_item_automation() {
    let (_app, _) = application();
    let p = picker();
    p.set_show_actions(true);
    let w = window(&p);
    let cell: Ref<MaterialDateCell> = named(&p, "Date_2026_07_18");
    invoke(&cell);
    w.update_layout();
    assert_eq!(Some(date(2026, 7, 18)), p.selected_date());
    assert!(cell.is_selected());
    let peer = ControlAutomationPeer::create_peer_for_element(&cell);
    let item = peer.get_provider::<dyn ISelectionItemProvider>().unwrap();
    assert!(item.is_selected());
    let provider = ControlAutomationPeer::create_peer_for_element(&p)
        .get_provider::<dyn ISelectionProvider>()
        .unwrap();
    assert!(!provider.can_select_multiple());
    assert_eq!(1, provider.get_selection().len());
    let confirm: Ref<Button> = named(&p, "PART_Confirm");
    assert!(confirm.is_enabled());
    let confirmed = Rc::new(Cell::new(0));
    let count = confirmed.clone();
    p.add_handler(MaterialDatePicker::confirmed_event(), move |_, _| {
        count.set(count.get() + 1)
    });
    invoke(&confirm);
    assert_eq!(1, confirmed.get());
    item.remove_from_selection().unwrap();
    assert_eq!(None, p.selected_date());
    assert!(!p.is_selection_complete());
}

#[test]
fn blackout_and_year_policies_disable_cells_and_input_but_not_range_interiors() {
    let (_app, _) = application();
    let p = picker();
    p.set_selection_mode(MaterialDateSelectionMode::Range);
    p.blackout_dates().add(CalendarDateRange::new_range(
        date(2026, 7, 16),
        date(2026, 7, 16),
    ));
    p.set_year_validator(Some(Rc::new(|year| year != 2027)));
    let _w = window(&p);
    assert!(!p.select_date(date(2026, 7, 16)));
    assert!(!p.select_date(date(2027, 7, 15)));
    assert!(p.select_date(date(2026, 7, 15)));
    assert!(p.select_date(date(2026, 7, 17)));
    assert!(p.is_selection_complete());
    assert_eq!(Some(date(2026, 7, 17)), p.selected_end_date());
    let list: Ref<MaterialDateMonthsPanel> = named(&p, "PART_RangeMonths");
    let disabled: Ref<MaterialDateCell> = named(&list, "Date_2026_07_16");
    assert!(!disabled.is_enabled());
    assert!(disabled.is_in_range());
}

#[test]
fn year_pane_has_three_columns_correct_surface_and_preserves_month_on_choice() {
    let (_app, _) = application();
    let p = picker();
    let w = window(&p);
    p.set_is_year_picker_open(true);
    w.update_layout();
    Dispatcher::ui_thread().run_jobs(None);
    w.update_layout();
    let years: Ref<ScrollViewer> = named(&p, "PART_Years");
    assert!(years.is_visible());
    assert_eq!(336.0, years.bounds().height);
    let year: Ref<MaterialDateCell> = named(&p, "Year_2028");
    let paint: Ref<Border> = named(&year, "DateSurface");
    assert_eq!((72.0, 36.0), (paint.bounds().width, paint.bounds().height));
    invoke(&year);
    assert_eq!(date(2028, 7, 1), p.displayed_month());
    assert!(!p.is_year_picker_open());
    assert_eq!(None, p.selected_date());
    let previous: Ref<Button> = named(&p, "PART_Previous");
    assert!(previous.is_visible());
}

#[test]
fn month_arrows_respect_year_limits_and_key_navigation_crosses_month_boundaries() {
    let (_app, _) = application();
    let p = picker();
    p.set_first_year(2026);
    p.set_last_year(2026);
    p.set_displayed_month(date(2026, 1, 1));
    let w = window(&p);
    let previous: Ref<Button> = named(&p, "PART_Previous");
    assert!(!previous.is_enabled());
    p.set_displayed_month(date(2026, 12, 1));
    w.update_layout();
    let next: Ref<Button> = named(&p, "PART_Next");
    assert!(!next.is_enabled());
    p.set_displayed_month(date(2026, 7, 1));
    w.update_layout();
    let last: Ref<MaterialDateCell> = named(&p, "Date_2026_07_31");
    last.focus_with(NavigationMethod::Tab, KeyModifiers::NONE);
    key(&last, Key::Right);
    w.update_layout();
    assert_eq!(date(2026, 8, 1), p.displayed_month());
    let first: Ref<MaterialDateCell> = named(&p, "Date_2026_08_01");
    assert!(first.is_focused());
    assert_eq!(None, p.selected_date());
}

#[test]
fn input_mode_uses_real_editor_and_only_complete_valid_dates_update_selection() {
    let (_app, _) = application();
    let p = picker();
    let w = window(&p);
    p.set_display_mode(MaterialDatePickerMode::Input);
    w.update_layout();
    let editor: Ref<TextBox> = named(&p, "PART_StartInput");
    assert!(editor.is_effectively_visible());
    editor.set_text(Some("02"));
    assert_eq!(Some("02/".into()), editor.text());
    assert_eq!(None, p.selected_date());
    assert_eq!(None, p.input_error(false));
    editor.set_text(Some("02292023"));
    assert!(p.input_error(false).unwrap().contains("MM/DD/YYYY"));
    assert_eq!(None, p.selected_date());
    editor.set_text(Some("02292024"));
    assert_eq!(Some(date(2024, 2, 29)), p.selected_date());
    assert_eq!(Some("02/29/2024".into()), editor.text());
    assert_eq!(None, p.input_error(false));
    editor.set_text(Some("not a date"));
    assert_eq!(Some("02/29/2024".into()), editor.text());
    assert_eq!(Some(date(2024, 2, 29)), p.selected_date());
    editor.set_text(Some("022"));
    assert_eq!(None, p.selected_date());
    assert!(!p.is_selection_complete());
}

#[test]
fn range_input_rejects_reverse_order_and_keeps_editors_available_for_correction() {
    let (_app, _) = application();
    let p = picker();
    p.set_selection_mode(MaterialDateSelectionMode::Range);
    p.set_display_mode(MaterialDatePickerMode::Input);
    let _w = window(&p);
    let start: Ref<TextBox> = named(&p, "PART_StartInput");
    let end: Ref<TextBox> = named(&p, "PART_EndInput");
    start.set_text(Some("07152026"));
    end.set_text(Some("07142026"));
    assert_eq!(Some(date(2026, 7, 15)), p.selected_start_date());
    assert!(p.input_error(true).unwrap().contains("before"));
    assert_eq!(Some("07/15/2026".into()), start.text());
    end.set_text(Some("07152026"));
    assert!(p.is_selection_complete());
    assert_eq!(p.selected_start_date(), p.selected_end_date());
}

#[test]
fn backspace_treats_inserted_date_delimiters_as_display_characters() {
    let (_app, _) = application();
    let p = picker();
    p.set_display_mode(MaterialDatePickerMode::Input);
    let _w = window(&p);
    let editor: Ref<TextBox> = named(&p, "PART_StartInput");
    editor.set_text(Some("02"));
    editor.set_caret_index(3);
    let mut event = KeyEventArgs::new();
    event.key = Key::Back;
    event.set_routed_event(Some(InputElement::key_down_event()));
    editor.raise_event(&event);
    assert_eq!(Some("0".to_owned()), editor.text());
    assert_eq!(None, p.selected_date());
}

#[test]
fn range_calendar_scrolls_real_months_virtualizes_and_draws_half_cell_endpoints() {
    let (_app, _) = application();
    let p = picker();
    p.set_selection_mode(MaterialDateSelectionMode::Range);
    let w = window(&p);
    let header: Ref<Grid> = named(&p, "PART_Header");
    assert_eq!(68.0, header.min_height());
    assert_eq!(81.0, header.bounds().height);
    let scroll: Ref<ScrollViewer> = named(&p, "PART_Range");
    let months: Ref<MaterialDateMonthsPanel> = named(&p, "PART_RangeMonths");
    assert_eq!(2412.0 * 336.0, scroll.extent().height);
    assert!(months.realized_month_count() <= 5);
    assert_eq!(
        (2026 - 1900) as f64 * 12.0 * 336.0 + 6.0 * 336.0,
        scroll.offset().y
    );
    p.set_selection(Some(date(2026, 7, 3)), Some(date(2026, 7, 6)));
    w.update_layout();
    let month = months
        .get_visual_descendants()
        .filter_map(|v| v.cast::<MaterialDateMonthView>())
        .find(|v| v.month() == date(2026, 7, 1))
        .unwrap();
    let rects = month.range_rectangles(336.0);
    assert_eq!(2, rects.len());
    assert_eq!(
        (264.0, 4.0, 72.0, 40.0),
        (rects[0].x, rects[0].y, rects[0].width, rects[0].height)
    );
    assert_eq!(
        (0.0, 52.0, 72.0, 40.0),
        (rects[1].x, rects[1].y, rects[1].width, rects[1].height)
    );
    scroll.set_offset(Vector::new(0.0, scroll.offset().y + 336.0));
    w.update_layout();
    assert_eq!(date(2026, 8, 1), p.displayed_month());
    assert!(months.realized_month_count() <= 5);
}

#[test]
fn complete_range_header_wraps_remaining_width_without_overlapping_mode_button() {
    let (_app, _) = application();
    let p = picker();
    p.set_selection_mode(MaterialDateSelectionMode::Range);
    p.set_selection(Some(date(2026, 10, 22)), Some(date(2026, 10, 29)));
    let w = window(&p);
    let header: Ref<Grid> = named(&p, "PART_Header");
    let headline: Ref<Panel> = named(&p, "PART_RangeHeadline");
    let start: Ref<TextBlock> = named(&p, "PART_RangeHeadlineStart");
    let end: Ref<TextBlock> = named(&p, "PART_RangeHeadlineEnd");
    let toggle: Ref<Button> = named(&p, "PART_ModeToggle");
    assert_eq!(360.0, p.bounds().width);
    assert_eq!(224.0, headline.bounds().width);
    assert_eq!(28.0, start.bounds().height);
    assert!(
        end.bounds().height >= 56.0,
        "the end date wraps in the remaining width"
    );
    assert!(end.bounds().right() <= headline.bounds().width + 1e-9);
    assert_eq!(
        20.0 + end.bounds().height.max(48.0) + 12.0 + 1.0,
        header.bounds().height,
        "the header grows with its content and includes the divider"
    );
    let horizontal_bounds = |control: &Visual| {
        let left = control.translate_point(Point::default(), &w).unwrap().x;
        let right = control
            .translate_point(Point::new(control.bounds().width, 0.0), &w)
            .unwrap()
            .x;
        (left.min(right), left.max(right))
    };
    assert!(horizontal_bounds(&end).1 <= horizontal_bounds(&toggle).0);
    p.set_flow_direction(FlowDirection::RightToLeft);
    w.update_layout();
    assert!(horizontal_bounds(&end).0 >= horizontal_bounds(&toggle).1);
    assert!(end.bounds().right() <= headline.bounds().width + 1e-9);
}

#[test]
fn culture_changes_week_start_input_order_and_rtl_screen_geometry() {
    let (_app, _) = application();
    let p = picker();
    p.set_culture(Some(CultureInfo::get_culture_info("en-GB")));
    let w = window(&p);
    assert_eq!(DayOfWeek::Monday, p.first_day_of_week());
    assert_eq!("DD/MM/YYYY", p.input_format().pattern());
    let view: Ref<MaterialDateMonthView> = named(&p, "PART_Month");
    let first: Ref<MaterialDateCell> = named(&view, "Date_2026_07_01");
    let second: Ref<MaterialDateCell> = named(&view, "Date_2026_07_02");
    assert_eq!(96.0, first.bounds().x);
    assert!(
        first.translate_point(Point::default(), &w).unwrap().x
            < second.translate_point(Point::default(), &w).unwrap().x
    );
    p.set_flow_direction(FlowDirection::RightToLeft);
    w.update_layout();
    assert!(
        first.translate_point(Point::default(), &w).unwrap().x
            > second.translate_point(Point::default(), &w).unwrap().x
    );
    first.focus_with(NavigationMethod::Tab, KeyModifiers::NONE);
    key(&first, Key::Left);
    assert!(second.is_focused());
}

#[test]
fn mode_motion_uses_source_spatial_and_effect_roles_and_reduced_motion_finishes() {
    let (_app, clock) = application();
    let p = picker();
    p.set_reduced_motion(false);
    let w = window(&p);
    p.set_display_mode(MaterialDatePickerMode::Input);
    clock.subject.on_next(TimeSpan::ZERO);
    clock.subject.on_next(TimeSpan::from_seconds(0.05));
    w.update_layout();
    let body: Ref<Panel> = named(&p, "PART_Body");
    let expected = crate::MaterialMotionScheme::Expressive
        .spring(crate::MaterialMotionRole::DefaultSpatial)
        .sample(0.05, 0.0, 1.0, 0.0)
        .value;
    assert!((body.height() - (392.0 + (90.0 - 392.0) * expected)).abs() < 1e-6);
    assert!(p.has_clock_subscription());
    p.set_reduced_motion(true);
    w.update_layout();
    assert_eq!(90.0, body.height());
    assert!(!p.has_clock_subscription());
}

#[test]
fn range_mode_uses_native_fast_effects_crossfade_without_spatial_slide() {
    let (_app, clock) = application();
    let p = picker();
    p.set_selection_mode(MaterialDateSelectionMode::Range);
    p.set_reduced_motion(false);
    let w = window(&p);
    p.set_display_mode(MaterialDatePickerMode::Input);
    clock.subject.on_next(TimeSpan::ZERO);
    clock.subject.on_next(TimeSpan::from_seconds(0.05));
    w.update_layout();
    let input: Ref<Grid> = named(&p, "PART_Input");
    let body: Ref<Panel> = named(&p, "PART_Body");
    let expected = crate::MaterialMotionScheme::Expressive
        .spring(crate::MaterialMotionRole::FastEffects)
        .sample(0.05, 0.0, 1.0, 0.0)
        .value;
    assert!((input.opacity() - expected).abs() < 1e-7);
    assert_eq!(384.0, body.height());
    p.set_reduced_motion(true);
    assert_eq!(90.0, body.height());
}

#[test]
fn detach_releases_own_clock_and_weak_callbacks_and_picker_is_collectable() {
    let (_app, _) = application();
    let p = picker();
    p.set_reduced_motion(false);
    let w = window(&p);
    let weak = p.downgrade();
    p.set_display_mode(MaterialDatePickerMode::Input);
    assert!(p.has_clock_subscription());
    w.set_content(None);
    w.update_layout();
    assert!(!p.has_clock_subscription());
    drop(p);
    Dispatcher::ui_thread().run_jobs(None);
    w.update_layout();
    assert!(weak.upgrade().is_none());
}
