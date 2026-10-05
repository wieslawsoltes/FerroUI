// (c) Copyright Microsoft Corporation.
// This source is subject to the Microsoft Public License (Ms-PL).
// Please see https://go.microsoft.com/fwlink/?LinkID=131993 for details.
// All other rights reserved.
//
// Ported from the Silverlight Toolkit sources as adapted by the upstream
// project; the license text is in the `NOTICE.md` of this crate.
//
// This file holds both parts of the upstream partial class: the control and
// its properties.

use super::{CalendarDatePickerDateValidationErrorEventArgs, CalendarDatePickerFormat};
use crate::calendar::{
    Calendar, CalendarBlackoutDatesCollection, CalendarDateChangedEventArgs, CalendarMode, CalendarSelectionMode,
    DateTimeHelper,
};
use crate::primitives::{
    Popup, SelectingItemsControl, TemplateAppliedEventArgs, TemplatedControl, TemplatedControlImpl,
};
use crate::{Button, ContentControl, ControlImpl, SelectionChangedEventArgs, TextBox};
use ferroui_base::data::converters::IValueConverter;
use ferroui_base::data::core::ValueType;
use ferroui_base::data::BindingMode;
use ferroui_base::input::navigation::XYFocusHelpers;
use ferroui_base::input::{
    FocusChangedEventArgs, InputElement, InputElementImpl, InputElementImplExt, Key, KeyEventArgs, KeyModifiers,
    MouseButton, NavigationMethod, PointerCaptureLostEventArgs, PointerPressedEventArgs, PointerReleasedEventArgs,
    PointerWheelEventArgs,
};
use ferroui_base::interactivity::{Interactive, InteractiveImpl, RoutingStrategies};
use ferroui_base::layout::{HorizontalAlignment, LayoutableImpl, VerticalAlignment};
use ferroui_base::media::IBrush;
use ferroui_base::reactive::{CompositeDisposable, Disposable, IDisposable, ObservableExt};
use ferroui_base::threading::Dispatcher;
use ferroui_base::utilities::{CalendarWeekRule, DateTime, DayOfWeek, FormatError, HandlerList};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate, BoxedValue, FerroObject,
    FerroObjectExtensions, FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Ref,
    StyledElementImpl, StyledProperty, StyledPropertyOptions, Visual, VisualImpl,
};
use std::cell::{Cell, RefCell};
use std::error::Error;
use std::fmt;
use std::rc::Rc;

const PC_PRESSED: &str = ":pressed";
const PC_FLYOUT_OPEN: &str = ":flyout-open";

const ELEMENT_TEXT_BOX: &str = "PART_TextBox";
const ELEMENT_BUTTON: &str = "PART_Button";
const ELEMENT_POPUP: &str = "PART_Popup";
const ELEMENT_CALENDAR: &str = "PART_Calendar";

/// The error of a date that cannot be selected: what the reference reports
/// with an exception for an argument that is out of range.
#[derive(Debug)]
struct ArgumentOutOfRangeError {
    message: &'static str,
    parameter: &'static str,
}

impl fmt::Display for ArgumentOutOfRangeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} (Parameter '{}')", self.message, self.parameter)
    }
}

impl Error for ArgumentOutOfRangeError {}

/// The outcome of the conversion of a text to a date: the date (or none),
/// or the error of a text that is not in a correct format.
type ParseResult = Result<Option<DateTime>, Rc<dyn Error>>;

/// The length of a text in UTF-16 code units, the unit of the positions of
/// a text box.
fn utf16_length(text: &str) -> i32 {
    text.encode_utf16().count() as i32
}

/// Formats a composite format string that has no arguments: doubled braces
/// are the braces themselves, and any other brace makes the string invalid
/// (the reference formats the text of a date this way, so such a text
/// panics as the reference throws).
fn format_without_arguments(format: &str) -> Result<String, FormatError> {
    let mut result = String::with_capacity(format.len());
    let mut chars = format.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '{' || c == '}' {
            if chars.peek() == Some(&c) {
                chars.next();
            } else {
                return Err(FormatError::new("Input string was not in a correct format."));
            }
        }
        result.push(c);
    }

    Ok(result)
}

/// A control to allow the user to select a date.
#[repr(C)]
pub struct CalendarDatePicker {
    base: TemplatedControl,

    calendar: RefCell<Option<Ref<Calendar>>>,
    /// The subscriptions to the events of the calendar.
    calendar_subscriptions: RefCell<Option<Rc<dyn IDisposable>>>,
    default_text: RefCell<String>,
    drop_down_button: RefCell<Option<Ref<Button>>>,
    /// The subscription to the click event of the drop-down button.
    drop_down_button_click_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    pop_up: RefCell<Option<Ref<Popup>>>,
    /// The subscription to the closed event of the popup.
    pop_up_closed_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    text_box: RefCell<Option<Ref<TextBox>>>,
    /// The subscriptions to the key down and got focus events of the text
    /// box.
    text_box_subscriptions: RefCell<Option<Rc<dyn IDisposable>>>,
    text_box_text_changed_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    button_pointer_pressed_subscription: RefCell<Option<Rc<dyn IDisposable>>>,

    on_open_selected_date: Cell<Option<DateTime>>,
    setting_selected_date: Cell<bool>,

    suspend_text_change_handler: Cell<bool>,
    is_popup_closing: Cell<bool>,
    ignore_button_click: Cell<bool>,
    is_flyout_open: Cell<bool>,
    is_pressed: Cell<bool>,

    blackout_dates: RefCell<Option<CalendarBlackoutDatesCollection>>,

    calendar_closed: HandlerList<dyn Fn()>,
    calendar_opened: HandlerList<dyn Fn()>,
    date_validation_error: HandlerList<dyn Fn(&CalendarDatePickerDateValidationErrorEventArgs)>,
    selected_date_changed: HandlerList<dyn Fn(&SelectionChangedEventArgs)>,
}

ferro_class! {
    CalendarDatePicker: TemplatedControl, virtuals CalendarDatePickerImpl: TemplatedControlImpl {
        /// Raises the `DateValidationError` event.
        fn on_date_validation_error(this, e: &CalendarDatePickerDateValidationErrorEventArgs);
    }
}

ferro_class_info!(CalendarDatePicker {
    new: CalendarDatePicker::new,
    markup: {
        properties: [
            BlackoutDates: Option<CalendarBlackoutDatesCollection> { get: CalendarDatePicker::blackout_dates },
        ],
        attributes: [
            TemplatePart("PART_Button", type(Ref<Button>)),
            TemplatePart("PART_Calendar", type(Ref<Calendar>)),
            TemplatePart("PART_Popup", type(Ref<Popup>)),
            TemplatePart("PART_TextBox", type(Ref<TextBox>)),
            PseudoClasses(":flyout-open", ":pressed"),
        ],
    },
});

ferro_impl_classes!(CalendarDatePicker: StyledElementImpl, VisualImpl, LayoutableImpl, InteractiveImpl, ControlImpl);

impl FerroObjectImpl for CalendarDatePicker {
    /// Initializes a new instance of the [`CalendarDatePicker`] class.
    fn constructed(this: &Self) {
        Self::parent_constructed(this);

        this.set_current_value(
            Self::first_day_of_week_property(),
            DateTimeHelper::get_current_date_format().first_day_of_week(),
        );
        this.default_text.borrow_mut().clear();
        this.set_current_value(Self::display_date_property(), DateTime::today());
    }

    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        let property = change.property();

        // CustomDateFormatString
        if property == Self::custom_date_format_string_property().as_property() {
            if this.selected_date_format() == CalendarDatePickerFormat::Custom {
                this.on_date_format_changed();
            }
        }
        // IsDropDownOpen
        else if property == Self::is_drop_down_open_property().as_property() {
            let (old_value, new_value) = change.get_old_and_new_value::<bool>();

            let pop_up = this.pop_up.borrow().clone();
            if let Some(pop_up) = pop_up.filter(|pop_up| pop_up.child().is_some()) {
                if new_value != old_value {
                    let calendar = this.calendar().expect("the date picker has a calendar");
                    if calendar.display_mode() != CalendarMode::Month {
                        calendar.set_display_mode(CalendarMode::Month);
                    }

                    if new_value {
                        this.open_drop_down();
                    } else {
                        pop_up.set_is_open(false);
                        this.is_flyout_open.set(pop_up.is_open());
                        this.is_pressed.set(false);

                        this.update_pseudo_classes();
                        this.on_calendar_closed();
                    }
                }
            }
        }
        // SelectedDate
        else if property == Self::selected_date_property().as_property() {
            let (removed_date, added_date) = change.get_old_and_new_value::<Option<DateTime>>();

            if let Some(day) = this.selected_date() {
                // When the SelectedDateProperty change is done from
                // OnTextPropertyChanged method, two-way binding breaks if
                // BeginInvoke is not used:
                let picker = this.to_ref();
                let _ = Dispatcher::ui_thread().invoke_async_local(move || {
                    picker.setting_selected_date.set(true);
                    picker.set_current_value(Self::text_property(), picker.date_time_to_string(day));
                    picker.setting_selected_date.set(false);
                    picker.on_date_selected(added_date, removed_date);
                });

                // When DatePickerDisplayDateFlag is TRUE, the SelectedDate
                // change is coming from the Calendar UI itself, so, we
                // shouldn't change the DisplayDate since it will automatically
                // be changed by the Calendar
                let calendar = this.calendar();
                let display_date = this.display_date();
                if (day.month() != display_date.month() || day.year() != display_date.year())
                    && !calendar.as_ref().is_some_and(|calendar| calendar.calendar_date_picker_display_date_flag())
                {
                    this.set_current_value(Self::display_date_property(), day);
                }

                if let Some(calendar) = calendar {
                    calendar.set_calendar_date_picker_display_date_flag(false);
                }
            } else {
                this.setting_selected_date.set(true);
                this.set_water_mark_text();
                this.setting_selected_date.set(false);
                this.on_date_selected(added_date, removed_date);
            }
        }
        // SelectedDateFormat
        else if property == Self::selected_date_format_property().as_property() {
            this.on_date_format_changed();
        }
        // Text
        else if property == Self::text_property().as_property() {
            let new_value = change.get_new_value::<Option<String>>();

            if !this.suspend_text_change_handler.get() {
                if let Some(new_value) = new_value {
                    let text_box = this.text_box();
                    match text_box {
                        Some(text_box) => text_box.set_text(Some(&new_value)),
                        None => *this.default_text.borrow_mut() = new_value,
                    }

                    if !this.setting_selected_date.get() {
                        this.set_selected_date_from_text();
                    }
                } else if !this.setting_selected_date.get() {
                    this.setting_selected_date.set(true);
                    this.set_current_value(Self::selected_date_property(), None);
                    this.setting_selected_date.set(false);
                }
            } else {
                this.set_water_mark_text();
            }
        }

        Self::parent_on_property_changed(this, change);
    }
}

impl InputElementImpl for CalendarDatePicker {
    fn on_pointer_pressed(this: &Self, e: &PointerPressedEventArgs) {
        Self::parent_on_pointer_pressed(this, e);

        if e.get_current_point(Some(this)).properties.is_left_button_pressed {
            e.set_handled(true);

            this.ignore_button_click.set(this.is_popup_closing.get());

            this.is_pressed.set(true);
            this.update_pseudo_classes();
        }
    }

    fn on_pointer_released(this: &Self, e: &PointerReleasedEventArgs) {
        Self::parent_on_pointer_released(this, e);

        if this.is_pressed.get() && e.initial_press_mouse_button() == MouseButton::Left {
            e.set_handled(true);

            if !this.ignore_button_click.get() {
                this.toggle_pop_up();
            } else {
                this.ignore_button_click.set(false);
            }

            this.is_pressed.set(false);
            this.update_pseudo_classes();
        }
    }

    fn on_pointer_capture_lost(this: &Self, e: &PointerCaptureLostEventArgs) {
        Self::parent_on_pointer_capture_lost(this, e);

        this.is_pressed.set(false);
        this.update_pseudo_classes();
    }

    fn on_pointer_wheel_changed(this: &Self, e: &PointerWheelEventArgs) {
        Self::parent_on_pointer_wheel_changed(this, e);

        if !e.handled() {
            if let (Some(selected_date), Some(calendar)) = (this.selected_date(), this.calendar()) {
                let new_date = DateTimeHelper::add_days(selected_date, if e.delta().y > 0.0 { -1 } else { 1 });
                if let Some(new_date) = new_date {
                    if Calendar::is_valid_date_selection(&calendar, Some(new_date)) {
                        this.set_current_value(Self::selected_date_property(), Some(new_date));
                        e.set_handled(true);
                    }
                }
            }
        }
    }

    fn on_got_focus(this: &Self, e: &FocusChangedEventArgs) {
        Self::parent_on_got_focus(this, e);

        let text_box = this.text_box();
        if let Some(text_box) = text_box {
            if this.is_enabled() && e.source().is_some_and(|source| source.ptr_eq(&this.to_ref())) {
                text_box.focus_with(e.navigation_method, KeyModifiers::NONE);

                if e.navigation_method == NavigationMethod::Tab {
                    let text = text_box.text();
                    if let Some(text) = text.filter(|text| !text.is_empty()) {
                        text_box.set_selection_start(0);
                        text_box.set_selection_end(utf16_length(&text));
                    }
                }
            }
        }
    }

    fn on_lost_focus(this: &Self, e: &FocusChangedEventArgs) {
        Self::parent_on_lost_focus(this, e);

        this.is_pressed.set(false);
        this.update_pseudo_classes();

        this.set_selected_date_from_text();
    }

    fn on_key_up(this: &Self, e: &KeyEventArgs) {
        let key = e.key;

        if (key == Key::Space || key == Key::Enter) && this.is_effectively_enabled() {
            // Since the TextBox is used for direct date entry,
            // it isn't supported to open the popup/flyout using these keys.
            // Other controls open the popup/flyout here.
        } else if key == Key::Down
            && e.key_modifiers.contains(KeyModifiers::ALT)
            && this.is_effectively_enabled()
            && !XYFocusHelpers::is_allowed_xy_navigation_mode(this, Some(e.key_device_type))
        {
            // It is only possible to open the popup using these keys.
            // This is important as the down key is handled by calendar.
            // If down also closed the popup, the date would move 1 week
            // and then close the popup. This isn't user friendly at all.
            // (calendar doesn't mark as handled either).
            // The Escape key will still close the popup.
            if !this.is_drop_down_open() {
                e.set_handled(true);

                if !this.ignore_button_click.get() {
                    this.toggle_pop_up();
                } else {
                    this.ignore_button_click.set(false);
                }

                this.update_pseudo_classes();
            }
        }

        Self::parent_on_key_up(this, e);
    }
}

impl TemplatedControlImpl for CalendarDatePicker {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        if let Some(subscriptions) = this.calendar_subscriptions.take() {
            subscriptions.dispose();
        }
        let calendar = e.name_scope().find_as::<Calendar>(ELEMENT_CALENDAR);
        *this.calendar.borrow_mut() = calendar.clone();
        if let Some(calendar) = &calendar {
            calendar.set_selection_mode(CalendarSelectionMode::SingleDate);

            let subscriptions = CompositeDisposable::new();
            let weak = this.to_ref().downgrade();
            subscriptions.add(calendar.day_button_mouse_up(move |_| {
                if let Some(this) = weak.upgrade() {
                    this.calendar_day_button_mouse_up();
                }
            }));
            let weak = this.to_ref().downgrade();
            subscriptions.add(calendar.display_date_changed(move |e| {
                if let Some(this) = weak.upgrade() {
                    this.calendar_display_date_changed(e);
                }
            }));
            let weak = this.to_ref().downgrade();
            subscriptions.add(calendar.selected_dates_changed(move |e| {
                if let Some(this) = weak.upgrade() {
                    this.calendar_selected_dates_changed(e);
                }
            }));
            subscriptions.add(calendar.add_disposable_handler(InputElement::pointer_released_event(), |_, e| {
                Self::calendar_pointer_released(e);
            }));
            let weak = this.to_ref().downgrade();
            subscriptions.add(calendar.add_disposable_handler(InputElement::key_down_event(), move |sender, e| {
                if let Some(this) = weak.upgrade() {
                    this.calendar_key_down(sender, e);
                }
            }));
            *this.calendar_subscriptions.borrow_mut() = Some(Rc::new(subscriptions));

            let current_blackout_days = this.blackout_dates();
            let blackout_dates = calendar.blackout_dates();
            *this.blackout_dates.borrow_mut() = Some(blackout_dates.clone());
            if let Some(current_blackout_days) = current_blackout_days {
                for range in current_blackout_days.to_vec() {
                    blackout_dates.add(range);
                }
            }
        }

        let old_pop_up = this.pop_up.take();
        if let Some(old_pop_up) = old_pop_up {
            old_pop_up.set_child(None);
            if let Some(subscription) = this.pop_up_closed_subscription.take() {
                subscription.dispose();
            }
        }
        let pop_up = e.name_scope().find_as::<Popup>(ELEMENT_POPUP);
        *this.pop_up.borrow_mut() = pop_up.clone();
        if let Some(pop_up) = &pop_up {
            let weak = this.to_ref().downgrade();
            let closed = pop_up.closed(move || {
                if let Some(this) = weak.upgrade() {
                    this.pop_up_closed();
                }
            });
            *this.pop_up_closed_subscription.borrow_mut() = Some(closed);

            if this.is_drop_down_open() {
                this.open_drop_down();
            }
        }

        if this.drop_down_button.take().is_some() {
            if let Some(subscription) = this.drop_down_button_click_subscription.take() {
                subscription.dispose();
            }
            if let Some(subscription) = this.button_pointer_pressed_subscription.take() {
                subscription.dispose();
            }
        }
        let drop_down_button = e.name_scope().find_as::<Button>(ELEMENT_BUTTON);
        *this.drop_down_button.borrow_mut() = drop_down_button.clone();
        if let Some(drop_down_button) = &drop_down_button {
            let weak = this.to_ref().downgrade();
            let click = drop_down_button.add_disposable_handler(Button::click_event(), move |_, _| {
                if let Some(this) = weak.upgrade() {
                    this.drop_down_button_click();
                }
            });
            *this.drop_down_button_click_subscription.borrow_mut() = Some(click);

            let routes = RoutingStrategies::DIRECT | RoutingStrategies::BUBBLE;
            let weak = this.to_ref().downgrade();
            let pressed = drop_down_button.add_disposable_handler_with(
                InputElement::pointer_pressed_event(),
                move |_, e| {
                    if let Some(this) = weak.upgrade() {
                        this.drop_down_button_pointer_pressed(e);
                    }
                },
                routes,
                true,
            );
            let weak = this.to_ref().downgrade();
            let released = drop_down_button.add_disposable_handler_with(
                InputElement::pointer_released_event(),
                move |_, _| {
                    if let Some(this) = weak.upgrade() {
                        this.drop_down_button_pointer_released();
                    }
                },
                routes,
                true,
            );
            *this.button_pointer_pressed_subscription.borrow_mut() =
                Some(Rc::new(CompositeDisposable::from_disposables([pressed, released])));
        }

        if this.text_box.take().is_some() {
            if let Some(subscriptions) = this.text_box_subscriptions.take() {
                subscriptions.dispose();
            }
            if let Some(subscription) = this.text_box_text_changed_subscription.take() {
                subscription.dispose();
            }
        }
        let text_box = e.name_scope().find_as::<TextBox>(ELEMENT_TEXT_BOX);
        *this.text_box.borrow_mut() = text_box.clone();

        if this.selected_date().is_none() {
            this.set_water_mark_text();
        }

        if let Some(text_box) = &text_box {
            let subscriptions = CompositeDisposable::new();
            let weak = this.to_ref().downgrade();
            subscriptions.add(text_box.add_disposable_handler(InputElement::key_down_event(), move |_, e| {
                if let Some(this) = weak.upgrade() {
                    this.text_box_key_down(e);
                }
            }));
            let weak = this.to_ref().downgrade();
            subscriptions.add(text_box.add_disposable_handler(InputElement::got_focus_event(), move |_, _| {
                if let Some(this) = weak.upgrade() {
                    this.text_box_got_focus();
                }
            }));
            *this.text_box_subscriptions.borrow_mut() = Some(Rc::new(subscriptions));

            let weak = this.to_ref().downgrade();
            let object: &FerroObject = text_box;
            let subscription =
                FerroObjectExtensions::get_observable(object, TextBox::text_property()).subscribe_fn(move |_| {
                    if let Some(this) = weak.upgrade() {
                        this.text_box_text_changed();
                    }
                });
            *this.text_box_text_changed_subscription.borrow_mut() = Some(subscription);

            if let Some(selected_date) = this.selected_date() {
                text_box.set_text(this.date_time_to_string(selected_date).as_deref());
            } else {
                let default_text = this.default_text.borrow().clone();
                if !default_text.is_empty() {
                    text_box.set_text(Some(&default_text));
                    this.set_selected_date_from_text();
                }
            }
        }

        this.update_pseudo_classes();
    }

    // AUTOMATION-SEAM: OnCreateAutomationPeer -> CalendarDatePickerAutomationPeer (automation pass)
}

impl CalendarDatePickerImpl for CalendarDatePicker {
    fn on_date_validation_error(this: &Self, e: &CalendarDatePickerDateValidationErrorEventArgs) {
        for (_, handler) in this.date_validation_error.snapshot().iter() {
            handler(e);
        }
    }
}

ferro_properties! {
    impl CalendarDatePicker {
        /// Defines the `DisplayDate` property.
        pub fn display_date_property() -> StyledProperty<DateTime> {
            FerroProperty::register::<CalendarDatePicker, _>("DisplayDate", DateTime::MIN_VALUE)
        }

        /// Defines the `DisplayDateStart` property.
        pub fn display_date_start_property() -> StyledProperty<Option<DateTime>> {
            FerroProperty::register::<CalendarDatePicker, _>("DisplayDateStart", None)
        }

        /// Defines the `DisplayDateEnd` property.
        pub fn display_date_end_property() -> StyledProperty<Option<DateTime>> {
            FerroProperty::register::<CalendarDatePicker, _>("DisplayDateEnd", None)
        }

        /// Defines the `FirstDayOfWeek` property.
        pub fn first_day_of_week_property() -> StyledProperty<DayOfWeek> {
            FerroProperty::register::<CalendarDatePicker, _>("FirstDayOfWeek", DayOfWeek::Sunday)
        }

        /// Defines the `IsDropDownOpen` property.
        pub fn is_drop_down_open_property() -> StyledProperty<bool> {
            FerroProperty::register::<CalendarDatePicker, _>("IsDropDownOpen", false)
        }

        /// Defines the `IsTodayHighlighted` property.
        pub fn is_today_highlighted_property() -> StyledProperty<bool> {
            FerroProperty::register::<CalendarDatePicker, _>("IsTodayHighlighted", false)
        }

        /// Defines the `SelectedDate` property.
        pub fn selected_date_property() -> StyledProperty<Option<DateTime>> {
            FerroProperty::register_with::<CalendarDatePicker, _>(
                "SelectedDate",
                StyledPropertyOptions::new(None).enable_data_validation(true).default_binding_mode(BindingMode::TwoWay),
            )
        }

        /// Defines the `SelectedDateFormat` property.
        pub fn selected_date_format_property() -> StyledProperty<CalendarDatePickerFormat> {
            FerroProperty::register_with::<CalendarDatePicker, _>(
                "SelectedDateFormat",
                StyledPropertyOptions::new(CalendarDatePickerFormat::Short)
                    .validate(CalendarDatePicker::is_valid_selected_date_format),
            )
        }

        /// Defines the `CustomDateFormatString` property.
        pub fn custom_date_format_string_property() -> StyledProperty<String> {
            FerroProperty::register_with::<CalendarDatePicker, _>(
                "CustomDateFormatString",
                StyledPropertyOptions::new("d".to_string()).validate(CalendarDatePicker::is_valid_date_format_string),
            )
        }

        /// Defines the `TextConverter` property.
        pub fn text_converter_property() -> StyledProperty<Option<Rc<dyn IValueConverter>>> {
            FerroProperty::register_with::<CalendarDatePicker, _>(
                "TextConverter",
                StyledPropertyOptions::new(None).default_binding_mode(BindingMode::OneWay),
            )
        }

        /// Defines the `Text` property.
        pub fn text_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<CalendarDatePicker, _>("Text", None)
        }

        /// Defines the `PlaceholderText` property.
        pub fn placeholder_text_property() -> StyledProperty<Option<String>> {
            TextBox::placeholder_text_property().add_owner::<CalendarDatePicker>()
        }

        /// Defines the `UseFloatingPlaceholder` property.
        pub fn use_floating_placeholder_property() -> StyledProperty<bool> {
            TextBox::use_floating_placeholder_property().add_owner::<CalendarDatePicker>()
        }

        /// Defines the `PlaceholderForeground` property.
        pub fn placeholder_foreground_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
            TextBox::placeholder_foreground_property().add_owner::<CalendarDatePicker>()
        }

        /// Defines the `HorizontalContentAlignment` property.
        pub fn horizontal_content_alignment_property() -> StyledProperty<HorizontalAlignment> {
            ContentControl::horizontal_content_alignment_property().add_owner::<CalendarDatePicker>()
        }

        /// Defines the `VerticalContentAlignment` property.
        pub fn vertical_content_alignment_property() -> StyledProperty<VerticalAlignment> {
            ContentControl::vertical_content_alignment_property().add_owner::<CalendarDatePicker>()
        }

        /// Defines the `IsWeekNumberVisible` property.
        pub fn is_week_number_visible_property() -> StyledProperty<bool> {
            Calendar::is_week_number_visible_property().add_owner::<CalendarDatePicker>()
        }

        /// Defines the `WeekNumberRule` property.
        pub fn week_number_rule_property() -> StyledProperty<CalendarWeekRule> {
            Calendar::week_number_rule_property().add_owner::<CalendarDatePicker>()
        }
    }
}

impl CalendarDatePicker {
    /// Defines the `Watermark` property.
    #[deprecated(note = "Use placeholder_text_property instead.")]
    pub fn watermark_property() -> &'static StyledProperty<Option<String>> {
        Self::placeholder_text_property()
    }

    /// Defines the `UseFloatingWatermark` property.
    #[deprecated(note = "Use use_floating_placeholder_property instead.")]
    pub fn use_floating_watermark_property() -> &'static StyledProperty<bool> {
        Self::use_floating_placeholder_property()
    }

    /// Defines the `WatermarkForeground` property.
    #[deprecated(note = "Use placeholder_foreground_property instead.")]
    pub fn watermark_foreground_property() -> &'static StyledProperty<Option<Rc<dyn IBrush>>> {
        Self::placeholder_foreground_property()
    }

    fn static_constructor() {
        InputElement::focusable_property().override_default_value::<CalendarDatePicker>(true);
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: TemplatedControl::construct(),
            calendar: RefCell::new(None),
            calendar_subscriptions: RefCell::new(None),
            default_text: RefCell::new(String::new()),
            drop_down_button: RefCell::new(None),
            drop_down_button_click_subscription: RefCell::new(None),
            pop_up: RefCell::new(None),
            pop_up_closed_subscription: RefCell::new(None),
            text_box: RefCell::new(None),
            text_box_subscriptions: RefCell::new(None),
            text_box_text_changed_subscription: RefCell::new(None),
            button_pointer_pressed_subscription: RefCell::new(None),
            on_open_selected_date: Cell::new(None),
            setting_selected_date: Cell::new(false),
            suspend_text_change_handler: Cell::new(false),
            is_popup_closing: Cell::new(false),
            ignore_button_click: Cell::new(false),
            is_flyout_open: Cell::new(false),
            is_pressed: Cell::new(false),
            blackout_dates: RefCell::new(None),
            calendar_closed: HandlerList::new(),
            calendar_opened: HandlerList::new(),
            date_validation_error: HandlerList::new(),
            selected_date_changed: HandlerList::new(),
        }
    }

    /// Initializes a new instance of the [`CalendarDatePicker`] class.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    fn calendar(&self) -> Option<Ref<Calendar>> {
        self.calendar.borrow().clone()
    }

    fn text_box(&self) -> Option<Ref<TextBox>> {
        self.text_box.borrow().clone()
    }

    // --- properties -----------------------------------------------------------

    /// Gets a collection of dates that are marked as not selectable: the
    /// collection of the calendar of the template, `None` before a
    /// template with a calendar is applied.
    pub fn blackout_dates(&self) -> Option<CalendarBlackoutDatesCollection> {
        self.blackout_dates.borrow().clone()
    }

    /// Gets or sets the date to display.
    pub fn display_date(&self) -> DateTime {
        self.get_value(Self::display_date_property())
    }

    pub fn set_display_date(&self, value: DateTime) {
        self.set_value(Self::display_date_property(), value)
    }

    /// Gets or sets the first date to be displayed.
    pub fn display_date_start(&self) -> Option<DateTime> {
        self.get_value(Self::display_date_start_property())
    }

    pub fn set_display_date_start(&self, value: Option<DateTime>) {
        self.set_value(Self::display_date_start_property(), value)
    }

    /// Gets or sets the last date to be displayed.
    pub fn display_date_end(&self) -> Option<DateTime> {
        self.get_value(Self::display_date_end_property())
    }

    pub fn set_display_date_end(&self, value: Option<DateTime>) {
        self.set_value(Self::display_date_end_property(), value)
    }

    /// Gets or sets the day that is considered the beginning of the week.
    pub fn first_day_of_week(&self) -> DayOfWeek {
        self.get_value(Self::first_day_of_week_property())
    }

    pub fn set_first_day_of_week(&self, value: DayOfWeek) {
        self.set_value(Self::first_day_of_week_property(), value)
    }

    /// Gets or sets a value indicating whether the drop-down calendar is
    /// open or closed.
    pub fn is_drop_down_open(&self) -> bool {
        self.get_value(Self::is_drop_down_open_property())
    }

    pub fn set_is_drop_down_open(&self, value: bool) {
        self.set_value(Self::is_drop_down_open_property(), value)
    }

    /// Gets or sets a value indicating whether the current date will be
    /// highlighted.
    pub fn is_today_highlighted(&self) -> bool {
        self.get_value(Self::is_today_highlighted_property())
    }

    pub fn set_is_today_highlighted(&self, value: bool) {
        self.set_value(Self::is_today_highlighted_property(), value)
    }

    /// Gets or sets the currently selected date. The default is `None`.
    ///
    /// Setting it panics when the date is in the blackout dates.
    pub fn selected_date(&self) -> Option<DateTime> {
        self.get_value(Self::selected_date_property())
    }

    pub fn set_selected_date(&self, value: Option<DateTime>) {
        self.set_value(Self::selected_date_property(), value)
    }

    /// Gets or sets the format that is used to display the selected date.
    pub fn selected_date_format(&self) -> CalendarDatePickerFormat {
        self.get_value(Self::selected_date_format_property())
    }

    pub fn set_selected_date_format(&self, value: CalendarDatePickerFormat) {
        self.set_value(Self::selected_date_format_property(), value)
    }

    /// Gets or sets the format string of the custom date format.
    pub fn custom_date_format_string(&self) -> String {
        self.get_value(Self::custom_date_format_string_property())
    }

    pub fn set_custom_date_format_string(&self, value: &str) {
        self.set_value(Self::custom_date_format_string_property(), value.to_string())
    }

    /// Gets or sets the converter between a date and its text. With a
    /// converter the date format and the custom format string are not
    /// used.
    pub fn text_converter(&self) -> Option<Rc<dyn IValueConverter>> {
        self.get_value(Self::text_converter_property())
    }

    pub fn set_text_converter(&self, value: Option<Rc<dyn IValueConverter>>) {
        self.set_value(Self::text_converter_property(), value)
    }

    /// Gets or sets the text that is displayed by the date picker.
    ///
    /// Setting a text that is not in an acceptable format raises the
    /// `DateValidationError` event (and panics when a handler asks for the
    /// exception to be thrown).
    pub fn text(&self) -> Option<String> {
        self.get_value(Self::text_property())
    }

    pub fn set_text(&self, value: Option<&str>) {
        self.set_value(Self::text_property(), value.map(str::to_owned))
    }

    /// Gets or sets the placeholder text.
    pub fn placeholder_text(&self) -> Option<String> {
        self.get_value(Self::placeholder_text_property())
    }

    pub fn set_placeholder_text(&self, value: Option<&str>) {
        self.set_value(Self::placeholder_text_property(), value.map(str::to_owned))
    }

    /// Gets or sets the placeholder text.
    #[deprecated(note = "Use placeholder_text instead.")]
    pub fn watermark(&self) -> Option<String> {
        self.placeholder_text()
    }

    #[deprecated(note = "Use set_placeholder_text instead.")]
    pub fn set_watermark(&self, value: Option<&str>) {
        self.set_placeholder_text(value)
    }

    /// Gets or sets a value indicating whether the placeholder floats
    /// above the text.
    pub fn use_floating_placeholder(&self) -> bool {
        self.get_value(Self::use_floating_placeholder_property())
    }

    pub fn set_use_floating_placeholder(&self, value: bool) {
        self.set_value(Self::use_floating_placeholder_property(), value)
    }

    /// Gets or sets a value indicating whether the placeholder floats
    /// above the text.
    #[deprecated(note = "Use use_floating_placeholder instead.")]
    pub fn use_floating_watermark(&self) -> bool {
        self.use_floating_placeholder()
    }

    #[deprecated(note = "Use set_use_floating_placeholder instead.")]
    pub fn set_use_floating_watermark(&self, value: bool) {
        self.set_use_floating_placeholder(value)
    }

    /// Gets or sets the brush used for the foreground color of the
    /// placeholder text.
    pub fn placeholder_foreground(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::placeholder_foreground_property())
    }

    pub fn set_placeholder_foreground(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::placeholder_foreground_property(), value)
    }

    /// Gets or sets the brush used for the foreground color of the
    /// placeholder text.
    #[deprecated(note = "Use placeholder_foreground instead.")]
    pub fn watermark_foreground(&self) -> Option<Rc<dyn IBrush>> {
        self.placeholder_foreground()
    }

    #[deprecated(note = "Use set_placeholder_foreground instead.")]
    pub fn set_watermark_foreground(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_placeholder_foreground(value)
    }

    /// Gets or sets the horizontal alignment of the content within the
    /// control.
    pub fn horizontal_content_alignment(&self) -> HorizontalAlignment {
        self.get_value(Self::horizontal_content_alignment_property())
    }

    pub fn set_horizontal_content_alignment(&self, value: HorizontalAlignment) {
        self.set_value(Self::horizontal_content_alignment_property(), value)
    }

    /// Gets or sets the vertical alignment of the content within the
    /// control.
    pub fn vertical_content_alignment(&self) -> VerticalAlignment {
        self.get_value(Self::vertical_content_alignment_property())
    }

    pub fn set_vertical_content_alignment(&self, value: VerticalAlignment) {
        self.set_value(Self::vertical_content_alignment_property(), value)
    }

    /// Gets or sets a value indicating whether week numbers are shown in
    /// the calendar popup.
    pub fn is_week_number_visible(&self) -> bool {
        self.get_value(Self::is_week_number_visible_property())
    }

    pub fn set_is_week_number_visible(&self, value: bool) {
        self.set_value(Self::is_week_number_visible_property(), value)
    }

    /// Gets or sets the rule used to determine the first week of the year
    /// for week number display. The default is taken from the culture that
    /// was current when the calendar class was initialised.
    pub fn week_number_rule(&self) -> CalendarWeekRule {
        self.get_value(Self::week_number_rule_property())
    }

    pub fn set_week_number_rule(&self, value: CalendarWeekRule) {
        self.set_value(Self::week_number_rule_property(), value)
    }

    // --- events ---------------------------------------------------------------

    fn subscribe<F: ?Sized + 'static>(
        &self,
        list: fn(&CalendarDatePicker) -> &HandlerList<F>,
        handler: Rc<F>,
    ) -> Rc<dyn IDisposable> {
        let token = list(self).add(handler);
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                list(&this).remove(token);
            }
        })
    }

    /// Occurs when the drop-down [`Calendar`] is closed.
    pub fn calendar_closed(&self, handler: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        self.subscribe::<dyn Fn()>(|picker| &picker.calendar_closed, Rc::new(handler))
    }

    /// Occurs when the drop-down [`Calendar`] is opened.
    pub fn calendar_opened(&self, handler: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        self.subscribe::<dyn Fn()>(|picker| &picker.calendar_opened, Rc::new(handler))
    }

    /// Occurs when the text is assigned a value that cannot be interpreted
    /// as a date.
    pub fn date_validation_error(
        &self,
        handler: impl Fn(&CalendarDatePickerDateValidationErrorEventArgs) + 'static,
    ) -> Rc<dyn IDisposable> {
        self.subscribe::<dyn Fn(&CalendarDatePickerDateValidationErrorEventArgs)>(
            |picker| &picker.date_validation_error,
            Rc::new(handler),
        )
    }

    /// Occurs when the selected date is changed.
    pub fn selected_date_changed(
        &self,
        handler: impl Fn(&SelectionChangedEventArgs) + 'static,
    ) -> Rc<dyn IDisposable> {
        self.subscribe::<dyn Fn(&SelectionChangedEventArgs)>(|picker| &picker.selected_date_changed, Rc::new(handler))
    }

    // --- the control ------------------------------------------------------------

    /// Updates the visual state of the control by applying the latest
    /// pseudo classes.
    pub fn update_pseudo_classes(&self) {
        self.pseudo_classes().set(PC_FLYOUT_OPEN, self.is_flyout_open.get());
        self.pseudo_classes().set(PC_PRESSED, self.is_pressed.get());
    }

    fn on_date_format_changed(&self) {
        if let Some(text_box) = self.text_box() {
            if let Some(selected_date) = self.selected_date() {
                self.set_current_value(Self::text_property(), self.date_time_to_string(selected_date));
            } else {
                let text = text_box.text().unwrap_or_default();
                if text.is_empty() {
                    self.set_water_mark_text();
                } else if let Some(date) = self.parse_text(&text) {
                    let s = self.date_time_to_string(date);
                    self.set_current_value(Self::text_property(), s);
                }
            }
        }
    }

    fn on_date_selected(&self, added_date: Option<DateTime>, removed_date: Option<DateTime>) {
        let handlers = self.selected_date_changed.snapshot();
        if !handlers.is_empty() {
            let boxed = |date: Option<DateTime>| -> Vec<Option<BoxedValue>> {
                date.map(|date| Some(Rc::new(date) as BoxedValue)).into_iter().collect()
            };

            let e = SelectionChangedEventArgs::new(
                Some(SelectingItemsControl::selection_changed_event()),
                boxed(removed_date),
                boxed(added_date),
            );
            for (_, handler) in handlers.iter() {
                handler(&e);
            }
        }
    }

    fn on_calendar_closed(&self) {
        for (_, handler) in self.calendar_closed.snapshot().iter() {
            handler();
        }
    }

    fn on_calendar_opened(&self) {
        for (_, handler) in self.calendar_opened.snapshot().iter() {
            handler();
        }
    }

    fn calendar_day_button_mouse_up(&self) {
        self.focus();
        self.set_current_value(Self::is_drop_down_open_property(), false);
    }

    fn calendar_display_date_changed(&self, e: &CalendarDateChangedEventArgs) {
        if e.added_date() != Some(self.display_date()) {
            self.set_value(Self::display_date_property(), e.added_date().expect("Nullable object must have a value."));
        }
    }

    fn calendar_selected_dates_changed(&self, e: &SelectionChangedEventArgs) {
        debug_assert!(e.added_items().len() < 2, "There should be less than 2 AddedItems!");

        let added_date = e.added_items().first().map(|item| {
            item.as_ref()
                .and_then(|item| item.downcast_ref::<DateTime>().copied())
                .expect("the added item of the calendar is a date")
        });
        let selected_date = self.selected_date();

        match (added_date, selected_date) {
            (Some(added_date), Some(selected_date)) if DateTime::compare(added_date, selected_date) != 0 => {
                self.set_current_value(Self::selected_date_property(), Some(added_date));
            }
            (None, _) => {
                self.set_current_value(Self::selected_date_property(), None);
            }
            (Some(added_date), None) => {
                self.set_current_value(Self::selected_date_property(), Some(added_date));
            }
            _ => {}
        }
    }

    fn calendar_pointer_released(e: &PointerReleasedEventArgs) {
        if e.initial_press_mouse_button() == MouseButton::Left {
            e.set_handled(true);
        }
    }

    fn calendar_key_down(&self, sender: &Interactive, e: &KeyEventArgs) {
        if !e.handled()
            && sender.to_ref().cast::<Calendar>().is_some_and(|calendar| calendar.display_mode() == CalendarMode::Month)
            && (e.key == Key::Enter || e.key == Key::Space || e.key == Key::Escape)
        {
            self.focus();
            self.set_current_value(Self::is_drop_down_open_property(), false);

            if e.key == Key::Escape {
                self.set_current_value(Self::selected_date_property(), self.on_open_selected_date.get());
            }
        }
    }

    fn text_box_got_focus(&self) {
        self.set_current_value(Self::is_drop_down_open_property(), false);
    }

    fn text_box_key_down(&self, e: &KeyEventArgs) {
        if !e.handled() {
            e.set_handled(self.process_date_picker_key(e));
        }
    }

    fn text_box_text_changed(&self) {
        if let Some(text_box) = self.text_box() {
            self.suspend_text_change_handler.set(true);
            self.set_current_value(Self::text_property(), text_box.text());
            self.suspend_text_change_handler.set(false);
        }
    }

    fn drop_down_button_pointer_pressed(&self, e: &PointerPressedEventArgs) {
        let drop_down_button = self.drop_down_button.borrow().clone();
        let drop_down_button_visual: Option<&Visual> = match &drop_down_button {
            Some(button) => Some(button),
            None => None,
        };
        if self.is_flyout_open.get()
            && drop_down_button.as_ref().is_some_and(|button| button.is_effectively_enabled())
            && e.get_current_point(drop_down_button_visual).properties.is_left_button_pressed
        {
            // When a flyout is open with OverlayDismissEventPassThrough enabled and the drop-down button
            // is pressed, close the flyout
            self.ignore_button_click.set(true);
            e.set_handled(true);
            self.toggle_pop_up();
        } else {
            self.ignore_button_click.set(self.is_popup_closing.get());

            self.is_pressed.set(true);
            self.update_pseudo_classes();
        }
    }

    fn drop_down_button_pointer_released(&self) {
        self.is_pressed.set(false);
        self.update_pseudo_classes();
    }

    fn drop_down_button_click(&self) {
        if !self.ignore_button_click.get() {
            self.toggle_pop_up();
        } else {
            self.ignore_button_click.set(false);
        }
    }

    fn pop_up_closed(&self) {
        self.set_current_value(Self::is_drop_down_open_property(), false);

        if !self.is_popup_closing.get() {
            self.is_popup_closing.set(true);
            let picker = self.to_ref();
            let _ = Dispatcher::ui_thread().invoke_async_local(move || picker.is_popup_closing.set(false));
        }
    }

    fn toggle_pop_up(&self) {
        if self.is_drop_down_open() {
            self.focus();
            self.set_current_value(Self::is_drop_down_open_property(), false);
        } else {
            self.set_selected_date_from_text();
            self.set_current_value(Self::is_drop_down_open_property(), true);
            self.calendar().expect("the date picker has a calendar").focus();
        }
    }

    fn open_drop_down(&self) {
        if let Some(calendar) = self.calendar() {
            calendar.focus();

            // Open the PopUp
            self.on_open_selected_date.set(self.selected_date());
            let pop_up = self.pop_up.borrow().clone().expect("the date picker has a popup");
            pop_up.set_is_open(true);
            self.is_flyout_open.set(pop_up.is_open());

            self.update_pseudo_classes();
            calendar.reset_states();
            self.on_calendar_opened();
        }
    }

    /// Input text is parsed in the correct format and changed into a
    /// DateTime object.  If the text can not be parsed TextParseError Event
    /// is thrown.
    ///
    /// Returns `None` if the text can not be parsed, or the date that is
    /// represented by the text.
    fn parse_text(&self, text: &str) -> Option<DateTime> {
        // The parse result keeps the error apart in order to be able to
        // pass it to the DateValidationError event
        match self.convert_text(text) {
            Ok(new_selected_date) => {
                let is_valid = match (self.calendar(), new_selected_date) {
                    (_, None) => true,
                    (Some(calendar), new_selected_date) => Calendar::is_valid_date_selection(&calendar, new_selected_date),
                    (None, Some(_)) => panic!("Object reference not set to an instance of an object."),
                };

                if is_valid {
                    return new_selected_date;
                }

                let date_validation_error = CalendarDatePickerDateValidationErrorEventArgs::new(
                    Rc::new(ArgumentOutOfRangeError { message: "SelectedDate value is not valid.", parameter: "text" }),
                    text,
                );
                self.on_date_validation_error(&date_validation_error);

                if date_validation_error.throw_exception() {
                    panic!("{}", date_validation_error.exception());
                }
            }
            Err(error) => {
                let text_parse_error = CalendarDatePickerDateValidationErrorEventArgs::new(error, text);
                self.on_date_validation_error(&text_parse_error);

                if text_parse_error.throw_exception() {
                    panic!("{}", text_parse_error.exception());
                }
            }
        }

        None
    }

    /// Converts a text to a date with the text converter, or else with the
    /// date format of the control and the conventions of the current
    /// culture.
    fn convert_text(&self, text: &str) -> ParseResult {
        if let Some(text_converter) = self.text_converter() {
            let boxed: BoxedValue = Rc::new(text.to_string());
            return match text_converter.convert_back(Some(&boxed), ValueType::of::<Option<DateTime>>(), None) {
                Ok(value) => Ok(value.and_then(|value| {
                    value
                        .downcast_ref::<DateTime>()
                        .copied()
                        .or_else(|| value.downcast_ref::<Option<DateTime>>().copied().flatten())
                })),
                // A text that is not in a correct format is reported as the
                // parse error of the control; any other error is not
                // handled, as in the reference.
                Err(error) => match error.inner().downcast_ref::<FormatError>() {
                    Some(format_error) => Err(Rc::new(format_error.clone())),
                    None => panic!("{error}"),
                },
            };
        }

        let date_format = DateTimeHelper::get_current_date_format();
        let custom_date_format_string = self.custom_date_format_string();
        let parsed = if self.selected_date_format() == CalendarDatePickerFormat::Custom
            && !custom_date_format_string.is_empty()
        {
            DateTime::parse_exact(text, &custom_date_format_string, &date_format)
        } else {
            DateTime::parse(text, &date_format)
        };

        match parsed {
            Ok(date) => Ok(Some(date)),
            Err(error) => Err(Rc::new(error)),
        }
    }

    fn date_time_to_string(&self, d: DateTime) -> Option<String> {
        if let Some(text_converter) = self.text_converter() {
            let boxed: BoxedValue = Rc::new(d);
            return match text_converter.convert(Some(&boxed), ValueType::of::<String>(), None) {
                Ok(value) => value.and_then(|value| {
                    value
                        .downcast_ref::<String>()
                        .cloned()
                        .or_else(|| value.downcast_ref::<Option<String>>().cloned().flatten())
                }),
                Err(error) => panic!("{error}"),
            };
        }

        let date_format = DateTimeHelper::get_current_date_format();
        let text = match self.selected_date_format() {
            CalendarDatePickerFormat::Short => d.to_string_format(date_format.short_date_pattern(), &date_format),
            CalendarDatePickerFormat::Long => d.to_string_format(date_format.long_date_pattern(), &date_format),
            CalendarDatePickerFormat::Custom => d.to_string_format(&self.custom_date_format_string(), &date_format),
        };

        match format_without_arguments(&text) {
            Ok(text) => Some(text),
            Err(error) => panic!("{error}"),
        }
    }

    fn process_date_picker_key(&self, e: &KeyEventArgs) -> bool {
        match e.key {
            Key::Enter => {
                self.set_selected_date_from_text();
                true
            }
            Key::Down => {
                if e.key_modifiers.contains(KeyModifiers::CONTROL) {
                    self.toggle_pop_up();
                    return true;
                }
                false
            }
            _ => false,
        }
    }

    /// Sets the selected date from the text of the text box (or, without a
    /// text box, from the text that was set before the template was
    /// applied): the `SetSelectedDate` of the reference.
    fn set_selected_date_from_text(&self) {
        if let Some(text_box) = self.text_box() {
            let s = text_box.text().unwrap_or_default();
            if !s.is_empty() {
                if let Some(selected_date) = self.selected_date() {
                    // If the string value of the SelectedDate and the
                    // TextBox string value are equal, we do not parse the
                    // string again if we do an extra parse, we lose data in
                    // M/d/yy format.
                    // ex: SelectedDate = DateTime(1008,12,19) but when
                    // "12/19/08" is parsed it is interpreted as
                    // DateTime(2008,12,19)
                    let selected_date = self.date_time_to_string(selected_date);
                    if selected_date.as_deref() == Some(s.as_str()) {
                        return;
                    }
                }
                let d = self.set_text_box_value(&s);

                if self.selected_date() != d {
                    self.set_current_value(Self::selected_date_property(), d);
                }
            } else if self.selected_date().is_some() {
                self.set_current_value(Self::selected_date_property(), None);
            }
        } else {
            let default_text = self.default_text.borrow().clone();
            let d = self.set_text_box_value(&default_text);

            if self.selected_date() != d {
                self.set_current_value(Self::selected_date_property(), d);
            }
        }
    }

    fn set_text_box_value(&self, s: &str) -> Option<DateTime> {
        if s.is_empty() {
            self.set_value(Self::text_property(), Some(s.to_string()));
            return self.selected_date();
        }

        match self.parse_text(s) {
            Some(d) => {
                // make sure displayed text is reformatted to correct date format
                let new_text = self.date_time_to_string(d);
                self.set_value(Self::text_property(), new_text);
                Some(d)
            }
            None => {
                // If parse error: TextBox should have the latest valid
                // SelectedDate value:
                match self.selected_date() {
                    Some(selected_date) => {
                        let new_text = self.date_time_to_string(selected_date);
                        self.set_value(Self::text_property(), new_text);
                        Some(selected_date)
                    }
                    None => {
                        self.set_water_mark_text();
                        None
                    }
                }
            }
        }
    }

    fn set_water_mark_text(&self) {
        if let Some(text_box) = self.text_box() {
            self.set_current_value(Self::text_property(), Some(String::new()));

            if self.placeholder_text().is_none_or(|text| text.is_empty()) && !self.use_floating_placeholder() {
                let date_format = DateTimeHelper::get_current_date_format();
                self.default_text.borrow_mut().clear();

                let placeholder_text = match self.selected_date_format() {
                    CalendarDatePickerFormat::Custom => format!("<{}>", self.custom_date_format_string()),
                    CalendarDatePickerFormat::Long => format!("<{}>", date_format.long_date_pattern()),
                    CalendarDatePickerFormat::Short => format!("<{}>", date_format.short_date_pattern()),
                };
                text_box.set_placeholder_text(Some(&placeholder_text));
            } else {
                text_box.clear_value(TextBox::placeholder_text_property());
            }
        }
    }

    fn is_valid_selected_date_format(value: &CalendarDatePickerFormat) -> bool {
        matches!(
            value,
            CalendarDatePickerFormat::Long | CalendarDatePickerFormat::Short | CalendarDatePickerFormat::Custom
        )
    }

    #[allow(clippy::ptr_arg)] // the signature of a property validator
    fn is_valid_date_format_string(format_string: &String) -> bool {
        !format_string.trim().is_empty()
    }

    /// Clear the date picker.
    pub fn clear(&self) {
        self.set_current_value(Self::selected_date_property(), None);
    }
}
