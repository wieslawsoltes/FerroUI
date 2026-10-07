// (c) Copyright Microsoft Corporation.
// This source is subject to the Microsoft Public License (Ms-PL).
// Please see https://go.microsoft.com/fwlink/?LinkID=131993 for details.
// All other rights reserved.
//
// Ported from the Silverlight Toolkit sources as adapted by the upstream
// project; the license text is in the `NOTICE.md` of this crate.

use super::{
    Calendar, CalendarButton, CalendarDayButton, CalendarExtensions, CalendarMode, CalendarSelectionMode,
    DateTimeHelper,
};
use crate::primitives::{TemplateAppliedEventArgs, TemplatedControl, TemplatedControlImpl};
use crate::templates::ITemplateOf;
use crate::{Button, ContentControl, Control, ControlImpl, Grid};
use ferroui_base::data::BindingMode;
use ferroui_base::input::{InputElement, InputElementImpl, PointerPressedEventArgs, PointerReleasedEventArgs};
use ferroui_base::interactivity::{InteractiveImpl, RoutedEventHandlerToken};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::IBrush;
use ferroui_base::utilities::{DateTime, DayOfWeek, GregorianCalendar};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate, BoxedValue, FerroObject,
    FerroObjectImpl, FerroProperty, Ref, StyledElementImpl, StyledProperty, StyledPropertyOptions, VisualImpl,
    VisualImplExt, VisualTreeAttachmentEventArgs, WeakRef,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The number of days per week.
const NUMBER_OF_DAYS_PER_WEEK: i32 = 7;

const PART_ELEMENT_HEADER_BUTTON: &str = "PART_HeaderButton";
const PART_ELEMENT_PREVIOUS_BUTTON: &str = "PART_PreviousButton";
const PART_ELEMENT_NEXT_BUTTON: &str = "PART_NextButton";
const PART_ELEMENT_MONTH_VIEW: &str = "PART_MonthView";
const PART_ELEMENT_YEAR_VIEW: &str = "PART_YearView";
const PART_ELEMENT_WEEK_NUMBER_LABELS: &str = "PART_ElementWeekNumberLabels";

/// A button part and the handler attached to its click event.
type ButtonPart = (Ref<Button>, RoutedEventHandlerToken);

fn boxed_text(text: impl Into<String>) -> Option<BoxedValue> {
    Some(Rc::new(text.into()) as BoxedValue)
}

fn boxed_date(date: DateTime) -> Option<BoxedValue> {
    Some(Rc::new(date) as BoxedValue)
}

/// The date a control has as its data context, if it has one.
pub(crate) fn date_of(control: &Control) -> Option<DateTime> {
    let data_context = control.data_context()?;
    data_context
        .downcast_ref::<DateTime>()
        .copied()
        .or_else(|| data_context.downcast_ref::<Option<DateTime>>().copied().flatten())
}

/// Represents the currently displayed month or year on a [`Calendar`].
#[repr(C)]
pub struct CalendarItem {
    base: TemplatedControl,

    header_button: RefCell<Option<ButtonPart>>,
    next_button: RefCell<Option<ButtonPart>>,
    previous_button: RefCell<Option<ButtonPart>>,

    current_month: Cell<DateTime>,
    is_mouse_left_button_down: Cell<bool>,
    is_mouse_left_button_down_year_view: Cell<bool>,
    is_control_pressed: Cell<bool>,

    calendar: GregorianCalendar,

    owner: RefCell<Option<WeakRef<Calendar>>>,

    /// The Grid that hosts the content when in month mode.
    month_view: RefCell<Option<Ref<Grid>>>,
    /// The Grid that hosts the content when in year or decade mode.
    year_view: RefCell<Option<Ref<Grid>>>,
    /// The Grid that hosts the week number labels when in month mode.
    week_number_labels: RefCell<Option<Ref<Grid>>>,
}

ferro_class!(CalendarItem: TemplatedControl);

ferro_class_info!(CalendarItem {
    new: CalendarItem::new,
    markup: {
        attributes: [
            TemplatePart("PART_HeaderButton", type(Ref<Button>)),
            TemplatePart("PART_MonthView", type(Ref<Grid>)),
            TemplatePart("PART_NextButton", type(Ref<Button>)),
            TemplatePart("PART_PreviousButton", type(Ref<Button>)),
            TemplatePart("PART_YearView", type(Ref<Grid>)),
            TemplatePart("PART_ElementWeekNumberLabels", type(Ref<Grid>)),
            PseudoClasses(":calendardisabled", ":hasweeknumbers"),
        ],
    },
});

ferro_impl_classes!(
    CalendarItem: FerroObjectImpl,
    StyledElementImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);

impl VisualImpl for CalendarItem {
    fn on_detached_from_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_detached_from_visual_tree(this, e);

        // Reset mouse button tracking state. When the calendar popup closes
        // (e.g. due to a programmatic window change during date selection),
        // the pointer released event never fires, leaving these flags
        // stuck.
        this.is_mouse_left_button_down.set(false);
        this.is_mouse_left_button_down_year_view.set(false);
    }
}

impl TemplatedControlImpl for CalendarItem {
    /// Builds the visual tree for the [`CalendarItem`] when a new template
    /// is applied.
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        this.set_header_button(e.name_scope().find_as::<Button>(PART_ELEMENT_HEADER_BUTTON));
        this.set_previous_button(e.name_scope().find_as::<Button>(PART_ELEMENT_PREVIOUS_BUTTON));
        this.set_next_button(e.name_scope().find_as::<Button>(PART_ELEMENT_NEXT_BUTTON));
        *this.month_view.borrow_mut() = e.name_scope().find_as::<Grid>(PART_ELEMENT_MONTH_VIEW);
        *this.year_view.borrow_mut() = e.name_scope().find_as::<Grid>(PART_ELEMENT_YEAR_VIEW);
        *this.week_number_labels.borrow_mut() = e.name_scope().find_as::<Grid>(PART_ELEMENT_WEEK_NUMBER_LABELS);

        let owner = this.owner();

        if let Some(owner) = &owner {
            this.update_disabled(owner.is_enabled());
        }

        this.populate_grids();

        if let (Some(month_view), Some(year_view)) = (this.month_view(), this.year_view()) {
            if let Some(owner) = &owner {
                owner.set_selected_month(owner.display_date_internal());
                owner.set_selected_year(owner.display_date_internal());

                if owner.display_mode() == CalendarMode::Year {
                    this.update_year_mode();
                } else if owner.display_mode() == CalendarMode::Decade {
                    this.update_decade_mode();
                }

                if owner.display_mode() == CalendarMode::Month {
                    this.update_month_mode();
                    month_view.set_is_visible(true);
                    year_view.set_is_visible(false);
                } else {
                    year_view.set_is_visible(true);
                    month_view.set_is_visible(false);
                }
            } else {
                this.update_month_mode();
                month_view.set_is_visible(true);
                year_view.set_is_visible(false);
            }
        }
    }
}

ferro_properties! {
    impl CalendarItem {
        /// Defines the `HeaderBackground` property.
        pub fn header_background_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
            Calendar::header_background_property().add_owner::<CalendarItem>()
        }

        /// Defines the `DayTitleTemplate` property.
        pub fn day_title_template_property() -> StyledProperty<Option<Rc<dyn ITemplateOf<Option<Ref<Control>>>>>> {
            FerroProperty::register_with::<CalendarItem, _>(
                "DayTitleTemplate",
                StyledPropertyOptions::new(None).default_binding_mode(BindingMode::OneTime),
            )
        }
    }
}

impl CalendarItem {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: TemplatedControl::construct(),
            header_button: RefCell::new(None),
            next_button: RefCell::new(None),
            previous_button: RefCell::new(None),
            current_month: Cell::new(DateTime::MIN_VALUE),
            is_mouse_left_button_down: Cell::new(false),
            is_mouse_left_button_down_year_view: Cell::new(false),
            is_control_pressed: Cell::new(false),
            calendar: GregorianCalendar::new(),
            owner: RefCell::new(None),
            month_view: RefCell::new(None),
            year_view: RefCell::new(None),
            week_number_labels: RefCell::new(None),
        }
    }

    /// Initializes a new instance of the [`CalendarItem`] class.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The calendar this item displays a month or year of.
    pub(crate) fn owner(&self) -> Option<Ref<Calendar>> {
        self.owner.borrow().as_ref().and_then(WeakRef::upgrade)
    }

    pub(crate) fn set_owner(&self, value: Option<&Ref<Calendar>>) {
        *self.owner.borrow_mut() = value.map(Ref::downgrade);
    }

    /// The brush of the background of the header.
    pub fn header_background(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::header_background_property())
    }

    pub fn set_header_background(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::header_background_property(), value)
    }

    /// The template of the titles of the days of the week.
    pub fn day_title_template(&self) -> Option<Rc<dyn ITemplateOf<Option<Ref<Control>>>>> {
        self.get_value(Self::day_title_template_property())
    }

    pub fn set_day_title_template(&self, value: Option<Rc<dyn ITemplateOf<Option<Ref<Control>>>>>) {
        self.set_value(Self::day_title_template_property(), value)
    }

    /// Attaches a handler of this item to the click event of a button
    /// part; the handler receives the button.
    fn attach_click(&self, button: &Ref<Button>, handler: fn(&CalendarItem, &Button)) -> RoutedEventHandlerToken {
        let weak = self.to_ref().downgrade();
        let weak_button = button.downgrade();
        button.click(move |_, _| {
            if let (Some(this), Some(button)) = (weak.upgrade(), weak_button.upgrade()) {
                handler(&this, &button);
            }
        })
    }

    /// Gets the button that allows switching between month mode, year mode,
    /// and decade mode.
    pub(crate) fn header_button(&self) -> Option<Ref<Button>> {
        self.header_button.borrow().as_ref().map(|(button, _)| button.clone())
    }

    fn set_header_button(&self, value: Option<Ref<Button>>) {
        if let Some((button, token)) = self.header_button.take() {
            button.remove_handler(Button::click_event(), token);
        }

        if let Some(button) = value {
            let token = self.attach_click(&button, Self::header_button_click);
            *self.header_button.borrow_mut() = Some((button.clone(), token));
            button.set_focusable(false);
        }
    }

    /// Gets the button that displays the next page of the calendar when it
    /// is clicked.
    pub(crate) fn next_button(&self) -> Option<Ref<Button>> {
        self.next_button.borrow().as_ref().map(|(button, _)| button.clone())
    }

    fn set_next_button(&self, value: Option<Ref<Button>>) {
        if let Some((button, token)) = self.next_button.take() {
            button.remove_handler(Button::click_event(), token);
        }

        if let Some(button) = value {
            // If the user does not provide a Content value in template,
            // we provide a helper text that can be used in
            // Accessibility this text is not shown on the UI, just used
            // for Accessibility purposes
            if button.content().is_none() {
                button.set_content(boxed_text("next button"));
            }

            button.set_is_visible(true);
            let token = self.attach_click(&button, Self::next_button_click);
            *self.next_button.borrow_mut() = Some((button.clone(), token));
            button.set_focusable(false);
        }
    }

    /// Gets the button that displays the previous page of the calendar when
    /// it is clicked.
    pub(crate) fn previous_button(&self) -> Option<Ref<Button>> {
        self.previous_button.borrow().as_ref().map(|(button, _)| button.clone())
    }

    fn set_previous_button(&self, value: Option<Ref<Button>>) {
        if let Some((button, token)) = self.previous_button.take() {
            button.remove_handler(Button::click_event(), token);
        }

        if let Some(button) = value {
            // If the user does not provide a Content value in template,
            // we provide a helper text that can be used in
            // Accessibility this text is not shown on the UI, just used
            // for Accessibility purposes
            if button.content().is_none() {
                button.set_content(boxed_text("previous button"));
            }

            button.set_is_visible(true);
            let token = self.attach_click(&button, Self::previous_button_click);
            *self.previous_button.borrow_mut() = Some((button.clone(), token));
            button.set_focusable(false);
        }
    }

    /// Gets the Grid that hosts the content when in month mode.
    pub(crate) fn month_view(&self) -> Option<Ref<Grid>> {
        self.month_view.borrow().clone()
    }

    pub(crate) fn set_month_view(&self, value: Option<Ref<Grid>>) {
        *self.month_view.borrow_mut() = value;
    }

    /// Gets the Grid that hosts the content when in year or decade mode.
    pub(crate) fn year_view(&self) -> Option<Ref<Grid>> {
        self.year_view.borrow().clone()
    }

    /// Gets the Grid that hosts the week number labels when in month mode.
    fn week_number_labels(&self) -> Option<Ref<Grid>> {
        self.week_number_labels.borrow().clone()
    }

    fn populate_grids(&self) {
        let owner = self.owner();

        if let Some(month_view) = self.month_view() {
            let child_count = Calendar::ROWS_PER_MONTH + Calendar::ROWS_PER_MONTH * Calendar::COLUMNS_PER_MONTH;
            let mut children: Vec<Ref<Control>> = Vec::with_capacity(child_count as usize);

            let day_title_template = self.day_title_template();
            for i in 0..Calendar::COLUMNS_PER_MONTH {
                if let Some(cell) = day_title_template.as_ref().and_then(|template| template.build_typed()) {
                    cell.set_data_context(boxed_text(""));
                    cell.set_value(Grid::row_property(), 0);
                    cell.set_value(Grid::column_property(), i);
                    children.push(cell);
                }
            }

            for i in 1..Calendar::ROWS_PER_MONTH {
                for j in 0..Calendar::COLUMNS_PER_MONTH {
                    let cell = CalendarDayButton::new();

                    if let Some(owner) = &owner {
                        cell.set_owner(Some(owner));
                    }
                    cell.set_value(Grid::row_property(), i);
                    cell.set_value(Grid::column_property(), j);

                    // The cells keep these subscriptions for as long as
                    // they live.
                    let weak = self.to_ref().downgrade();
                    let _ = cell.calendar_day_button_mouse_down(move |sender, e| {
                        if let Some(this) = weak.upgrade() {
                            this.cell_mouse_left_button_down(sender, e);
                        }
                    });
                    let weak = self.to_ref().downgrade();
                    let _ = cell.calendar_day_button_mouse_up(move |sender, e| {
                        if let Some(this) = weak.upgrade() {
                            this.cell_mouse_left_button_up(sender, e);
                        }
                    });
                    let weak = self.to_ref().downgrade();
                    let weak_cell = cell.downgrade();
                    cell.add_handler(InputElement::pointer_entered_event(), move |_, _| {
                        if let (Some(this), Some(cell)) = (weak.upgrade(), weak_cell.upgrade()) {
                            this.cell_mouse_entered(&cell);
                        }
                    });
                    let weak = self.to_ref().downgrade();
                    let weak_cell = cell.downgrade();
                    cell.click(move |_, _| {
                        if let Some(this) = weak.upgrade() {
                            this.cell_click(weak_cell.upgrade().as_deref());
                        }
                    });
                    children.push(cell.upcast());
                }
            }

            month_view.children().add_range(children);
        }

        if let Some(week_number_labels) = self.week_number_labels() {
            let mut children: Vec<Ref<Control>> = Vec::with_capacity(Calendar::ROWS_PER_MONTH as usize);

            for i in 1..Calendar::ROWS_PER_MONTH {
                let cell = ContentControl::new();
                cell.set_value(Grid::row_property(), i);
                cell.set_templated_parent(self.to_ref().upcast::<FerroObject>());
                children.push(cell.upcast());
            }

            week_number_labels.children().add_range(children);
        }

        if let Some(year_view) = self.year_view() {
            let child_count = Calendar::ROWS_PER_YEAR * Calendar::COLUMNS_PER_YEAR;
            let mut children: Vec<Ref<Control>> = Vec::with_capacity(child_count as usize);

            for i in 0..Calendar::ROWS_PER_YEAR {
                for j in 0..Calendar::COLUMNS_PER_YEAR {
                    let month = CalendarButton::new();

                    if let Some(owner) = &owner {
                        month.set_owner(Some(owner));
                    }
                    month.set_value(Grid::row_property(), i);
                    month.set_value(Grid::column_property(), j);

                    let weak = self.to_ref().downgrade();
                    let _ = month.calendar_left_mouse_button_down(move |sender, _| {
                        if let Some(this) = weak.upgrade() {
                            this.month_calendar_button_mouse_down(Some(sender));
                        }
                    });
                    let weak = self.to_ref().downgrade();
                    let _ = month.calendar_left_mouse_button_up(move |sender, _| {
                        if let Some(this) = weak.upgrade() {
                            this.month_calendar_button_mouse_up(Some(sender));
                        }
                    });
                    let weak = self.to_ref().downgrade();
                    let weak_month = month.downgrade();
                    month.add_handler(InputElement::pointer_entered_event(), move |_, _| {
                        if let Some(this) = weak.upgrade() {
                            this.month_mouse_entered(weak_month.upgrade().as_deref());
                        }
                    });
                    children.push(month.upcast());
                }
            }

            year_view.children().add_range(children);
        }
    }

    fn set_day_titles(&self) {
        let month_view = self.month_view().expect("the month view is present");
        let date_format = DateTimeHelper::get_current_date_format();
        let first_day_of_week = match self.owner() {
            Some(owner) => owner.first_day_of_week(),
            None => date_format.first_day_of_week(),
        };

        for child_index in 0..Calendar::COLUMNS_PER_MONTH {
            let day_title = month_view.children().get(child_index as usize);
            let name = &date_format.shortest_day_names()
                [((child_index + first_day_of_week as i32) % NUMBER_OF_DAYS_PER_WEEK) as usize];
            day_title.set_data_context(boxed_text(name.as_str()));
        }
    }

    /// How many days of the previous month need to be displayed.
    fn previous_month_days(&self, first_of_month: DateTime) -> i32 {
        let day: DayOfWeek = self.calendar.get_day_of_week(first_of_month);
        let first_day_of_week = match self.owner() {
            Some(owner) => owner.first_day_of_week(),
            None => DateTimeHelper::get_current_date_format().first_day_of_week(),
        };

        let i = (day as i32 - first_day_of_week as i32 + NUMBER_OF_DAYS_PER_WEEK) % NUMBER_OF_DAYS_PER_WEEK;

        if i == 0 {
            NUMBER_OF_DAYS_PER_WEEK
        } else {
            i
        }
    }

    pub(crate) fn update_month_mode(&self) {
        match self.owner() {
            Some(owner) => self.current_month.set(owner.display_date_internal()),
            None => self.current_month.set(DateTime::today()),
        }

        self.set_month_mode_header_button();
        self.set_month_mode_previous_button(self.current_month.get());
        self.set_month_mode_next_button(self.current_month.get());

        if self.month_view().is_some() {
            self.set_day_titles();
            self.set_calendar_day_buttons(self.current_month.get());
            self.update_week_number_labels(self.current_month.get());
        }
    }

    fn set_month_mode_header_button(&self) {
        if let Some(header_button) = self.header_button() {
            let date_format = DateTimeHelper::get_current_date_format();
            if let Some(owner) = self.owner() {
                header_button.set_content(boxed_text(owner.display_date_internal().to_string_format("Y", &date_format)));
                header_button.set_is_enabled(true);
            } else {
                header_button.set_content(boxed_text(DateTime::today().to_string_format("Y", &date_format)));
            }
        }
    }

    fn set_month_mode_next_button(&self, first_day_of_month: DateTime) {
        if let (Some(owner), Some(next_button)) = (self.owner(), self.next_button()) {
            // DisplayDate is equal to DateTime.MaxValue
            if DateTimeHelper::compare_year_month(first_day_of_month, DateTime::MAX_VALUE) == 0 {
                next_button.set_is_enabled(false);
            } else {
                // Since we are sure DisplayDate is not equal to
                // DateTime.MaxValue, it is safe to use AddMonths
                let first_day_of_next_month = self.calendar.add_months(first_day_of_month, 1);
                next_button.set_is_enabled(
                    DateTimeHelper::compare_days(owner.display_date_range_end(), first_day_of_next_month) > -1,
                );
            }
        }
    }

    fn set_month_mode_previous_button(&self, first_day_of_month: DateTime) {
        if let (Some(owner), Some(previous_button)) = (self.owner(), self.previous_button()) {
            previous_button
                .set_is_enabled(DateTimeHelper::compare_days(owner.display_date_range_start(), first_day_of_month) < 0);
        }
    }

    fn set_button_state(&self, child_button: &Ref<CalendarDayButton>, date_to_add: DateTime) {
        if let Some(owner) = self.owner() {
            child_button.set_opacity(1.0);

            // If the day is outside the DisplayDateStart/End boundary, do
            // not show it
            if DateTimeHelper::compare_days(date_to_add, owner.display_date_range_start()) < 0
                || DateTimeHelper::compare_days(date_to_add, owner.display_date_range_end()) > 0
            {
                child_button.set_is_enabled(false);
                child_button.set_is_today(false);
                child_button.set_is_selected(false);
                child_button.set_opacity(0.0);
            } else {
                // SET IF THE DAY IS SELECTABLE OR NOT
                child_button.set_is_blackout(owner.blackout_dates().contains_date(date_to_add));
                child_button.set_is_enabled(true);

                // SET IF THE DAY IS INACTIVE OR NOT: set if the day is a
                // trailing day or not
                child_button.set_is_inactive(
                    DateTimeHelper::compare_year_month(date_to_add, owner.display_date_internal()) != 0,
                );

                // SET IF THE DAY IS TODAY OR NOT
                child_button.set_is_today(owner.is_today_highlighted() && date_to_add == DateTime::today());

                // SET IF THE DAY IS SELECTED OR NOT
                child_button.set_is_selected(false);
                for item in owner.selected_dates().to_vec() {
                    // Since we should be comparing the Date values not
                    // DateTime values, we can't use
                    // Owner.SelectedDates.Contains(dateToAdd) directly
                    child_button
                        .set_is_selected(child_button.is_selected() | (DateTimeHelper::compare_days(date_to_add, item) == 0));
                }

                // SET THE FOCUS ELEMENT
                if let Some(last_selected_date) = owner.last_selected_date() {
                    if DateTimeHelper::compare_days(last_selected_date, date_to_add) == 0 {
                        if let Some(focus_button) = owner.focus_button() {
                            focus_button.set_is_current(false);
                        }
                        owner.set_focus_button(Some(child_button.clone()));
                        if owner.has_focus_internal() {
                            child_button.set_is_current(true);
                        }
                    } else {
                        child_button.set_is_current(false);
                    }
                }
            }
        }
    }

    fn day_button_at(month_view: &Grid, index: i32) -> Ref<CalendarDayButton> {
        month_view
            .children()
            .get(index as usize)
            .cast::<CalendarDayButton>()
            .expect("the child of the month view is a CalendarDayButton")
    }

    fn set_calendar_day_buttons(&self, first_day_of_month: DateTime) {
        let month_view = self.month_view().expect("the month view is present");
        let owner = self.owner();
        let last_month_to_display = self.previous_month_days(first_day_of_month);
        let mut date_to_add;

        if DateTimeHelper::compare_year_month(first_day_of_month, DateTime::MIN_VALUE) > 0 {
            // DisplayDate is not equal to DateTime.MinValue we can subtract
            // days from the DisplayDate
            date_to_add = self.calendar.add_days(first_day_of_month, -last_month_to_display);
        } else {
            date_to_add = first_day_of_month;
        }

        if let Some(owner) = &owner {
            if owner.hover_end().is_some() && owner.hover_start().is_some() {
                owner.set_hover_end_index(None);
                owner.set_hover_start_index(None);
            }
        }

        let count = Calendar::ROWS_PER_MONTH * Calendar::COLUMNS_PER_MONTH;

        let mut child_index = Calendar::COLUMNS_PER_MONTH;
        while child_index < count {
            let child_button = Self::day_button_at(&month_view, child_index);

            child_button.set_index(child_index);
            self.set_button_state(&child_button, date_to_add);

            // Update the indexes of hoverStart and hoverEnd
            if let Some(owner) = &owner {
                if let (Some(hover_end), Some(hover_start)) = (owner.hover_end(), owner.hover_start()) {
                    if DateTimeHelper::compare_days(date_to_add, hover_end) == 0 {
                        owner.set_hover_end_index(Some(child_index));
                    }

                    if DateTimeHelper::compare_days(date_to_add, hover_start) == 0 {
                        owner.set_hover_start_index(Some(child_index));
                    }
                }
            }

            child_button.set_content(boxed_text(DateTimeHelper::format_number(date_to_add.day())));
            child_button.set_data_context(boxed_date(date_to_add));

            if DateTime::compare(DateTimeHelper::discard_time(DateTime::MAX_VALUE), date_to_add) > 0 {
                // Since we are sure DisplayDate is not equal to
                // DateTime.MaxValue, it is safe to use AddDays
                date_to_add = self.calendar.add_days(date_to_add, 1);
            } else {
                // DisplayDate is equal to the DateTime.MaxValue, so there
                // are no trailing days
                child_index += 1;
                for i in child_index..count {
                    let child_button = Self::day_button_at(&month_view, i);
                    // button needs a content to occupy the necessary space
                    // for the content presenter
                    child_button.set_content(boxed_text(DateTimeHelper::format_number(i)));
                    child_button.set_is_enabled(false);
                    child_button.set_opacity(0.0);
                }
                return;
            }

            child_index += 1;
        }

        // If the HoverStart or HoverEndInternal could not be found on the
        // DisplayMonth set the values of the HoverStartIndex or
        // HoverEndIndex to be the first or last day indexes on the current
        // month
        if let Some(owner) = &owner {
            if let (Some(hover_start), Some(hover_end_internal)) = (owner.hover_start(), owner.hover_end_internal()) {
                if owner.hover_end_index().is_none() {
                    if DateTimeHelper::compare_days(hover_end_internal, hover_start) > 0 {
                        owner.set_hover_end_index(Some(Calendar::COLUMNS_PER_MONTH * Calendar::ROWS_PER_MONTH - 1));
                    } else {
                        owner.set_hover_end_index(Some(Calendar::COLUMNS_PER_MONTH));
                    }
                }

                if owner.hover_start_index().is_none() {
                    if DateTimeHelper::compare_days(hover_end_internal, hover_start) > 0 {
                        owner.set_hover_start_index(Some(Calendar::COLUMNS_PER_MONTH));
                    } else {
                        owner.set_hover_start_index(Some(Calendar::COLUMNS_PER_MONTH * Calendar::ROWS_PER_MONTH - 1));
                    }
                }
            }
        }
    }

    /// Updates the week number labels if `Calendar::is_week_number_visible`
    /// is true and the display mode of the calendar is
    /// [`CalendarMode::Month`].
    fn update_week_number_labels(&self, first_day_of_month: DateTime) {
        let owner = self.owner();

        // first set the pseudo classes
        let show = owner.as_ref().is_some_and(|owner| owner.is_week_number_visible());
        self.pseudo_classes().set(":hasweeknumbers", show);

        // if we don't have a week number labels grid, or it has no children, then we don't need to update
        let Some(week_number_labels) = self.week_number_labels() else {
            return;
        };
        if week_number_labels.children().count() == 0 {
            return;
        }

        self.update_week_number_labels_visibility();

        let last_month_to_display = self.previous_month_days(first_day_of_month);
        let first_date_displayed = if DateTimeHelper::compare_year_month(first_day_of_month, DateTime::MIN_VALUE) > 0 {
            self.calendar.add_days(first_day_of_month, -last_month_to_display)
        } else {
            first_day_of_month
        };

        let date_format = DateTimeHelper::get_current_date_format();
        let rule = owner.as_ref().map_or_else(|| date_format.calendar_week_rule(), |owner| owner.week_number_rule());
        let first_day_of_week =
            owner.as_ref().map_or_else(|| date_format.first_day_of_week(), |owner| owner.first_day_of_week());

        // We have 6 rows with weeks. The control theme may have added more children to the Grid (Header, Background, ...).
        // We only want to update the last 6 children of the Grid, which are the week number labels.
        let count = week_number_labels.children().count() as i32;
        let start_index = count - (Calendar::ROWS_PER_MONTH - 1);
        for i in start_index..count {
            let days_to_add = (i - start_index) * NUMBER_OF_DAYS_PER_WEEK;
            let first_day_of_row = self.calendar.add_days(first_date_displayed, days_to_add);

            let label = week_number_labels.children().get(i as usize).cast::<ContentControl>();
            if let Some(label) = label {
                let week = DateTimeHelper::get_week_of_year(first_day_of_row, rule, first_day_of_week, &self.calendar);
                label.set_content(Some(Rc::new(week) as BoxedValue));
            }
        }
    }

    /// Updates the visibility of the week number labels based on the
    /// `IsWeekNumberVisible` property and the display mode of the calendar.
    pub(crate) fn update_week_number_labels_visibility(&self) {
        if let (Some(week_number_labels), Some(owner)) = (self.week_number_labels(), self.owner()) {
            week_number_labels
                .set_is_visible(owner.is_week_number_visible() && owner.display_mode() == CalendarMode::Month);
        }
    }

    pub(crate) fn update_year_mode(&self) {
        match self.owner() {
            Some(owner) => self.current_month.set(owner.selected_month()),
            None => self.current_month.set(DateTime::today()),
        }

        self.set_year_mode_header_button();
        self.set_year_mode_previous_button();
        self.set_year_mode_next_button();

        if self.year_view().is_some() {
            self.set_month_buttons_for_year_mode();
        }
    }

    fn set_year_mode_header_button(&self) {
        if let Some(header_button) = self.header_button() {
            header_button.set_is_enabled(true);
            header_button.set_content(boxed_text(DateTimeHelper::format_number(self.current_month.get().year())));
        }
    }

    fn set_year_mode_previous_button(&self) {
        if let (Some(owner), Some(previous_button)) = (self.owner(), self.previous_button()) {
            previous_button.set_is_enabled(owner.display_date_range_start().year() != self.current_month.get().year());
        }
    }

    fn set_year_mode_next_button(&self) {
        if let (Some(owner), Some(next_button)) = (self.owner(), self.next_button()) {
            next_button.set_is_enabled(owner.display_date_range_end().year() != self.current_month.get().year());
        }
    }

    fn calendar_button_of(child: &Ref<Control>) -> Ref<CalendarButton> {
        child.cast::<CalendarButton>().expect("the child of the year view is a CalendarButton")
    }

    fn set_month_buttons_for_year_mode(&self) {
        let year_view = self.year_view().expect("the year view is present");
        let owner = self.owner();
        let current_month = self.current_month.get();
        let date_format = DateTimeHelper::get_current_date_format();

        for (count, child) in year_view.children().snapshot().iter().enumerate() {
            let child_button = Self::calendar_button_of(child);
            // There should be no time component. Time is 12:00 AM
            let day = DateTime::new(current_month.year(), count as i32 + 1, 1);
            child_button.set_data_context(boxed_date(day));

            child_button.set_content(boxed_text(date_format.abbreviated_month_names()[count].as_str()));
            child_button.set_is_visible(true);

            if let Some(owner) = &owner {
                if day.year() == current_month.year()
                    && day.month() == current_month.month()
                    && day.day() == current_month.day()
                {
                    owner.set_focus_calendar_button(Some(child_button.clone()));
                    child_button.set_is_calendar_button_focused(owner.has_focus_internal());
                } else {
                    child_button.set_is_calendar_button_focused(false);
                }

                child_button
                    .set_is_selected(DateTimeHelper::compare_year_month(day, owner.display_date_internal()) == 0);

                if DateTimeHelper::compare_year_month(day, owner.display_date_range_start()) < 0
                    || DateTimeHelper::compare_year_month(day, owner.display_date_range_end()) > 0
                {
                    child_button.set_is_enabled(false);
                    child_button.set_opacity(0.0);
                } else {
                    child_button.set_is_enabled(true);
                    child_button.set_opacity(1.0);
                }
            }

            child_button.set_is_inactive(false);
        }
    }

    pub(crate) fn update_decade_mode(&self) {
        let selected_year;

        match self.owner() {
            Some(owner) => {
                selected_year = owner.selected_year();
                self.current_month.set(owner.selected_month());
            }
            None => {
                self.current_month.set(DateTime::today());
                selected_year = DateTime::today();
            }
        }

        let decade = DateTimeHelper::decade_of_date(selected_year);
        let decade_end = DateTimeHelper::end_of_decade(selected_year);

        self.set_decade_mode_header_button(decade, decade_end);
        self.set_decade_mode_previous_button(decade);
        self.set_decade_mode_next_button(decade_end);

        if self.year_view().is_some() {
            self.set_year_buttons(decade, decade_end);
        }
    }

    pub(crate) fn update_year_view_selection(&self, calendar_button: Option<&CalendarButton>) {
        let Some(owner) = self.owner() else {
            return;
        };
        let Some(calendar_button) = calendar_button else {
            return;
        };
        if let Some(selected_date) = date_of(calendar_button) {
            owner
                .focus_calendar_button()
                .expect("the calendar has a focused calendar button")
                .set_is_calendar_button_focused(false);
            owner.set_focus_calendar_button(Some(calendar_button.to_ref()));
            calendar_button.set_is_calendar_button_focused(owner.has_focus_internal());

            if owner.display_mode() == CalendarMode::Year {
                owner.set_selected_month(selected_date);
            } else {
                owner.set_selected_year(selected_date);
            }
        }
    }

    fn set_year_buttons(&self, decade: i32, decade_end: i32) {
        let year_view = self.year_view().expect("the year view is present");
        let owner = self.owner();

        for (count, child) in (-1..).zip(year_view.children().snapshot().iter()) {
            let child_button = Self::calendar_button_of(child);
            let year = decade + count;

            if year <= DateTime::MAX_VALUE.year() && year >= DateTime::MIN_VALUE.year() {
                // There should be no time component. Time is 12:00 AM
                let day = DateTime::new(year, 1, 1);
                child_button.set_data_context(boxed_date(day));
                child_button.set_content(boxed_text(DateTimeHelper::format_number(year)));
                child_button.set_is_visible(true);

                if let Some(owner) = &owner {
                    if year == owner.selected_year().year() {
                        owner.set_focus_calendar_button(Some(child_button.clone()));
                        child_button.set_is_calendar_button_focused(owner.has_focus_internal());
                    } else {
                        child_button.set_is_calendar_button_focused(false);
                    }
                    child_button.set_is_selected(owner.display_date().year() == year);

                    if year < owner.display_date_range_start().year() || year > owner.display_date_range_end().year() {
                        child_button.set_is_enabled(false);
                        child_button.set_opacity(0.0);
                    } else {
                        child_button.set_is_enabled(true);
                        child_button.set_opacity(1.0);
                    }
                }

                // SET IF THE YEAR IS INACTIVE OR NOT: set if the year is a
                // trailing year or not
                child_button.set_is_inactive(year < decade || year > decade_end);
            } else {
                child_button.set_is_enabled(false);
                child_button.set_opacity(0.0);
            }
        }
    }

    fn set_decade_mode_header_button(&self, decade: i32, decade_end: i32) {
        if let Some(header_button) = self.header_button() {
            header_button.set_content(boxed_text(format!(
                "{}-{}",
                DateTimeHelper::format_number(decade),
                DateTimeHelper::format_number(decade_end)
            )));
            header_button.set_is_enabled(false);
        }
    }

    fn set_decade_mode_next_button(&self, decade_end: i32) {
        if let (Some(owner), Some(next_button)) = (self.owner(), self.next_button()) {
            next_button.set_is_enabled(owner.display_date_range_end().year() > decade_end);
        }
    }

    fn set_decade_mode_previous_button(&self, decade: i32) {
        if let (Some(owner), Some(previous_button)) = (self.owner(), self.previous_button()) {
            previous_button.set_is_enabled(decade > owner.display_date_range_start().year());
        }
    }

    /// The handler of the click event of the header button; `sender` is
    /// the button.
    pub(crate) fn header_button_click(&self, sender: &Button) {
        if let Some(owner) = self.owner() {
            if !owner.has_focus_internal() {
                owner.focus();
            }

            if sender.is_enabled() {
                if owner.display_mode() == CalendarMode::Month {
                    let d = owner.display_date_internal();
                    owner.set_selected_month(DateTime::new(d.year(), d.month(), 1));
                    owner.set_display_mode(CalendarMode::Year);
                } else {
                    debug_assert!(
                        owner.display_mode() == CalendarMode::Year,
                        "The Owner Calendar's DisplayMode should be Year!"
                    );
                    let d = owner.selected_month();
                    owner.set_selected_year(DateTime::new(d.year(), d.month(), 1));
                    owner.set_display_mode(CalendarMode::Decade);
                }
            }
        }
    }

    /// The handler of the click event of the previous button; `sender` is
    /// the button.
    pub(crate) fn previous_button_click(&self, sender: &Button) {
        if let Some(owner) = self.owner() {
            if !owner.has_focus_internal() {
                owner.focus();
            }

            if sender.is_enabled() {
                owner.on_previous_click();
            }
        }
    }

    /// The handler of the click event of the next button; `sender` is the
    /// button.
    pub(crate) fn next_button_click(&self, sender: &Button) {
        if let Some(owner) = self.owner() {
            if !owner.has_focus_internal() {
                owner.focus();
            }

            if sender.is_enabled() {
                owner.on_next_click();
            }
        }
    }

    /// The handler of the pointer entered event of a day button; `sender`
    /// is the button.
    pub(crate) fn cell_mouse_entered(&self, sender: &CalendarDayButton) {
        if let Some(owner) = self.owner() {
            if !self.is_mouse_left_button_down.get() || !sender.is_enabled() || sender.is_blackout() {
                return;
            }
            let Some(selected_date) = date_of(sender) else {
                return;
            };

            // Update the states of all buttons to be selected starting
            // from HoverStart to b
            match owner.selection_mode() {
                CalendarSelectionMode::SingleDate => {
                    owner.set_calendar_date_picker_display_date_flag(true);
                    if owner.selected_dates().count() == 0 {
                        owner.selected_dates().add(selected_date);
                    } else {
                        owner.selected_dates().set(0, selected_date);
                    }
                }
                CalendarSelectionMode::SingleRange | CalendarSelectionMode::MultipleRange => {
                    owner.un_highlight_days();
                    owner.set_hover_end_index(Some(sender.index()));
                    owner.set_hover_end(Some(selected_date));
                    // Update the States of the buttons
                    owner.highlight_days();
                }
                CalendarSelectionMode::None => {}
            }
        }
    }

    /// The handler of the mouse down event of a day button; `sender` is
    /// the button.
    pub(crate) fn cell_mouse_left_button_down(&self, sender: &CalendarDayButton, e: &PointerPressedEventArgs) {
        if let Some(owner) = self.owner() {
            if !owner.has_focus_internal() {
                owner.focus();
            }

            let (ctrl, shift) = CalendarExtensions::get_meta_key_state(e.key_modifiers());

            // The sender is always a day button here, so the branch of the
            // reference for another kind of sender (which clears the
            // control state) has no counterpart.
            let b = sender;
            self.is_control_pressed.set(ctrl);
            let selected_date = date_of(b).filter(|_| b.is_enabled() && !b.is_blackout());
            if let Some(selected_date) = selected_date {
                self.is_mouse_left_button_down.set(true);

                match owner.selection_mode() {
                    CalendarSelectionMode::None => {}
                    CalendarSelectionMode::SingleDate => {
                        owner.set_calendar_date_picker_display_date_flag(true);
                        if owner.selected_dates().count() == 0 {
                            owner.selected_dates().add(selected_date);
                        } else {
                            owner.selected_dates().set(0, selected_date);
                        }
                    }
                    CalendarSelectionMode::SingleRange => {
                        // Set the start or end of the selection
                        // range
                        if shift {
                            owner.un_highlight_days();
                            owner.set_hover_end(Some(selected_date));
                            owner.set_hover_end_index(Some(b.index()));
                            owner.highlight_days();
                        } else {
                            owner.un_highlight_days();
                            owner.set_hover_start(Some(selected_date));
                            owner.set_hover_start_index(Some(b.index()));
                        }
                    }
                    CalendarSelectionMode::MultipleRange => {
                        if shift {
                            if !ctrl {
                                // clear the list, set the states to
                                // default
                                owner.add_removed_items(&owner.selected_dates().to_vec());
                                owner.selected_dates().clear_internal();
                            }
                            owner.set_hover_end(Some(selected_date));
                            owner.set_hover_end_index(Some(b.index()));
                            owner.highlight_days();
                        } else {
                            if !ctrl {
                                // clear the list, set the states to
                                // default
                                owner.add_removed_items(&owner.selected_dates().to_vec());
                                owner.selected_dates().clear_internal();
                                owner.un_highlight_days();
                            }
                            owner.set_hover_start(Some(selected_date));
                            owner.set_hover_start_index(Some(b.index()));
                        }
                    }
                }
            } else {
                // If a click occurs on a BlackOutDay we set the
                // HoverStart to be null
                owner.set_hover_start(None);
            }
        }
    }

    fn add_selection(&self, b: &CalendarDayButton, selected_date: DateTime) {
        if let Some(owner) = self.owner() {
            owner.set_hover_end_index(Some(b.index()));
            owner.set_hover_end(Some(selected_date));

            if let (Some(hover_end), Some(hover_start)) = (owner.hover_end(), owner.hover_start()) {
                // this is selection with Mouse, we do not guarantee the
                // range does not include BlackOutDates.  AddRange method
                // will throw away the BlackOutDates based on the
                // SelectionMode
                owner.set_is_mouse_selection(true);
                owner.selected_dates().add_range(hover_start, hover_end);
                owner.on_day_click(selected_date);
            }
        }
    }

    /// The handler of the mouse up event of a day button; `sender` is the
    /// button.
    pub(crate) fn cell_mouse_left_button_up(&self, sender: &CalendarDayButton, e: &PointerReleasedEventArgs) {
        if let Some(owner) = self.owner() {
            let b = sender;
            if !b.is_blackout() {
                owner.on_day_button_mouse_up(e);
            }
            self.is_mouse_left_button_down.set(false);
            if let Some(selected_date) = date_of(b) {
                if owner.selection_mode() == CalendarSelectionMode::None
                    || owner.selection_mode() == CalendarSelectionMode::SingleDate
                {
                    owner.on_day_click(selected_date);
                    return;
                }
                if owner.allow_tap_range_selection()
                    && (owner.selection_mode() == CalendarSelectionMode::SingleRange
                        || owner.selection_mode() == CalendarSelectionMode::MultipleRange)
                    && owner.process_tap_range_selection(selected_date)
                {
                    owner.on_day_click(selected_date);
                    return;
                }
                if owner.hover_start().is_some() {
                    match owner.selection_mode() {
                        CalendarSelectionMode::SingleRange => {
                            // Update SelectedDates
                            owner.add_removed_items(&owner.selected_dates().to_vec());
                            owner.selected_dates().clear_internal();
                            self.add_selection(b, selected_date);
                        }
                        CalendarSelectionMode::MultipleRange => {
                            // add the selection (either single day or
                            // SingleRange day)
                            self.add_selection(b, selected_date);
                        }
                        _ => {}
                    }
                } else {
                    // If the day is Disabled but a trailing day we should
                    // be able to switch months
                    if b.is_inactive() && b.is_blackout() {
                        owner.on_day_click(selected_date);
                    }
                }
            }
        }
    }

    /// The handler of the click event of a day button; `sender` is the
    /// button.
    fn cell_click(&self, sender: Option<&CalendarDayButton>) {
        if let Some(owner) = self.owner() {
            if self.is_control_pressed.get() && owner.selection_mode() == CalendarSelectionMode::MultipleRange {
                let b = sender.expect("the sender of the click is a CalendarDayButton");

                if b.is_selected() {
                    owner.set_hover_start(None);
                    self.is_mouse_left_button_down.set(false);
                    b.set_is_selected(false);
                    if let Some(selected_date) = date_of(b) {
                        owner.selected_dates().remove(selected_date);
                    }
                }
            }
        }
        self.is_control_pressed.set(false);
    }

    fn month_calendar_button_mouse_down(&self, sender: Option<&CalendarButton>) {
        self.is_mouse_left_button_down_year_view.set(true);

        self.update_year_view_selection(sender);
    }

    /// The handler of the mouse up event of a month or year button;
    /// `sender` is the button.
    pub(crate) fn month_calendar_button_mouse_up(&self, sender: Option<&CalendarButton>) {
        self.is_mouse_left_button_down_year_view.set(false);

        if let (Some(owner), Some(new_month)) = (self.owner(), sender.and_then(|sender| date_of(sender))) {
            if owner.display_mode() == CalendarMode::Year {
                owner.set_display_date(new_month);
                owner.set_display_mode(CalendarMode::Month);
            } else {
                debug_assert!(
                    owner.display_mode() == CalendarMode::Decade,
                    "The owning Calendar should be in decade mode!"
                );
                owner.set_selected_month(new_month);
                owner.set_display_mode(CalendarMode::Year);
            }
        }
    }

    fn month_mouse_entered(&self, sender: Option<&CalendarButton>) {
        if self.is_mouse_left_button_down_year_view.get() {
            self.update_year_view_selection(sender);
        }
    }

    pub(crate) fn update_disabled(&self, is_enabled: bool) {
        self.pseudo_classes().set(":calendardisabled", !is_enabled);
    }

    /// The state of the tracking of the left mouse button in the year view.
    #[cfg(test)]
    pub(crate) fn is_mouse_left_button_down_year_view(&self) -> bool {
        self.is_mouse_left_button_down_year_view.get()
    }

    #[cfg(test)]
    pub(crate) fn set_is_mouse_left_button_down_year_view(&self, value: bool) {
        self.is_mouse_left_button_down_year_view.set(value);
    }
}
