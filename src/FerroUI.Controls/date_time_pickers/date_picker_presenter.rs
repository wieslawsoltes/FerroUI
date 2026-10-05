use super::date_picker::index_of_ignore_case;
use super::{
    DatePicker, DateTimePickerPanel, DateTimePickerPanelType, PickerPresenterBase, PickerPresenterBaseImpl,
};
use crate::primitives::{TemplateAppliedEventArgs, TemplatedControlImpl, TemplatedControlImplExt};
use crate::{Button, ColumnDefinition, Control, ControlImpl, Grid, GridUnitType, Panel, SpinDirection};
use ferroui_base::input::{
    FocusManager, InputElementImpl, InputElementImplExt, Key, KeyEventArgs, KeyModifiers, KeyboardNavigation, KeyboardNavigationHandler,
    KeyboardNavigationMode, NavigationDirection, NavigationMethod,
};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::utilities::{CultureInfo, DateTimeOffset, GregorianCalendar};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate, FerroObject, FerroObjectImpl, FerroObjectImplExt,
    FerroProperty, FerroPropertyChangedEventArgs, Ref, StyledElementImpl, StyledProperty, StyledPropertyOptions,
    VisualImpl,
};
use std::cell::{Cell, RefCell};

const PICKER_CONTAINER_NAME: &str = "PART_PickerContainer";
const ACCEPT_BUTTON_NAME: &str = "PART_AcceptButton";
const DISMISS_BUTTON_NAME: &str = "PART_DismissButton";
const FIRST_SPACER_NAME: &str = "PART_FirstSpacer";
const SECOND_SPACER_NAME: &str = "PART_SecondSpacer";
const MONTH_HOST_NAME: &str = "PART_MonthHost";
const YEAR_HOST_NAME: &str = "PART_YearHost";
const DAY_HOST_NAME: &str = "PART_DayHost";
const MONTH_SELECTOR_NAME: &str = "PART_MonthSelector";
const YEAR_SELECTOR_NAME: &str = "PART_YearSelector";
const DAY_SELECTOR_NAME: &str = "PART_DaySelector";
const MONTH_UP_BUTTON_NAME: &str = "PART_MonthUpButton";
const DAY_UP_BUTTON_NAME: &str = "PART_DayUpButton";
const YEAR_UP_BUTTON_NAME: &str = "PART_YearUpButton";
const MONTH_DOWN_BUTTON_NAME: &str = "PART_MonthDownButton";
const DAY_DOWN_BUTTON_NAME: &str = "PART_DayDownButton";
const YEAR_DOWN_BUTTON_NAME: &str = "PART_YearDownButton";

/// The parts of the control template.
#[derive(Clone)]
#[allow(dead_code)]
struct TemplateItems {
    picker_container: Ref<Grid>,
    accept_button: Ref<Button>,
    dismiss_button: Option<Ref<Button>>,
    first_spacer: Option<Ref<Control>>,
    second_spacer: Option<Ref<Control>>,
    month_host: Ref<Panel>,
    year_host: Ref<Panel>,
    day_host: Ref<Panel>,
    month_selector: Ref<DateTimePickerPanel>,
    year_selector: Ref<DateTimePickerPanel>,
    day_selector: Ref<DateTimePickerPanel>,
    month_up_button: Option<Ref<Button>>,
    day_up_button: Option<Ref<Button>>,
    year_up_button: Option<Ref<Button>>,
    month_down_button: Option<Ref<Button>>,
    day_down_button: Option<Ref<Button>>,
    year_down_button: Option<Ref<Button>>,
}

/// Defines the presenter used for selecting a date for a [`DatePicker`].
#[repr(C)]
pub struct DatePickerPresenter {
    base: PickerPresenterBase,
    template_items: RefCell<Option<TemplateItems>>,
    sync_date: Cell<DateTimeOffset>,
    calendar: GregorianCalendar,
    suppress_update_selection: Cell<bool>,
}

ferro_class!(DatePickerPresenter: PickerPresenterBase);

ferro_class_info!(DatePickerPresenter {
    new: DatePickerPresenter::new,
    markup: {
        attributes: [
            TemplatePart("PART_AcceptButton", type(Ref<Button>), IsRequired = true),
            TemplatePart("PART_DayDownButton", type(Ref<Button>)),
            TemplatePart("PART_DayHost", type(Ref<Panel>), IsRequired = true),
            TemplatePart("PART_DaySelector", type(Ref<DateTimePickerPanel>), IsRequired = true),
            TemplatePart("PART_DayUpButton", type(Ref<Button>)),
            TemplatePart("PART_DismissButton", type(Ref<Button>)),
            TemplatePart("PART_FirstSpacer", type(Ref<Control>)),
            TemplatePart("PART_MonthDownButton", type(Ref<Button>)),
            TemplatePart("PART_MonthHost", type(Ref<Panel>), IsRequired = true),
            TemplatePart("PART_MonthSelector", type(Ref<DateTimePickerPanel>), IsRequired = true),
            TemplatePart("PART_MonthUpButton", type(Ref<Button>)),
            TemplatePart("PART_PickerContainer", type(Ref<Grid>), IsRequired = true),
            TemplatePart("PART_SecondSpacer", type(Ref<Control>)),
            TemplatePart("PART_YearDownButton", type(Ref<Button>)),
            TemplatePart("PART_YearHost", type(Ref<Panel>), IsRequired = true),
            TemplatePart("PART_YearSelector", type(Ref<DateTimePickerPanel>), IsRequired = true),
            TemplatePart("PART_YearUpButton", type(Ref<Button>)),
        ],
    },
});

ferro_impl_classes!(
    DatePickerPresenter: StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    ControlImpl,
    PickerPresenterBaseImpl
);

impl FerroObjectImpl for DatePickerPresenter {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);

        let now = DateTimeOffset::now();
        this.set_current_value(
            Self::min_year_property(),
            DateTimeOffset::new(now.year() - 100, 1, 1, 0, 0, 0, now.offset()),
        );
        this.set_current_value(
            Self::max_year_property(),
            DateTimeOffset::new(now.year() + 100, 12, 31, 0, 0, 0, now.offset()),
        );
        this.set_current_value(Self::date_property(), now);
    }

    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        let property = change.property();
        if property == Self::date_property().as_property() {
            this.on_date_changed(change.get_new_value::<DateTimeOffset>());
        } else if property == Self::max_year_property().as_property()
            || property == Self::min_year_property().as_property()
        {
            this.on_date_range_changed();
        } else if property == Self::month_format_property().as_property()
            || property == Self::year_format_property().as_property()
            || property == Self::day_format_property().as_property()
        {
            this.init_picker();
        }
    }
}

impl TemplatedControlImpl for DatePickerPresenter {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        Self::parent_on_apply_template(this, e);

        let name_scope = e.name_scope();
        let weak_this = this.to_ref().downgrade();
        let selector_button = |name: &str, type_: DateTimePickerPanelType, direction: SpinDirection| {
            let button = name_scope.find_as::<Button>(name)?;
            let weak = weak_this.clone();
            button.click(move |_, _| {
                if let Some(this) = weak.upgrade() {
                    this.on_selector_button_click(type_, direction);
                }
            });
            Some(button)
        };

        let items = TemplateItems {
            // These are requirements, so panic if not found
            picker_container: name_scope.get_as::<Grid>(PICKER_CONTAINER_NAME),
            month_host: name_scope.get_as::<Panel>(MONTH_HOST_NAME),
            day_host: name_scope.get_as::<Panel>(DAY_HOST_NAME),
            year_host: name_scope.get_as::<Panel>(YEAR_HOST_NAME),

            month_selector: name_scope.get_as::<DateTimePickerPanel>(MONTH_SELECTOR_NAME),
            day_selector: name_scope.get_as::<DateTimePickerPanel>(DAY_SELECTOR_NAME),
            year_selector: name_scope.get_as::<DateTimePickerPanel>(YEAR_SELECTOR_NAME),

            accept_button: name_scope.get_as::<Button>(ACCEPT_BUTTON_NAME),

            month_up_button: selector_button(MONTH_UP_BUTTON_NAME, DateTimePickerPanelType::Month, SpinDirection::Decrease),
            month_down_button: selector_button(
                MONTH_DOWN_BUTTON_NAME,
                DateTimePickerPanelType::Month,
                SpinDirection::Increase,
            ),
            day_up_button: selector_button(DAY_UP_BUTTON_NAME, DateTimePickerPanelType::Day, SpinDirection::Decrease),
            day_down_button: selector_button(DAY_DOWN_BUTTON_NAME, DateTimePickerPanelType::Day, SpinDirection::Increase),
            year_up_button: selector_button(YEAR_UP_BUTTON_NAME, DateTimePickerPanelType::Year, SpinDirection::Decrease),
            year_down_button: selector_button(
                YEAR_DOWN_BUTTON_NAME,
                DateTimePickerPanelType::Year,
                SpinDirection::Increase,
            ),

            dismiss_button: name_scope.find_as::<Button>(DISMISS_BUTTON_NAME),
            first_spacer: name_scope.find_as::<Control>(FIRST_SPACER_NAME),
            second_spacer: name_scope.find_as::<Control>(SECOND_SPACER_NAME),
        };
        *this.template_items.borrow_mut() = Some(items.clone());

        let weak = weak_this.clone();
        items.accept_button.click(move |_, _| {
            if let Some(this) = weak.upgrade() {
                this.on_accept_button_clicked();
            }
        });
        let weak = weak_this.clone();
        items.month_selector.selection_changed(move || {
            if let Some(this) = weak.upgrade() {
                this.on_month_changed();
            }
        });
        let weak = weak_this.clone();
        items.day_selector.selection_changed(move || {
            if let Some(this) = weak.upgrade() {
                this.on_day_changed();
            }
        });
        let weak = weak_this.clone();
        items.year_selector.selection_changed(move || {
            if let Some(this) = weak.upgrade() {
                this.on_year_changed();
            }
        });

        if let Some(dismiss_button) = &items.dismiss_button {
            let weak = weak_this.clone();
            dismiss_button.click(move |_, _| {
                if let Some(this) = weak.upgrade() {
                    this.on_dismiss_button_clicked();
                }
            });
        }

        this.init_picker();
    }
}

impl InputElementImpl for DatePickerPresenter {
    fn on_key_down(this: &Self, e: &KeyEventArgs) {
        match e.key {
            Key::Escape => {
                this.on_dismiss();
                e.set_handled(true);
            }
            Key::Tab => {
                let focus_manager = FocusManager::get_focus_manager(this);
                if let Some(focus) = focus_manager.and_then(|focus_manager| focus_manager.get_focused_element()) {
                    let next_focus = KeyboardNavigationHandler::get_next(&focus, NavigationDirection::Next);
                    if let Some(next_focus) = next_focus {
                        next_focus.focus_with(NavigationMethod::Tab, KeyModifiers::NONE);
                    }
                    e.set_handled(true);
                }
            }
            Key::Enter => {
                this.set_current_value(Self::date_property(), this.sync_date.get());
                this.on_confirmed();
                e.set_handled(true);
            }
            _ => {}
        }
        Self::parent_on_key_down(this, e);
    }
}

ferro_properties! {
    impl DatePickerPresenter {
        /// Defines the `Date` property.
        pub fn date_property() -> StyledProperty<DateTimeOffset> {
            FerroProperty::register_with::<DatePickerPresenter, _>(
                "Date",
                StyledPropertyOptions::new(DateTimeOffset::default()).coerce(DatePickerPresenter::coerce_date),
            )
        }

        /// Defines the `DayFormat` property.
        pub fn day_format_property() -> StyledProperty<String> {
            DatePicker::day_format_property().add_owner::<DatePickerPresenter>()
        }

        /// Defines the `DayVisible` property.
        pub fn day_visible_property() -> StyledProperty<bool> {
            DatePicker::day_visible_property().add_owner::<DatePickerPresenter>()
        }

        /// Defines the `MaxYear` property.
        pub fn max_year_property() -> StyledProperty<DateTimeOffset> {
            DatePicker::max_year_property().add_owner::<DatePickerPresenter>()
        }

        /// Defines the `MinYear` property.
        pub fn min_year_property() -> StyledProperty<DateTimeOffset> {
            DatePicker::min_year_property().add_owner::<DatePickerPresenter>()
        }

        /// Defines the `MonthFormat` property.
        pub fn month_format_property() -> StyledProperty<String> {
            DatePicker::month_format_property().add_owner::<DatePickerPresenter>()
        }

        /// Defines the `MonthVisible` property.
        pub fn month_visible_property() -> StyledProperty<bool> {
            DatePicker::month_visible_property().add_owner::<DatePickerPresenter>()
        }

        /// Defines the `YearFormat` property.
        pub fn year_format_property() -> StyledProperty<String> {
            DatePicker::year_format_property().add_owner::<DatePickerPresenter>()
        }

        /// Defines the `YearVisible` property.
        pub fn year_visible_property() -> StyledProperty<bool> {
            DatePicker::year_visible_property().add_owner::<DatePickerPresenter>()
        }
    }
}

impl DatePickerPresenter {
    fn static_constructor() {
        KeyboardNavigation::tab_navigation_property()
            .override_default_value::<DatePickerPresenter>(KeyboardNavigationMode::Cycle);
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: PickerPresenterBase::construct(),
            template_items: RefCell::new(None),
            sync_date: Cell::new(DateTimeOffset::default()),
            calendar: GregorianCalendar::new(),
            suppress_update_selection: Cell::new(false),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    fn coerce_date(sender: &FerroObject, value: DateTimeOffset) -> DateTimeOffset {
        let max = sender.get_value(Self::max_year_property());
        if value > max {
            return max;
        }
        let min = sender.get_value(Self::min_year_property());
        if value < min {
            return min;
        }

        value
    }

    fn on_date_range_changed(&self) {
        self.coerce_value(Self::date_property().as_property());
    }

    /// The current date for the picker.
    pub fn date(&self) -> DateTimeOffset {
        self.get_value(Self::date_property())
    }

    pub fn set_date(&self, value: DateTimeOffset) {
        self.set_value(Self::date_property(), value)
    }

    fn on_date_changed(&self, new_value: DateTimeOffset) {
        self.sync_date.set(new_value);
        self.init_picker();
    }

    /// The day format.
    pub fn day_format(&self) -> String {
        self.get_value(Self::day_format_property())
    }

    pub fn set_day_format(&self, value: &str) {
        self.set_value(Self::day_format_property(), value.to_string())
    }

    /// Whether the day selector is visible.
    pub fn day_visible(&self) -> bool {
        self.get_value(Self::day_visible_property())
    }

    pub fn set_day_visible(&self, value: bool) {
        self.set_value(Self::day_visible_property(), value)
    }

    /// The maximum pickable year.
    pub fn max_year(&self) -> DateTimeOffset {
        self.get_value(Self::max_year_property())
    }

    pub fn set_max_year(&self, value: DateTimeOffset) {
        self.set_value(Self::max_year_property(), value)
    }

    /// The minimum pickable year.
    pub fn min_year(&self) -> DateTimeOffset {
        self.get_value(Self::min_year_property())
    }

    pub fn set_min_year(&self, value: DateTimeOffset) {
        self.set_value(Self::min_year_property(), value)
    }

    /// The month format.
    pub fn month_format(&self) -> String {
        self.get_value(Self::month_format_property())
    }

    pub fn set_month_format(&self, value: &str) {
        self.set_value(Self::month_format_property(), value.to_string())
    }

    /// Whether the month selector is visible.
    pub fn month_visible(&self) -> bool {
        self.get_value(Self::month_visible_property())
    }

    pub fn set_month_visible(&self, value: bool) {
        self.set_value(Self::month_visible_property(), value)
    }

    /// The year format.
    pub fn year_format(&self) -> String {
        self.get_value(Self::year_format_property())
    }

    pub fn set_year_format(&self, value: &str) {
        self.set_value(Self::year_format_property(), value.to_string())
    }

    /// Whether the year selector is visible.
    pub fn year_visible(&self) -> bool {
        self.get_value(Self::year_visible_property())
    }

    pub fn set_year_visible(&self, value: bool) {
        self.set_value(Self::year_visible_property(), value)
    }

    fn items(&self) -> Option<TemplateItems> {
        self.template_items.borrow().clone()
    }

    /// Initializes the picker selectors.
    fn init_picker(&self) {
        // The template must have been applied before we can init here...
        let Some(items) = self.items() else {
            return;
        };

        self.suppress_update_selection.set(true);

        items.month_selector.set_maximum_value(12);
        items.month_selector.set_minimum_value(1);
        items.month_selector.set_item_format(&self.month_format());

        items.day_selector.set_item_format(&self.day_format());

        items.year_selector.set_maximum_value(self.max_year().year());
        items.year_selector.set_minimum_value(self.min_year().year());
        items.year_selector.set_item_format(&self.year_format());

        self.set_grid(&items);

        // Date should've been set when we reach this point
        let dt = self.date();
        if self.day_visible() {
            items.day_selector.set_format_date(dt.date());
            let max_days = self.calendar.get_days_in_month(dt.year(), dt.month());
            items.day_selector.set_maximum_value(max_days);
            items.day_selector.set_minimum_value(1);
            items.day_selector.set_selected_value(dt.day());
        }

        if self.month_visible() {
            items.month_selector.set_selected_value(dt.month());
            items.month_selector.set_format_date(dt.date());
        }

        if self.year_visible() {
            items.year_selector.set_selected_value(dt.year());
            items.year_selector.set_format_date(dt.date());
        }

        self.suppress_update_selection.set(false);

        self.set_initial_focus(&items);
    }

    fn set_grid(&self, items: &TemplateItems) {
        let date_time_format = CultureInfo::current_culture().date_time_format();
        let fmt = date_time_format.short_date_pattern();
        let mut columns = [
            (&items.month_host, if self.month_visible() { index_of_ignore_case(fmt, 'm') } else { -1 }),
            (&items.year_host, if self.year_visible() { index_of_ignore_case(fmt, 'y') } else { -1 }),
            (&items.day_host, if self.day_visible() { index_of_ignore_case(fmt, 'd') } else { -1 }),
        ];

        columns.sort_by_key(|column| column.1);
        items.picker_container.column_definitions().clear();

        let mut column_index = 0;

        for (host, index) in columns {
            host.set_is_visible(index != -1);

            if index != -1 {
                if column_index > 0 {
                    items.picker_container.column_definitions().add(ColumnDefinition::with_value(0.0, GridUnitType::Auto));
                }

                items.picker_container.column_definitions().add(ColumnDefinition::with_value(
                    if host.ptr_eq(&items.month_host) { 138.0 } else { 78.0 },
                    GridUnitType::Star,
                ));

                if host.parent().is_none() {
                    items.picker_container.children().add(host.clone());
                }

                Grid::set_column(host, column_index * 2);
                column_index += 1;
            }
        }

        fn configure_spacer(spacer: &Option<Ref<Control>>, visible: bool, column: i32) {
            let Some(spacer) = spacer else {
                return;
            };
            // The conditional is used to make sure grid cells will be validated
            Grid::set_column(spacer, if visible { column } else { 0 });
            spacer.set_is_visible(visible);
        }

        configure_spacer(&items.first_spacer, column_index > 1, 1);
        configure_spacer(&items.second_spacer, column_index > 2, 3);
    }

    fn set_initial_focus(&self, items: &TemplateItems) {
        let candidates = [
            (self.month_visible(), &items.month_host, &items.month_selector),
            (self.day_visible(), &items.day_host, &items.day_selector),
            (self.year_visible(), &items.year_host, &items.year_selector),
        ];

        let mut leftmost: Option<&Ref<DateTimePickerPanel>> = None;
        let mut min_col = i32::MAX;

        for (visible, host, selector) in candidates {
            if !visible {
                continue;
            }

            let col = Grid::get_column(host);
            if col < min_col {
                min_col = col;
                leftmost = Some(selector);
            }
        }

        if let Some(leftmost) = leftmost {
            leftmost.focus_with(NavigationMethod::Pointer, KeyModifiers::NONE);
        }
    }

    fn on_dismiss_button_clicked(&self) {
        self.on_dismiss();
    }

    fn on_accept_button_clicked(&self) {
        self.set_current_value(Self::date_property(), self.sync_date.get());
        self.on_confirmed();
    }

    fn on_selector_button_click(&self, type_: DateTimePickerPanelType, direction: SpinDirection) {
        let items = self.items();
        let target = match type_ {
            DateTimePickerPanelType::Month => items.map(|items| items.month_selector),
            DateTimePickerPanelType::Day => items.map(|items| items.day_selector),
            DateTimePickerPanelType::Year => items.map(|items| items.year_selector),
            _ => panic!("The method or operation is not implemented."),
        };

        let Some(target) = target else {
            return;
        };

        match direction {
            SpinDirection::Increase => target.scroll_down(1),
            SpinDirection::Decrease => target.scroll_up(1),
        }
    }

    fn on_year_changed(&self) {
        if self.suppress_update_selection.get() {
            return;
        }
        let Some(items) = self.items() else {
            return;
        };

        let sync_date = self.sync_date.get();
        let max_days = self.calendar.get_days_in_month(items.year_selector.selected_value(), sync_date.month());
        let new_date = DateTimeOffset::new(
            items.year_selector.selected_value(),
            sync_date.month(),
            if sync_date.day() > max_days { max_days } else { sync_date.day() },
            0,
            0,
            0,
            sync_date.offset(),
        );

        self.sync_date.set(new_date);

        // We don't need to update the days if not displaying day, not february
        if !self.day_visible() || new_date.month() != 2 {
            return;
        }

        self.suppress_update_selection.set(true);

        items.day_selector.set_format_date(new_date.date());

        if items.day_selector.maximum_value() != max_days {
            items.day_selector.set_maximum_value(max_days);
        } else {
            items.day_selector.refresh_items();
        }

        self.suppress_update_selection.set(false);
    }

    fn on_day_changed(&self) {
        if self.suppress_update_selection.get() {
            return;
        }
        let Some(items) = self.items() else {
            return;
        };
        let sync_date = self.sync_date.get();
        self.sync_date.set(DateTimeOffset::new(
            sync_date.year(),
            sync_date.month(),
            items.day_selector.selected_value(),
            0,
            0,
            0,
            sync_date.offset(),
        ));
    }

    fn on_month_changed(&self) {
        if self.suppress_update_selection.get() {
            return;
        }
        let Some(items) = self.items() else {
            return;
        };

        let sync_date = self.sync_date.get();
        let max_days = self.calendar.get_days_in_month(sync_date.year(), items.month_selector.selected_value());
        let new_date = DateTimeOffset::new(
            sync_date.year(),
            items.month_selector.selected_value(),
            if sync_date.day() > max_days { max_days } else { sync_date.day() },
            0,
            0,
            0,
            sync_date.offset(),
        );

        if !self.day_visible() {
            self.sync_date.set(new_date);
            return;
        }

        self.suppress_update_selection.set(true);

        items.day_selector.set_format_date(new_date.date());
        self.sync_date.set(new_date);

        if items.day_selector.maximum_value() != max_days {
            items.day_selector.set_maximum_value(max_days);
        } else {
            items.day_selector.refresh_items();
        }

        self.suppress_update_selection.set(false);
    }

    pub(crate) fn get_offset_for_popup(&self) -> f64 {
        let Some(items) = self.items() else {
            return 0.0;
        };

        let accept_dismiss_button_height = items.accept_button.bounds().height;
        -(self.max_height() - accept_dismiss_button_height) / 2.0 - (items.month_selector.item_height() / 2.0)
    }
}
