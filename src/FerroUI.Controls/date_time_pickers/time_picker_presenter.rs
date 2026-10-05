use super::{
    DateTimePickerPanel, DateTimePickerPanelType, PickerPresenterBase, PickerPresenterBaseImpl, PickerPresenterBaseImplExt, TimePicker,
};
use crate::primitives::{TemplateAppliedEventArgs, TemplatedControlImpl, TemplatedControlImplExt};
use crate::{Button, ColumnDefinition, ColumnDefinitions, Control, ControlImpl, Grid, GridLength, Panel, SpinDirection};
use ferroui_base::animation::TimeSpan;
use ferroui_base::input::{
    FocusManager, InputElementImpl, InputElementImplExt, Key, KeyEventArgs, KeyModifiers, KeyboardNavigation, KeyboardNavigationHandler,
    KeyboardNavigationMode, NavigationDirection, NavigationMethod,
};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::utilities::DateTime;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    FerroPropertyChangedEventArgs, Ref, StyledElementImpl, StyledProperty, VisualImpl,
};
use std::cell::RefCell;

const PICKER_CONTAINER_NAME: &str = "PART_PickerContainer";
const ACCEPT_BUTTON_NAME: &str = "PART_AcceptButton";
const DISMISS_BUTTON_NAME: &str = "PART_DismissButton";
const SECOND_SPACER_NAME: &str = "PART_SecondSpacer";
const THIRD_SPACER_NAME: &str = "PART_ThirdSpacer";
const SECOND_HOST_NAME: &str = "PART_SecondHost";
const PERIOD_HOST_NAME: &str = "PART_PeriodHost";
const HOUR_SELECTOR_NAME: &str = "PART_HourSelector";
const MINUTE_SELECTOR_NAME: &str = "PART_MinuteSelector";
const SECOND_SELECTOR_NAME: &str = "PART_SecondSelector";
const PERIOD_SELECTOR_NAME: &str = "PART_PeriodSelector";
const HOUR_UP_BUTTON_NAME: &str = "PART_HourUpButton";
const MINUTE_UP_BUTTON_NAME: &str = "PART_MinuteUpButton";
const SECOND_UP_BUTTON_NAME: &str = "PART_SecondUpButton";
const PERIOD_UP_BUTTON_NAME: &str = "PART_PeriodUpButton";
const HOUR_DOWN_BUTTON_NAME: &str = "PART_HourDownButton";
const MINUTE_DOWN_BUTTON_NAME: &str = "PART_MinuteDownButton";
const SECOND_DOWN_BUTTON_NAME: &str = "PART_SecondDownButton";
const PERIOD_DOWN_BUTTON_NAME: &str = "PART_PeriodDownButton";

/// The parts of the control template.
#[derive(Clone)]
#[allow(dead_code)]
struct TemplateItems {
    picker_container: Ref<Grid>,
    accept_button: Ref<Button>,
    dismiss_button: Option<Ref<Button>>,
    /// The 2nd spacer, not seconds of time.
    second_spacer: Ref<Control>,
    third_spacer: Option<Ref<Control>>,
    second_host: Option<Ref<Panel>>,
    period_host: Ref<Panel>,
    hour_selector: Ref<DateTimePickerPanel>,
    minute_selector: Ref<DateTimePickerPanel>,
    second_selector: Option<Ref<DateTimePickerPanel>>,
    period_selector: Ref<DateTimePickerPanel>,
    hour_up_button: Option<Ref<Button>>,
    minute_up_button: Option<Ref<Button>>,
    second_up_button: Option<Ref<Button>>,
    period_up_button: Option<Ref<Button>>,
    hour_down_button: Option<Ref<Button>>,
    minute_down_button: Option<Ref<Button>>,
    second_down_button: Option<Ref<Button>>,
    period_down_button: Option<Ref<Button>>,
}

/// Defines the presenter used for selecting a time. Intended for use with
/// [`TimePicker`] but can be used independently.
#[repr(C)]
pub struct TimePickerPresenter {
    base: PickerPresenterBase,
    template_items: RefCell<Option<TemplateItems>>,
}

ferro_class!(TimePickerPresenter: PickerPresenterBase);

ferro_class_info!(TimePickerPresenter {
    new: TimePickerPresenter::new,
    markup: {
        attributes: [
            TemplatePart("PART_AcceptButton", type(Ref<Button>), IsRequired = true),
            TemplatePart("PART_DismissButton", type(Ref<Button>)),
            TemplatePart("PART_HourDownButton", type(Ref<Button>)),
            TemplatePart("PART_HourSelector", type(Ref<DateTimePickerPanel>), IsRequired = true),
            TemplatePart("PART_HourUpButton", type(Ref<Button>)),
            TemplatePart("PART_MinuteDownButton", type(Ref<Button>)),
            TemplatePart("PART_MinuteSelector", type(Ref<DateTimePickerPanel>), IsRequired = true),
            TemplatePart("PART_MinuteUpButton", type(Ref<Button>)),
            TemplatePart("PART_SecondDownButton", type(Ref<Button>)),
            TemplatePart("PART_SecondHost", type(Ref<Panel>)),
            TemplatePart("PART_SecondSelector", type(Ref<DateTimePickerPanel>)),
            TemplatePart("PART_SecondUpButton", type(Ref<Button>)),
            TemplatePart("PART_PeriodDownButton", type(Ref<Button>)),
            TemplatePart("PART_PeriodHost", type(Ref<Panel>), IsRequired = true),
            TemplatePart("PART_PeriodSelector", type(Ref<DateTimePickerPanel>), IsRequired = true),
            TemplatePart("PART_PeriodUpButton", type(Ref<Button>)),
            TemplatePart("PART_PickerContainer", type(Ref<Grid>), IsRequired = true),
            TemplatePart("PART_SecondSpacer", type(Ref<Control>), IsRequired = true),
            TemplatePart("PART_ThirdSpacer", type(Ref<Control>)),
        ],
    },
});

ferro_impl_classes!(TimePickerPresenter: StyledElementImpl, VisualImpl, LayoutableImpl, InteractiveImpl, ControlImpl);

impl FerroObjectImpl for TimePickerPresenter {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);

        this.set_current_value(Self::time_property(), DateTime::now().time_of_day());
    }

    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        let property = change.property();
        if property == Self::minute_increment_property().as_property()
            || property == Self::second_increment_property().as_property()
            || property == Self::clock_identifier_property().as_property()
            || property == Self::use_seconds_property().as_property()
            || property == Self::time_property().as_property()
        {
            this.init_picker();
        }
    }
}

impl TemplatedControlImpl for TimePickerPresenter {
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
            picker_container: name_scope.get_as::<Grid>(PICKER_CONTAINER_NAME),
            period_host: name_scope.get_as::<Panel>(PERIOD_HOST_NAME),
            second_host: name_scope.find_as::<Panel>(SECOND_HOST_NAME),

            hour_selector: name_scope.get_as::<DateTimePickerPanel>(HOUR_SELECTOR_NAME),
            minute_selector: name_scope.get_as::<DateTimePickerPanel>(MINUTE_SELECTOR_NAME),
            second_selector: name_scope.find_as::<DateTimePickerPanel>(SECOND_SELECTOR_NAME),
            period_selector: name_scope.get_as::<DateTimePickerPanel>(PERIOD_SELECTOR_NAME),

            second_spacer: name_scope.get_as::<Control>(SECOND_SPACER_NAME),
            third_spacer: name_scope.find_as::<Control>(THIRD_SPACER_NAME),

            accept_button: name_scope.get_as::<Button>(ACCEPT_BUTTON_NAME),

            hour_up_button: selector_button(HOUR_UP_BUTTON_NAME, DateTimePickerPanelType::Hour, SpinDirection::Decrease),
            hour_down_button: selector_button(HOUR_DOWN_BUTTON_NAME, DateTimePickerPanelType::Hour, SpinDirection::Increase),

            minute_up_button: selector_button(
                MINUTE_UP_BUTTON_NAME,
                DateTimePickerPanelType::Minute,
                SpinDirection::Decrease,
            ),
            minute_down_button: selector_button(
                MINUTE_DOWN_BUTTON_NAME,
                DateTimePickerPanelType::Minute,
                SpinDirection::Increase,
            ),

            second_up_button: selector_button(
                SECOND_UP_BUTTON_NAME,
                DateTimePickerPanelType::Second,
                SpinDirection::Decrease,
            ),
            second_down_button: selector_button(
                SECOND_DOWN_BUTTON_NAME,
                DateTimePickerPanelType::Second,
                SpinDirection::Increase,
            ),

            period_up_button: selector_button(
                PERIOD_UP_BUTTON_NAME,
                DateTimePickerPanelType::TimePeriod,
                SpinDirection::Decrease,
            ),
            period_down_button: selector_button(
                PERIOD_DOWN_BUTTON_NAME,
                DateTimePickerPanelType::TimePeriod,
                SpinDirection::Increase,
            ),

            dismiss_button: name_scope.find_as::<Button>(DISMISS_BUTTON_NAME),
        };
        *this.template_items.borrow_mut() = Some(items.clone());

        let weak = weak_this.clone();
        items.accept_button.click(move |_, _| {
            if let Some(this) = weak.upgrade() {
                this.on_accept_button_clicked();
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

impl InputElementImpl for TimePickerPresenter {
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
                this.on_confirmed();
                e.set_handled(true);
            }
            _ => {}
        }
        Self::parent_on_key_down(this, e);
    }
}

impl PickerPresenterBaseImpl for TimePickerPresenter {
    fn on_confirmed(this: &Self) {
        if let Some(items) = this.items() {
            let mut hr = items.hour_selector.selected_value();
            let min = items.minute_selector.selected_value();
            let sec = items.second_selector.as_ref().map_or(0, |selector| selector.selected_value());
            let per = items.period_selector.selected_value();

            if this.clock_identifier() == "12HourClock" {
                hr = if per == 1 {
                    if hr == 12 {
                        12
                    } else {
                        hr + 12
                    }
                } else if per == 0 && hr == 12 {
                    0
                } else {
                    hr
                };
            }

            this.set_current_value(
                Self::time_property(),
                TimeSpan::from_hms(hr, min, if this.use_seconds() { sec } else { 0 }),
            );
        }
        Self::parent_on_confirmed(this);
    }
}

ferro_properties! {
    impl TimePickerPresenter {
        /// Defines the `MinuteIncrement` property.
        pub fn minute_increment_property() -> StyledProperty<i32> {
            TimePicker::minute_increment_property().add_owner::<TimePickerPresenter>()
        }

        /// Defines the `SecondIncrement` property.
        pub fn second_increment_property() -> StyledProperty<i32> {
            TimePicker::second_increment_property().add_owner::<TimePickerPresenter>()
        }

        /// Defines the `ClockIdentifier` property.
        pub fn clock_identifier_property() -> StyledProperty<String> {
            TimePicker::clock_identifier_property().add_owner::<TimePickerPresenter>()
        }

        /// Defines the `UseSeconds` property.
        pub fn use_seconds_property() -> StyledProperty<bool> {
            TimePicker::use_seconds_property().add_owner::<TimePickerPresenter>()
        }

        /// Defines the `Time` property.
        pub fn time_property() -> StyledProperty<TimeSpan> {
            FerroProperty::register::<TimePickerPresenter, _>("Time", TimeSpan::default())
        }
    }
}

impl TimePickerPresenter {
    fn static_constructor() {
        KeyboardNavigation::tab_navigation_property()
            .override_default_value::<TimePickerPresenter>(KeyboardNavigationMode::Cycle);
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: PickerPresenterBase::construct(), template_items: RefCell::new(None) }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The minute increment in the selector.
    pub fn minute_increment(&self) -> i32 {
        self.get_value(Self::minute_increment_property())
    }

    pub fn set_minute_increment(&self, value: i32) {
        self.set_value(Self::minute_increment_property(), value)
    }

    /// The second increment in the selector.
    pub fn second_increment(&self) -> i32 {
        self.get_value(Self::second_increment_property())
    }

    pub fn set_second_increment(&self, value: i32) {
        self.set_value(Self::second_increment_property(), value)
    }

    /// The current clock identifier, either `12HourClock` or `24HourClock`.
    pub fn clock_identifier(&self) -> String {
        self.get_value(Self::clock_identifier_property())
    }

    pub fn set_clock_identifier(&self, value: &str) {
        self.set_value(Self::clock_identifier_property(), value.to_string())
    }

    /// Whether the seconds are selected too.
    pub fn use_seconds(&self) -> bool {
        self.get_value(Self::use_seconds_property())
    }

    pub fn set_use_seconds(&self, value: bool) {
        self.set_value(Self::use_seconds_property(), value)
    }

    /// The current time.
    pub fn time(&self) -> TimeSpan {
        self.get_value(Self::time_property())
    }

    pub fn set_time(&self, value: TimeSpan) {
        self.set_value(Self::time_property(), value)
    }

    fn items(&self) -> Option<TemplateItems> {
        self.template_items.borrow().clone()
    }

    fn init_picker(&self) {
        let Some(items) = self.items() else {
            return;
        };

        let clock12 = self.clock_identifier() == "12HourClock";
        items.hour_selector.set_maximum_value(if clock12 { 12 } else { 23 });
        items.hour_selector.set_minimum_value(if clock12 { 1 } else { 0 });
        items.hour_selector.set_item_format("%h");
        let hr = self.time().hours();
        items.hour_selector.set_selected_value(if !clock12 {
            hr
        } else if hr > 12 {
            hr - 12
        } else if hr == 0 {
            12
        } else {
            hr
        });

        items.minute_selector.set_maximum_value(59);
        items.minute_selector.set_minimum_value(0);
        items.minute_selector.set_increment(self.minute_increment());
        items.minute_selector.set_item_format("mm");
        items.minute_selector.set_selected_value(self.time().minutes());

        if let Some(second_selector) = &items.second_selector {
            second_selector.set_maximum_value(59);
            second_selector.set_minimum_value(0);
            second_selector.set_increment(self.second_increment());
            second_selector.set_item_format("ss");
            second_selector.set_selected_value(self.time().seconds());
        }

        items.period_selector.set_maximum_value(1);
        items.period_selector.set_minimum_value(0);
        items.period_selector.set_selected_value(if hr >= 12 { 1 } else { 0 });

        self.set_grid(&items);
        items.hour_selector.focus_with(NavigationMethod::Pointer, KeyModifiers::NONE);
    }

    fn set_grid(&self, items: &TemplateItems) {
        let use_24_hour_clock = self.clock_identifier() == "24HourClock";
        let use_seconds = self.use_seconds();

        let columns_d = ColumnDefinitions::new();
        columns_d.add(ColumnDefinition::with_width(GridLength::STAR));
        columns_d.add(ColumnDefinition::with_width(GridLength::AUTO));
        columns_d.add(ColumnDefinition::with_width(GridLength::STAR));

        if let (Some(second_host), Some(third_spacer)) = (&items.second_host, &items.third_spacer) {
            if use_seconds {
                columns_d.add(ColumnDefinition::with_width(GridLength::AUTO));
                columns_d.add(ColumnDefinition::with_width(GridLength::STAR));
            }

            items.second_spacer.set_is_visible(use_seconds);
            second_host.set_is_visible(use_seconds);
            third_spacer.set_is_visible(!use_24_hour_clock);
            items.period_host.set_is_visible(!use_24_hour_clock);

            let am_pm_column = if use_seconds { 6 } else { 4 };

            Grid::set_column(&items.second_spacer, if use_seconds { 3 } else { 0 });
            Grid::set_column(second_host, if use_seconds { 4 } else { 0 });
            Grid::set_column(third_spacer, if use_24_hour_clock { 0 } else { am_pm_column - 1 });
            Grid::set_column(&items.period_host, if use_24_hour_clock { 0 } else { am_pm_column });
        } else {
            items.second_spacer.set_is_visible(!use_24_hour_clock);
            items.period_host.set_is_visible(!use_24_hour_clock);
            Grid::set_column(&items.second_spacer, if use_24_hour_clock { 0 } else { 3 });
            Grid::set_column(&items.period_host, if use_24_hour_clock { 0 } else { 4 });
        }

        if !use_24_hour_clock {
            columns_d.add(ColumnDefinition::with_width(GridLength::AUTO));
            columns_d.add(ColumnDefinition::with_width(GridLength::STAR));
        }

        items.picker_container.set_column_definitions(columns_d);
    }

    fn on_dismiss_button_clicked(&self) {
        self.on_dismiss();
    }

    fn on_accept_button_clicked(&self) {
        self.on_confirmed();
    }

    fn on_selector_button_click(&self, type_: DateTimePickerPanelType, direction: SpinDirection) {
        let items = self.items();
        let target = match type_ {
            DateTimePickerPanelType::Hour => items.map(|items| items.hour_selector),
            DateTimePickerPanelType::Minute => items.map(|items| items.minute_selector),
            DateTimePickerPanelType::Second => items.and_then(|items| items.second_selector),
            DateTimePickerPanelType::TimePeriod => items.map(|items| items.period_selector),
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

    pub(crate) fn get_offset_for_popup(&self) -> f64 {
        let Some(items) = self.items() else {
            return 0.0;
        };

        let accept_dismiss_button_height = items.accept_button.bounds().height;
        -(self.max_height() - accept_dismiss_button_height) / 2.0 - (items.hour_selector.item_height() / 2.0)
    }
}
