//! Port of the reference `CalendarDatePickerTests`.

use super::{CalendarDatePicker, CalendarDatePickerFormat};
use crate::calendar::{Calendar, CalendarDateRange};
use crate::primitives::Popup;
use crate::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions, IControlTemplate};
use crate::test_support::TestRoot;
use crate::test_support_buttons::{focus_scope, FocusScope};
use crate::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use crate::text_box_tests::{raise_key_event, raise_text_event};
use crate::{Button, Panel, TextBox};
use ferroui_base::data::converters::IValueConverter;
use ferroui_base::data::core::ValueType;
use ferroui_base::data::BindingError;
use ferroui_base::input::{Key, KeyModifiers, NavigationMethod};
use ferroui_base::media::text_formatting::testing::TextTestScope;
use ferroui_base::media::{Brushes, IBrush};
use ferroui_base::threading::Dispatcher;
use ferroui_base::utilities::{CultureInfo, DateTime, DateTimeStyles, TestCultureDataProvider};
use ferroui_base::{BoxedValue, FerroProperty, Ref};
use std::cell::Cell;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Rc;

fn compare_dates(first: DateTime, second: DateTime) -> bool {
    first.year() == second.year() && first.month() == second.month() && first.day() == second.day()
}

/// A running unit test application, with the text services of the tests
/// registered under the services of the application.
struct AppScope {
    // Dropped in this order: the application first.
    _app: UnitTestApplicationScope,
    _text: TextTestScope,
}

/// Starts an application with the `Services` of the reference tests.
fn start() -> AppScope {
    let text = TextTestScope::new();
    let app = UnitTestApplication::start(TestServices::mock_threading_interface());
    AppScope { _app: app, _text: text }
}

/// Starts an application with the `FocusServices` of the reference tests.
fn start_focus_services() -> (AppScope, FocusScope) {
    let text = TextTestScope::new();
    let app = UnitTestApplication::start(TestServices::real_focus());
    (AppScope { _app: app, _text: text }, focus_scope())
}

/// Restores the current cultures of the thread when dropped.
struct CultureScope {
    culture: CultureInfo,
    ui_culture: CultureInfo,
}

impl Drop for CultureScope {
    fn drop(&mut self) {
        CultureInfo::set_current_culture(self.culture.clone());
        CultureInfo::set_current_ui_culture(self.ui_culture.clone());
    }
}

/// Makes `en-US` the current culture and the current UI culture. The data
/// of the culture is registered with the services of the running
/// application.
fn use_en_us() -> CultureScope {
    let scope =
        CultureScope { culture: CultureInfo::current_culture(), ui_culture: CultureInfo::current_ui_culture() };
    TestCultureDataProvider::register();
    let culture = CultureInfo::get_culture_info("en-US");
    CultureInfo::set_current_culture(culture.clone());
    CultureInfo::set_current_ui_culture(culture);
    scope
}

/// Runs an action that the reference expects to throw and returns the
/// message of the panic that stands for the exception.
fn panic_message(action: impl FnOnce()) -> String {
    let payload = catch_unwind(AssertUnwindSafe(action)).expect_err("the action panics");
    payload
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| payload.downcast_ref::<&str>().map(|message| message.to_string()))
        .unwrap_or_default()
}

/// Asserts that an action fails the way the reference fails with an
/// exception for an argument that is out of range.
fn assert_argument_out_of_range(action: impl FnOnce()) {
    let message = panic_message(action);
    assert!(message.contains("is not valid."), "{message}");
}

fn days_from_today(days: i32) -> DateTime {
    DateTime::today().add_days(days as f64)
}

// The reference skips this test ("FIX ME ASAP"); it passes here.
#[test]
fn selected_date_changed_should_fire_when_selected_date_set() {
    let _app = start();

    let handled = Rc::new(Cell::new(false));
    let date_picker = create_control();
    let _subscription = date_picker.selected_date_changed({
        let handled = handled.clone();
        move |_| handled.set(true)
    });
    let value = DateTime::new(2000, 10, 10);
    date_picker.set_selected_date(Some(value));
    Dispatcher::ui_thread().run_jobs(None);
    assert!(handled.get());
}

#[test]
fn setting_selected_date_to_blackout_date_should_throw() {
    let _app = start();

    let date_picker = create_control();
    assert!(date_picker.blackout_dates().is_some());
    date_picker.blackout_dates().unwrap().add_dates_in_past();

    let good_value = days_from_today(1);
    date_picker.set_selected_date(Some(good_value));
    assert!(compare_dates(date_picker.selected_date().unwrap(), good_value));

    let bad_value = days_from_today(-1);
    assert_argument_out_of_range(|| date_picker.set_selected_date(Some(bad_value)));
}

#[test]
fn adding_blackout_dates_containing_selected_date_should_throw() {
    let _app = start();

    let date_picker = create_control();
    date_picker.set_selected_date(Some(days_from_today(5)));

    assert_argument_out_of_range(|| {
        date_picker
            .blackout_dates()
            .unwrap()
            .add(CalendarDateRange::new_range(DateTime::today(), days_from_today(10)))
    });
}

#[test]
fn setting_date_manually_with_custom_date_format_string_should_be_accepted() {
    let _app = start();
    let _culture = use_en_us();

    let date_picker = create_control();
    date_picker.set_selected_date_format(CalendarDatePickerFormat::Custom);
    date_picker.set_custom_date_format_string("dd.MM.yyyy");

    let tb = get_text_box(&date_picker);

    tb.clear();
    raise_text_event(&tb, "17.10.2024");
    raise_key_event(&tb, Key::Enter, KeyModifiers::NONE);

    assert_eq!(Some("17.10.2024"), date_picker.text().as_deref());
    assert!(compare_dates(date_picker.selected_date().unwrap(), DateTime::new(2024, 10, 17)));

    tb.clear();
    raise_text_event(&tb, "12.10.2024");
    raise_key_event(&tb, Key::Enter, KeyModifiers::NONE);

    assert_eq!(Some("12.10.2024"), date_picker.text().as_deref());
    assert!(compare_dates(date_picker.selected_date().unwrap(), DateTime::new(2024, 10, 12)));
}

struct CalendarDatePickerTextConverter;

impl IValueConverter for CalendarDatePickerTextConverter {
    // date to text
    fn convert(
        &self,
        value: Option<&BoxedValue>,
        _target_type: ValueType,
        _parameter: Option<&BoxedValue>,
    ) -> Result<Option<BoxedValue>, BindingError> {
        if let Some(d) = value.and_then(|value| value.downcast_ref::<DateTime>()) {
            // always return a single format (for this test)
            return Ok(Some(Rc::new(d.to_string_with("yyyy-MM-dd")) as BoxedValue));
        }
        Ok(Some(FerroProperty::unset_value()))
    }

    // text to date
    fn convert_back(
        &self,
        value: Option<&BoxedValue>,
        _target_type: ValueType,
        _parameter: Option<&BoxedValue>,
    ) -> Result<Option<BoxedValue>, BindingError> {
        let Some(str) = value.and_then(|value| value.downcast_ref::<String>()) else {
            return Ok(Some(FerroProperty::unset_value()));
        };
        // allow for a few different date formats
        let formats = ["yyyy-MM-dd", "MM dd yyyy", "dd.MM.yyyy"];
        if let Some(date_value) =
            DateTime::try_parse_exact_multiple(str, &formats, &CultureInfo::invariant_culture(), DateTimeStyles::NONE)
        {
            return Ok(Some(Rc::new(date_value) as BoxedValue));
        }
        Ok(Some(FerroProperty::unset_value()))
    }
}

#[test]
fn setting_date_manually_uses_text_converter() {
    let _app = start();
    let _culture = use_en_us();

    let date_picker = create_control();
    date_picker.set_selected_date_format(CalendarDatePickerFormat::Custom);
    date_picker.set_custom_date_format_string("dd.MM.yyyy");
    date_picker.set_text_converter(Some(Rc::new(CalendarDatePickerTextConverter)));
    let _tb = get_text_box(&date_picker);

    date_picker.set_selected_date(Some(DateTime::new(2024, 2, 13)));
    // The text of the date is set asynchronously so need to let that complete before testing value
    Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(Some("2024-02-13"), date_picker.text().as_deref());
    assert!(compare_dates(date_picker.selected_date().unwrap(), DateTime::new(2024, 2, 13)));

    // null input results in empty string for text
    date_picker.set_selected_date(None);

    assert_eq!(Some(""), date_picker.text().as_deref());
    assert!(date_picker.selected_date().is_none());
}

#[test]
fn setting_date_string_manually_can_accept_multiple_formats() {
    let _app = start();
    let _culture = use_en_us();

    let date_picker = create_control();
    date_picker.set_selected_date_format(CalendarDatePickerFormat::Custom);
    date_picker.set_custom_date_format_string("dd.MM.yyyy");
    date_picker.set_text_converter(Some(Rc::new(CalendarDatePickerTextConverter)));
    let tb = get_text_box(&date_picker);

    // parser can work with same format as CustomDateFormatString (but TextConverter must handle it)
    tb.clear();
    raise_text_event(&tb, "17.10.2024");
    raise_key_event(&tb, Key::Enter, KeyModifiers::NONE);
    assert_eq!(Some("2024-10-17"), date_picker.text().as_deref());
    assert!(compare_dates(date_picker.selected_date().unwrap(), DateTime::new(2024, 10, 17)));

    // can also handle parsing other formats that the user enters, too
    tb.clear();
    raise_text_event(&tb, "2024-02-13");
    raise_key_event(&tb, Key::Enter, KeyModifiers::NONE);

    assert_eq!(Some("2024-02-13"), date_picker.text().as_deref());
    assert!(compare_dates(date_picker.selected_date().unwrap(), DateTime::new(2024, 2, 13)));

    tb.clear();
    raise_text_event(&tb, "04 22 2026");
    raise_key_event(&tb, Key::Enter, KeyModifiers::NONE);

    assert_eq!(Some("2026-04-22"), date_picker.text().as_deref());
    assert!(compare_dates(date_picker.selected_date().unwrap(), DateTime::new(2026, 4, 22)));

    // invalid input results in going back to last known (valid) date
    tb.clear();
    raise_text_event(&tb, "Not A Valid Date");
    raise_key_event(&tb, Key::Enter, KeyModifiers::NONE);

    assert_eq!(Some("2026-04-22"), date_picker.text().as_deref());
    assert!(compare_dates(date_picker.selected_date().unwrap(), DateTime::new(2026, 4, 22)));
}

#[test]
fn tab_focus_should_move_focus_to_text_box() {
    let _services = start_focus_services();

    let date_picker = CalendarDatePicker::new();
    date_picker.set_template(Some(create_template()));
    let root = TestRoot::with_child(date_picker.clone());
    root.execute_initial_layout_pass();

    date_picker.focus_with(NavigationMethod::Tab, KeyModifiers::NONE);

    let focused = root.focus_manager().get_focused_element();
    assert!(focused.is_some_and(|focused| focused.ptr_eq(&get_text_box(&date_picker))));
}

#[test]
fn programmatic_focus_should_move_focus_to_text_box() {
    let _services = start_focus_services();

    let date_picker = CalendarDatePicker::new();
    date_picker.set_template(Some(create_template()));
    let root = TestRoot::with_child(date_picker.clone());
    root.execute_initial_layout_pass();

    date_picker.focus();

    let focused = root.focus_manager().get_focused_element();
    assert!(focused.is_some_and(|focused| focused.ptr_eq(&get_text_box(&date_picker))));
}

fn create_control() -> Ref<CalendarDatePicker> {
    let date_picker = CalendarDatePicker::new();
    date_picker.set_template(Some(create_template()));

    date_picker.apply_template();
    date_picker
}

fn create_template() -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::for_type::<CalendarDatePicker>(|control, scope| {
        let text_box = TextBox::new();
        text_box.set_name(Some("PART_TextBox".to_string()));
        let text_box = text_box.register_in_name_scope(&**scope);

        let button = Button::new();
        button.set_name(Some("PART_Button".to_string()));
        let button = button.register_in_name_scope(&**scope);

        let calendar = Calendar::new();
        calendar.set_name(Some("PART_Calendar".to_string()));
        let bindings = [
            (Calendar::selected_date_property().as_property(), CalendarDatePicker::selected_date_property().as_property()),
            (Calendar::display_date_property().as_property(), CalendarDatePicker::display_date_property().as_property()),
            (
                Calendar::display_date_start_property().as_property(),
                CalendarDatePicker::display_date_start_property().as_property(),
            ),
            (
                Calendar::display_date_end_property().as_property(),
                CalendarDatePicker::display_date_end_property().as_property(),
            ),
        ];
        for (target, source) in bindings {
            calendar.bind_indexer(&target.bind(), &control.indexer(&source.bind()));
        }
        let calendar = calendar.register_in_name_scope(&**scope);

        let popup = Popup::new();
        popup.set_name(Some("PART_Popup".to_string()));
        let popup = popup.register_in_name_scope(&**scope);

        let panel = Panel::new();
        panel.children().add(text_box);
        panel.children().add(button);
        panel.children().add(popup);
        panel.children().add(calendar);

        panel.upcast()
    })
}

fn get_text_box(control: &Ref<CalendarDatePicker>) -> Ref<TextBox> {
    control
        .get_template_descendants()
        .into_iter()
        .find_map(|descendant| descendant.cast::<TextBox>())
        .expect("the template has a text box")
}

#[test]
fn placeholder_foreground_can_be_set() {
    let _app = start();

    let control = create_control();
    control.set_placeholder_text(Some("Select date"));
    let purple: Rc<dyn IBrush> = Brushes::purple();
    control.set_placeholder_foreground(Some(purple.clone()));

    assert!(control.placeholder_foreground() == Some(purple));
}

// ============================================================================
//  ADDITIONAL TESTS - NOT PORTS OF REFERENCE TESTS.
//
//  The reference tests reach neither the popup nor most of the paths that
//  commit a text. The tests of this module do; every expected value is derived
//  from the logic of the reference sources (the results of parsing and
//  formatting were checked against .NET 10 with the en-US culture).
// ============================================================================
mod additional {
    use super::*;
    use crate::calendar::CalendarMode;
    use crate::{Control, Window};
    use ferroui_base::input::{
        InputElement, IPointer, KeyEventArgs, MouseButton, Pointer, PointerPointProperties, PointerReleasedEventArgs,
        PointerType, PointerUpdateKind, PointerWheelEventArgs, RawInputModifiers,
    };
    use ferroui_base::interactivity::RoutedEventArgs;
    use ferroui_base::layout::ILayoutManager;
    use ferroui_base::{Point, Vector};
    use std::cell::RefCell;

    fn date(year: i32, month: i32, day: i32) -> DateTime {
        DateTime::new(year, month, day)
    }

    /// Starts an application with the services of the tests that show
    /// windows.
    fn start_window_app() -> AppScope {
        let text = TextTestScope::new();
        let app = UnitTestApplication::start(TestServices::styled_window());
        AppScope { _app: app, _text: text }
    }

    /// A template with the structure of the template of the reference
    /// theme: the calendar is the child of the popup.
    fn create_popup_template() -> Rc<dyn IControlTemplate> {
        FuncControlTemplate::for_type::<CalendarDatePicker>(|control, scope| {
            let text_box = TextBox::new();
            text_box.set_name(Some("PART_TextBox".to_string()));
            let text_box = text_box.register_in_name_scope(&**scope);

            let button = Button::new();
            button.set_name(Some("PART_Button".to_string()));
            let button = button.register_in_name_scope(&**scope);

            let calendar = Calendar::new();
            calendar.set_name(Some("PART_Calendar".to_string()));
            let bindings = [
                (
                    Calendar::selected_date_property().as_property(),
                    CalendarDatePicker::selected_date_property().as_property(),
                ),
                (
                    Calendar::display_date_property().as_property(),
                    CalendarDatePicker::display_date_property().as_property(),
                ),
                (
                    Calendar::display_date_start_property().as_property(),
                    CalendarDatePicker::display_date_start_property().as_property(),
                ),
                (
                    Calendar::display_date_end_property().as_property(),
                    CalendarDatePicker::display_date_end_property().as_property(),
                ),
            ];
            for (target, source) in bindings {
                calendar.bind_indexer(&target.bind(), &control.indexer(&source.bind()));
            }
            let calendar = calendar.register_in_name_scope(&**scope);

            let popup = Popup::new();
            popup.set_name(Some("PART_Popup".to_string()));
            popup.set_placement_target(control.clone());
            popup.set_child(calendar);
            let popup = popup.register_in_name_scope(&**scope);

            let panel = Panel::new();
            panel.children().add(text_box);
            panel.children().add(button);
            panel.children().add(popup);

            panel.upcast()
        })
    }

    /// A date picker with the popup template, shown in a window.
    fn show_picker() -> (Ref<CalendarDatePicker>, Ref<Window>) {
        let date_picker = CalendarDatePicker::new();
        date_picker.set_template(Some(create_popup_template()));
        let window = Window::new();
        window.set_content(Some(Control::boxed(date_picker.clone())));
        window.show();
        window.layout_manager().execute_initial_layout_pass();
        (date_picker, window)
    }

    fn get_popup(control: &Ref<CalendarDatePicker>) -> Ref<Popup> {
        control
            .get_template_descendants()
            .into_iter()
            .find_map(|descendant| descendant.cast::<Popup>())
            .expect("the template has a popup")
    }

    fn get_button(control: &Ref<CalendarDatePicker>) -> Ref<Button> {
        control
            .get_template_descendants()
            .into_iter()
            .find_map(|descendant| descendant.cast::<Button>())
            .expect("the template has a button")
    }

    fn get_calendar(control: &Ref<CalendarDatePicker>) -> Ref<Calendar> {
        get_popup(control).child().and_then(|child| child.cast::<Calendar>()).expect("the popup has a calendar")
    }

    fn raise_key(target: &Control, key: Key, modifiers: KeyModifiers, up: bool) -> bool {
        let mut args = KeyEventArgs::new();
        args.set_routed_event(Some(if up { InputElement::key_up_event() } else { InputElement::key_down_event() }));
        args.key = key;
        args.key_modifiers = modifiers;
        target.raise_event(&args);
        args.handled()
    }

    fn click(button: &Button) {
        button.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    }

    /// Types a text into the text box and commits it with the enter key.
    fn commit(date_picker: &Ref<CalendarDatePicker>, text: &str) {
        let tb = get_text_box(date_picker);
        tb.clear();
        raise_text_event(&tb, text);
        raise_key_event(&tb, Key::Enter, KeyModifiers::NONE);
    }

    fn wheel(date_picker: &Ref<CalendarDatePicker>, delta_y: f64) -> bool {
        let pointer: Rc<dyn IPointer> = Pointer::new(Pointer::get_next_free_id(), PointerType::Mouse, true);
        let e = PointerWheelEventArgs::new(
            date_picker.clone(),
            pointer,
            date_picker,
            Point::default(),
            0,
            PointerPointProperties::new(RawInputModifiers::NONE, PointerUpdateKind::Other),
            KeyModifiers::NONE,
            Vector::new(0.0, delta_y),
        );
        date_picker.raise_event(&e);
        e.handled()
    }

    /// Records the texts and the errors of the `DateValidationError` event.
    fn errors_of(date_picker: &CalendarDatePicker) -> Rc<RefCell<Vec<(String, String)>>> {
        let errors = Rc::new(RefCell::new(Vec::new()));
        // The date picker keeps the handler for as long as it lives.
        let _ = date_picker.date_validation_error({
            let errors = errors.clone();
            move |e| errors.borrow_mut().push((e.text().to_string(), e.exception().to_string()))
        });
        errors
    }

    fn counter(subscribe: impl FnOnce(Box<dyn Fn()>) -> Rc<dyn ferroui_base::reactive::IDisposable>) -> Rc<Cell<i32>> {
        let count = Rc::new(Cell::new(0));
        let _ = subscribe(Box::new({
            let count = count.clone();
            move || count.set(count.get() + 1)
        }));
        count
    }

    // --- popup -----------------------------------------------------------------

    #[test]
    fn additional_is_drop_down_open_opens_and_closes_the_popup() {
        let _app = start_window_app();
        let _culture = use_en_us();
        let (date_picker, window) = show_picker();
        let popup = get_popup(&date_picker);
        let calendar = get_calendar(&date_picker);
        let opened = counter(|handler| date_picker.calendar_opened(handler));
        let closed = counter(|handler| date_picker.calendar_closed(handler));

        // The calendar of the template selects single dates and shares its
        // blackout dates with the date picker.
        assert_eq!(crate::calendar::CalendarSelectionMode::SingleDate, calendar.selection_mode());
        assert!(date_picker.blackout_dates() == Some(calendar.blackout_dates()));
        assert!(!popup.is_open());
        assert!(!date_picker.classes().contains(":flyout-open"));

        // The calendar shows months whenever the popup opens or closes.
        calendar.set_display_mode(CalendarMode::Year);
        date_picker.set_is_drop_down_open(true);
        assert!(popup.is_open());
        assert_eq!(1, opened.get());
        assert_eq!(0, closed.get());
        assert!(date_picker.classes().contains(":flyout-open"));
        assert_eq!(CalendarMode::Month, calendar.display_mode());

        date_picker.set_is_drop_down_open(false);
        assert!(!popup.is_open());
        assert_eq!(1, opened.get());
        assert_eq!(1, closed.get());
        assert!(!date_picker.classes().contains(":flyout-open"));

        // Closing the popup itself closes the drop-down.
        date_picker.set_is_drop_down_open(true);
        assert_eq!(2, opened.get());
        popup.set_is_open(false);
        assert!(!date_picker.is_drop_down_open());
        assert_eq!(2, closed.get());
        assert!(!date_picker.classes().contains(":flyout-open"));

        Dispatcher::ui_thread().run_jobs(None);
        window.close();
    }

    #[test]
    fn additional_button_and_keys_toggle_the_popup() {
        let _app = start_window_app();
        let _culture = use_en_us();
        let (date_picker, window) = show_picker();
        let popup = get_popup(&date_picker);
        let calendar = get_calendar(&date_picker);
        let button = get_button(&date_picker);
        let tb = get_text_box(&date_picker);

        click(&button);
        assert!(date_picker.is_drop_down_open());
        assert!(popup.is_open());
        click(&button);
        assert!(!date_picker.is_drop_down_open());
        assert!(!popup.is_open());

        // Ctrl+Down in the text box opens the popup; Down alone does not.
        raise_key(&tb, Key::Down, KeyModifiers::NONE, false);
        assert!(!popup.is_open());
        assert!(raise_key(&tb, Key::Down, KeyModifiers::CONTROL, false));
        assert!(popup.is_open());

        // Enter on the calendar in month mode closes it.
        raise_key(&calendar, Key::Enter, KeyModifiers::NONE, false);
        assert!(!date_picker.is_drop_down_open());
        assert!(!popup.is_open());

        // Releasing Alt+Down opens the popup, and only opens it.
        assert!(raise_key(&date_picker, Key::Down, KeyModifiers::ALT, true));
        assert!(popup.is_open());
        assert!(!raise_key(&date_picker, Key::Down, KeyModifiers::ALT, true));
        assert!(popup.is_open());
        // Space and enter do not toggle the popup.
        assert!(!raise_key(&date_picker, Key::Space, KeyModifiers::NONE, true));
        assert!(!raise_key(&date_picker, Key::Enter, KeyModifiers::NONE, true));
        assert!(popup.is_open());

        // Space on the calendar closes it as well.
        raise_key(&calendar, Key::Space, KeyModifiers::NONE, false);
        assert!(!popup.is_open());

        // Releasing the mouse over a day of the calendar closes the popup.
        date_picker.set_is_drop_down_open(true);
        let pointer = Pointer::new(Pointer::get_next_free_id(), PointerType::Mouse, true);
        let released = PointerReleasedEventArgs::new(
            calendar.clone(),
            pointer,
            &calendar,
            Point::default(),
            0,
            PointerPointProperties::new(RawInputModifiers::NONE, PointerUpdateKind::LeftButtonReleased),
            KeyModifiers::NONE,
            MouseButton::Left,
        );
        calendar.on_day_button_mouse_up(&released);
        assert!(!date_picker.is_drop_down_open());
        assert!(!popup.is_open());

        Dispatcher::ui_thread().run_jobs(None);
        window.close();
    }

    #[test]
    fn additional_escape_closes_the_popup_and_restores_the_selected_date() {
        let _app = start_window_app();
        let _culture = use_en_us();
        let (date_picker, window) = show_picker();
        let popup = get_popup(&date_picker);
        let calendar = get_calendar(&date_picker);

        date_picker.set_selected_date(Some(date(2024, 2, 5)));
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(Some(date(2024, 2, 5)), calendar.selected_date());

        date_picker.set_is_drop_down_open(true);
        // A date picked in the calendar is the date of the date picker.
        calendar.selected_dates().set(0, date(2024, 2, 9));
        assert_eq!(Some(date(2024, 2, 9)), date_picker.selected_date());

        raise_key(&calendar, Key::Escape, KeyModifiers::NONE, false);
        assert!(!popup.is_open());
        assert_eq!(Some(date(2024, 2, 5)), date_picker.selected_date());
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(Some("2/5/2024"), date_picker.text().as_deref());

        // Enter keeps the date that was picked.
        date_picker.set_is_drop_down_open(true);
        calendar.selected_dates().set(0, date(2024, 2, 9));
        raise_key(&calendar, Key::Enter, KeyModifiers::NONE, false);
        assert!(!popup.is_open());
        assert_eq!(Some(date(2024, 2, 9)), date_picker.selected_date());
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(Some("2/9/2024"), date_picker.text().as_deref());

        // In year mode the keys belong to the calendar: escape does nothing
        // and enter shows the months; the popup stays open.
        date_picker.set_is_drop_down_open(true);
        calendar.set_display_mode(CalendarMode::Year);
        raise_key(&calendar, Key::Escape, KeyModifiers::NONE, false);
        assert!(popup.is_open());
        assert!(raise_key(&calendar, Key::Enter, KeyModifiers::NONE, false));
        assert_eq!(CalendarMode::Month, calendar.display_mode());
        assert!(popup.is_open());
        assert_eq!(Some(date(2024, 2, 9)), date_picker.selected_date());

        date_picker.set_is_drop_down_open(false);
        Dispatcher::ui_thread().run_jobs(None);
        window.close();
    }

    // --- committing a text -------------------------------------------------------

    #[test]
    fn additional_committing_a_valid_text_selects_and_reformats_the_date() {
        let _app = start();
        let _culture = use_en_us();
        let date_picker = create_control();
        let errors = errors_of(&date_picker);
        let tb = get_text_box(&date_picker);

        // Without a date the text box shows the short date pattern.
        assert_eq!(Some("<M/d/yyyy>"), tb.placeholder_text().as_deref());
        assert_eq!(Some(""), date_picker.text().as_deref());

        commit(&date_picker, "2/5/2024");
        assert_eq!(Some(date(2024, 2, 5)), date_picker.selected_date());
        assert_eq!(Some("2/5/2024"), date_picker.text().as_deref());
        assert_eq!(Some("2/5/2024"), tb.text().as_deref());
        // The date picker displays the month of the date.
        assert_eq!(date(2024, 2, 5), date_picker.display_date());

        // Any text the culture can parse is accepted and reformatted.
        commit(&date_picker, "Feb 7, 2024");
        assert_eq!(Some(date(2024, 2, 7)), date_picker.selected_date());
        assert_eq!(Some("2/7/2024"), date_picker.text().as_deref());
        assert_eq!(Some("2/7/2024"), tb.text().as_deref());
        commit(&date_picker, "2024-03-09");
        assert_eq!(Some(date(2024, 3, 9)), date_picker.selected_date());
        assert_eq!(Some("3/9/2024"), date_picker.text().as_deref());
        assert_eq!(date(2024, 3, 9), date_picker.display_date());

        // A two digit year is not parsed again when the text is unchanged.
        Dispatcher::ui_thread().run_jobs(None);
        raise_key_event(&tb, Key::Enter, KeyModifiers::NONE);
        assert_eq!(Some(date(2024, 3, 9)), date_picker.selected_date());

        // An empty text clears the date.
        tb.clear();
        raise_key_event(&tb, Key::Enter, KeyModifiers::NONE);
        assert_eq!(None, date_picker.selected_date());
        assert_eq!(Some(""), date_picker.text().as_deref());

        assert!(errors.borrow().is_empty());

        // Setting the text of the date picker selects the date as well.
        date_picker.set_text(Some("12/19/08"));
        assert_eq!(Some(date(2008, 12, 19)), date_picker.selected_date());
        assert_eq!(Some("12/19/2008"), date_picker.text().as_deref());
        Dispatcher::ui_thread().run_jobs(None);
        date_picker.set_text(None);
        assert_eq!(None, date_picker.selected_date());
        assert_eq!(Some(""), date_picker.text().as_deref());
    }

    #[test]
    fn additional_committing_an_invalid_text_raises_date_validation_error() {
        let _app = start();
        let _culture = use_en_us();
        let date_picker = create_control();
        let errors = errors_of(&date_picker);
        let tb = get_text_box(&date_picker);

        // Without a date: no date, and the text of the date picker is empty.
        commit(&date_picker, "not a date");
        assert_eq!(1, errors.borrow().len());
        assert_eq!("not a date", errors.borrow()[0].0);
        assert_eq!(None, date_picker.selected_date());
        assert_eq!(Some(""), date_picker.text().as_deref());

        // With a date: the text goes back to the text of the date. The day
        // comes second in en-US, so that there is no month 13.
        commit(&date_picker, "2/5/2024");
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(1, errors.borrow().len());
        commit(&date_picker, "13/2/2024");
        assert_eq!(2, errors.borrow().len());
        assert_eq!("13/2/2024", errors.borrow()[1].0);
        assert_eq!(Some(date(2024, 2, 5)), date_picker.selected_date());
        assert_eq!(Some("2/5/2024"), date_picker.text().as_deref());
        assert_eq!(Some("2/5/2024"), tb.text().as_deref());

        // A day the month does not have.
        commit(&date_picker, "2/30/2024");
        assert_eq!(3, errors.borrow().len());
        assert_eq!(Some(date(2024, 2, 5)), date_picker.selected_date());
        assert_eq!(Some("2/5/2024"), date_picker.text().as_deref());
    }

    #[test]
    fn additional_committing_a_blackout_date_raises_date_validation_error() {
        let _app = start();
        let _culture = use_en_us();
        let date_picker = create_control();
        let errors = errors_of(&date_picker);
        date_picker.blackout_dates().unwrap().add(CalendarDateRange::new(date(2024, 2, 6)));

        commit(&date_picker, "2/6/2024");
        assert_eq!(
            vec![("2/6/2024".to_string(), "SelectedDate value is not valid. (Parameter 'text')".to_string())],
            *errors.borrow()
        );
        assert_eq!(None, date_picker.selected_date());
        assert_eq!(Some(""), date_picker.text().as_deref());

        commit(&date_picker, "2/7/2024");
        assert_eq!(Some(date(2024, 2, 7)), date_picker.selected_date());
        assert_eq!(1, errors.borrow().len());
    }

    #[test]
    fn additional_date_validation_error_can_ask_for_the_exception() {
        let _app = start();
        let _culture = use_en_us();

        // A date that cannot be selected.
        let date_picker = create_control();
        date_picker.blackout_dates().unwrap().add(CalendarDateRange::new(date(2024, 2, 6)));
        let _subscription = date_picker.date_validation_error(|e| e.set_throw_exception(true));
        let message = panic_message(|| commit(&date_picker, "2/6/2024"));
        assert_eq!("SelectedDate value is not valid. (Parameter 'text')", message);

        // A text that is not a date: the error of the parser.
        let date_picker = create_control();
        let error = Rc::new(RefCell::new(String::new()));
        let _subscription = date_picker.date_validation_error({
            let error = error.clone();
            move |e| {
                *error.borrow_mut() = e.exception().to_string();
                e.set_throw_exception(true);
            }
        });
        let message = panic_message(|| commit(&date_picker, "not a date"));
        assert!(!message.is_empty());
        assert_eq!(*error.borrow(), message);
    }

    #[test]
    fn additional_long_and_custom_formats_are_used_for_the_text() {
        let _app = start();
        let _culture = use_en_us();
        let date_picker = create_control();
        let errors = errors_of(&date_picker);
        let tb = get_text_box(&date_picker);

        // The placeholder follows the format while there is no date.
        date_picker.set_selected_date_format(CalendarDatePickerFormat::Long);
        assert_eq!(Some("<dddd, MMMM d, yyyy>"), tb.placeholder_text().as_deref());

        commit(&date_picker, "February 7, 2024");
        assert_eq!(Some(date(2024, 2, 7)), date_picker.selected_date());
        assert_eq!(Some("Wednesday, February 7, 2024"), date_picker.text().as_deref());
        assert_eq!(Some("Wednesday, February 7, 2024"), tb.text().as_deref());
        Dispatcher::ui_thread().run_jobs(None);

        // Changing the format reformats the text of the selected date.
        date_picker.set_selected_date_format(CalendarDatePickerFormat::Short);
        assert_eq!(Some("2/7/2024"), date_picker.text().as_deref());
        date_picker.set_selected_date_format(CalendarDatePickerFormat::Custom);
        // The default custom format is the short date.
        assert_eq!(Some("2/7/2024"), date_picker.text().as_deref());
        date_picker.set_custom_date_format_string("yyyy-MM-dd");
        assert_eq!(Some("2024-02-07"), date_picker.text().as_deref());
        date_picker.set_custom_date_format_string("dd.MM.yyyy");
        assert_eq!(Some("07.02.2024"), date_picker.text().as_deref());
        assert_eq!(Some("07.02.2024"), tb.text().as_deref());

        // A custom format accepts exactly its own form.
        assert!(errors.borrow().is_empty());
        commit(&date_picker, "17/10/2024");
        assert_eq!(1, errors.borrow().len());
        assert_eq!("17/10/2024", errors.borrow()[0].0);
        assert_eq!(Some(date(2024, 2, 7)), date_picker.selected_date());
        assert_eq!(Some("07.02.2024"), date_picker.text().as_deref());
        commit(&date_picker, "7.1.2024");
        assert_eq!(2, errors.borrow().len());
        commit(&date_picker, "17.10.2024");
        assert_eq!(2, errors.borrow().len());
        assert_eq!(Some(date(2024, 10, 17)), date_picker.selected_date());

        // The custom format string does not matter in another format.
        Dispatcher::ui_thread().run_jobs(None);
        date_picker.set_selected_date_format(CalendarDatePickerFormat::Long);
        assert_eq!(Some("Thursday, October 17, 2024"), date_picker.text().as_deref());
        date_picker.set_custom_date_format_string("yyyy");
        assert_eq!(Some("Thursday, October 17, 2024"), date_picker.text().as_deref());

        // Without a date the placeholder shows the custom format.
        date_picker.clear();
        date_picker.set_selected_date_format(CalendarDatePickerFormat::Custom);
        assert_eq!(Some("<yyyy>"), tb.placeholder_text().as_deref());

        // A placeholder text of the date picker replaces the pattern.
        date_picker.set_placeholder_text(Some("Select date"));
        date_picker.set_selected_date_format(CalendarDatePickerFormat::Short);
        assert!(!tb.is_set(TextBox::placeholder_text_property().as_property()));
    }

    #[test]
    fn additional_custom_date_format_string_must_not_be_blank() {
        let _app = start();
        let date_picker = create_control();
        let result = catch_unwind(AssertUnwindSafe(|| date_picker.set_custom_date_format_string("  ")));
        assert!(result.is_err());
        assert_eq!("d", date_picker.custom_date_format_string());
    }

    // --- focus ---------------------------------------------------------------------

    /// Starts an application with the services of the tests that show
    /// windows and move the keyboard focus.
    fn start_focus_window_app() -> AppScope {
        use ferroui_base::input::{IKeyboardDevice, IKeyboardNavigationHandler, KeyboardDevice, KeyboardNavigationHandler};
        let text = TextTestScope::new();
        let app = UnitTestApplication::start(
            TestServices::styled_window()
                .with_keyboard_device(|| Some(KeyboardDevice::new() as Rc<dyn IKeyboardDevice>))
                .with_keyboard_navigation(|| {
                    Some(KeyboardNavigationHandler::new() as Rc<dyn IKeyboardNavigationHandler>)
                }),
        );
        AppScope { _app: app, _text: text }
    }

    #[test]
    fn additional_focus_moves_to_a_focusable_calendar_and_back_to_the_text_box() {
        let _app = start_focus_window_app();
        let _culture = use_en_us();
        let (date_picker, window) = show_picker();
        let popup = get_popup(&date_picker);
        let calendar = get_calendar(&date_picker);
        let button = get_button(&date_picker);
        let tb = get_text_box(&date_picker);
        // The button of the reference theme is not focusable.
        button.set_focusable(false);

        // The date picker hands its focus to the text box.
        date_picker.focus();
        assert!(tb.is_focused());

        // A calendar is not focusable unless the application makes it so
        // (the reference themes do not): opening the popup leaves the focus
        // where it is.
        assert!(!calendar.focusable());
        click(&button);
        assert!(popup.is_open());
        assert!(!calendar.is_focused());
        assert!(tb.is_focused());
        click(&button);
        assert!(!popup.is_open());
        assert!(tb.is_focused());

        // A focusable calendar gets the focus once the popup is open ...
        calendar.set_focusable(true);
        click(&button);
        assert!(popup.is_open());
        assert!(calendar.is_focused());
        assert!(!tb.is_focused());

        // ... and closing gives the focus to the date picker, which hands
        // it to the text box.
        raise_key(&calendar, Key::Enter, KeyModifiers::NONE, false);
        assert!(!popup.is_open());
        assert!(!calendar.is_focused());
        assert!(tb.is_focused());

        click(&button);
        assert!(calendar.is_focused());
        click(&button);
        assert!(!popup.is_open());
        assert!(tb.is_focused());

        Dispatcher::ui_thread().run_jobs(None);
        window.close();
    }

    #[test]
    fn additional_losing_the_focus_commits_the_typed_text() {
        let _services = start_focus_services();
        let _culture = use_en_us();

        let date_picker = CalendarDatePicker::new();
        date_picker.set_template(Some(create_template()));
        let other = Button::new();
        let panel = Panel::new();
        panel.children().add(date_picker.clone());
        panel.children().add(other.clone());
        let root = TestRoot::with_child(panel);
        root.execute_initial_layout_pass();
        let errors = errors_of(&date_picker);
        let tb = get_text_box(&date_picker);

        date_picker.focus();
        assert!(tb.is_focused());
        tb.clear();
        raise_text_event(&tb, "Feb 5, 2024");
        // Nothing is committed while the text is typed.
        assert_eq!(None, date_picker.selected_date());

        other.focus();
        assert!(other.is_focused());
        assert_eq!(Some(date(2024, 2, 5)), date_picker.selected_date());
        assert_eq!(Some("2/5/2024"), date_picker.text().as_deref());
        assert_eq!(Some("2/5/2024"), tb.text().as_deref());
        assert!(errors.borrow().is_empty());

        // A text that is not a date is reported and replaced by the text
        // of the date.
        Dispatcher::ui_thread().run_jobs(None);
        date_picker.focus();
        tb.clear();
        raise_text_event(&tb, "not a date");
        other.focus();
        assert_eq!(1, errors.borrow().len());
        assert_eq!("not a date", errors.borrow()[0].0);
        assert_eq!(Some(date(2024, 2, 5)), date_picker.selected_date());
        assert_eq!(Some("2/5/2024"), tb.text().as_deref());
    }

    // --- text set before the template is applied -------------------------------------

    #[test]
    fn additional_text_set_before_the_template_is_handed_to_the_text_box() {
        let _app = start();
        let _culture = use_en_us();

        // Without a placeholder of its own the date picker forgets the text
        // when the template is applied: showing the pattern of the format
        // clears it before it is handed over.
        let date_picker = CalendarDatePicker::new();
        let errors = errors_of(&date_picker);
        date_picker.set_text(Some("abc"));
        assert_eq!(1, errors.borrow().len());
        assert_eq!(Some("abc"), date_picker.text().as_deref());
        assert_eq!(None, date_picker.selected_date());
        date_picker.set_template(Some(create_template()));
        date_picker.apply_template();
        let tb = get_text_box(&date_picker);
        assert_eq!(Some(""), tb.text().as_deref());
        assert_eq!(Some(""), date_picker.text().as_deref());
        assert_eq!(Some("<M/d/yyyy>"), tb.placeholder_text().as_deref());
        assert_eq!(1, errors.borrow().len());

        // With a placeholder text the text is handed to the text box and
        // committed again.
        let date_picker = CalendarDatePicker::new();
        let errors = errors_of(&date_picker);
        date_picker.set_placeholder_text(Some("Pick a date"));
        date_picker.set_text(Some("abc"));
        assert_eq!(1, errors.borrow().len());
        date_picker.set_template(Some(create_template()));
        date_picker.apply_template();
        let tb = get_text_box(&date_picker);
        assert_eq!(Some("abc"), tb.text().as_deref());
        assert_eq!(Some(""), date_picker.text().as_deref());
        assert_eq!(None, date_picker.selected_date());
        assert_eq!(2, errors.borrow().len());
        assert_eq!("abc", errors.borrow()[1].0);
    }

    #[test]
    fn additional_parsable_text_before_the_template_fails_like_the_reference() {
        let _app = start();
        let _culture = use_en_us();

        // The reference validates the parsed date against the calendar of
        // the template, which does not exist yet: a null reference there.
        let date_picker = CalendarDatePicker::new();
        let message = panic_message(|| date_picker.set_text(Some("2/5/2024")));
        assert_eq!("Object reference not set to an instance of an object.", message);
    }

    // --- mouse wheel ---------------------------------------------------------------

    #[test]
    fn additional_mouse_wheel_changes_the_selected_date_by_a_day() {
        let _app = start();
        let _culture = use_en_us();
        let date_picker = create_control();

        // Without a date the wheel is not handled.
        assert!(!wheel(&date_picker, 1.0));
        assert_eq!(None, date_picker.selected_date());

        date_picker.blackout_dates().unwrap().add(CalendarDateRange::new(date(2024, 2, 3)));
        date_picker.set_selected_date(Some(date(2024, 2, 5)));
        Dispatcher::ui_thread().run_jobs(None);

        // Up is the day before, down the day after.
        assert!(wheel(&date_picker, 1.0));
        assert_eq!(Some(date(2024, 2, 4)), date_picker.selected_date());
        assert!(wheel(&date_picker, -1.0));
        assert_eq!(Some(date(2024, 2, 5)), date_picker.selected_date());
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(Some("2/5/2024"), date_picker.text().as_deref());

        // A blackout date is not selected and the wheel is not handled.
        assert!(wheel(&date_picker, 1.0));
        assert_eq!(Some(date(2024, 2, 4)), date_picker.selected_date());
        assert!(!wheel(&date_picker, 1.0));
        assert_eq!(Some(date(2024, 2, 4)), date_picker.selected_date());
    }
}
