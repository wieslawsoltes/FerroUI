// (c) Copyright Microsoft Corporation.
// This source is subject to the Microsoft Public License (Ms-PL).
// Please see https://go.microsoft.com/fwlink/?LinkID=131993 for details.
// All other rights reserved.
//
// Ported from the Silverlight Toolkit sources as adapted by the upstream
// project; the license text is in the `NOTICE.md` of this crate.

use super::calendar_item::date_of;
use super::{
    CalendarBlackoutDatesCollection, CalendarButton, CalendarDateRange, CalendarDayButton, CalendarExtensions,
    CalendarItem, DateTimeHelper, SelectedDatesCollection,
};
use crate::primitives::{SelectingItemsControl, TemplateAppliedEventArgs, TemplatedControl, TemplatedControlImpl};
use crate::{ControlImpl, Panel, SelectionChangedEventArgs};
use ferroui_base::data::BindingMode;
use ferroui_base::input::{
    FocusChangedEventArgs, InputElement, InputElementImpl, InputElementImplExt, Key, KeyEventArgs, MouseButton,
    PointerReleasedEventArgs, PointerWheelEventArgs,
};
use ferroui_base::interactivity::{InteractiveImpl, RoutedEventArgs};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::IBrush;
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::utilities::{CalendarWeekRule, DateTime, DayOfWeek, HandlerList};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, ferro_routed_event_args, instantiate,
    BoxedValue, FerroObjectImpl, FerroObjectImplExt, FerroProperty, Ref, StyledElementImpl, StyledProperty,
    StyledPropertyOptions, VisualImpl,
};
use std::cell::{Cell, OnceCell, RefCell};
use std::rc::Rc;

/// Specifies values for the different modes of operation of a
/// [`Calendar`].
#[repr(i32)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum CalendarMode {
    /// The [`Calendar`] displays a month at a time.
    #[default]
    Month = 0,

    /// The [`Calendar`] displays a year at a time.
    Year = 1,

    /// The [`Calendar`] displays a decade at a time.
    Decade = 2,
}

/// Specifies values that describe the available selection modes for a
/// [`Calendar`].
///
/// This enumeration provides the values that are used by the SelectionMode
/// property.
#[repr(i32)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum CalendarSelectionMode {
    /// Only a single date can be selected. Use the
    /// [`Calendar::selected_date`] property to retrieve the selected date.
    #[default]
    SingleDate = 0,

    /// A single range of dates can be selected. Use the
    /// [`Calendar::selected_dates`] property to retrieve the selected
    /// dates.
    SingleRange = 1,

    /// Multiple non-contiguous ranges of dates can be selected. Use the
    /// [`Calendar::selected_dates`] property to retrieve the selected
    /// dates.
    MultipleRange = 2,

    /// No selections are allowed.
    None = 3,
}

/// Provides data for the `DisplayDateChanged` event of the [`Calendar`].
#[derive(Clone)]
pub struct CalendarDateChangedEventArgs {
    base: RoutedEventArgs,
    removed_date: Option<DateTime>,
    added_date: Option<DateTime>,
}

ferro_routed_event_args!(CalendarDateChangedEventArgs: RoutedEventArgs);

impl CalendarDateChangedEventArgs {
    /// Initializes a new instance of the [`CalendarDateChangedEventArgs`]
    /// class.
    ///
    /// `removed_date` is the date that was previously displayed and
    /// `added_date` the date to be newly displayed.
    pub(crate) fn new(removed_date: Option<DateTime>, added_date: Option<DateTime>) -> Self {
        Self { base: RoutedEventArgs::new(), removed_date, added_date }
    }

    /// Gets the date that was previously displayed.
    pub fn removed_date(&self) -> Option<DateTime> {
        self.removed_date
    }

    /// Gets the date to be newly displayed.
    pub fn added_date(&self) -> Option<DateTime> {
        self.added_date
    }
}

/// Provides data for the `DisplayModeChanged` event of the [`Calendar`].
#[derive(Clone)]
pub struct CalendarModeChangedEventArgs {
    base: RoutedEventArgs,
    old_mode: CalendarMode,
    new_mode: CalendarMode,
}

ferro_routed_event_args!(CalendarModeChangedEventArgs: RoutedEventArgs);

impl CalendarModeChangedEventArgs {
    /// Initializes a new instance of the [`CalendarModeChangedEventArgs`]
    /// class with the previous and the new mode.
    pub fn new(old_mode: CalendarMode, new_mode: CalendarMode) -> Self {
        Self { base: RoutedEventArgs::new(), old_mode, new_mode }
    }

    /// Gets the previous mode of the [`Calendar`].
    pub fn old_mode(&self) -> CalendarMode {
        self.old_mode
    }

    /// Gets the new mode of the [`Calendar`].
    pub fn new_mode(&self) -> CalendarMode {
        self.new_mode
    }
}

/// The text of the selected date in the general format of the current
/// culture, or an empty text when no date is selected.
impl std::fmt::Display for Calendar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.selected_date() {
            Some(selected_date) => {
                f.write_str(&selected_date.to_string_provider(&DateTimeHelper::get_current_date_format()))
            }
            None => Ok(()),
        }
    }
}

const PART_ELEMENT_ROOT: &str = "PART_Root";
const PART_ELEMENT_MONTH: &str = "PART_CalendarItem";

/// The date of an item of the arguments of a selection change.
fn date_of_item(item: &Option<BoxedValue>) -> Option<DateTime> {
    item.as_ref().and_then(|item| item.downcast_ref::<DateTime>().copied())
}

/// Represents a control that enables a user to select a date by using a
/// visual calendar display.
///
/// A Calendar control can be used on its own, or as a drop-down part of a
/// date picker control. A Calendar displays either the days of a month, the
/// months of a year, or the years of a decade, depending on the value of
/// the DisplayMode property.  When displaying the days of a month, the user
/// can select a date, a range of dates, or multiple ranges of dates.  The
/// kinds of selections that are allowed are controlled by the SelectionMode
/// property.
///
/// The range of dates displayed is governed by the DisplayDateStart and
/// DisplayDateEnd properties.  If DisplayMode is Year or Decade, only
/// months or years that contain displayable dates will be displayed.
///
/// The BlackoutDates property can be used to specify dates that cannot be
/// selected. These dates will be displayed as dimmed and disabled.
///
/// By default, Today is highlighted.  This can be disabled by setting
/// IsTodayHighlighted to false.
///
/// The Calendar control provides basic navigation using either the mouse or
/// keyboard. The following table summarizes keyboard navigation.
///
/// ```text
///     Key Combination     DisplayMode     Action
///     ARROW               Any             Change focused date, unselect
///                                         all selected dates, and select
///                                         new focused date.
///
///     SHIFT+ARROW         Any             If SelectionMode is not set to
///                                         SingleDate or None begin
///                                         selecting a range of dates.
///
///     CTRL+UP ARROW       Any             Switch to the next larger
///                                         DisplayMode.  If DisplayMode is
///                                         already Decade, no action.
///
///     CTRL+DOWN ARROW     Any             Switch to the next smaller
///                                         DisplayMode.  If DisplayMode is
///                                         already Month, no action.
///
///     SPACEBAR            Month           Select focused date.
///
///     SPACEBAR            Year or Decade  Switch DisplayMode to the Month
///                                         or Year represented by focused
///                                         item.
/// ```
#[repr(C)]
pub struct Calendar {
    base: TemplatedControl,

    selected_month: Cell<DateTime>,
    selected_year: Cell<DateTime>,

    is_shift_pressed: Cell<bool>,
    display_date_is_changing: Cell<bool>,
    is_tap_range_selection_active: Cell<bool>,
    tap_range_start: Cell<Option<DateTime>>,

    focus_button: RefCell<Option<Ref<CalendarDayButton>>>,
    focus_calendar_button: RefCell<Option<Ref<CalendarButton>>>,

    root: RefCell<Option<Ref<Panel>>>,

    selected_dates: OnceCell<SelectedDatesCollection>,
    blackout_dates: OnceCell<CalendarBlackoutDatesCollection>,
    removed_items: RefCell<Vec<DateTime>>,
    last_selected_date_internal: Cell<Option<DateTime>>,
    display_date_internal: Cell<DateTime>,

    hover_start: Cell<Option<DateTime>>,
    hover_start_index: Cell<Option<i32>>,
    hover_end_internal: Cell<Option<DateTime>>,
    hover_end_index: Cell<Option<i32>>,
    has_focus_internal: Cell<bool>,
    is_mouse_selection: Cell<bool>,

    /// A value indicating whether the date picker should change its
    /// DisplayDate because of a SelectedDate change on its Calendar.
    calendar_date_picker_display_date_flag: Cell<bool>,

    selected_dates_changed: HandlerList<dyn Fn(&SelectionChangedEventArgs)>,
    display_date_changed: HandlerList<dyn Fn(&CalendarDateChangedEventArgs)>,
    display_mode_changed: HandlerList<dyn Fn(&CalendarModeChangedEventArgs)>,
    day_button_mouse_up: HandlerList<dyn Fn(&PointerReleasedEventArgs)>,
}

ferro_class!(Calendar: TemplatedControl);

ferro_class_info!(Calendar {
    new: Calendar::new,
    markup: {
        properties: [
            SelectedDates: SelectedDatesCollection { get: Calendar::selected_dates },
            BlackoutDates: CalendarBlackoutDatesCollection { get: Calendar::blackout_dates },
        ],
        attributes: [
            TemplatePart("PART_CalendarItem", type(Ref<CalendarItem>)),
            TemplatePart("PART_Root", type(Ref<Panel>)),
        ],
    },
});

ferro_impl_classes!(Calendar: StyledElementImpl, VisualImpl, LayoutableImpl, InteractiveImpl);

impl FerroObjectImpl for Calendar {
    /// Initializes a new instance of the [`Calendar`] class.
    fn constructed(this: &Self) {
        Self::parent_constructed(this);

        this.set_current_value(Self::display_date_property(), DateTime::today());
        Self::update_display_date(this, this.display_date(), DateTime::MIN_VALUE);
        let this_ref = this.to_ref();
        let _ = this.blackout_dates.set(CalendarBlackoutDatesCollection::new(&this_ref));
        let _ = this.selected_dates.set(SelectedDatesCollection::new(&this_ref));
    }
}

impl InputElementImpl for Calendar {
    fn on_pointer_released(this: &Self, e: &PointerReleasedEventArgs) {
        Self::parent_on_pointer_released(this, e);
        if !this.has_focus_internal() && e.initial_press_mouse_button() == MouseButton::Left {
            this.focus();
        }
    }

    /// Default mouse wheel handler for the calendar control.
    fn on_pointer_wheel_changed(this: &Self, e: &PointerWheelEventArgs) {
        Self::parent_on_pointer_wheel_changed(this, e);
        if !e.handled() {
            let (ctrl, shift) = CalendarExtensions::get_meta_key_state(e.key_modifiers());

            if !ctrl {
                if e.delta().y > 0.0 {
                    this.process_page_up_key(false);
                } else {
                    this.process_page_down_key(false);
                }
            } else if e.delta().y > 0.0 {
                this.process_down_key(ctrl, shift);
            } else {
                this.process_up_key(ctrl, shift);
            }
            e.set_handled(true);
        }
    }

    fn on_got_focus(this: &Self, e: &FocusChangedEventArgs) {
        Self::parent_on_got_focus(this, e);
        this.has_focus_internal.set(true);

        match this.display_mode() {
            CalendarMode::Month => {
                let focus_date;
                match this.last_selected_date() {
                    Some(last_selected_date)
                        if DateTimeHelper::compare_year_month(this.display_date_internal(), last_selected_date) == 0 =>
                    {
                        focus_date = last_selected_date;
                    }
                    _ => {
                        focus_date = this.display_date();
                        this.set_last_selected_date(Some(this.display_date()));
                    }
                }
                let focus_button = this.find_day_button_from_day(focus_date);
                this.set_focus_button(focus_button.clone());

                if let Some(focus_button) = focus_button {
                    focus_button.set_is_current(true);
                }
            }
            CalendarMode::Year | CalendarMode::Decade => {
                if let Some(focus_calendar_button) = this.focus_calendar_button() {
                    focus_calendar_button.set_is_calendar_button_focused(true);
                }
            }
        }
    }

    fn on_lost_focus(this: &Self, e: &FocusChangedEventArgs) {
        Self::parent_on_lost_focus(this, e);
        this.has_focus_internal.set(false);

        match this.display_mode() {
            CalendarMode::Month => {
                if let Some(focus_button) = this.focus_button() {
                    focus_button.set_is_current(false);
                }
            }
            CalendarMode::Year | CalendarMode::Decade => {
                if let Some(focus_calendar_button) = this.focus_calendar_button() {
                    focus_calendar_button.set_is_calendar_button_focused(false);
                }
            }
        }
    }
}

impl TemplatedControlImpl for Calendar {
    /// Builds the visual tree for the [`Calendar`] when a new template is
    /// applied.
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        let root = e.name_scope().find_as::<Panel>(PART_ELEMENT_ROOT);
        *this.root.borrow_mut() = root.clone();

        this.set_selected_month(this.display_date());
        this.set_selected_year(this.display_date());

        if root.is_some() {
            let month = e.name_scope().find_as::<CalendarItem>(PART_ELEMENT_MONTH);

            if let Some(month) = month {
                month.set_owner(Some(&this.to_ref()));
            }
        }
    }
}

impl ControlImpl for Calendar {
    fn on_create_automation_peer(this: &Self) -> Ref<crate::automation::peers::AutomationPeer> {
        crate::automation::peers::CalendarAutomationPeer::new(this).upcast()
    }
}

ferro_properties! {
    impl Calendar {
        /// Defines the `FirstDayOfWeek` property.
        pub fn first_day_of_week_property() -> StyledProperty<DayOfWeek> {
            FerroProperty::register::<Calendar, _>(
                "FirstDayOfWeek",
                DateTimeHelper::get_current_date_format().first_day_of_week(),
            )
        }

        /// Defines the `IsTodayHighlighted` property.
        pub fn is_today_highlighted_property() -> StyledProperty<bool> {
            FerroProperty::register::<Calendar, _>("IsTodayHighlighted", true)
        }

        /// Defines the `HeaderBackground` property.
        pub fn header_background_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
            FerroProperty::register::<Calendar, _>("HeaderBackground", None)
        }

        /// Defines the `IsWeekNumberVisible` property.
        pub fn is_week_number_visible_property() -> StyledProperty<bool> {
            FerroProperty::register::<Calendar, _>("IsWeekNumberVisible", false)
        }

        /// Defines the `WeekNumberRule` property.
        pub fn week_number_rule_property() -> StyledProperty<CalendarWeekRule> {
            FerroProperty::register::<Calendar, _>(
                "WeekNumberRule",
                DateTimeHelper::get_current_date_format().calendar_week_rule(),
            )
        }

        /// Defines the `DisplayMode` property.
        pub fn display_mode_property() -> StyledProperty<CalendarMode> {
            FerroProperty::register_with::<Calendar, _>(
                "DisplayMode",
                StyledPropertyOptions::new(CalendarMode::Month).validate(Calendar::is_valid_display_mode),
            )
        }

        /// Defines the `SelectionMode` property.
        pub fn selection_mode_property() -> StyledProperty<CalendarSelectionMode> {
            FerroProperty::register::<Calendar, _>("SelectionMode", CalendarSelectionMode::SingleDate)
        }

        /// Defines the `AllowTapRangeSelection` property.
        pub fn allow_tap_range_selection_property() -> StyledProperty<bool> {
            FerroProperty::register::<Calendar, _>("AllowTapRangeSelection", true)
        }

        /// Defines the `SelectedDate` property.
        pub fn selected_date_property() -> StyledProperty<Option<DateTime>> {
            FerroProperty::register_with::<Calendar, _>(
                "SelectedDate",
                StyledPropertyOptions::new(None).default_binding_mode(BindingMode::TwoWay),
            )
        }

        /// Defines the `DisplayDate` property.
        pub fn display_date_property() -> StyledProperty<DateTime> {
            FerroProperty::register_with::<Calendar, _>(
                "DisplayDate",
                StyledPropertyOptions::new(DateTime::MIN_VALUE).default_binding_mode(BindingMode::TwoWay),
            )
        }

        /// Defines the `DisplayDateStart` property.
        pub fn display_date_start_property() -> StyledProperty<Option<DateTime>> {
            FerroProperty::register_with::<Calendar, _>(
                "DisplayDateStart",
                StyledPropertyOptions::new(None).default_binding_mode(BindingMode::TwoWay),
            )
        }

        /// Defines the `DisplayDateEnd` property.
        pub fn display_date_end_property() -> StyledProperty<Option<DateTime>> {
            FerroProperty::register_with::<Calendar, _>(
                "DisplayDateEnd",
                StyledPropertyOptions::new(None).default_binding_mode(BindingMode::TwoWay),
            )
        }
    }
}

impl Calendar {
    pub(crate) const ROWS_PER_MONTH: i32 = 7;
    pub(crate) const COLUMNS_PER_MONTH: i32 = 7;
    pub(crate) const ROWS_PER_YEAR: i32 = 3;
    pub(crate) const COLUMNS_PER_YEAR: i32 = 4;

    fn static_constructor() {
        InputElement::is_enabled_property()
            .changed()
            .add_class_handler::<Calendar>(|x, e| x.on_is_enabled_changed(e.get_new_value::<bool>()));
        Self::first_day_of_week_property()
            .changed()
            .add_class_handler::<Calendar>(|x, e| x.on_first_day_of_week_changed(e.get_new_value::<DayOfWeek>()));
        Self::is_today_highlighted_property()
            .changed()
            .add_class_handler::<Calendar>(|x, _| x.on_is_today_highlighted_changed());
        Self::display_mode_property().changed().add_class_handler::<Calendar>(|x, e| {
            let (old_mode, mode) = e.get_old_and_new_value::<CalendarMode>();
            x.on_display_mode_property_changed(old_mode, mode);
        });
        Self::selection_mode_property().changed().add_class_handler::<Calendar>(|x, e| {
            x.on_selection_mode_changed(e.get_new_value::<CalendarSelectionMode>())
        });
        Self::allow_tap_range_selection_property()
            .changed()
            .add_class_handler::<Calendar>(|x, _| x.on_allow_tap_range_selection_changed());
        Self::selected_date_property()
            .changed()
            .add_class_handler::<Calendar>(|x, e| x.on_selected_date_changed(e.get_new_value::<Option<DateTime>>()));
        Self::display_date_property().changed().add_class_handler::<Calendar>(|x, e| {
            let (old_value, new_value) = e.get_old_and_new_value::<DateTime>();
            x.on_display_date_changed(new_value, old_value);
        });
        Self::display_date_start_property().changed().add_class_handler::<Calendar>(|x, e| {
            x.on_display_date_start_changed(e.get_new_value::<Option<DateTime>>())
        });
        Self::display_date_end_property()
            .changed()
            .add_class_handler::<Calendar>(|x, e| x.on_display_date_end_changed(e.get_new_value::<Option<DateTime>>()));
        Self::is_week_number_visible_property().changed().add_class_handler::<Calendar>(|x, _| x.update_months());
        Self::week_number_rule_property().changed().add_class_handler::<Calendar>(|x, _| x.update_months());
        InputElement::key_down_event().add_class_handler::<Calendar>(|x, e| x.calendar_key_down(e));
        InputElement::key_up_event().add_class_handler::<Calendar>(|x, e| x.calendar_key_up(e));
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: TemplatedControl::construct(),
            selected_month: Cell::new(DateTime::MIN_VALUE),
            selected_year: Cell::new(DateTime::MIN_VALUE),
            is_shift_pressed: Cell::new(false),
            display_date_is_changing: Cell::new(false),
            is_tap_range_selection_active: Cell::new(false),
            tap_range_start: Cell::new(None),
            focus_button: RefCell::new(None),
            focus_calendar_button: RefCell::new(None),
            root: RefCell::new(None),
            selected_dates: OnceCell::new(),
            blackout_dates: OnceCell::new(),
            removed_items: RefCell::new(Vec::new()),
            last_selected_date_internal: Cell::new(None),
            display_date_internal: Cell::new(DateTime::MIN_VALUE),
            hover_start: Cell::new(None),
            hover_start_index: Cell::new(None),
            hover_end_internal: Cell::new(None),
            hover_end_index: Cell::new(None),
            has_focus_internal: Cell::new(false),
            is_mouse_selection: Cell::new(false),
            calendar_date_picker_display_date_flag: Cell::new(false),
            selected_dates_changed: HandlerList::new(),
            display_date_changed: HandlerList::new(),
            display_mode_changed: HandlerList::new(),
            day_button_mouse_up: HandlerList::new(),
        }
    }

    /// Initializes a new instance of the [`Calendar`] class.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    pub(crate) fn focus_button(&self) -> Option<Ref<CalendarDayButton>> {
        self.focus_button.borrow().clone()
    }

    pub(crate) fn set_focus_button(&self, value: Option<Ref<CalendarDayButton>>) {
        *self.focus_button.borrow_mut() = value;
    }

    pub(crate) fn focus_calendar_button(&self) -> Option<Ref<CalendarButton>> {
        self.focus_calendar_button.borrow().clone()
    }

    pub(crate) fn set_focus_calendar_button(&self, value: Option<Ref<CalendarButton>>) {
        *self.focus_calendar_button.borrow_mut() = value;
    }

    pub(crate) fn root(&self) -> Option<Ref<Panel>> {
        self.root.borrow().clone()
    }

    pub(crate) fn month_control(&self) -> Option<Ref<CalendarItem>> {
        let root = self.root()?;
        if root.children().count() > 0 {
            return root.children().get(0).cast::<CalendarItem>();
        }
        None
    }

    /// Gets or sets the day that is considered the beginning of the week.
    /// The default is the first day of the week of the culture that was
    /// current when the class was initialised.
    pub fn first_day_of_week(&self) -> DayOfWeek {
        self.get_value(Self::first_day_of_week_property())
    }

    pub fn set_first_day_of_week(&self, value: DayOfWeek) {
        self.set_value(Self::first_day_of_week_property(), value)
    }

    /// FirstDayOfWeekProperty property changed handler.
    fn on_first_day_of_week_changed(&self, new_value: DayOfWeek) {
        if Self::is_valid_first_day_of_week(new_value) {
            self.update_months();
        } else {
            panic!("Invalid DayOfWeek (Parameter 'e')");
        }
    }

    /// Whether a value is a day of the week. Every value of the
    /// enumeration is one.
    fn is_valid_first_day_of_week(day: DayOfWeek) -> bool {
        matches!(
            day,
            DayOfWeek::Sunday
                | DayOfWeek::Monday
                | DayOfWeek::Tuesday
                | DayOfWeek::Wednesday
                | DayOfWeek::Thursday
                | DayOfWeek::Friday
                | DayOfWeek::Saturday
        )
    }

    /// Gets or sets a value indicating whether the current date is
    /// highlighted. The default is true.
    pub fn is_today_highlighted(&self) -> bool {
        self.get_value(Self::is_today_highlighted_property())
    }

    pub fn set_is_today_highlighted(&self, value: bool) {
        self.set_value(Self::is_today_highlighted_property(), value)
    }

    /// IsTodayHighlightedProperty property changed handler.
    fn on_is_today_highlighted_changed(&self) {
        let i = DateTimeHelper::compare_year_month(self.display_date_internal(), DateTime::today());

        if i > -2 && i < 2 {
            self.update_months();
        }
    }

    /// The brush of the background of the header.
    pub fn header_background(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::header_background_property())
    }

    pub fn set_header_background(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::header_background_property(), value)
    }

    /// Gets or sets a value indicating whether week numbers are shown in
    /// the month view.
    pub fn is_week_number_visible(&self) -> bool {
        self.get_value(Self::is_week_number_visible_property())
    }

    pub fn set_is_week_number_visible(&self, value: bool) {
        self.set_value(Self::is_week_number_visible_property(), value)
    }

    /// Gets or sets the rule used to determine the first week of the year
    /// for week number display. The default is taken from the culture that
    /// was current when the class was initialised.
    ///
    /// Use [`CalendarWeekRule::FirstFourDayWeek`] in combination with a
    /// first day of the week of [`DayOfWeek::Monday`] for ISO 8601 week
    /// numbering.
    pub fn week_number_rule(&self) -> CalendarWeekRule {
        self.get_value(Self::week_number_rule_property())
    }

    pub fn set_week_number_rule(&self, value: CalendarWeekRule) {
        self.set_value(Self::week_number_rule_property(), value)
    }

    /// Gets or sets a value indicating whether the calendar is displayed in
    /// months, years, or decades.
    pub fn display_mode(&self) -> CalendarMode {
        self.get_value(Self::display_mode_property())
    }

    pub fn set_display_mode(&self, value: CalendarMode) {
        self.set_value(Self::display_mode_property(), value)
    }

    /// DisplayModeProperty property changed handler.
    fn on_display_mode_property_changed(&self, old_mode: CalendarMode, mode: CalendarMode) {
        let month_control = self.month_control();

        if month_control.is_some() {
            match old_mode {
                CalendarMode::Month => {
                    self.set_selected_year(self.display_date_internal());
                    self.set_selected_month(self.display_date_internal());
                }
                CalendarMode::Year => {
                    self.set_current_value(Self::display_date_property(), self.selected_month());
                    self.set_selected_year(self.selected_month());
                }
                CalendarMode::Decade => {
                    self.set_current_value(Self::display_date_property(), self.selected_year());
                    self.set_selected_month(self.selected_year());
                }
            }

            match mode {
                CalendarMode::Month => self.on_month_click(),
                CalendarMode::Year | CalendarMode::Decade => self.on_header_click(),
            }
        }
        if let Some(month_control) = &month_control {
            month_control.update_week_number_labels_visibility();
        }
        self.on_display_mode_changed(&CalendarModeChangedEventArgs::new(old_mode, mode));
    }

    fn is_valid_display_mode(mode: &CalendarMode) -> bool {
        matches!(mode, CalendarMode::Month | CalendarMode::Year | CalendarMode::Decade)
    }

    fn on_display_mode_changed(&self, args: &CalendarModeChangedEventArgs) {
        for (_, handler) in self.display_mode_changed.snapshot().iter() {
            handler(args);
        }
    }

    /// Gets or sets a value that indicates what kind of selections are
    /// allowed. The default is [`CalendarSelectionMode::SingleDate`].
    ///
    /// This property determines whether the Calendar allows no selection,
    /// selection of a single date, or selection of multiple dates.
    ///
    /// When this property is changed, all selected dates will be cleared.
    pub fn selection_mode(&self) -> CalendarSelectionMode {
        self.get_value(Self::selection_mode_property())
    }

    pub fn set_selection_mode(&self, value: CalendarSelectionMode) {
        self.set_value(Self::selection_mode_property(), value)
    }

    /// Gets or sets a value indicating whether tap-to-select range mode is
    /// enabled. When enabled, users can tap a start date and then tap an
    /// end date to select a range. The default is true.
    ///
    /// This feature only works when SelectionMode is set to a range mode.
    /// When enabled, the first tap selects the start date, and the second
    /// tap selects the end date to complete the range. Tapping a third date
    /// starts a new range.
    pub fn allow_tap_range_selection(&self) -> bool {
        self.get_value(Self::allow_tap_range_selection_property())
    }

    pub fn set_allow_tap_range_selection(&self, value: bool) {
        self.set_value(Self::allow_tap_range_selection_property(), value)
    }

    fn on_selection_mode_changed(&self, new_value: CalendarSelectionMode) {
        if Self::is_valid_selection_mode(new_value) {
            self.display_date_is_changing.set(true);
            self.set_current_value(Self::selected_date_property(), None);
            self.display_date_is_changing.set(false);
            self.selected_dates().clear();

            // Reset tap range selection state when mode changes
            self.is_tap_range_selection_active.set(false);
            self.tap_range_start.set(None);
        } else {
            panic!("Invalid SelectionMode (Parameter 'e')");
        }
    }

    fn on_allow_tap_range_selection_changed(&self) {
        self.is_tap_range_selection_active.set(false);
        self.tap_range_start.set(None);
    }

    /// Whether a value is a selection mode. Every value of the enumeration
    /// is one.
    fn is_valid_selection_mode(mode: CalendarSelectionMode) -> bool {
        matches!(
            mode,
            CalendarSelectionMode::SingleDate
                | CalendarSelectionMode::SingleRange
                | CalendarSelectionMode::MultipleRange
                | CalendarSelectionMode::None
        )
    }

    /// Gets or sets the currently selected date. The default is `None`.
    ///
    /// Setting it panics when the given date is in the blackout dates, or
    /// when it is set to anything other than `None` while the selection
    /// mode is [`CalendarSelectionMode::None`].
    ///
    /// Use this property when SelectionMode is set to SingleDate.  In other
    /// modes, this property will always be the first date in SelectedDates.
    pub fn selected_date(&self) -> Option<DateTime> {
        self.get_value(Self::selected_date_property())
    }

    pub fn set_selected_date(&self, value: Option<DateTime>) {
        self.set_value(Self::selected_date_property(), value)
    }

    fn on_selected_date_changed(&self, added_date: Option<DateTime>) {
        if !self.display_date_is_changing.get() {
            if self.selection_mode() != CalendarSelectionMode::None {
                if Self::is_valid_date_selection(self, added_date) {
                    let selected_dates = self.selected_dates();
                    match added_date {
                        None => selected_dates.clear(),
                        Some(added_date) => {
                            if !(selected_dates.count() > 0 && selected_dates.get(0) == added_date) {
                                self.add_removed_items(&selected_dates.to_vec());
                                selected_dates.clear_internal();
                                // the value is added as a range so that the
                                // SelectedDatesChanged event can be thrown with
                                // all the removed items
                                selected_dates.add_range(added_date, added_date);
                            }
                        }
                    }

                    // We update the LastSelectedDate for only the Single
                    // mode.  For the other modes it automatically gets
                    // updated when the HoverEnd is updated.
                    if self.selection_mode() == CalendarSelectionMode::SingleDate {
                        self.set_last_selected_date(added_date);
                    }
                } else {
                    panic!("SelectedDate value is not valid. (Parameter 'e')");
                }
            } else {
                panic!("The SelectedDate property cannot be set when the selection mode is None.");
            }
        }
    }

    /// Gets a collection of selected dates. The default is an empty
    /// collection.
    ///
    /// Dates can be added to the collection either individually or in a
    /// range using the AddRange method.  Depending on the value of the
    /// SelectionMode property, adding a date or range to the collection may
    /// cause it to be cleared.  The following table lists how the selection
    /// mode affects the SelectedDates property.
    ///
    /// ```text
    ///     CalendarSelectionMode   Description
    ///     None                    No selections are allowed.  SelectedDate
    ///                             cannot be set and no values can be added
    ///                             to SelectedDates.
    ///
    ///     SingleDate              Only a single date can be selected,
    ///                             either by setting SelectedDate or the
    ///                             first value in SelectedDates.  AddRange
    ///                             cannot be used.
    ///
    ///     SingleRange             A single range of dates can be selected.
    ///                             Setting SelectedDate, adding a date
    ///                             individually to SelectedDates, or using
    ///                             AddRange will clear all previous values
    ///                             from SelectedDates.
    ///     MultipleRange           Multiple non-contiguous ranges of dates
    ///                             can be selected. Adding a date
    ///                             individually to SelectedDates or using
    ///                             AddRange will not clear SelectedDates.
    ///                             Setting SelectedDate will still clear
    ///                             SelectedDates, but additional dates or
    ///                             range can then be added.  Adding a range
    ///                             that includes some dates that are
    ///                             already selected or overlaps with
    ///                             another range results in the union of
    ///                             the ranges and does not cause an
    ///                             exception.
    /// ```
    pub fn selected_dates(&self) -> SelectedDatesCollection {
        self.selected_dates.get().expect("the calendar is constructed").clone()
    }

    fn is_selection_changed(e: &SelectionChangedEventArgs) -> bool {
        if e.added_items().len() != e.removed_items().len() {
            return true;
        }
        for added_date in e.added_items() {
            let added_date = date_of_item(added_date);
            if !e.removed_items().iter().any(|removed_date| date_of_item(removed_date) == added_date) {
                return true;
            }
        }
        false
    }

    pub(crate) fn on_selected_dates_collection_changed(&self, e: &SelectionChangedEventArgs) {
        if Self::is_selection_changed(e) {
            e.set_routed_event(Some(SelectingItemsControl::selection_changed_event()));
            e.set_source(self.to_ref());
            for (_, handler) in self.selected_dates_changed.snapshot().iter() {
                handler(e);
            }
        }
    }

    /// The dates removed from the selection that the next notification of
    /// a selection change reports.
    pub(crate) fn removed_items(&self) -> Vec<DateTime> {
        self.removed_items.borrow().clone()
    }

    pub(crate) fn add_removed_items(&self, items: &[DateTime]) {
        self.removed_items.borrow_mut().extend_from_slice(items);
    }

    pub(crate) fn clear_removed_items(&self) {
        self.removed_items.borrow_mut().clear();
    }

    pub(crate) fn last_selected_date_internal(&self) -> Option<DateTime> {
        self.last_selected_date_internal.get()
    }

    pub(crate) fn last_selected_date(&self) -> Option<DateTime> {
        self.last_selected_date_internal()
    }

    pub(crate) fn set_last_selected_date(&self, value: Option<DateTime>) {
        self.last_selected_date_internal.set(value);

        if self.selection_mode() == CalendarSelectionMode::None {
            if let Some(focus_button) = self.focus_button() {
                focus_button.set_is_current(false);
            }
            let focus_button = self
                .find_day_button_from_day(self.last_selected_date().expect("Nullable object must have a value."));
            self.set_focus_button(focus_button.clone());
            if let Some(focus_button) = focus_button {
                focus_button.set_is_current(self.has_focus_internal());
            }
        }
    }

    pub(crate) fn selected_month(&self) -> DateTime {
        self.selected_month.get()
    }

    pub(crate) fn set_selected_month(&self, value: DateTime) {
        let month_difference_start = DateTimeHelper::compare_year_month(value, self.display_date_range_start());
        let month_difference_end = DateTimeHelper::compare_year_month(value, self.display_date_range_end());

        if month_difference_start >= 0 && month_difference_end <= 0 {
            self.selected_month.set(DateTimeHelper::discard_day_time(value));
        } else if month_difference_start < 0 {
            self.selected_month.set(DateTimeHelper::discard_day_time(self.display_date_range_start()));
        } else {
            debug_assert!(month_difference_end > 0, "monthDifferenceEnd should be greater than 0!");
            self.selected_month.set(DateTimeHelper::discard_day_time(self.display_date_range_end()));
        }
    }

    pub(crate) fn selected_year(&self) -> DateTime {
        self.selected_year.get()
    }

    pub(crate) fn set_selected_year(&self, value: DateTime) {
        if value.year() < self.display_date_range_start().year() {
            self.selected_year.set(self.display_date_range_start());
        } else if value.year() > self.display_date_range_end().year() {
            self.selected_year.set(self.display_date_range_end());
        } else {
            self.selected_year.set(value);
        }
    }

    /// Gets or sets the date to display. The default is today.
    ///
    /// A date that is not in the range specified by the DisplayDateStart
    /// and DisplayDateEnd properties is replaced by the nearest end of
    /// that range.
    pub fn display_date(&self) -> DateTime {
        self.get_value(Self::display_date_property())
    }

    pub fn set_display_date(&self, value: DateTime) {
        self.set_value(Self::display_date_property(), value)
    }

    pub(crate) fn display_date_internal(&self) -> DateTime {
        self.display_date_internal.get()
    }

    fn on_display_date_changed(&self, new_value: DateTime, old_value: DateTime) {
        Self::update_display_date(self, new_value, old_value);
    }

    fn update_display_date(c: &Calendar, added_date: DateTime, removed_date: DateTime) {
        // If DisplayDate < DisplayDateStart, DisplayDate = DisplayDateStart
        if DateTime::compare(added_date, c.display_date_range_start()) < 0 {
            c.set_display_date(c.display_date_range_start());
            return;
        }

        // If DisplayDate > DisplayDateEnd, DisplayDate = DisplayDateEnd
        if DateTime::compare(added_date, c.display_date_range_end()) > 0 {
            c.set_display_date(c.display_date_range_end());
            return;
        }

        c.display_date_internal.set(DateTimeHelper::discard_day_time(added_date));
        c.update_months();
        c.on_display_date(&CalendarDateChangedEventArgs::new(Some(removed_date), Some(added_date)));
    }

    fn on_display_date(&self, e: &CalendarDateChangedEventArgs) {
        for (_, handler) in self.display_date_changed.snapshot().iter() {
            handler(e);
        }
    }

    /// Gets or sets the first date to be displayed.
    pub fn display_date_start(&self) -> Option<DateTime> {
        self.get_value(Self::display_date_start_property())
    }

    pub fn set_display_date_start(&self, value: Option<DateTime>) {
        self.set_value(Self::display_date_start_property(), value)
    }

    fn on_display_date_start_changed(&self, new_value: Option<DateTime>) {
        if !self.display_date_is_changing.get() {
            if let Some(new_value) = new_value {
                // DisplayDateStart coerces to the value of the
                // SelectedDateMin if SelectedDateMin < DisplayDateStart
                let selected_date_min = Self::selected_date_min(self);

                if let Some(selected_date_min) = selected_date_min {
                    if DateTime::compare(selected_date_min, new_value) < 0 {
                        self.set_current_value(Self::display_date_start_property(), Some(selected_date_min));
                        return;
                    }
                }

                // if DisplayDateStart > DisplayDateEnd,
                // DisplayDateEnd = DisplayDateStart
                if DateTime::compare(new_value, self.display_date_range_end()) > 0 {
                    self.set_current_value(Self::display_date_end_property(), self.display_date_start());
                }

                // If DisplayDate < DisplayDateStart,
                // DisplayDate = DisplayDateStart
                if DateTimeHelper::compare_year_month(new_value, self.display_date_internal()) > 0 {
                    self.set_current_value(Self::display_date_property(), new_value);
                }
            }
            self.update_months();
        }
    }

    /// Gets a collection of dates that are marked as not selectable. The
    /// default value is an empty collection.
    ///
    /// Adding a date to this collection when it is already selected
    /// panics.
    ///
    /// Dates in this collection will appear as disabled on the calendar.
    ///
    /// To make all past dates not selectable, you can use the
    /// `add_dates_in_past` method provided by the collection returned by
    /// this property.
    pub fn blackout_dates(&self) -> CalendarBlackoutDatesCollection {
        self.blackout_dates.get().expect("the calendar is constructed").clone()
    }

    fn selected_date_min(cal: &Calendar) -> Option<DateTime> {
        let selected_dates = cal.selected_dates().to_vec();
        let mut selected_date_min = *selected_dates.first()?;
        debug_assert!(
            cal.selected_date().is_some_and(|selected| DateTime::compare(selected, selected_date_min) == 0),
            "The SelectedDate should be the minimum selected date!"
        );

        for selected_date in selected_dates {
            if DateTime::compare(selected_date, selected_date_min) < 0 {
                selected_date_min = selected_date;
            }
        }
        Some(selected_date_min)
    }

    pub(crate) fn display_date_range_start(&self) -> DateTime {
        self.display_date_start().unwrap_or(DateTime::MIN_VALUE)
    }

    /// Gets or sets the last date to be displayed.
    pub fn display_date_end(&self) -> Option<DateTime> {
        self.get_value(Self::display_date_end_property())
    }

    pub fn set_display_date_end(&self, value: Option<DateTime>) {
        self.set_value(Self::display_date_end_property(), value)
    }

    fn on_display_date_end_changed(&self, new_value: Option<DateTime>) {
        if !self.display_date_is_changing.get() {
            if let Some(new_value) = new_value {
                // DisplayDateEnd coerces to the value of the
                // SelectedDateMax if SelectedDateMax > DisplayDateEnd
                let selected_date_max = Self::selected_date_max(self);

                if let Some(selected_date_max) = selected_date_max {
                    if DateTime::compare(selected_date_max, new_value) > 0 {
                        self.set_current_value(Self::display_date_end_property(), Some(selected_date_max));
                        return;
                    }
                }

                // if DisplayDateEnd < DisplayDateStart,
                // DisplayDateEnd = DisplayDateStart
                if DateTime::compare(new_value, self.display_date_range_start()) < 0 {
                    self.set_current_value(Self::display_date_end_property(), self.display_date_start());
                    return;
                }

                // If DisplayDate > DisplayDateEnd,
                // DisplayDate = DisplayDateEnd
                if DateTimeHelper::compare_year_month(new_value, self.display_date_internal()) < 0 {
                    self.set_current_value(Self::display_date_property(), new_value);
                }
            }
            self.update_months();
        }
    }

    fn selected_date_max(cal: &Calendar) -> Option<DateTime> {
        let selected_dates = cal.selected_dates().to_vec();
        let mut selected_date_max = *selected_dates.first()?;
        debug_assert!(
            cal.selected_date().is_some_and(|selected| DateTime::compare(selected, selected_date_max) == 0),
            "The SelectedDate should be the maximum SelectedDate!"
        );

        for selected_date in selected_dates {
            if DateTime::compare(selected_date, selected_date_max) > 0 {
                selected_date_max = selected_date;
            }
        }
        Some(selected_date_max)
    }

    pub(crate) fn display_date_range_end(&self) -> DateTime {
        self.display_date_end().unwrap_or(DateTime::MAX_VALUE)
    }

    pub(crate) fn hover_start(&self) -> Option<DateTime> {
        self.hover_start.get()
    }

    pub(crate) fn set_hover_start(&self, value: Option<DateTime>) {
        self.hover_start.set(value);
    }

    pub(crate) fn hover_start_index(&self) -> Option<i32> {
        self.hover_start_index.get()
    }

    pub(crate) fn set_hover_start_index(&self, value: Option<i32>) {
        self.hover_start_index.set(value);
    }

    pub(crate) fn hover_end_internal(&self) -> Option<DateTime> {
        self.hover_end_internal.get()
    }

    pub(crate) fn hover_end(&self) -> Option<DateTime> {
        self.hover_end_internal()
    }

    pub(crate) fn set_hover_end(&self, value: Option<DateTime>) {
        self.hover_end_internal.set(value);
        self.set_last_selected_date(value);
    }

    pub(crate) fn hover_end_index(&self) -> Option<i32> {
        self.hover_end_index.get()
    }

    pub(crate) fn set_hover_end_index(&self, value: Option<i32>) {
        self.hover_end_index.set(value);
    }

    pub(crate) fn has_focus_internal(&self) -> bool {
        self.has_focus_internal.get()
    }

    pub(crate) fn is_mouse_selection(&self) -> bool {
        self.is_mouse_selection.get()
    }

    pub(crate) fn set_is_mouse_selection(&self, value: bool) {
        self.is_mouse_selection.set(value);
    }

    /// Gets a value indicating whether the date picker should change its
    /// DisplayDate because of a SelectedDate change on its Calendar.
    pub(crate) fn calendar_date_picker_display_date_flag(&self) -> bool {
        self.calendar_date_picker_display_date_flag.get()
    }

    pub(crate) fn set_calendar_date_picker_display_date_flag(&self, value: bool) {
        self.calendar_date_picker_display_date_flag.set(value);
    }

    pub(crate) fn find_day_button_from_day(&self, day: DateTime) -> Option<Ref<CalendarDayButton>> {
        let month_control = self.month_control();

        // REMOVE_RTM: should be updated if we support MultiCalendar
        let count = Self::ROWS_PER_MONTH * Self::COLUMNS_PER_MONTH;
        if let Some(month_view) = month_control.and_then(|month_control| month_control.month_view()) {
            for child_index in Self::COLUMNS_PER_MONTH..count {
                if let Some(b) = month_view.children().get(child_index as usize).cast::<CalendarDayButton>() {
                    if let Some(d) = date_of(&b) {
                        if DateTimeHelper::compare_days(d, day) == 0 {
                            return Some(b);
                        }
                    }
                }
            }
        }
        None
    }

    fn on_selected_month_changed(&self, selected_month: Option<DateTime>) {
        if let Some(selected_month) = selected_month {
            debug_assert!(self.display_mode() == CalendarMode::Year, "DisplayMode should be Year!");
            self.set_selected_month(selected_month);
            self.update_months();
        }
    }

    fn on_selected_year_changed(&self, selected_year: Option<DateTime>) {
        if let Some(selected_year) = selected_year {
            debug_assert!(self.display_mode() == CalendarMode::Decade, "DisplayMode should be Decade!");
            self.set_selected_year(selected_year);
            self.update_months();
        }
    }

    pub(crate) fn on_header_click(&self) {
        debug_assert!(
            self.display_mode() == CalendarMode::Year || self.display_mode() == CalendarMode::Decade,
            "The DisplayMode should be Year or Decade"
        );
        if let Some(month_control) = self.month_control() {
            if let (Some(month_view), Some(year_view)) = (month_control.month_view(), month_control.year_view()) {
                month_view.set_is_visible(false);
                year_view.set_is_visible(true);
                self.update_months();
            }
        }
    }

    pub(crate) fn reset_states(&self) {
        let month_control = self.month_control();
        let count = Self::ROWS_PER_MONTH * Self::COLUMNS_PER_MONTH;
        if let Some(month_view) = month_control.and_then(|month_control| month_control.month_view()) {
            for child_index in Self::COLUMNS_PER_MONTH..count {
                let d = month_view
                    .children()
                    .get(child_index as usize)
                    .cast::<CalendarDayButton>()
                    .expect("the child of the month view is a CalendarDayButton");
                d.ignore_mouse_over_state();
            }
        }
    }

    pub(crate) fn update_months(&self) {
        if let Some(month_control) = self.month_control() {
            match self.display_mode() {
                CalendarMode::Month => month_control.update_month_mode(),
                CalendarMode::Year => month_control.update_year_mode(),
                CalendarMode::Decade => month_control.update_decade_mode(),
            }
        }
    }

    pub(crate) fn is_valid_date_selection(cal: &Calendar, value: Option<DateTime>) -> bool {
        let Some(value) = value else {
            return true;
        };

        if cal.blackout_dates().contains_date(value) {
            false
        } else {
            cal.display_date_is_changing.set(true);
            if DateTime::compare(value, cal.display_date_range_start()) < 0 {
                cal.set_display_date_start(Some(value));
            } else if DateTime::compare(value, cal.display_date_range_end()) > 0 {
                cal.set_display_date_end(Some(value));
            }
            cal.display_date_is_changing.set(false);

            true
        }
    }

    fn is_valid_keyboard_selection(cal: &Calendar, value: Option<DateTime>) -> bool {
        let Some(value) = value else {
            return true;
        };

        if cal.blackout_dates().contains_date(value) {
            false
        } else {
            DateTime::compare(value, cal.display_date_range_start()) >= 0
                && DateTime::compare(value, cal.display_date_range_end()) <= 0
        }
    }

    /// This method highlights the days in MultiSelection mode without
    /// adding them to the SelectedDates collection.
    pub(crate) fn highlight_days(&self) {
        if let (Some(hover_end), Some(_)) = (self.hover_end(), self.hover_start()) {
            let month_control = self.month_control();
            debug_assert!(month_control.is_some());

            // This assumes a contiguous set of dates:
            if self.hover_end_index().is_some() && self.hover_start_index().is_some() {
                let month_view = month_control
                    .and_then(|month_control| month_control.month_view())
                    .expect("the calendar has a month view");
                let (start_index, end_index) = self.sort_hover_indexes();

                for i in start_index..=end_index {
                    if let Some(b) = month_view.children().get(i as usize).cast::<CalendarDayButton>() {
                        b.set_is_selected(true);

                        if date_of(&b).is_some_and(|d| DateTimeHelper::compare_days(hover_end, d) == 0) {
                            if let Some(focus_button) = self.focus_button() {
                                focus_button.set_is_current(false);
                            }
                            b.set_is_current(self.has_focus_internal());
                            self.set_focus_button(Some(b));
                        }
                    }
                }
            }
        }
    }

    /// This method un-highlights the days that were hovered over but not
    /// added to the SelectedDates collection or un-highlighted the
    /// previously selected days in SingleRange Mode.
    pub(crate) fn un_highlight_days(&self) {
        if self.hover_end().is_some() && self.hover_start().is_some() {
            let month_control = self.month_control();
            debug_assert!(month_control.is_some());

            if self.hover_end_index().is_some() && self.hover_start_index().is_some() {
                let month_view = month_control
                    .and_then(|month_control| month_control.month_view())
                    .expect("the calendar has a month view");
                let (start_index, end_index) = self.sort_hover_indexes();

                if self.selection_mode() == CalendarSelectionMode::MultipleRange {
                    for i in start_index..=end_index {
                        if let Some(b) = month_view.children().get(i as usize).cast::<CalendarDayButton>() {
                            if let Some(d) = date_of(&b) {
                                if !self.selected_dates().contains(d) {
                                    b.set_is_selected(false);
                                }
                            }
                        }
                    }
                } else {
                    // It is SingleRange
                    for i in start_index..=end_index {
                        month_view
                            .children()
                            .get(i as usize)
                            .cast::<CalendarDayButton>()
                            .expect("the child of the month view is a CalendarDayButton")
                            .set_is_selected(false);
                    }
                }
            }
        }
    }

    /// The indexes of the start and of the end of the hovered range, in
    /// that order.
    pub(crate) fn sort_hover_indexes(&self) -> (i32, i32) {
        let hover_start = self.hover_start().expect("the hover range has a start");
        let hover_end = self.hover_end().expect("the hover range has an end");
        let hover_start_index = self.hover_start_index().expect("the hover range has a start index");
        let hover_end_index = self.hover_end_index().expect("the hover range has an end index");

        if DateTimeHelper::compare_days(hover_end, hover_start) > 0 {
            (hover_start_index, hover_end_index)
        } else {
            (hover_end_index, hover_start_index)
        }
    }

    pub(crate) fn on_previous_click(&self) {
        if self.display_mode() == CalendarMode::Month {
            let d = DateTimeHelper::add_months(DateTimeHelper::discard_day_time(self.display_date()), -1);
            if let Some(d) = d {
                if !self
                    .last_selected_date()
                    .is_some_and(|last_selected_date| DateTimeHelper::compare_year_month(last_selected_date, d) == 0)
                {
                    self.set_last_selected_date(Some(d));
                }
                self.set_current_value(Self::display_date_property(), d);
            }
        } else {
            if self.display_mode() == CalendarMode::Year {
                let d = DateTimeHelper::add_years(DateTime::new(self.selected_month().year(), 1, 1), -1);

                match d {
                    Some(d) => self.set_selected_month(d),
                    None => self.set_selected_month(DateTimeHelper::discard_day_time(self.display_date_range_start())),
                }
            } else {
                debug_assert!(self.display_mode() == CalendarMode::Decade, "DisplayMode should be Decade!");

                let d = DateTimeHelper::add_years(DateTime::new(self.selected_year().year(), 1, 1), -10);

                match d {
                    Some(d) => {
                        let decade = DateTimeHelper::decade_of_date(d).max(1);
                        self.set_selected_year(DateTime::new(decade, 1, 1));
                    }
                    None => self.set_selected_year(DateTimeHelper::discard_day_time(self.display_date_range_start())),
                }
            }
            self.update_months();
        }
    }

    pub(crate) fn on_next_click(&self) {
        if self.display_mode() == CalendarMode::Month {
            let d = DateTimeHelper::add_months(DateTimeHelper::discard_day_time(self.display_date()), 1);
            if let Some(d) = d {
                if !self
                    .last_selected_date()
                    .is_some_and(|last_selected_date| DateTimeHelper::compare_year_month(last_selected_date, d) == 0)
                {
                    self.set_last_selected_date(Some(d));
                }
                self.set_current_value(Self::display_date_property(), d);
            }
        } else {
            if self.display_mode() == CalendarMode::Year {
                let d = DateTimeHelper::add_years(DateTime::new(self.selected_month().year(), 1, 1), 1);

                match d {
                    Some(d) => self.set_selected_month(d),
                    None => self.set_selected_month(DateTimeHelper::discard_day_time(self.display_date_range_end())),
                }
            } else {
                debug_assert!(self.display_mode() == CalendarMode::Decade, "DisplayMode should be Decade");

                let d = DateTimeHelper::add_years(DateTime::new(self.selected_year().year(), 1, 1), 10);

                match d {
                    Some(d) => {
                        let decade = DateTimeHelper::decade_of_date(d).max(1);
                        self.set_selected_year(DateTime::new(decade, 1, 1));
                    }
                    None => self.set_selected_year(DateTimeHelper::discard_day_time(self.display_date_range_end())),
                }
            }
            self.update_months();
        }
    }

    /// If the day is a trailing day, Update the DisplayDate.
    pub(crate) fn on_day_click(&self, selected_date: DateTime) {
        debug_assert!(self.display_mode() == CalendarMode::Month, "DisplayMode should be Month!");
        let i = DateTimeHelper::compare_year_month(selected_date, self.display_date_internal());

        if self.selection_mode() == CalendarSelectionMode::None {
            self.set_last_selected_date(Some(selected_date));
        }

        if i > 0 {
            self.on_next_click();
        } else if i < 0 {
            self.on_previous_click();
        }
    }

    fn on_month_click(&self) {
        let month_control = self.month_control();
        debug_assert!(month_control.is_some());

        if let Some(month_control) = month_control {
            if let (Some(year_view), Some(month_view)) = (month_control.year_view(), month_control.month_view()) {
                year_view.set_is_visible(false);
                month_view.set_is_visible(true);

                if !self.last_selected_date().is_some_and(|last_selected_date| {
                    DateTimeHelper::compare_year_month(last_selected_date, self.display_date()) == 0
                }) {
                    self.set_last_selected_date(Some(self.display_date()));
                }

                self.update_months();
            }
        }
    }

    /// Occurs when the collection returned by the SelectedDates property is
    /// changed.
    pub fn selected_dates_changed(
        &self,
        handler: impl Fn(&SelectionChangedEventArgs) + 'static,
    ) -> Rc<dyn IDisposable> {
        let token = self.selected_dates_changed.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.selected_dates_changed.remove(token);
            }
        })
    }

    /// Occurs when the DisplayDate property is changed.
    ///
    /// This event occurs after DisplayDate is assigned its new value.
    pub fn display_date_changed(
        &self,
        handler: impl Fn(&CalendarDateChangedEventArgs) + 'static,
    ) -> Rc<dyn IDisposable> {
        let token = self.display_date_changed.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.display_date_changed.remove(token);
            }
        })
    }

    /// Occurs when the DisplayMode property is changed.
    pub fn display_mode_changed(
        &self,
        handler: impl Fn(&CalendarModeChangedEventArgs) + 'static,
    ) -> Rc<dyn IDisposable> {
        let token = self.display_mode_changed.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.display_mode_changed.remove(token);
            }
        })
    }

    /// Occurs when the left mouse button is released over a day button
    /// that is not a blackout date.
    pub(crate) fn day_button_mouse_up(
        &self,
        handler: impl Fn(&PointerReleasedEventArgs) + 'static,
    ) -> Rc<dyn IDisposable> {
        let token = self.day_button_mouse_up.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.day_button_mouse_up.remove(token);
            }
        })
    }

    /// This method adds the days that were selected by Keyboard to the
    /// SelectedDays Collection.
    fn add_selection(&self) {
        if let (Some(hover_end), Some(hover_start)) = (self.hover_end(), self.hover_start()) {
            let selected_dates = self.selected_dates();
            self.add_removed_items(&selected_dates.to_vec());

            selected_dates.clear_internal();
            // In keyboard selection, we are sure that the collection does
            // not include any blackout days
            selected_dates.add_range(hover_start, hover_end);
        }
    }

    /// Moves the selected dates to the removed items and empties the
    /// collection without notifications.
    fn move_selection_to_removed_items(&self) {
        let selected_dates = self.selected_dates();
        self.add_removed_items(&selected_dates.to_vec());
        selected_dates.clear_internal();
    }

    /// Handles tap range selection logic for date range selection.
    ///
    /// `selected_date` is the date that was tapped. Returns true if the tap
    /// was handled as part of range selection; otherwise, false.
    pub(crate) fn process_tap_range_selection(&self, selected_date: DateTime) -> bool {
        if !self.allow_tap_range_selection()
            || (self.selection_mode() != CalendarSelectionMode::SingleRange
                && self.selection_mode() != CalendarSelectionMode::MultipleRange)
        {
            return false;
        }

        if !Self::is_valid_date_selection(self, Some(selected_date)) {
            return false;
        }

        let selected_dates = self.selected_dates();

        match self.tap_range_start.get().filter(|_| self.is_tap_range_selection_active.get()) {
            None => {
                self.is_tap_range_selection_active.set(true);
                self.tap_range_start.set(Some(selected_date));

                if self.selection_mode() == CalendarSelectionMode::SingleRange {
                    self.move_selection_to_removed_items();
                }

                if !selected_dates.contains(selected_date) {
                    selected_dates.add(selected_date);
                }

                true
            }
            Some(tap_range_start) => {
                let mut start_date = tap_range_start;
                let mut end_date = selected_date;

                if DateTime::compare(start_date, end_date) > 0 {
                    std::mem::swap(&mut start_date, &mut end_date);
                }

                let range = CalendarDateRange::new_range(start_date, end_date);
                if self.blackout_dates().contains_any(&range) {
                    self.tap_range_start.set(Some(selected_date));

                    if self.selection_mode() == CalendarSelectionMode::SingleRange {
                        self.move_selection_to_removed_items();
                    }

                    if !selected_dates.contains(selected_date) {
                        selected_dates.add(selected_date);
                    }
                    return true;
                }

                if self.selection_mode() == CalendarSelectionMode::SingleRange {
                    self.move_selection_to_removed_items();
                }

                selected_dates.add_range(start_date, end_date);

                self.is_tap_range_selection_active.set(false);
                self.tap_range_start.set(None);

                true
            }
        }
    }

    fn process_selection(&self, shift: bool, last_selected_date: Option<DateTime>, index: Option<i32>) {
        if let (CalendarSelectionMode::None, Some(last_selected_date)) = (self.selection_mode(), last_selected_date) {
            self.on_day_click(last_selected_date);
            return;
        }

        // Handle tap range selection.
        if let Some(last_selected_date) = last_selected_date {
            if index.is_none() && !shift && self.process_tap_range_selection(last_selected_date) {
                self.on_day_click(last_selected_date);
                return;
            }
        }

        let Some(last_selected_date) = last_selected_date else {
            return;
        };

        if Self::is_valid_keyboard_selection(self, Some(last_selected_date)) {
            if self.selection_mode() == CalendarSelectionMode::SingleRange
                || self.selection_mode() == CalendarSelectionMode::MultipleRange
            {
                self.move_selection_to_removed_items();
                if shift {
                    self.is_shift_pressed.set(true);
                    let hover_start = match self.hover_start() {
                        Some(hover_start) => hover_start,
                        None => {
                            let hover_start = match self.last_selected_date() {
                                Some(last_selected_date) => last_selected_date,
                                None => {
                                    if DateTimeHelper::compare_year_month(self.display_date_internal(), DateTime::today())
                                        == 0
                                    {
                                        DateTime::today()
                                    } else {
                                        self.display_date_internal()
                                    }
                                }
                            };
                            self.set_hover_start(Some(hover_start));

                            if let Some(b) = self.find_day_button_from_day(hover_start) {
                                self.set_hover_start_index(Some(b.index()));
                            }
                            hover_start
                        }
                    };
                    // the index of the SelectedDate is always the last
                    // selectedDate's index
                    self.un_highlight_days();
                    // If we hit a BlackOutDay with keyboard we do not
                    // update the HoverEnd
                    let range = if DateTime::compare(hover_start, last_selected_date) < 0 {
                        CalendarDateRange::new_range(hover_start, last_selected_date)
                    } else {
                        CalendarDateRange::new_range(last_selected_date, hover_start)
                    };

                    if !self.blackout_dates().contains_any(&range) {
                        self.set_hover_end(Some(last_selected_date));

                        match index {
                            Some(index) => {
                                self.set_hover_end_index(self.hover_end_index().map(|current| current + index));
                            }
                            None => {
                                // For Home, End, PageUp and PageDown Keys there
                                // is no easy way to predict the index value
                                let hover_end_internal =
                                    self.hover_end_internal().expect("the hover range has an end");
                                if let Some(b) = self.find_day_button_from_day(hover_end_internal) {
                                    self.set_hover_end_index(Some(b.index()));
                                }
                            }
                        }
                    }

                    self.on_day_click(self.hover_end().expect("the hover range has an end"));
                    self.highlight_days();
                } else {
                    self.set_hover_start(Some(last_selected_date));
                    self.set_hover_end(Some(last_selected_date));
                    self.add_selection();
                    self.on_day_click(last_selected_date);
                }
            } else {
                // ON CLEAR
                self.set_last_selected_date(Some(last_selected_date));
                let selected_dates = self.selected_dates();
                if selected_dates.count() > 0 {
                    selected_dates.set(0, last_selected_date);
                } else {
                    selected_dates.add(last_selected_date);
                }
                self.on_day_click(last_selected_date);
            }
        }
    }

    pub(crate) fn on_day_button_mouse_up(&self, e: &PointerReleasedEventArgs) {
        for (_, handler) in self.day_button_mouse_up.snapshot().iter() {
            handler(e);
        }
    }

    pub(crate) fn calendar_key_down(&self, e: &KeyEventArgs) {
        if !e.handled() && self.is_enabled() {
            e.set_handled(self.process_calendar_key(e));
        }
    }

    pub(crate) fn process_calendar_key(&self, e: &KeyEventArgs) -> bool {
        if self.display_mode() == CalendarMode::Month {
            if let Some(last_selected_date) = self.last_selected_date() {
                // If a blackout day is inactive, when clicked on it, the
                // previous inactive day which is not a blackout day can get
                // the focus.  In this case we should allow keyboard
                // functions on that inactive day
                if DateTimeHelper::compare_year_month(last_selected_date, self.display_date_internal()) != 0
                    && self.focus_button().is_some_and(|focus_button| !focus_button.is_inactive())
                {
                    return true;
                }
            }
        }

        // Some keys (e.g. Left/Right) need to be translated in RightToLeft mode
        let invariant_key = e.key;

        let (ctrl, shift) = CalendarExtensions::get_meta_key_state(e.key_modifiers);

        match invariant_key {
            Key::Up => {
                self.process_up_key(ctrl, shift);
                true
            }
            Key::Down => {
                self.process_down_key(ctrl, shift);
                true
            }
            Key::Left => {
                self.process_left_key(shift);
                true
            }
            Key::Right => {
                self.process_right_key(shift);
                true
            }
            Key::PageDown => {
                self.process_page_down_key(shift);
                true
            }
            Key::PageUp => {
                self.process_page_up_key(shift);
                true
            }
            Key::Home => {
                self.process_home_key(shift);
                true
            }
            Key::End => {
                self.process_end_key(shift);
                true
            }
            Key::Enter | Key::Space => self.process_enter_key(),
            _ => false,
        }
    }

    /// The last selected date, or today when there is none.
    fn last_selected_date_or_today(&self) -> DateTime {
        self.last_selected_date().unwrap_or_else(DateTime::today)
    }

    pub(crate) fn process_up_key(&self, ctrl: bool, shift: bool) {
        match self.display_mode() {
            CalendarMode::Month => {
                if ctrl {
                    self.set_selected_month(self.display_date_internal());
                    self.set_current_value(Self::display_mode_property(), CalendarMode::Year);
                } else {
                    let selected_date =
                        DateTimeHelper::add_days(self.last_selected_date_or_today(), -Self::COLUMNS_PER_MONTH);
                    self.process_selection(shift, selected_date, Some(-Self::COLUMNS_PER_MONTH));
                }
            }
            CalendarMode::Year => {
                if ctrl {
                    self.set_selected_year(self.selected_month());
                    self.set_current_value(Self::display_mode_property(), CalendarMode::Decade);
                } else {
                    let selected_month = DateTimeHelper::add_months(self.selected_month.get(), -Self::COLUMNS_PER_YEAR);
                    self.on_selected_month_changed(selected_month);
                }
            }
            CalendarMode::Decade => {
                if !ctrl {
                    let selected_year = DateTimeHelper::add_years(self.selected_year(), -Self::COLUMNS_PER_YEAR);
                    self.on_selected_year_changed(selected_year);
                }
            }
        }
    }

    pub(crate) fn process_down_key(&self, ctrl: bool, shift: bool) {
        match self.display_mode() {
            CalendarMode::Month => {
                if !ctrl || shift {
                    let selected_date =
                        DateTimeHelper::add_days(self.last_selected_date_or_today(), Self::COLUMNS_PER_MONTH);
                    self.process_selection(shift, selected_date, Some(Self::COLUMNS_PER_MONTH));
                }
            }
            CalendarMode::Year => {
                if ctrl {
                    self.set_current_value(Self::display_date_property(), self.selected_month());
                    self.set_current_value(Self::display_mode_property(), CalendarMode::Month);
                } else {
                    let selected_month = DateTimeHelper::add_months(self.selected_month.get(), Self::COLUMNS_PER_YEAR);
                    self.on_selected_month_changed(selected_month);
                }
            }
            CalendarMode::Decade => {
                if ctrl {
                    self.set_selected_month(self.selected_year());
                    self.set_current_value(Self::display_mode_property(), CalendarMode::Year);
                } else {
                    let selected_year = DateTimeHelper::add_years(self.selected_year(), Self::COLUMNS_PER_YEAR);
                    self.on_selected_year_changed(selected_year);
                }
            }
        }
    }

    pub(crate) fn process_left_key(&self, shift: bool) {
        match self.display_mode() {
            CalendarMode::Month => {
                let selected_date = DateTimeHelper::add_days(self.last_selected_date_or_today(), -1);
                self.process_selection(shift, selected_date, Some(-1));
            }
            CalendarMode::Year => {
                let selected_month = DateTimeHelper::add_months(self.selected_month.get(), -1);
                self.on_selected_month_changed(selected_month);
            }
            CalendarMode::Decade => {
                let selected_year = DateTimeHelper::add_years(self.selected_year(), -1);
                self.on_selected_year_changed(selected_year);
            }
        }
    }

    pub(crate) fn process_right_key(&self, shift: bool) {
        match self.display_mode() {
            CalendarMode::Month => {
                let selected_date = DateTimeHelper::add_days(self.last_selected_date_or_today(), 1);
                self.process_selection(shift, selected_date, Some(1));
            }
            CalendarMode::Year => {
                let selected_month = DateTimeHelper::add_months(self.selected_month.get(), 1);
                self.on_selected_month_changed(selected_month);
            }
            CalendarMode::Decade => {
                let selected_year = DateTimeHelper::add_years(self.selected_year(), 1);
                self.on_selected_year_changed(selected_year);
            }
        }
    }

    fn process_enter_key(&self) -> bool {
        match self.display_mode() {
            CalendarMode::Year => {
                self.set_current_value(Self::display_date_property(), self.selected_month());
                self.set_current_value(Self::display_mode_property(), CalendarMode::Month);
                true
            }
            CalendarMode::Decade => {
                self.set_selected_month(self.selected_year());
                self.set_current_value(Self::display_mode_property(), CalendarMode::Year);
                true
            }
            CalendarMode::Month => false,
        }
    }

    pub(crate) fn process_home_key(&self, shift: bool) {
        match self.display_mode() {
            CalendarMode::Month => {
                // REMOVE_RTM: Not all types of calendars start with Day1. If Non-Gregorian is supported check this:
                let display_date_internal = self.display_date_internal();
                let selected_date = DateTime::new(display_date_internal.year(), display_date_internal.month(), 1);
                self.process_selection(shift, Some(selected_date), None);
            }
            CalendarMode::Year => {
                let selected_month = DateTime::new(self.selected_month.get().year(), 1, 1);
                self.on_selected_month_changed(Some(selected_month));
            }
            CalendarMode::Decade => {
                let selected_year = DateTime::new(DateTimeHelper::decade_of_date(self.selected_year()), 1, 1);
                self.on_selected_year_changed(Some(selected_year));
            }
        }
    }

    pub(crate) fn process_end_key(&self, shift: bool) {
        match self.display_mode() {
            CalendarMode::Month => {
                let display_date_internal = self.display_date_internal();
                let mut selected_date = DateTime::new(display_date_internal.year(), display_date_internal.month(), 1);

                if DateTimeHelper::compare_year_month(DateTime::MAX_VALUE, selected_date) > 0 {
                    // since DisplayDate is not equal to
                    // DateTime.MaxValue we are sure selectedDate is
                    // not null
                    selected_date =
                        DateTimeHelper::add_months(selected_date, 1).expect("the next month is a representable date");
                    selected_date =
                        DateTimeHelper::add_days(selected_date, -1).expect("the previous day is a representable date");
                } else {
                    selected_date = DateTime::MAX_VALUE;
                }
                self.process_selection(shift, Some(selected_date), None);
            }
            CalendarMode::Year => {
                let selected_month = DateTime::new(self.selected_month.get().year(), 12, 1);
                self.on_selected_month_changed(Some(selected_month));
            }
            CalendarMode::Decade => {
                let selected_year = DateTime::new(DateTimeHelper::end_of_decade(self.selected_year()), 1, 1);
                self.on_selected_year_changed(Some(selected_year));
            }
        }
    }

    pub(crate) fn process_page_down_key(&self, shift: bool) {
        if !shift {
            self.on_next_click();
            return;
        }
        match self.display_mode() {
            CalendarMode::Month => {
                let selected_date = DateTimeHelper::add_months(self.last_selected_date_or_today(), 1);
                self.process_selection(shift, selected_date, None);
            }
            CalendarMode::Year => {
                let selected_month = DateTimeHelper::add_years(self.selected_month.get(), 1);
                self.on_selected_month_changed(selected_month);
            }
            CalendarMode::Decade => {
                let selected_year = DateTimeHelper::add_years(self.selected_year(), 10);
                self.on_selected_year_changed(selected_year);
            }
        }
    }

    pub(crate) fn process_page_up_key(&self, shift: bool) {
        if !shift {
            self.on_previous_click();
            return;
        }
        match self.display_mode() {
            CalendarMode::Month => {
                let selected_date = DateTimeHelper::add_months(self.last_selected_date_or_today(), -1);
                self.process_selection(shift, selected_date, None);
            }
            CalendarMode::Year => {
                let selected_month = DateTimeHelper::add_years(self.selected_month.get(), -1);
                self.on_selected_month_changed(selected_month);
            }
            CalendarMode::Decade => {
                let selected_year = DateTimeHelper::add_years(self.selected_year(), -10);
                self.on_selected_year_changed(selected_year);
            }
        }
    }

    fn calendar_key_up(&self, e: &KeyEventArgs) {
        if !e.handled() && (e.key == Key::LeftShift || e.key == Key::RightShift) {
            self.process_shift_key_up();
        }
    }

    pub(crate) fn process_shift_key_up(&self) {
        if self.is_shift_pressed.get()
            && (self.selection_mode() == CalendarSelectionMode::SingleRange
                || self.selection_mode() == CalendarSelectionMode::MultipleRange)
        {
            self.add_selection();
            self.is_shift_pressed.set(false);
        }
    }

    /// Called when the IsEnabled property changes.
    fn on_is_enabled_changed(&self, is_enabled: bool) {
        if let Some(month_control) = self.month_control() {
            month_control.update_disabled(is_enabled);
        }
    }
}
