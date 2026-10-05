//! Port of the reference `DatePickerTests`.

use super::{DatePicker, DatePickerPresenter, DateTimePickerPanel, DateTimePickerPanelType};
use crate::presenters::ScrollContentPresenter;
use crate::primitives::{Popup, ScrollBarVisibility, TemplatedControl};
use crate::shapes::Rectangle;
use crate::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions, IControlTemplate};
use crate::test_support::TestRoot;
use crate::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use crate::{
    Button, Control, DataValidationErrors, Grid, GridLength, ListBoxItem, Panel, RowDefinition, ScrollViewer, StackPanel,
    TextBlock, Window,
};
use ferroui_base::animation::TimeSpan;
use ferroui_base::data::{BindingError, BindingErrorType, BindingNotification, BindingPriority};
use ferroui_base::input::{InputElement, Key, KeyEventArgs};
use ferroui_base::layout::{ILayoutManager, VerticalAlignment};
use ferroui_base::media::text_formatting::testing::TextTestScope;
use ferroui_base::reactive::Observable;
use ferroui_base::threading::Dispatcher;
use ferroui_base::utilities::{CultureInfo, DateTimeOffset, TestCultureDataProvider};
use ferroui_base::{AnyValue, BoxedValue, Rect, Ref, Size, Visual};
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

fn start(services: TestServices) -> AppScope {
    let app = UnitTestApplication::start(services);
    AppScope { _text: TextTestScope::new(), _app: app }
}

fn services() -> TestServices {
    TestServices::mock_threading_interface()
}

fn focus_services() -> TestServices {
    TestServices::real_focus()
}

/// Restores the current culture when dropped.
struct CultureScope(CultureInfo);

impl CultureScope {
    fn new() -> Self {
        Self(CultureInfo::current_culture())
    }
}

impl Drop for CultureScope {
    fn drop(&mut self) {
        CultureInfo::set_current_culture(self.0.clone());
    }
}

fn date(year: i32, month: i32, day: i32) -> DateTimeOffset {
    DateTimeOffset::new(year, month, day, 0, 0, 0, TimeSpan::default())
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

fn create_template() -> Rc<dyn IControlTemplate> {
    create_template_with(false)
}

/// The template of the reference tests; with `include_popup` (not in the
/// reference) also the popup with the presenter, for the window tests.
fn create_template_with(include_popup: bool) -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::new(move |_, scope| {
        let layout_root = named(Grid::new(), "LayoutRoot").register_in_name_scope(&**scope);
        // Skip the content presenter
        let flyout_button = named(Button::new(), "PART_FlyoutButton").register_in_name_scope(&**scope);
        let content_grid = named(Grid::new(), "PART_ButtonContentGrid").register_in_name_scope(&**scope);
        let day_text = named(TextBlock::new(), "PART_DayTextBlock").register_in_name_scope(&**scope);
        let month_text = named(TextBlock::new(), "PART_MonthTextBlock").register_in_name_scope(&**scope);
        let year_text = named(TextBlock::new(), "PART_YearTextBlock").register_in_name_scope(&**scope);
        let first_spacer = named(Rectangle::new(), "PART_FirstSpacer").register_in_name_scope(&**scope);
        let second_spacer = named(Rectangle::new(), "PART_SecondSpacer").register_in_name_scope(&**scope);
        let third_spacer = named(Rectangle::new(), "PART_ThirdSpacer").register_in_name_scope(&**scope);

        content_grid.children().add(day_text);
        content_grid.children().add(month_text);
        content_grid.children().add(year_text);
        content_grid.children().add(first_spacer);
        content_grid.children().add(second_spacer);
        content_grid.children().add(third_spacer);
        flyout_button.set_content(Some(Control::boxed(content_grid)));
        layout_root.children().add(flyout_button);

        if include_popup {
            let popup = named(Popup::new(), "PART_Popup").register_in_name_scope(&**scope);

            let picker_presenter = named(DatePickerPresenter::new(), "PART_PickerPresenter");
            picker_presenter.set_template(Some(create_window_picker_template()));
            picker_presenter.set_width(296.0);
            picker_presenter.set_height(398.0);
            picker_presenter.set_max_height(398.0);
            popup.set_child(picker_presenter.register_in_name_scope(&**scope));

            layout_root.children().add(popup);
        }

        layout_root.upcast()
    })
}

fn create_picker_template() -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::new(|_, scope| {
        let selector = |name: &str, panel_type: DateTimePickerPanelType| {
            let selector = named(DateTimePickerPanel::new(), name);
            selector.set_panel_type(panel_type);
            selector.set_should_loop(true);
            selector.register_in_name_scope(&**scope)
        };

        let day_host = named(Panel::new(), "PART_DayHost").register_in_name_scope(&**scope);
        let day_selector = selector("PART_DaySelector", DateTimePickerPanelType::Day);
        let month_host = named(Panel::new(), "PART_MonthHost").register_in_name_scope(&**scope);
        let month_selector = selector("PART_MonthSelector", DateTimePickerPanelType::Month);
        let year_host = named(Panel::new(), "PART_YearHost").register_in_name_scope(&**scope);
        let year_selector = selector("PART_YearSelector", DateTimePickerPanelType::Year);
        let accept_button = named(Button::new(), "PART_AcceptButton").register_in_name_scope(&**scope);
        let picker_container = named(Grid::new(), "PART_PickerContainer").register_in_name_scope(&**scope);
        let first_spacer = named(Rectangle::new(), "PART_FirstSpacer").register_in_name_scope(&**scope);
        let second_spacer = named(Rectangle::new(), "PART_SecondSpacer").register_in_name_scope(&**scope);

        let content_panel = Panel::new();
        content_panel.children().add(day_host);
        content_panel.children().add(day_selector);
        content_panel.children().add(month_host);
        content_panel.children().add(month_selector);
        content_panel.children().add(year_host);
        content_panel.children().add(year_selector);
        content_panel.children().add(accept_button);
        content_panel.children().add(picker_container);
        content_panel.children().add(first_spacer);
        content_panel.children().add(second_spacer);
        content_panel.upcast()
    })
}

/// The content grid of the flyout button of an applied picker template.
fn button_content_grid(date_picker: &DatePicker) -> Ref<Grid> {
    let desc: Vec<Ref<Visual>> = date_picker.get_visual_descendants().collect();
    assert!(desc.len() > 1); // Should be the layout root grid and the button

    let button = desc[1].cast::<Button>().expect("the second descendant is the button");
    let content = button.content().expect("the button has content");
    Control::from_boxed(&content).and_then(|content| content.cast::<Grid>()).expect("the content is the grid")
}

fn find_text(container: &Grid, name: &str) -> Option<Ref<TextBlock>> {
    container
        .children()
        .snapshot()
        .iter()
        .filter_map(|child| child.cast::<TextBlock>())
        .find(|text| text.name().as_deref() == Some(name))
}

#[test]
fn selected_date_changed_should_fire_when_selected_date_set() {
    let _app = start(services());
    let handled = Rc::new(Cell::new(false));
    let date_picker = DatePicker::new();
    let flag = handled.clone();
    date_picker.selected_date_changed(move |_| flag.set(true));
    let value = date(2000, 10, 10);
    date_picker.set_selected_date(Some(value));
    Dispatcher::ui_thread().run_jobs(None);
    assert!(handled.get());
}

#[test]
fn day_visible_false_should_hide_day() {
    let _app = start(services());
    let date_picker = DatePicker::new();
    date_picker.set_template(Some(create_template()));
    date_picker.set_day_visible(false);
    date_picker.apply_template();
    Dispatcher::ui_thread().run_jobs(None);

    let container = button_content_grid(&date_picker);
    let day_text = find_text(&container, "PART_DayTextBlock").expect("the day text block");

    assert!(!day_text.is_visible());
    assert_eq!(3, container.column_definitions().count());
}

#[test]
fn month_visible_false_should_hide_month() {
    let _app = start(services());
    let date_picker = DatePicker::new();
    date_picker.set_template(Some(create_template()));
    date_picker.set_month_visible(false);
    date_picker.apply_template();
    Dispatcher::ui_thread().run_jobs(None);

    let container = button_content_grid(&date_picker);
    let month_text = find_text(&container, "PART_MonthTextBlock").expect("the month text block");

    assert!(!month_text.is_visible());
    assert_eq!(3, container.column_definitions().count());
}

#[test]
fn year_visible_false_should_hide_year() {
    let _app = start(services());
    let date_picker = DatePicker::new();
    date_picker.set_template(Some(create_template()));
    date_picker.set_year_visible(false);
    date_picker.apply_template();
    Dispatcher::ui_thread().run_jobs(None);

    let container = button_content_grid(&date_picker);
    let year_text = find_text(&container, "PART_YearTextBlock").expect("the year text block");

    assert!(!year_text.is_visible());
    assert_eq!(3, container.column_definitions().count());
}

#[test]
fn selected_date_null_should_use_placeholders() {
    let _app = start(services());
    let date_picker = DatePicker::new();
    date_picker.set_template(Some(create_template()));
    date_picker.set_year_visible(false);
    date_picker.apply_template();
    Dispatcher::ui_thread().run_jobs(None);

    let container = button_content_grid(&date_picker);
    let year_text = find_text(&container, "PART_YearTextBlock").expect("the year text block");
    let month_text = find_text(&container, "PART_MonthTextBlock").expect("the month text block");
    let day_text = find_text(&container, "PART_DayTextBlock").expect("the day text block");

    let value = date(2000, 10, 10);
    date_picker.set_selected_date(Some(value));

    assert!(day_text.text().is_some());
    assert!(month_text.text().is_some());
    assert!(year_text.text().is_some());
    assert!(!date_picker.classes().contains(":hasnodate"));

    date_picker.set_selected_date(None);

    assert!(day_text.text().is_none());
    assert!(month_text.text().is_none());
    assert!(year_text.text().is_none());
    assert!(date_picker.classes().contains(":hasnodate"));
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
fn selected_date_enable_data_validation() {
    let _app = start(services());
    let handled = Rc::new(Cell::new(false));
    let date_picker = DatePicker::new();

    let flag = handled.clone();
    date_picker.selected_date_changed(move |e| {
        let min_date_time = date(2000, 1, 1);
        let max_date_time = date(2010, 1, 1);

        if e.new_date().is_some_and(|new_date| new_date < min_date_time) {
            std::panic::panic_any(ThrownDataValidationException {
                message: format!("dateTime is less than {min_date_time}"),
            });
        }
        if e.new_date().is_some_and(|new_date| new_date > max_date_time) {
            std::panic::panic_any(ThrownDataValidationException { message: format!("dateTime is over {max_date_time}") });
        }

        flag.set(true);
    });

    // dateTime is less than
    let exception = assert_throws_data_validation_exception(|| date_picker.set_selected_date(Some(date(1999, 1, 1))));
    assert!(exception.message.starts_with("dateTime is less than"));

    // dateTime is over
    let exception = assert_throws_data_validation_exception(|| date_picker.set_selected_date(Some(date(2021, 1, 1))));
    assert!(exception.message.starts_with("dateTime is over"));

    let exception = BindingError::message("failed validation");
    let observable = Observable::single_value(Rc::new(BindingNotification::with_error(
        exception.clone(),
        BindingErrorType::DataValidationError,
    )) as BoxedValue);
    date_picker.bind_property_untyped(
        DatePicker::selected_date_property().as_property(),
        observable,
        BindingPriority::LocalValue,
    );

    assert!(DataValidationErrors::get_has_errors(&date_picker));
    // Not asserted by the reference test: the error of the control is the
    // one the binding reported.
    assert_errors_are(&date_picker, &exception);

    Dispatcher::ui_thread().run_jobs(None);
    date_picker.set_selected_date(Some(DateTimeOffset::new(2005, 5, 10, 11, 12, 13, TimeSpan::default())));
    assert!(handled.get());
}

fn test_selector_scrolling(selector_name: &str, scroll: impl Fn(&DateTimePickerPanel)) {
    let _app = start(services());

    let presenter = DatePickerPresenter::new();
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
    for selector_name in ["PART_DaySelector", "PART_MonthSelector", "PART_YearSelector"] {
        test_selector_scrolling(selector_name, |panel| panel.scroll_up(1));
    }
}

#[test]
fn selector_scroll_down_should_work() {
    for selector_name in ["PART_DaySelector", "PART_MonthSelector", "PART_YearSelector"] {
        test_selector_scrolling(selector_name, |panel| panel.scroll_down(1));
    }
}

#[test]
fn selector_item_height_change_should_update_layout_state() {
    for (should_loop, expected_extent_height, expected_offset_y) in [(false, 120.0, 40.0), (true, 12000.0, 6040.0)] {
        let _app = start(services());

        let panel = DateTimePickerPanel::new();
        panel.set_minimum_value(1);
        panel.set_maximum_value(3);
        panel.set_should_loop(should_loop);
        panel.set_item_height(20.0);

        panel.measure(Size::new(100.0, 100.0));
        panel.arrange(Rect::new(0.0, 0.0, 100.0, 100.0));
        panel.set_selected_value(2);

        panel.set_item_height(40.0);

        assert_eq!(expected_extent_height, panel.extent().height);
        assert_eq!(expected_offset_y, panel.offset().y);
        for child in panel.children().snapshot().iter() {
            assert_eq!(40.0, child.height());
        }

        panel.measure(Size::new(100.0, 100.0));
        panel.arrange(Rect::new(0.0, 0.0, 100.0, 100.0));

        let selected_items: Vec<Ref<ListBoxItem>> = panel
            .children()
            .snapshot()
            .iter()
            .filter_map(|child| child.cast::<ListBoxItem>())
            .filter(|item| item.is_selected())
            .collect();
        assert_eq!(1, selected_items.len());
        let selected_item = &selected_items[0];
        let tag = selected_item.tag().expect("the item has a tag");
        let tag: &dyn AnyValue = &*tag;
        assert_eq!(Some(&2), tag.downcast_ref::<i32>());
        assert_eq!(50.0, selected_item.bounds().center().y);
    }
}

#[test]
fn set_initial_focus_should_focus_day_selector_for_day_first_locale() {
    let _culture = CultureScope::new();

    let _app = start(focus_services());
    // en-GB uses dd/MM/yyyy: the day appears first in the short date pattern
    TestCultureDataProvider::register();
    CultureInfo::set_current_culture(CultureInfo::get_culture_info("en-GB"));
    assert_eq!("dd/MM/yyyy", CultureInfo::current_culture().date_time_format().short_date_pattern());

    let presenter = DatePickerPresenter::new();
    presenter.set_template(Some(create_picker_template()));
    let root = TestRoot::with_child(presenter.clone());
    root.layout_manager().execute_initial_layout_pass();

    // Trigger the initialization of the picker again now that the visual
    // tree is fully connected, so that the initial focus can be set.
    presenter.set_date(date(2024, 6, 15));

    let day_selector = presenter
        .get_visual_descendants()
        .filter_map(|visual| visual.cast::<DateTimePickerPanel>())
        .find(|panel| panel.name().as_deref() == Some("PART_DaySelector"))
        .expect("the day selector");

    let focused = root.focus_manager().get_focused_element().expect("an element has the focus");
    assert!(focused.ptr_eq(&day_selector));
}

#[test]
fn vertical_content_alignment_round_trips() {
    let _app = start(services());
    for value in
        [VerticalAlignment::Top, VerticalAlignment::Center, VerticalAlignment::Bottom, VerticalAlignment::Stretch]
    {
        let date_picker = DatePicker::new();
        date_picker.set_vertical_content_alignment(value);
        assert_eq!(value, date_picker.vertical_content_alignment());
    }
}

#[test]
fn vertical_content_alignment_default_is_stretch() {
    let _app = start(services());
    let date_picker = DatePicker::new();
    assert_eq!(VerticalAlignment::Stretch, date_picker.vertical_content_alignment());
}

/// Not in the reference suite: the picker and its presenter apply the
/// templates of the tests without a templated parent of another kind.
#[test]
fn presenter_template_applies() {
    let _app = start(services());
    let presenter = DatePickerPresenter::new();
    presenter.set_template(Some(create_picker_template()));
    presenter.apply_template();
    let presenter: &TemplatedControl = &presenter;
    assert_eq!(1, presenter.visual_children().count());
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

/// Not in the reference suite: inside a scroll viewer the panel scrolls
/// logically, as the selectors of the reference theme do. The scroll viewer
/// shows the extent and the offset of the panel, an offset set through the
/// scroll viewer selects the value at that offset, and the keys scroll by
/// items.
#[test]
fn selector_scrolls_logically_inside_a_scroll_viewer() {
    let _app = start(services());

    let panel = DateTimePickerPanel::new();
    panel.set_panel_type(DateTimePickerPanelType::Month);
    panel.set_item_format("%M");
    panel.set_minimum_value(1);
    panel.set_maximum_value(12);
    panel.set_should_loop(true);

    let scroll_viewer = ScrollViewer::new();
    scroll_viewer.set_template(Some(scroll_viewer_template()));
    scroll_viewer.set_horizontal_scroll_bar_visibility(ScrollBarVisibility::Disabled);
    scroll_viewer.set_vertical_scroll_bar_visibility(ScrollBarVisibility::Hidden);
    scroll_viewer.set_content(Some(Control::boxed(panel.clone())));
    scroll_viewer.set_width(100.0);
    scroll_viewer.set_height(200.0);

    let root = TestRoot::with_child(scroll_viewer.clone());
    root.layout_manager().execute_initial_layout_pass();

    // The presenters select the value once the template is laid out.
    panel.set_selected_value(3);
    root.layout_manager().execute_layout_pass();

    // Twelve items of 40, a hundred times; the selection is in the middle set.
    assert_eq!(Size::new(0.0, 48000.0), panel.extent());
    assert_eq!(panel.extent(), scroll_viewer.extent());
    assert_eq!(24000.0 + 2.0 * 40.0, panel.offset().y);
    assert_eq!(panel.offset(), scroll_viewer.offset());
    assert_eq!(panel.bounds().size(), scroll_viewer.viewport());

    let selected = |panel: &DateTimePickerPanel| -> Vec<String> {
        panel
            .children()
            .snapshot()
            .iter()
            .filter_map(|child| child.cast::<ListBoxItem>())
            .filter(|item| item.is_selected())
            .filter_map(|item| item.content().as_ref().and_then(crate::test_support::string_of))
            .collect()
    };
    assert_eq!(vec!["3".to_string()], selected(&panel));

    let offset = scroll_viewer.offset();
    scroll_viewer.set_offset(ferroui_base::Vector::new(offset.x, offset.y + 40.0));
    root.layout_manager().execute_layout_pass();
    assert_eq!(4, panel.selected_value());
    assert_eq!(vec!["4".to_string()], selected(&panel));
    assert_eq!(panel.offset(), scroll_viewer.offset());

    let key_down = |key: Key| {
        let mut args = KeyEventArgs::new();
        args.set_routed_event(Some(InputElement::key_down_event()));
        args.key = key;
        panel.raise_event(&args);
    };
    key_down(Key::PageDown);
    assert_eq!(8, panel.selected_value());
    key_down(Key::Up);
    assert_eq!(7, panel.selected_value());
    root.layout_manager().execute_layout_pass();
    assert_eq!(panel.offset(), scroll_viewer.offset());

    // Past the last value the selection wraps around.
    panel.scroll_down(6);
    assert_eq!(1, panel.selected_value());
}

/// Not in the reference suite. A value selected before the panel becomes
/// the child of a scroll content presenter is kept when the extent of the
/// panel was current at that time (the looping was set before the range),
/// and is replaced by the first value when the extent was stale (the
/// looping was set after the range: the presenter then coerces the offset
/// to the stale extent and gives it back to the panel). The expected values
/// are the ones the reference implementation gives for the same steps.
#[test]
fn selected_value_set_before_the_panel_is_attached_to_a_scroll_content_presenter() {
    // (looping set first, selected value, offset and extent before the
    // attachment, after it and after the layout)
    let rows = [
        (false, [(3, 24080.0, 480.0), (1, 480.0, 480.0), (1, 24000.0, 48000.0)]),
        (true, [(3, 24080.0, 48000.0), (3, 24080.0, 48000.0), (3, 24080.0, 48000.0)]),
    ];

    for (loop_first, expected) in rows {
        let _app = start(services());

        let panel = DateTimePickerPanel::new();
        if loop_first {
            panel.set_should_loop(true);
        }
        panel.set_minimum_value(1);
        panel.set_maximum_value(12);
        if !loop_first {
            panel.set_should_loop(true);
        }
        panel.set_selected_value(3);

        let state = |panel: &DateTimePickerPanel| (panel.selected_value(), panel.offset().y, panel.extent().height);
        assert_eq!(expected[0], state(&panel), "before, looping first: {loop_first}");

        let presenter = ScrollContentPresenter::new();
        presenter.set_content(Some(Control::boxed(panel.clone())));
        presenter.update_child();
        assert_eq!(expected[1], state(&panel), "attached, looping first: {loop_first}");
        assert_eq!(panel.offset(), presenter.offset());

        presenter.measure(Size::new(100.0, 200.0));
        presenter.arrange(Rect::new(0.0, 0.0, 100.0, 200.0));
        assert_eq!(expected[2], state(&panel), "laid out, looping first: {loop_first}");
        assert_eq!(panel.offset(), presenter.offset());
    }
}

/// Not in the reference suite. A panel that was never measured has no items:
/// scrolling it up moves an empty range to the start, which changes nothing,
/// while scrolling it down moves the empty range to the index before the
/// first item, which fails in the reference implementation (the range is
/// inserted at the index -1 of the children).
#[test]
fn scrolling_down_a_panel_without_items_fails_as_the_move_of_the_children_does() {
    let _app = start(services());

    let panel = DateTimePickerPanel::new();
    panel.set_should_loop(true);
    panel.set_minimum_value(1);
    panel.set_maximum_value(3);
    assert_eq!(0, panel.children().count());
    assert_eq!((12000.0, 6000.0), (panel.extent().height, panel.offset().y));

    panel.scroll_up(1);
    assert_eq!(0, panel.children().count());
    assert_eq!((3, 5960.0), (panel.selected_value(), panel.offset().y));

    let panel = DateTimePickerPanel::new();
    panel.set_minimum_value(1);
    panel.set_maximum_value(3);
    assert_eq!(0, panel.children().count());
    assert_eq!(120.0, panel.extent().height);
    let payload = catch_unwind(AssertUnwindSafe(|| panel.scroll_down(1))).expect_err("the move fails");
    let message = payload.downcast_ref::<&str>().map(|message| message.to_string()).or_else(|| payload.downcast_ref::<String>().cloned());
    assert!(message.is_some_and(|message| message.starts_with("Index was out of range.")));
    // The offset is set before the children are moved.
    assert_eq!(40.0, panel.offset().y);
    assert_eq!(1, panel.selected_value());
}

/// The picker template of the window tests: the hosts are in the picker
/// container and hold the selectors inside scroll viewers, and there is a
/// dismiss button next to the accept button.
fn create_window_picker_template() -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::new(|_, scope| {
        let picker_container = named(Grid::new(), "PART_PickerContainer").register_in_name_scope(&**scope);

        for (host_name, selector_name, panel_type, should_loop) in [
            ("PART_MonthHost", "PART_MonthSelector", DateTimePickerPanelType::Month, true),
            ("PART_DayHost", "PART_DaySelector", DateTimePickerPanelType::Day, true),
            ("PART_YearHost", "PART_YearSelector", DateTimePickerPanelType::Year, false),
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
            picker_container.children().add(host);
        }

        picker_container.children().add(named(Rectangle::new(), "PART_FirstSpacer").register_in_name_scope(&**scope));
        picker_container.children().add(named(Rectangle::new(), "PART_SecondSpacer").register_in_name_scope(&**scope));

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
    date_picker: Ref<DatePicker>,
    flyout_button: Ref<Button>,
    popup: Ref<Popup>,
    presenter: Ref<DatePickerPresenter>,
}

impl WindowTarget {
    /// Shows a window with a date picker whose selected date is `selected`.
    fn show(selected: DateTimeOffset) -> Self {
        let date_picker = DatePicker::new();
        date_picker.set_template(Some(create_template_with(true)));
        date_picker.set_selected_date(Some(selected));

        let window = Window::new();
        window.set_content(Some(Control::boxed(date_picker.clone())));
        window.show();
        window.layout_manager().execute_initial_layout_pass();

        let find = |name: &str| {
            date_picker
                .get_visual_descendants()
                .filter_map(|visual| visual.cast::<Control>())
                .find(|control| control.name().as_deref() == Some(name))
                .unwrap_or_else(|| panic!("no {name}"))
        };
        let flyout_button = find("PART_FlyoutButton").cast::<Button>().expect("a button");
        let popup = find("PART_Popup").cast::<Popup>().expect("a popup");
        let presenter =
            popup.child().and_then(|child| child.cast::<DatePickerPresenter>()).expect("the child is the presenter");
        Self { _window: window, date_picker, flyout_button, popup, presenter }
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
/// presenter in a popup host with the selected date; selecting another
/// month and accepting sets the selected date and closes the popup.
#[test]
fn window_popup_accept_updates_selected_date_and_closes() {
    let _app = start(TestServices::styled_window());
    let selected = date(2000, 10, 10);
    let target = WindowTarget::show(selected);
    let changes = Rc::new(Cell::new(0));
    let counter = changes.clone();
    target.date_picker.selected_date_changed(move |_| counter.set(counter.get() + 1));

    assert!(!target.popup.is_open());
    assert!(target.popup.host().is_none());

    target.flyout_button.perform_click();
    Dispatcher::ui_thread().run_jobs(None);

    assert!(target.popup.is_open());
    assert!(target.popup.host().is_some());
    assert_eq!(selected, target.presenter.date());

    // The presenter is laid out in the popup: the selectors show the date.
    let month_selector = target.part::<DateTimePickerPanel>("PART_MonthSelector");
    let day_selector = target.part::<DateTimePickerPanel>("PART_DaySelector");
    let year_selector = target.part::<DateTimePickerPanel>("PART_YearSelector");
    assert!(month_selector.is_measure_valid());
    assert!(month_selector.children().count() > 0);
    assert_eq!((10, 10, 2000), (month_selector.selected_value(), day_selector.selected_value(), year_selector.selected_value()));
    assert_eq!(31, day_selector.maximum_value());

    month_selector.scroll_down(1);
    assert_eq!(11, month_selector.selected_value());
    // November has 30 days.
    assert_eq!(30, day_selector.maximum_value());
    // Nothing changes before the selection is accepted.
    assert_eq!(Some(selected), target.date_picker.selected_date());
    assert_eq!(selected, target.presenter.date());

    target.part::<Button>("PART_AcceptButton").perform_click();
    Dispatcher::ui_thread().run_jobs(None);

    assert_eq!(Some(date(2000, 11, 10)), target.date_picker.selected_date());
    assert_eq!(date(2000, 11, 10), target.presenter.date());
    assert_eq!(1, changes.get());
    assert!(!target.popup.is_open());
    assert!(target.popup.host().is_none());
}

/// Additional test (not a port): in a window, dismissing the presenter
/// after selecting other values leaves the selected date unchanged and
/// closes the popup.
#[test]
fn window_popup_dismiss_leaves_selected_date_unchanged() {
    let _app = start(TestServices::styled_window());
    let selected = date(2000, 10, 10);
    let target = WindowTarget::show(selected);
    let changes = Rc::new(Cell::new(0));
    let counter = changes.clone();
    target.date_picker.selected_date_changed(move |_| counter.set(counter.get() + 1));

    target.flyout_button.perform_click();
    Dispatcher::ui_thread().run_jobs(None);
    assert!(target.popup.is_open());

    let day_selector = target.part::<DateTimePickerPanel>("PART_DaySelector");
    let year_selector = target.part::<DateTimePickerPanel>("PART_YearSelector");
    day_selector.scroll_up(1);
    year_selector.scroll_down(1);
    assert_eq!((9, 2001), (day_selector.selected_value(), year_selector.selected_value()));

    target.part::<Button>("PART_DismissButton").perform_click();
    Dispatcher::ui_thread().run_jobs(None);

    assert_eq!(Some(selected), target.date_picker.selected_date());
    assert_eq!(0, changes.get());
    assert!(!target.popup.is_open());
    assert!(target.popup.host().is_none());

    target.flyout_button.perform_click();
    Dispatcher::ui_thread().run_jobs(None);
    assert!(target.popup.is_open());
    assert_eq!(selected, target.presenter.date());
    // The date of the presenter did not change, so the selectors are not
    // initialized again and still show the dismissed selection: the picker
    // only re-initializes on a change of the date or of a format, as the
    // reference code does (read from the code, not run against it).
    assert_eq!((9, 2001), (day_selector.selected_value(), year_selector.selected_value()));
}
