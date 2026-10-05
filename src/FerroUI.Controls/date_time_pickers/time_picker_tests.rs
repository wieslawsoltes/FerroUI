//! Port of the reference `TimePickerTests`.

use super::{DateTimePickerPanel, DateTimePickerPanelType, TimePicker, TimePickerPresenter};
use crate::presenters::ScrollContentPresenter;
use crate::primitives::{Popup, ScrollBarVisibility};
use crate::shapes::Rectangle;
use crate::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions, IControlTemplate};
use crate::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use crate::{
    Border, Button, Control, DataValidationErrors, Grid, GridLength, Panel, RowDefinition, ScrollViewer, StackPanel,
    TextBlock, Window,
};
use ferroui_base::animation::TimeSpan;
use ferroui_base::data::{BindingError, BindingErrorType, BindingNotification, BindingPriority};
use ferroui_base::layout::{ILayoutManager, VerticalAlignment};
use ferroui_base::media::text_formatting::testing::TextTestScope;
use ferroui_base::reactive::Observable;
use ferroui_base::threading::Dispatcher;
use ferroui_base::utilities::{CultureInfo, DateTimeFormatInfo, TestCultureDataProvider};
use ferroui_base::{AnyValue, BoxedValue, Ref, Size, Visual};
use std::cell::Cell;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Rc;

/// A running unit test application, with the text services of the tests
/// registered over the services of the application.
struct AppScope {
    // Dropped in this order: the text services first.
    _text: TextTestScope,
    _app: UnitTestApplicationScope,
}

fn start() -> AppScope {
    start_with(TestServices::mock_threading_interface())
}

fn start_with(services: TestServices) -> AppScope {
    let app = UnitTestApplication::start(services);
    AppScope { _text: TextTestScope::new(), _app: app }
}

/// Restores the current cultures when dropped.
struct CultureScope(CultureInfo, CultureInfo);

impl CultureScope {
    fn new() -> Self {
        Self(CultureInfo::current_culture(), CultureInfo::current_ui_culture())
    }
}

impl Drop for CultureScope {
    fn drop(&mut self) {
        CultureInfo::set_current_culture(self.0.clone());
        CultureInfo::set_current_ui_culture(self.1.clone());
    }
}

/// The culture of the reference `UseEmptyDesignatorCulture` attribute: the
/// invariant culture with empty AM and PM designators.
fn use_empty_designator_culture() -> CultureScope {
    let scope = CultureScope::new();
    let mut date_time_format = DateTimeFormatInfo::new();
    date_time_format.set_am_designator("");
    date_time_format.set_pm_designator("");
    let culture = CultureInfo::invariant_culture().with_date_time_format(date_time_format);
    CultureInfo::set_current_culture(culture.clone());
    CultureInfo::set_current_ui_culture(culture);
    scope
}

fn named<T>(control: Ref<T>, name: &str) -> Ref<T>
where
    T: ferroui_base::ObjectType + std::ops::Deref,
    Ref<T>: ferroui_base::IntoRef<Control>,
{
    let as_control: Ref<Control> = ferroui_base::IntoRef::into_ref(control.clone());
    as_control.set_name(Some(name.to_string()));
    control
}

fn create_template(include_popup: bool) -> Rc<dyn IControlTemplate> {
    create_template_with(include_popup, false)
}

/// The template of the reference tests. With `for_window` (not in the
/// reference) the presenter in the popup gets the picker template of the
/// window tests and the size a theme gives it, and its template is applied
/// by the layout instead of at once.
fn create_template_with(include_popup: bool, for_window: bool) -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::new(move |_, scope| {
        let layout_root = named(Grid::new(), "LayoutRoot").register_in_name_scope(&**scope);

        // Skip the content presenter
        let flyout_button = named(Button::new(), "PART_FlyoutButton").register_in_name_scope(&**scope);
        let content_grid = named(Grid::new(), "PART_FlyoutButtonContentGrid").register_in_name_scope(&**scope);

        let picker_host = |host_name: &str, text_name: &str, column: i32| {
            let host = named(Border::new(), host_name);
            host.set_child(named(TextBlock::new(), text_name).register_in_name_scope(&**scope));
            let host = host.register_in_name_scope(&**scope);
            Grid::set_column(&host, column);
            host
        };
        let spacer = |name: &str, column: i32| {
            let spacer = named(Rectangle::new(), name).register_in_name_scope(&**scope);
            Grid::set_column(&spacer, column);
            spacer
        };

        let first_picker_host = picker_host("PART_FirstPickerHost", "PART_HourTextBlock", 0);
        let second_picker_host = picker_host("PART_SecondPickerHost", "PART_MinuteTextBlock", 2);
        let third_picker_host = picker_host("PART_ThirdPickerHost", "PART_SecondTextBlock", 4);
        let fourth_picker_host = picker_host("PART_FourthPickerHost", "PART_PeriodTextBlock", 6);

        let first_spacer = spacer("PART_FirstColumnDivider", 1);
        let second_spacer = spacer("PART_SecondColumnDivider", 3);
        let third_spacer = spacer("PART_ThirdColumnDivider", 5);

        content_grid.children().add(first_picker_host);
        content_grid.children().add(first_spacer);
        content_grid.children().add(second_picker_host);
        content_grid.children().add(second_spacer);
        content_grid.children().add(third_picker_host);
        content_grid.children().add(third_spacer);
        content_grid.children().add(fourth_picker_host);
        flyout_button.set_content(Some(Control::boxed(content_grid)));
        layout_root.children().add(flyout_button);

        if include_popup {
            let popup = named(Popup::new(), "PART_Popup").register_in_name_scope(&**scope);

            let picker_presenter = named(TimePickerPresenter::new(), "PART_PickerPresenter");
            if for_window {
                picker_presenter.set_template(Some(create_window_picker_template()));
                picker_presenter.set_width(242.0);
                picker_presenter.set_height(398.0);
                picker_presenter.set_max_height(398.0);
            } else {
                picker_presenter.set_template(Some(create_picker_template()));
            }
            let picker_presenter = picker_presenter.register_in_name_scope(&**scope);
            if !for_window {
                picker_presenter.apply_template();
            }

            popup.set_child(picker_presenter);

            layout_root.children().add(popup);
        }

        layout_root.upcast()
    })
}

fn create_picker_template() -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::new(|_, scope| {
        let accept_button = named(Button::new(), "PART_AcceptButton").register_in_name_scope(&**scope);

        let hour_selector = named(DateTimePickerPanel::new(), "PART_HourSelector");
        hour_selector.set_panel_type(DateTimePickerPanelType::Hour);
        hour_selector.set_should_loop(true);
        let hour_selector = hour_selector.register_in_name_scope(&**scope);

        let minute_selector = named(DateTimePickerPanel::new(), "PART_MinuteSelector");
        minute_selector.set_panel_type(DateTimePickerPanelType::Minute);
        minute_selector.set_should_loop(true);
        let minute_selector = minute_selector.register_in_name_scope(&**scope);

        let second_host = named(Panel::new(), "PART_SecondHost").register_in_name_scope(&**scope);

        let second_selector = named(DateTimePickerPanel::new(), "PART_SecondSelector");
        second_selector.set_panel_type(DateTimePickerPanelType::Second);
        second_selector.set_should_loop(true);
        let second_selector = second_selector.register_in_name_scope(&**scope);

        let period_host = named(Panel::new(), "PART_PeriodHost").register_in_name_scope(&**scope);

        let period_selector = named(DateTimePickerPanel::new(), "PART_PeriodSelector");
        period_selector.set_panel_type(DateTimePickerPanelType::TimePeriod);
        let period_selector = period_selector.register_in_name_scope(&**scope);

        let picker_container = named(Grid::new(), "PART_PickerContainer").register_in_name_scope(&**scope);
        let second_spacer = named(Rectangle::new(), "PART_SecondSpacer").register_in_name_scope(&**scope);
        let third_spacer = named(Rectangle::new(), "PART_ThirdSpacer").register_in_name_scope(&**scope);

        let content_panel = Panel::new();
        content_panel.children().add(accept_button);
        content_panel.children().add(hour_selector);
        content_panel.children().add(minute_selector);
        content_panel.children().add(second_host);
        content_panel.children().add(second_selector);
        content_panel.children().add(period_host);
        content_panel.children().add(period_selector);
        content_panel.children().add(picker_container);
        content_panel.children().add(second_spacer);
        content_panel.children().add(third_spacer);
        content_panel.upcast()
    })
}

/// The content grid of the flyout button of an applied picker template.
fn button_content_grid(time_picker: &TimePicker) -> Ref<Grid> {
    let desc: Vec<Ref<Visual>> = time_picker.get_visual_descendants().collect();
    assert!(desc.len() > 1); // Should be the layout root grid and the button

    let button = desc[1].cast::<Button>().expect("the second descendant is the button");
    let content = button.content().expect("the button has content");
    Control::from_boxed(&content).and_then(|content| content.cast::<Grid>()).expect("the content is the grid")
}

fn host_at(container: &Grid, index: usize) -> Ref<Border> {
    container.children().get(index).cast::<Border>().expect("the child is a border")
}

fn text_of(host: &Border) -> Ref<TextBlock> {
    host.child().and_then(|child| child.cast::<TextBlock>()).expect("the child of the host is a text block")
}

#[test]
fn selected_time_changed_should_fire_when_selected_time_set() {
    let _app = start();
    let handled = Rc::new(Cell::new(false));
    let time_picker = TimePicker::new();
    let flag = handled.clone();
    time_picker.selected_time_changed(move |_| flag.set(true));
    let value = TimeSpan::from_hours(10.0);
    time_picker.set_selected_time(Some(value));
    Dispatcher::ui_thread().run_jobs(None);
    assert!(handled.get());
}

#[test]
fn using_24_hour_clock_should_hide_period() {
    let _app = start();
    let time_picker = TimePicker::new();
    time_picker.set_clock_identifier("12HourClock");
    time_picker.set_template(Some(create_template(false)));
    time_picker.apply_template();

    let container = button_content_grid(&time_picker);

    let period_text_host = host_at(&container, 6);
    assert!(period_text_host.is_visible());

    time_picker.set_clock_identifier("24HourClock");
    assert!(!period_text_host.is_visible());
}

#[test]
fn use_seconds_equals_false_should_hide_seconds() {
    let _app = start();
    let time_picker = TimePicker::new();
    time_picker.set_use_seconds(true);
    time_picker.set_template(Some(create_template(false)));
    time_picker.apply_template();

    let container = button_content_grid(&time_picker);

    let period_text_host = host_at(&container, 4);
    assert!(period_text_host.is_visible());

    time_picker.set_use_seconds(false);
    assert!(!period_text_host.is_visible());
}

#[test]
fn use_seconds_equals_false_should_have_zero_seconds() {
    let _app = start();
    let time_picker = TimePicker::new();
    time_picker.set_use_seconds(false);
    time_picker.set_template(Some(create_template(true)));
    time_picker.apply_template();

    let desc: Vec<Ref<Visual>> = time_picker.get_visual_descendants().collect();
    assert!(desc.len() > 2);

    // find button
    let btn = desc[1].cast::<Button>().expect("the second descendant is the button");
    let popup = desc[2].cast::<Popup>().expect("the third descendant is the popup");

    let time_picker_presenter =
        popup.child().and_then(|child| child.cast::<TimePickerPresenter>()).expect("the child is the presenter");
    let panel = time_picker_presenter.visual_children().get(0).cast::<Panel>().expect("the template root is a panel");
    let accept_btn = panel.visual_children().get(0).cast::<Button>().expect("the first child is the accept button");

    assert!(!popup.is_open());
    btn.perform_click();
    assert!(popup.is_open());
    assert!(!time_picker_presenter.use_seconds());

    accept_btn.perform_click();

    assert_eq!(0, time_picker_presenter.time().seconds());
    assert_eq!(Some(0), time_picker.selected_time().map(|time| time.seconds()));
}

#[test]
fn time_picker_presenter_use_seconds_equals_false_should_have_zero_seconds() {
    let _app = start();
    let time_picker_presenter = TimePickerPresenter::new();
    time_picker_presenter.set_use_seconds(false);
    time_picker_presenter.set_template(Some(create_picker_template()));
    time_picker_presenter.apply_template();

    let panel = time_picker_presenter.visual_children().get(0).cast::<Panel>().expect("the template root is a panel");
    let accept_btn = panel.visual_children().get(0).cast::<Button>().expect("the first child is the accept button");

    accept_btn.perform_click();

    assert_eq!(0, time_picker_presenter.time().seconds());
}

#[test]
fn selected_time_null_should_use_placeholders() {
    let _app = start();
    let time_picker = TimePicker::new();
    time_picker.set_template(Some(create_template(false)));
    time_picker.apply_template();

    let container = button_content_grid(&time_picker);

    let hour_text = text_of(&host_at(&container, 0));
    let minute_text = text_of(&host_at(&container, 2));
    let second_text = text_of(&host_at(&container, 4));

    let ts = TimeSpan::from_hours(10.0);
    time_picker.set_selected_time(Some(ts));
    assert!(hour_text.text().is_some());
    assert!(minute_text.text().is_some());
    assert!(second_text.text().is_some());

    time_picker.set_selected_time(None);
    assert!(hour_text.text().is_none());
    assert!(minute_text.text().is_none());
    assert!(second_text.text().is_none());
}

#[test]
fn using_12_hour_clock_on_culture_with_empty_period_should_show_period() {
    let _culture = use_empty_designator_culture();
    let _app = start();
    let time_picker = TimePicker::new();
    time_picker.set_template(Some(create_template(false)));
    time_picker.set_clock_identifier("12HourClock");
    time_picker.apply_template();

    let container = button_content_grid(&time_picker);

    let period_text = text_of(&host_at(&container, 6));

    let ts = TimeSpan::from_hours(10.0);
    time_picker.set_selected_time(Some(ts));
    assert!(!period_text.text().unwrap_or_default().is_empty());

    time_picker.set_selected_time(None);
    assert!(!period_text.text().unwrap_or_default().is_empty());
}

/// What the handlers of the data validation tests fail with: the stand-in
/// for the data validation exception that the handlers of the reference
/// tests throw. A panic payload has to be `Send`, which the error type of
/// the framework (it holds shared error data) is not.
#[derive(Debug)]
struct ThrownDataValidationException {
    message: String,
}

/// Runs `action` and returns the data validation failure it fails with.
/// Fails when `action` completes or fails with anything else, as the
/// `Assert.Throws<DataValidationException>` of the reference tests does.
fn assert_throws_data_validation_exception(action: impl FnOnce()) -> ThrownDataValidationException {
    match catch_unwind(AssertUnwindSafe(action)) {
        Ok(()) => panic!("expected a data validation failure"),
        Err(payload) => match payload.downcast::<ThrownDataValidationException>() {
            Ok(exception) => *exception,
            Err(payload) => std::panic::resume_unwind(payload),
        },
    }
}

/// Asserts that the data validation errors of the control are exactly
/// `exception`.
fn assert_errors_are(target: &Control, exception: &BindingError) {
    let errors = DataValidationErrors::get_errors(target).map(|errors| errors.to_vec()).unwrap_or_default();
    assert_eq!(1, errors.len());
    let error = errors[0].clone();
    let error: &dyn AnyValue = &*error;
    assert!(error.downcast_ref::<BindingError>() == Some(exception));
}

#[test]
fn selected_time_enable_data_validation() {
    let _app = start();
    let handled = Rc::new(Cell::new(false));
    let time_picker = TimePicker::new();

    let flag = handled.clone();
    time_picker.selected_time_changed(move |e| {
        let min_time = TimeSpan::from_hms(10, 0, 0);
        let max_time = TimeSpan::from_hms(15, 0, 0);

        if e.new_time().is_some_and(|new_time| new_time < min_time) {
            std::panic::panic_any(ThrownDataValidationException { message: format!("time is less than {max_time}") });
        }

        if e.new_time().is_some_and(|new_time| new_time > max_time) {
            std::panic::panic_any(ThrownDataValidationException { message: format!("time is over {max_time}") });
        }

        flag.set(true);
    });

    // time is less than
    let exception =
        assert_throws_data_validation_exception(|| time_picker.set_selected_time(Some(TimeSpan::from_hms(1, 2, 3))));
    assert!(exception.message.starts_with("time is less than"));

    // time is over
    let exception =
        assert_throws_data_validation_exception(|| time_picker.set_selected_time(Some(TimeSpan::from_hms(21, 22, 23))));
    assert!(exception.message.starts_with("time is over"));

    let exception = BindingError::message("failed validation");
    let observable = Observable::single_value(Rc::new(BindingNotification::with_error(
        exception.clone(),
        BindingErrorType::DataValidationError,
    )) as BoxedValue);
    time_picker.bind_property_untyped(
        TimePicker::selected_time_property().as_property(),
        observable,
        BindingPriority::LocalValue,
    );

    assert!(DataValidationErrors::get_has_errors(&time_picker));
    // Not asserted by the reference test: the error of the control is the
    // one the binding reported.
    assert_errors_are(&time_picker, &exception);

    Dispatcher::ui_thread().run_jobs(None);
    time_picker.set_selected_time(Some(TimeSpan::from_hms(11, 12, 13)));
    assert!(handled.get());
}

fn test_selector_scrolling(selector_name: &str, scroll: impl Fn(&DateTimePickerPanel)) {
    let _app = start();

    let presenter = TimePickerPresenter::new();
    presenter.set_template(Some(create_picker_template()));
    presenter.apply_template();
    presenter.measure(Size::new(1000.0, 1000.0));

    let panel = presenter
        .get_visual_descendants()
        .filter_map(|visual| visual.cast::<DateTimePickerPanel>())
        .find(|panel| panel.name().as_deref() == Some(selector_name))
        .expect("the selector");

    let previous_offset = panel.offset();
    scroll(&panel);
    assert_ne!(previous_offset, panel.offset());
}

#[test]
fn selector_scroll_up_should_work() {
    for selector_name in ["PART_HourSelector", "PART_MinuteSelector", "PART_SecondSelector"] {
        test_selector_scrolling(selector_name, |panel| panel.scroll_up(1));
    }
}

#[test]
fn selector_scroll_down_should_work() {
    for selector_name in ["PART_HourSelector", "PART_MinuteSelector", "PART_SecondSelector"] {
        test_selector_scrolling(selector_name, |panel| panel.scroll_down(1));
    }
}

#[test]
fn vertical_content_alignment_round_trips() {
    let _app = start();
    for value in
        [VerticalAlignment::Top, VerticalAlignment::Center, VerticalAlignment::Bottom, VerticalAlignment::Stretch]
    {
        let time_picker = TimePicker::new();
        time_picker.set_vertical_content_alignment(value);
        assert_eq!(value, time_picker.vertical_content_alignment());
    }
}

#[test]
fn vertical_content_alignment_default_is_stretch() {
    let _app = start();
    let time_picker = TimePicker::new();
    assert_eq!(VerticalAlignment::Stretch, time_picker.vertical_content_alignment());
}

/// Not in the reference suite: the clock identifier a new picker starts
/// with follows the short time pattern of the current culture, and the
/// picker texts use the designators of the culture.
#[test]
fn clock_identifier_follows_the_culture() {
    let _culture = CultureScope::new();
    let _app = start();
    TestCultureDataProvider::register();

    // The invariant pattern is `HH:mm`.
    assert_eq!("24HourClock", TimePicker::new().clock_identifier());

    CultureInfo::set_current_culture(CultureInfo::get_culture_info("en-US"));
    let time_picker = TimePicker::new();
    assert_eq!("12HourClock", time_picker.clock_identifier());

    time_picker.set_use_seconds(true);
    time_picker.set_template(Some(create_template(false)));
    time_picker.apply_template();
    let container = button_content_grid(&time_picker);
    time_picker.set_selected_time(Some(TimeSpan::from_hms(15, 4, 5)));
    assert_eq!(Some("3"), text_of(&host_at(&container, 0)).text().as_deref());
    assert_eq!(Some("04"), text_of(&host_at(&container, 2)).text().as_deref());
    assert_eq!(Some("05"), text_of(&host_at(&container, 4)).text().as_deref());
    assert_eq!(Some("PM"), text_of(&host_at(&container, 6)).text().as_deref());
    assert_eq!(7, container.column_definitions().count());

    CultureInfo::set_current_culture(CultureInfo::get_culture_info("en-GB"));
    assert_eq!("24HourClock", TimePicker::new().clock_identifier());
    time_picker.set_selected_time(Some(TimeSpan::from_hms(9, 4, 5)));
    assert_eq!(Some("am"), text_of(&host_at(&container, 6)).text().as_deref());
}

fn scroll_viewer_template() -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::for_type::<ScrollViewer>(|_, scope| {
        let presenter = ScrollContentPresenter::new();
        presenter.set_name(Some("PART_ContentPresenter".to_string()));

        let panel = Panel::new();
        panel.children().add(presenter.register_in_name_scope(&**scope));
        panel.upcast()
    })
}

/// The picker template of the window tests: the hosts are in the picker
/// container and hold the selectors inside scroll viewers, and there is a
/// dismiss button next to the accept button.
fn create_window_picker_template() -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::new(|_, scope| {
        let picker_container = named(Grid::new(), "PART_PickerContainer").register_in_name_scope(&**scope);

        for (host_name, selector_name, panel_type, should_loop, column) in [
            ("PART_HourHost", "PART_HourSelector", DateTimePickerPanelType::Hour, true, 0),
            ("PART_MinuteHost", "PART_MinuteSelector", DateTimePickerPanelType::Minute, true, 2),
            ("PART_SecondHost", "PART_SecondSelector", DateTimePickerPanelType::Second, true, 4),
            ("PART_PeriodHost", "PART_PeriodSelector", DateTimePickerPanelType::TimePeriod, false, 6),
        ] {
            let selector = named(DateTimePickerPanel::new(), selector_name);
            selector.set_panel_type(panel_type);
            selector.set_should_loop(should_loop);
            let selector = selector.register_in_name_scope(&**scope);

            let scroll_viewer = ScrollViewer::new();
            scroll_viewer.set_template(Some(scroll_viewer_template()));
            scroll_viewer.set_horizontal_scroll_bar_visibility(ScrollBarVisibility::Disabled);
            scroll_viewer.set_vertical_scroll_bar_visibility(ScrollBarVisibility::Hidden);
            scroll_viewer.set_content(Some(Control::boxed(selector)));

            let host = named(Panel::new(), host_name).register_in_name_scope(&**scope);
            host.children().add(scroll_viewer);
            Grid::set_column(&host, column);
            picker_container.children().add(host);
        }

        for (name, column) in [("PART_FirstSpacer", 1), ("PART_SecondSpacer", 3), ("PART_ThirdSpacer", 5)] {
            let spacer = named(Rectangle::new(), name).register_in_name_scope(&**scope);
            Grid::set_column(&spacer, column);
            picker_container.children().add(spacer);
        }

        let accept_button = named(Button::new(), "PART_AcceptButton").register_in_name_scope(&**scope);
        let dismiss_button = named(Button::new(), "PART_DismissButton").register_in_name_scope(&**scope);
        let buttons = StackPanel::new();
        buttons.children().add(accept_button);
        buttons.children().add(dismiss_button);
        Grid::set_row(&buttons, 1);

        let root = Grid::new();
        root.row_definitions().add(RowDefinition::with_height(GridLength::STAR));
        root.row_definitions().add(RowDefinition::with_height(GridLength::AUTO));
        root.children().add(picker_container);
        root.children().add(buttons);
        root.upcast()
    })
}

struct WindowTarget {
    _window: Ref<Window>,
    time_picker: Ref<TimePicker>,
    flyout_button: Ref<Button>,
    popup: Ref<Popup>,
    presenter: Ref<TimePickerPresenter>,
}

impl WindowTarget {
    /// Shows a window with a 24 hour time picker with seconds whose
    /// selected time is `selected`.
    fn show(selected: TimeSpan) -> Self {
        let time_picker = TimePicker::new();
        time_picker.set_clock_identifier("24HourClock");
        time_picker.set_use_seconds(true);
        time_picker.set_template(Some(create_template_with(true, true)));
        time_picker.set_selected_time(Some(selected));

        let window = Window::new();
        window.set_content(Some(Control::boxed(time_picker.clone())));
        window.show();
        window.layout_manager().execute_initial_layout_pass();

        let find = |name: &str| {
            time_picker
                .get_visual_descendants()
                .filter_map(|visual| visual.cast::<Control>())
                .find(|control| control.name().as_deref() == Some(name))
                .unwrap_or_else(|| panic!("no {name}"))
        };
        let flyout_button = find("PART_FlyoutButton").cast::<Button>().expect("a button");
        let popup = find("PART_Popup").cast::<Popup>().expect("a popup");
        let presenter =
            popup.child().and_then(|child| child.cast::<TimePickerPresenter>()).expect("the child is the presenter");
        Self { _window: window, time_picker, flyout_button, popup, presenter }
    }

    /// A named part of the presenter shown in the popup.
    fn part<T: ferroui_base::ObjectType>(&self, name: &str) -> Ref<T> {
        self.presenter
            .get_visual_descendants()
            .filter_map(|visual| visual.cast::<Control>())
            .find(|control| control.name().as_deref() == Some(name))
            .and_then(|control| control.cast::<T>())
            .unwrap_or_else(|| panic!("no {name}"))
    }
}

/// Additional test (not a port): in a window, the flyout button shows the
/// presenter in a popup host with the selected time; selecting another
/// minute and accepting sets the selected time and closes the popup.
#[test]
fn window_popup_accept_updates_selected_time_and_closes() {
    let _app = start_with(TestServices::styled_window());
    let selected = TimeSpan::from_hms(10, 20, 30);
    let target = WindowTarget::show(selected);
    let changes = Rc::new(Cell::new(0));
    let counter = changes.clone();
    target.time_picker.selected_time_changed(move |_| counter.set(counter.get() + 1));

    assert!(!target.popup.is_open());
    assert!(target.popup.host().is_none());

    target.flyout_button.perform_click();
    Dispatcher::ui_thread().run_jobs(None);

    assert!(target.popup.is_open());
    assert!(target.popup.host().is_some());
    assert_eq!(selected, target.presenter.time());
    assert_eq!("24HourClock", target.presenter.clock_identifier());
    assert!(target.presenter.use_seconds());

    // The presenter is laid out in the popup: the selectors show the time.
    let hour_selector = target.part::<DateTimePickerPanel>("PART_HourSelector");
    let minute_selector = target.part::<DateTimePickerPanel>("PART_MinuteSelector");
    let second_selector = target.part::<DateTimePickerPanel>("PART_SecondSelector");
    assert!(minute_selector.is_measure_valid());
    assert!(minute_selector.children().count() > 0);
    assert_eq!(
        (10, 20, 30),
        (hour_selector.selected_value(), minute_selector.selected_value(), second_selector.selected_value())
    );
    assert!(!target.part::<Panel>("PART_PeriodHost").is_visible());

    minute_selector.scroll_down(1);
    hour_selector.scroll_up(1);
    assert_eq!((9, 21), (hour_selector.selected_value(), minute_selector.selected_value()));
    // Nothing changes before the selection is accepted.
    assert_eq!(Some(selected), target.time_picker.selected_time());
    assert_eq!(selected, target.presenter.time());

    target.part::<Button>("PART_AcceptButton").perform_click();
    Dispatcher::ui_thread().run_jobs(None);

    assert_eq!(Some(TimeSpan::from_hms(9, 21, 30)), target.time_picker.selected_time());
    assert_eq!(TimeSpan::from_hms(9, 21, 30), target.presenter.time());
    assert_eq!(1, changes.get());
    assert!(!target.popup.is_open());
    assert!(target.popup.host().is_none());
}

/// Additional test (not a port): in a window, dismissing the presenter
/// after selecting other values leaves the selected time unchanged and
/// closes the popup.
#[test]
fn window_popup_dismiss_leaves_selected_time_unchanged() {
    let _app = start_with(TestServices::styled_window());
    let selected = TimeSpan::from_hms(10, 20, 30);
    let target = WindowTarget::show(selected);
    let changes = Rc::new(Cell::new(0));
    let counter = changes.clone();
    target.time_picker.selected_time_changed(move |_| counter.set(counter.get() + 1));

    target.flyout_button.perform_click();
    Dispatcher::ui_thread().run_jobs(None);
    assert!(target.popup.is_open());

    let second_selector = target.part::<DateTimePickerPanel>("PART_SecondSelector");
    second_selector.scroll_down(4);
    assert_eq!(34, second_selector.selected_value());

    target.part::<Button>("PART_DismissButton").perform_click();
    Dispatcher::ui_thread().run_jobs(None);

    assert_eq!(Some(selected), target.time_picker.selected_time());
    assert_eq!(selected, target.presenter.time());
    assert_eq!(0, changes.get());
    assert!(!target.popup.is_open());
    assert!(target.popup.host().is_none());
}
