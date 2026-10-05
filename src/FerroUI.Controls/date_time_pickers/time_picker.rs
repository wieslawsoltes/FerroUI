use super::{PickerPresenterBase, TimePickerPresenter, TimePickerSelectedValueChangedEventArgs};
use crate::primitives::popup_positioning::{PopupAnchor, PopupGravity, PopupPositionerConstraintAdjustment};
use crate::primitives::{Popup, TemplateAppliedEventArgs, TemplatedControl, TemplatedControlImpl, TemplatedControlImplExt};
use crate::shapes::Rectangle;
use crate::utils::TimeUtils;
use crate::{
    Border, Button, ColumnDefinition, ColumnDefinitions, ContentControl, ControlImpl, Grid, GridLength, PlacementMode,
    TextBlock,
};
use ferroui_base::animation::TimeSpan;
use ferroui_base::data::BindingMode;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::{InteractiveImpl, RoutedEventHandlerToken};
use ferroui_base::layout::{LayoutableImpl, VerticalAlignment};
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::utilities::{CultureInfo, DateTime, HandlerList};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate, FerroObject, FerroObjectImpl, FerroObjectImplExt,
    FerroProperty, FerroPropertyChangedEventArgs, Ref, StyledElementImpl, StyledProperty, StyledPropertyOptions,
    VisualImpl,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

const PC_HAS_NO_TIME: &str = ":hasnotime";

/// A control to allow the user to select a time.
#[repr(C)]
pub struct TimePicker {
    base: TemplatedControl,

    // Template Items
    presenter: RefCell<Option<Ref<TimePickerPresenter>>>,
    flyout_button: RefCell<Option<Ref<Button>>>,
    first_picker_host: RefCell<Option<Ref<Border>>>,
    second_picker_host: RefCell<Option<Ref<Border>>>,
    third_picker_host: RefCell<Option<Ref<Border>>>,
    fourth_picker_host: RefCell<Option<Ref<Border>>>,
    hour_text: RefCell<Option<Ref<TextBlock>>>,
    minute_text: RefCell<Option<Ref<TextBlock>>>,
    second_text: RefCell<Option<Ref<TextBlock>>>,
    period_text: RefCell<Option<Ref<TextBlock>>>,
    first_splitter: RefCell<Option<Ref<Rectangle>>>,
    second_splitter: RefCell<Option<Ref<Rectangle>>>,
    third_splitter: RefCell<Option<Ref<Rectangle>>>,
    content_grid: RefCell<Option<Ref<Grid>>>,
    popup: RefCell<Option<Ref<Popup>>>,

    /// The click handler of the flyout button, while it is added.
    flyout_button_click: Cell<Option<RoutedEventHandlerToken>>,
    /// The subscriptions to the confirmed and dismissed events of the
    /// presenter.
    presenter_subscriptions: RefCell<Option<(Rc<dyn IDisposable>, Rc<dyn IDisposable>)>>,

    selected_time_changed: HandlerList<dyn Fn(&TimePickerSelectedValueChangedEventArgs)>,
}

ferro_class! {
    TimePicker: TemplatedControl, virtuals TimePickerImpl: TemplatedControlImpl {
        /// Raises the `SelectedTimeChanged` event.
        fn on_selected_time_changed(this, old_time: Option<TimeSpan>, new_time: Option<TimeSpan>);
    }
}

ferro_class_info!(TimePicker {
    new: TimePicker::new,
    markup: {
        attributes: [
            TemplatePart("PART_FirstColumnDivider", type(Ref<Rectangle>)),
            TemplatePart("PART_FirstPickerHost", type(Ref<Border>)),
            TemplatePart("PART_FlyoutButton", type(Ref<Button>)),
            TemplatePart("PART_FlyoutButtonContentGrid", type(Ref<Grid>)),
            TemplatePart("PART_HourTextBlock", type(Ref<TextBlock>)),
            TemplatePart("PART_MinuteTextBlock", type(Ref<TextBlock>)),
            TemplatePart("PART_SecondTextBlock", type(Ref<TextBlock>)),
            TemplatePart("PART_PeriodTextBlock", type(Ref<TextBlock>)),
            TemplatePart("PART_PickerPresenter", type(Ref<TimePickerPresenter>)),
            TemplatePart("PART_Popup", type(Ref<Popup>)),
            TemplatePart("PART_SecondColumnDivider", type(Ref<Rectangle>)),
            TemplatePart("PART_SecondPickerHost", type(Ref<Border>)),
            TemplatePart("PART_ThirdColumnDivider", type(Ref<Rectangle>)),
            TemplatePart("PART_ThirdPickerHost", type(Ref<Border>)),
            TemplatePart("PART_FourthPickerHost", type(Ref<Border>)),
            PseudoClasses(":hasnotime"),
        ],
    },
});

ferro_impl_classes!(
    TimePicker: StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);

impl FerroObjectImpl for TimePicker {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);

        this.pseudo_classes().set(PC_HAS_NO_TIME, true);

        let date_time_format = CultureInfo::current_culture().date_time_format();
        let time_pattern = date_time_format.short_time_pattern();
        if time_pattern.contains('H') {
            this.set_current_value(Self::clock_identifier_property(), "24HourClock".to_string());
        }
    }

    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        let property = change.property();
        if property == Self::minute_increment_property().as_property() {
            this.set_selected_time_text();
        } else if property == Self::second_increment_property().as_property() {
            this.set_selected_time_text();
        } else if property == Self::clock_identifier_property().as_property() {
            this.set_grid();
            this.set_selected_time_text();
        } else if property == Self::use_seconds_property().as_property() {
            this.set_grid();
            this.set_selected_time_text();
        } else if property == Self::selected_time_property().as_property() {
            let (old_value, new_value) = change.get_old_and_new_value::<Option<TimeSpan>>();
            this.on_selected_time_changed(old_value, new_value);
            this.set_selected_time_text();
        }
    }
}

impl TemplatedControlImpl for TimePicker {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        let flyout_button = this.flyout_button.borrow().clone();
        if let (Some(flyout_button), Some(token)) = (flyout_button, this.flyout_button_click.take()) {
            flyout_button.remove_handler(Button::click_event(), token);
        }

        if let Some((confirmed, dismissed)) = this.presenter_subscriptions.take() {
            confirmed.dispose();
            dismissed.dispose();
        }
        Self::parent_on_apply_template(this, e);

        let name_scope = e.name_scope();
        let flyout_button = name_scope.find_as::<Button>("PART_FlyoutButton");
        *this.flyout_button.borrow_mut() = flyout_button.clone();

        *this.first_picker_host.borrow_mut() = name_scope.find_as::<Border>("PART_FirstPickerHost");
        *this.second_picker_host.borrow_mut() = name_scope.find_as::<Border>("PART_SecondPickerHost");
        *this.third_picker_host.borrow_mut() = name_scope.find_as::<Border>("PART_ThirdPickerHost");
        *this.fourth_picker_host.borrow_mut() = name_scope.find_as::<Border>("PART_FourthPickerHost");

        *this.hour_text.borrow_mut() = name_scope.find_as::<TextBlock>("PART_HourTextBlock");
        *this.minute_text.borrow_mut() = name_scope.find_as::<TextBlock>("PART_MinuteTextBlock");
        *this.second_text.borrow_mut() = name_scope.find_as::<TextBlock>("PART_SecondTextBlock");
        *this.period_text.borrow_mut() = name_scope.find_as::<TextBlock>("PART_PeriodTextBlock");

        *this.first_splitter.borrow_mut() = name_scope.find_as::<Rectangle>("PART_FirstColumnDivider");
        *this.second_splitter.borrow_mut() = name_scope.find_as::<Rectangle>("PART_SecondColumnDivider");
        *this.third_splitter.borrow_mut() = name_scope.find_as::<Rectangle>("PART_ThirdColumnDivider");

        *this.content_grid.borrow_mut() = name_scope.find_as::<Grid>("PART_FlyoutButtonContentGrid");

        *this.popup.borrow_mut() = name_scope.find_as::<Popup>("PART_Popup");
        let presenter = name_scope.find_as::<TimePickerPresenter>("PART_PickerPresenter");
        *this.presenter.borrow_mut() = presenter.clone();

        if let Some(flyout_button) = flyout_button {
            let weak = this.to_ref().downgrade();
            let token = flyout_button.click(move |_, _| {
                if let Some(this) = weak.upgrade() {
                    this.on_flyout_button_clicked();
                }
            });
            this.flyout_button_click.set(Some(token));
        }

        this.set_grid();
        this.set_selected_time_text();

        if let Some(presenter) = presenter {
            let weak = this.to_ref().downgrade();
            let confirmed = PickerPresenterBase::confirmed(&presenter, move || {
                if let Some(this) = weak.upgrade() {
                    this.on_confirmed();
                }
            });
            let weak = this.to_ref().downgrade();
            let dismissed = PickerPresenterBase::dismissed(&presenter, move || {
                if let Some(this) = weak.upgrade() {
                    this.on_dismiss_picker();
                }
            });
            *this.presenter_subscriptions.borrow_mut() = Some((confirmed, dismissed));

            let bind = |target: &'static FerroProperty, source: &'static FerroProperty| {
                presenter.bind_indexer(&target.bind(), &this.indexer(&source.bind()));
            };

            bind(
                TimePickerPresenter::minute_increment_property().as_property(),
                Self::minute_increment_property().as_property(),
            );
            bind(
                TimePickerPresenter::second_increment_property().as_property(),
                Self::second_increment_property().as_property(),
            );
            bind(
                TimePickerPresenter::clock_identifier_property().as_property(),
                Self::clock_identifier_property().as_property(),
            );
            bind(TimePickerPresenter::use_seconds_property().as_property(), Self::use_seconds_property().as_property());
        }
    }
}

// AUTOMATION-SEAM: OnCreateAutomationPeer -> TimePickerAutomationPeer (automation pass)

impl TimePickerImpl for TimePicker {
    fn on_selected_time_changed(this: &Self, old_time: Option<TimeSpan>, new_time: Option<TimeSpan>) {
        let e = TimePickerSelectedValueChangedEventArgs::new(old_time, new_time);
        for (_, handler) in this.selected_time_changed.snapshot().iter() {
            handler(&e);
        }
    }
}

ferro_properties! {
    impl TimePicker {
        /// Defines the `MinuteIncrement` property.
        pub fn minute_increment_property() -> StyledProperty<i32> {
            FerroProperty::register_with::<TimePicker, _>(
                "MinuteIncrement",
                StyledPropertyOptions::new(1).coerce(TimePicker::coerce_minute_increment),
            )
        }

        /// Defines the `SecondIncrement` property.
        pub fn second_increment_property() -> StyledProperty<i32> {
            FerroProperty::register_with::<TimePicker, _>(
                "SecondIncrement",
                StyledPropertyOptions::new(1).coerce(TimePicker::coerce_second_increment),
            )
        }

        /// Defines the `ClockIdentifier` property.
        pub fn clock_identifier_property() -> StyledProperty<String> {
            FerroProperty::register_with::<TimePicker, _>(
                "ClockIdentifier",
                StyledPropertyOptions::new("12HourClock".to_string()).coerce(TimePicker::coerce_clock_identifier),
            )
        }

        /// Defines the `VerticalContentAlignment` property.
        pub fn vertical_content_alignment_property() -> StyledProperty<VerticalAlignment> {
            ContentControl::vertical_content_alignment_property().add_owner::<TimePicker>()
        }

        /// Defines the `UseSeconds` property.
        pub fn use_seconds_property() -> StyledProperty<bool> {
            FerroProperty::register_with::<TimePicker, _>(
                "UseSeconds",
                StyledPropertyOptions::new(false).coerce(TimePicker::coerce_use_seconds),
            )
        }

        /// Defines the `SelectedTime` property.
        pub fn selected_time_property() -> StyledProperty<Option<TimeSpan>> {
            FerroProperty::register_with::<TimePicker, _>(
                "SelectedTime",
                StyledPropertyOptions::new(None)
                    .default_binding_mode(BindingMode::TwoWay)
                    .enable_data_validation(true),
            )
        }
    }
}

impl TimePicker {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: TemplatedControl::construct(),
            presenter: RefCell::new(None),
            flyout_button: RefCell::new(None),
            first_picker_host: RefCell::new(None),
            second_picker_host: RefCell::new(None),
            third_picker_host: RefCell::new(None),
            fourth_picker_host: RefCell::new(None),
            hour_text: RefCell::new(None),
            minute_text: RefCell::new(None),
            second_text: RefCell::new(None),
            period_text: RefCell::new(None),
            first_splitter: RefCell::new(None),
            second_splitter: RefCell::new(None),
            third_splitter: RefCell::new(None),
            content_grid: RefCell::new(None),
            popup: RefCell::new(None),
            flyout_button_click: Cell::new(None),
            presenter_subscriptions: RefCell::new(None),
            selected_time_changed: HandlerList::new(),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The vertical alignment of the content within the control.
    pub fn vertical_content_alignment(&self) -> VerticalAlignment {
        self.get_value(Self::vertical_content_alignment_property())
    }

    pub fn set_vertical_content_alignment(&self, value: VerticalAlignment) {
        self.set_value(Self::vertical_content_alignment_property(), value)
    }

    /// The minute increment in the picker.
    pub fn minute_increment(&self) -> i32 {
        self.get_value(Self::minute_increment_property())
    }

    pub fn set_minute_increment(&self, value: i32) {
        self.set_value(Self::minute_increment_property(), value)
    }

    fn coerce_minute_increment(_sender: &FerroObject, value: i32) -> i32 {
        if !(1..=59).contains(&value) {
            panic!("1 >= MinuteIncrement <= 59");
        }

        value
    }

    /// The second increment in the picker.
    pub fn second_increment(&self) -> i32 {
        self.get_value(Self::second_increment_property())
    }

    pub fn set_second_increment(&self, value: i32) {
        self.set_value(Self::second_increment_property(), value)
    }

    fn coerce_second_increment(_sender: &FerroObject, value: i32) -> i32 {
        if !(1..=59).contains(&value) {
            panic!("1 >= SecondIncrement <= 59");
        }

        value
    }

    /// The clock identifier, either `12HourClock` or `24HourClock`.
    pub fn clock_identifier(&self) -> String {
        self.get_value(Self::clock_identifier_property())
    }

    pub fn set_clock_identifier(&self, value: &str) {
        self.set_value(Self::clock_identifier_property(), value.to_string())
    }

    fn coerce_clock_identifier(_sender: &FerroObject, value: String) -> String {
        if !(value.is_empty() || value == "12HourClock" || value == "24HourClock") {
            panic!("Invalid ClockIdentifier");
        }

        value
    }

    /// The use seconds switch.
    pub fn use_seconds(&self) -> bool {
        self.get_value(Self::use_seconds_property())
    }

    pub fn set_use_seconds(&self, value: bool) {
        self.set_value(Self::use_seconds_property(), value)
    }

    fn coerce_use_seconds(_sender: &FerroObject, value: bool) -> bool {
        // Upstream rejects a value that is neither true nor false, which no
        // value is.
        value
    }

    /// The selected time; can be `None`.
    pub fn selected_time(&self) -> Option<TimeSpan> {
        self.get_value(Self::selected_time_property())
    }

    pub fn set_selected_time(&self, value: Option<TimeSpan>) {
        self.set_value(Self::selected_time_property(), value)
    }

    /// Raised when the selected time changes. Disposing the returned handle
    /// removes the handler.
    pub fn selected_time_changed(
        &self,
        handler: impl Fn(&TimePickerSelectedValueChangedEventArgs) + 'static,
    ) -> Rc<dyn IDisposable> {
        let token = self.selected_time_changed.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.selected_time_changed.remove(token);
            }
        })
    }

    fn set_grid(&self) {
        let Some(content_grid) = self.content_grid.borrow().clone() else {
            return;
        };

        let first_picker_host = self.first_picker_host.borrow().clone();
        let second_picker_host = self.second_picker_host.borrow().clone();
        let third_picker_host = self.third_picker_host.borrow().clone();
        let fourth_picker_host = self.fourth_picker_host.borrow().clone();
        let first_splitter = self.first_splitter.borrow().clone();
        let second_splitter = self.second_splitter.borrow().clone();
        let third_splitter = self.third_splitter.borrow().clone();

        let use_24_hour_clock = self.clock_identifier() == "24HourClock";
        let use_seconds = self.use_seconds();
        let seconds_parts = match (self.second_text.borrow().is_some(), &fourth_picker_host, &third_splitter) {
            (true, Some(fourth_picker_host), Some(third_splitter)) => Some((fourth_picker_host, third_splitter)),
            _ => None,
        };
        let can_use_seconds = seconds_parts.is_some();

        let columns_d = ColumnDefinitions::new();
        columns_d.add(ColumnDefinition::with_width(GridLength::STAR));
        columns_d.add(ColumnDefinition::with_width(GridLength::AUTO));
        columns_d.add(ColumnDefinition::with_width(GridLength::STAR));
        if can_use_seconds && use_seconds {
            columns_d.add(ColumnDefinition::with_width(GridLength::AUTO));
            columns_d.add(ColumnDefinition::with_width(GridLength::STAR));
        }
        if !use_24_hour_clock {
            columns_d.add(ColumnDefinition::with_width(GridLength::AUTO));
            columns_d.add(ColumnDefinition::with_width(GridLength::STAR));
        }

        content_grid.set_column_definitions(columns_d);

        let third_picker_host = third_picker_host.expect("the template has a third picker host");
        let second_splitter = second_splitter.expect("the template has a second column divider");

        if let Some((fourth_picker_host, third_splitter)) = seconds_parts {
            third_picker_host.set_is_visible(use_seconds);
            second_splitter.set_is_visible(use_seconds);
            fourth_picker_host.set_is_visible(!use_24_hour_clock);
            third_splitter.set_is_visible(!use_24_hour_clock);
        } else {
            third_picker_host.set_is_visible(!use_24_hour_clock);
            second_splitter.set_is_visible(!use_24_hour_clock);
        }

        Grid::set_column(&first_picker_host.expect("the template has a first picker host"), 0);
        Grid::set_column(&second_picker_host.expect("the template has a second picker host"), 2);
        let first_splitter = first_splitter.expect("the template has a first column divider");

        if let Some((fourth_picker_host, third_splitter)) = seconds_parts {
            let am_pm_column = if use_seconds { 6 } else { 4 };
            Grid::set_column(&third_picker_host, if use_seconds { 4 } else { 0 });
            Grid::set_column(fourth_picker_host, if use_24_hour_clock { 0 } else { am_pm_column });
            Grid::set_column(&first_splitter, 1);
            Grid::set_column(&second_splitter, if use_seconds { 3 } else { 0 });
            Grid::set_column(third_splitter, if use_24_hour_clock { 0 } else { am_pm_column - 1 });
        } else {
            Grid::set_column(&third_picker_host, if use_24_hour_clock { 0 } else { 4 });
            Grid::set_column(&first_splitter, 1);
            Grid::set_column(&second_splitter, if use_24_hour_clock { 0 } else { 3 });
        }
    }

    fn set_selected_time_text(&self) {
        let (Some(hour_text), Some(minute_text), Some(period_text)) =
            (self.hour_text.borrow().clone(), self.minute_text.borrow().clone(), self.period_text.borrow().clone())
        else {
            return;
        };
        let second_text = self.second_text.borrow().clone();

        if let Some(time) = self.selected_time() {
            let mut new_time = time;

            if self.clock_identifier() == "12HourClock" {
                let hr = new_time.hours();
                let hr = if hr > 12 {
                    hr - 12
                } else if hr == 0 {
                    12
                } else {
                    hr
                };
                new_time = TimeSpan::from_hms(hr, new_time.minutes(), new_time.seconds());
            }

            hour_text.set_text(Some(&new_time.to_string_format("%h")));
            minute_text.set_text(Some(&new_time.to_string_format("mm")));
            if let Some(second_text) = &second_text {
                second_text.set_text(Some(&new_time.to_string_format("ss")));
            }

            self.pseudo_classes().set(PC_HAS_NO_TIME, false);

            period_text.set_text(Some(&if time.hours() >= 12 {
                TimeUtils::get_pm_designator()
            } else {
                TimeUtils::get_am_designator()
            }));
        } else {
            // By clearing local value, we reset text property to the value from the template.
            hour_text.clear_value(TextBlock::text_property());
            minute_text.clear_value(TextBlock::text_property());
            if let Some(second_text) = &second_text {
                second_text.clear_value(TextBlock::text_property());
            }

            self.pseudo_classes().set(PC_HAS_NO_TIME, true);

            period_text.set_text(Some(&if DateTime::now().hour() >= 12 {
                TimeUtils::get_pm_designator()
            } else {
                TimeUtils::get_am_designator()
            }));
        }
    }

    fn on_flyout_button_clicked(&self) {
        let Some(presenter) = self.presenter.borrow().clone() else {
            panic!("No DatePickerPresenter found.");
        };
        let Some(popup) = self.popup.borrow().clone() else {
            panic!("No Popup found.");
        };

        presenter.set_time(self.selected_time().unwrap_or_else(|| DateTime::now().time_of_day()));

        popup.set_placement(PlacementMode::AnchorAndGravity);
        popup.set_placement_anchor(PopupAnchor::BOTTOM);
        popup.set_placement_gravity(PopupGravity::BOTTOM);
        popup.set_placement_constraint_adjustment(PopupPositionerConstraintAdjustment::SLIDE_Y);
        popup.set_is_open(true);

        // Overlay popup hosts won't get measured until the next layout pass, but we need the
        // template to be applied to the presenter now. Detect this case and force a layout pass.
        if !presenter.is_measure_valid() {
            if let Some(layout_manager) = self.get_layout_manager() {
                layout_manager.execute_initial_layout_pass();
            }
        }

        let delta_y = presenter.get_offset_for_popup();

        // The extra 5 px I think is related to default popup placement behavior
        popup.set_vertical_offset(delta_y + 5.0);
    }

    fn on_dismiss_picker(&self) {
        let popup = self.popup.borrow().clone().expect("the template has a popup");
        popup.close();
        self.focus();
    }

    fn on_confirmed(&self) {
        let popup = self.popup.borrow().clone().expect("the template has a popup");
        popup.close();
        let presenter = self.presenter.borrow().clone().expect("the template has a presenter");
        self.set_current_value(Self::selected_time_property(), Some(presenter.time()));
    }

    /// Clears the selected time.
    pub fn clear(&self) {
        self.set_current_value(Self::selected_time_property(), None);
    }
}
