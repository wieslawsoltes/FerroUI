use super::{DatePickerPresenter, DatePickerSelectedValueChangedEventArgs, PickerPresenterBase};
use crate::primitives::popup_positioning::{PopupAnchor, PopupGravity, PopupPositionerConstraintAdjustment};
use crate::primitives::{Popup, TemplateAppliedEventArgs, TemplatedControl, TemplatedControlImpl, TemplatedControlImplExt};
use crate::shapes::Rectangle;
use crate::{Button, ColumnDefinition, ContentControl, ControlImpl, Grid, GridUnitType, PlacementMode, TextBlock};
use ferroui_base::data::BindingMode;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::{InteractiveImpl, RoutedEventHandlerToken};
use ferroui_base::layout::{LayoutableImpl, VerticalAlignment};
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::utilities::{CultureInfo, DateTimeOffset, HandlerList};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate, FerroObject, FerroObjectImpl, FerroObjectImplExt,
    FerroProperty, FerroPropertyChangedEventArgs, Ref, StyledElementImpl, StyledProperty, StyledPropertyOptions,
    VisualImpl,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

const PC_HAS_NO_DATE: &str = ":hasnodate";

/// The index of the first UTF-16 unit of `pattern` that is `letter` in
/// either case, or -1 (the ordinal, case-insensitive `IndexOf` of upstream).
pub(super) fn index_of_ignore_case(pattern: &str, letter: char) -> i32 {
    pattern
        .encode_utf16()
        .position(|unit| char::from_u32(u32::from(unit)).is_some_and(|c| c.eq_ignore_ascii_case(&letter)))
        .map_or(-1, |index| index as i32)
}

/// A control to allow the user to select a date.
#[repr(C)]
pub struct DatePicker {
    base: TemplatedControl,

    // Template Items
    flyout_button: RefCell<Option<Ref<Button>>>,
    day_text: RefCell<Option<Ref<TextBlock>>>,
    month_text: RefCell<Option<Ref<TextBlock>>>,
    year_text: RefCell<Option<Ref<TextBlock>>>,
    container: RefCell<Option<Ref<Grid>>>,
    spacer1: RefCell<Option<Ref<Rectangle>>>,
    spacer2: RefCell<Option<Ref<Rectangle>>>,
    popup: RefCell<Option<Ref<Popup>>>,
    presenter: RefCell<Option<Ref<DatePickerPresenter>>>,

    /// The click handler of the flyout button, while it is added.
    flyout_button_click: Cell<Option<RoutedEventHandlerToken>>,
    /// The subscriptions to the confirmed and dismissed events of the
    /// presenter.
    presenter_subscriptions: RefCell<Option<(Rc<dyn IDisposable>, Rc<dyn IDisposable>)>>,

    are_controls_available: Cell<bool>,

    selected_date_changed: HandlerList<dyn Fn(&DatePickerSelectedValueChangedEventArgs)>,
}

ferro_class! {
    DatePicker: TemplatedControl, virtuals DatePickerImpl: TemplatedControlImpl {
        /// Raises the `SelectedDateChanged` event.
        fn on_selected_date_changed(this, e: &DatePickerSelectedValueChangedEventArgs);
    }
}

ferro_class_info!(DatePicker {
    new: DatePicker::new,
    markup: {
        attributes: [
            TemplatePart("PART_ButtonContentGrid", type(Ref<Grid>)),
            TemplatePart("PART_DayTextBlock", type(Ref<TextBlock>)),
            TemplatePart("PART_FirstSpacer", type(Ref<Rectangle>)),
            TemplatePart("PART_FlyoutButton", type(Ref<Button>)),
            TemplatePart("PART_MonthTextBlock", type(Ref<TextBlock>)),
            TemplatePart("PART_PickerPresenter", type(Ref<DatePickerPresenter>)),
            TemplatePart("PART_Popup", type(Ref<Popup>)),
            TemplatePart("PART_SecondSpacer", type(Ref<Rectangle>)),
            TemplatePart("PART_YearTextBlock", type(Ref<TextBlock>)),
            PseudoClasses(":hasnodate"),
        ],
    },
});

ferro_impl_classes!(
    DatePicker: StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl
);

impl FerroObjectImpl for DatePicker {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);

        this.pseudo_classes().set(PC_HAS_NO_DATE, true);
        let now = DateTimeOffset::now();
        this.set_current_value(
            Self::min_year_property(),
            DateTimeOffset::new(now.date().year() - 100, 1, 1, 0, 0, 0, now.offset()),
        );
        this.set_current_value(
            Self::max_year_property(),
            DateTimeOffset::new(now.date().year() + 100, 12, 31, 0, 0, 0, now.offset()),
        );
    }

    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        let property = change.property();
        if property == Self::day_visible_property().as_property()
            || property == Self::month_visible_property().as_property()
            || property == Self::year_visible_property().as_property()
        {
            this.set_grid();
        } else if property == Self::max_year_property().as_property() {
            this.on_max_year_changed(change.get_new_value::<DateTimeOffset>());
        } else if property == Self::min_year_property().as_property() {
            this.on_min_year_changed(change.get_new_value::<DateTimeOffset>());
        } else if property == Self::selected_date_property().as_property() {
            this.set_selected_date_text();

            let (old_value, new_value) = change.get_old_and_new_value::<Option<DateTimeOffset>>();
            this.on_selected_date_changed(&DatePickerSelectedValueChangedEventArgs::new(old_value, new_value));
        } else if property == Self::month_format_property().as_property()
            || property == Self::year_format_property().as_property()
            || property == Self::day_format_property().as_property()
        {
            this.set_selected_date_text();
        }
    }
}

impl TemplatedControlImpl for DatePicker {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        this.are_controls_available.set(false);
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
        *this.day_text.borrow_mut() = name_scope.find_as::<TextBlock>("PART_DayTextBlock");
        *this.month_text.borrow_mut() = name_scope.find_as::<TextBlock>("PART_MonthTextBlock");
        *this.year_text.borrow_mut() = name_scope.find_as::<TextBlock>("PART_YearTextBlock");
        *this.container.borrow_mut() = name_scope.find_as::<Grid>("PART_ButtonContentGrid");
        *this.spacer1.borrow_mut() = name_scope.find_as::<Rectangle>("PART_FirstSpacer");
        *this.spacer2.borrow_mut() = name_scope.find_as::<Rectangle>("PART_SecondSpacer");
        *this.popup.borrow_mut() = name_scope.find_as::<Popup>("PART_Popup");
        let presenter = name_scope.find_as::<DatePickerPresenter>("PART_PickerPresenter");
        *this.presenter.borrow_mut() = presenter.clone();

        this.are_controls_available.set(true);

        this.set_grid();
        this.set_selected_date_text();

        if let Some(flyout_button) = flyout_button {
            let weak = this.to_ref().downgrade();
            let token = flyout_button.click(move |_, _| {
                if let Some(this) = weak.upgrade() {
                    this.on_flyout_button_clicked();
                }
            });
            this.flyout_button_click.set(Some(token));
        }

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

            bind(DatePickerPresenter::max_year_property().as_property(), Self::max_year_property().as_property());
            bind(DatePickerPresenter::min_year_property().as_property(), Self::min_year_property().as_property());

            bind(
                DatePickerPresenter::month_visible_property().as_property(),
                Self::month_visible_property().as_property(),
            );
            bind(
                DatePickerPresenter::month_format_property().as_property(),
                Self::month_format_property().as_property(),
            );

            bind(DatePickerPresenter::day_visible_property().as_property(), Self::day_visible_property().as_property());
            bind(DatePickerPresenter::day_format_property().as_property(), Self::day_format_property().as_property());

            bind(
                DatePickerPresenter::year_visible_property().as_property(),
                Self::year_visible_property().as_property(),
            );
            bind(DatePickerPresenter::year_format_property().as_property(), Self::year_format_property().as_property());
        }
    }
}

impl ControlImpl for DatePicker {
    fn on_create_automation_peer(this: &Self) -> Ref<crate::automation::peers::AutomationPeer> {
        crate::automation::peers::DatePickerAutomationPeer::new(this).upcast()
    }
}

impl DatePickerImpl for DatePicker {
    fn on_selected_date_changed(this: &Self, e: &DatePickerSelectedValueChangedEventArgs) {
        for (_, handler) in this.selected_date_changed.snapshot().iter() {
            handler(e);
        }
    }
}

ferro_properties! {
    impl DatePicker {
        /// Defines the `DayFormat` property.
        pub fn day_format_property() -> StyledProperty<String> {
            FerroProperty::register::<DatePicker, _>("DayFormat", "%d".to_string())
        }

        /// Defines the `DayVisible` property.
        pub fn day_visible_property() -> StyledProperty<bool> {
            FerroProperty::register::<DatePicker, _>("DayVisible", true)
        }

        /// Defines the `MaxYear` property.
        pub fn max_year_property() -> StyledProperty<DateTimeOffset> {
            FerroProperty::register_with::<DatePicker, _>(
                "MaxYear",
                StyledPropertyOptions::new(DateTimeOffset::MAX_VALUE).coerce(DatePicker::coerce_max_year),
            )
        }

        /// Defines the `MinYear` property.
        pub fn min_year_property() -> StyledProperty<DateTimeOffset> {
            FerroProperty::register_with::<DatePicker, _>(
                "MinYear",
                StyledPropertyOptions::new(DateTimeOffset::MIN_VALUE).coerce(DatePicker::coerce_min_year),
            )
        }

        /// Defines the `MonthFormat` property.
        pub fn month_format_property() -> StyledProperty<String> {
            FerroProperty::register::<DatePicker, _>("MonthFormat", "MMMM".to_string())
        }

        /// Defines the `MonthVisible` property.
        pub fn month_visible_property() -> StyledProperty<bool> {
            FerroProperty::register::<DatePicker, _>("MonthVisible", true)
        }

        /// Defines the `YearFormat` property.
        pub fn year_format_property() -> StyledProperty<String> {
            FerroProperty::register::<DatePicker, _>("YearFormat", "yyyy".to_string())
        }

        /// Defines the `YearVisible` property.
        pub fn year_visible_property() -> StyledProperty<bool> {
            FerroProperty::register::<DatePicker, _>("YearVisible", true)
        }

        /// Defines the `VerticalContentAlignment` property.
        pub fn vertical_content_alignment_property() -> StyledProperty<VerticalAlignment> {
            ContentControl::vertical_content_alignment_property().add_owner::<DatePicker>()
        }

        /// Defines the `SelectedDate` property.
        pub fn selected_date_property() -> StyledProperty<Option<DateTimeOffset>> {
            FerroProperty::register_with::<DatePicker, _>(
                "SelectedDate",
                StyledPropertyOptions::new(None)
                    .default_binding_mode(BindingMode::TwoWay)
                    .enable_data_validation(true),
            )
        }
    }
}

impl DatePicker {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: TemplatedControl::construct(),
            flyout_button: RefCell::new(None),
            day_text: RefCell::new(None),
            month_text: RefCell::new(None),
            year_text: RefCell::new(None),
            container: RefCell::new(None),
            spacer1: RefCell::new(None),
            spacer2: RefCell::new(None),
            popup: RefCell::new(None),
            presenter: RefCell::new(None),
            flyout_button_click: Cell::new(None),
            presenter_subscriptions: RefCell::new(None),
            are_controls_available: Cell::new(false),
            selected_date_changed: HandlerList::new(),
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

    /// The format of the day.
    pub fn day_format(&self) -> String {
        self.get_value(Self::day_format_property())
    }

    pub fn set_day_format(&self, value: &str) {
        self.set_value(Self::day_format_property(), value.to_string())
    }

    /// Whether the day is visible.
    pub fn day_visible(&self) -> bool {
        self.get_value(Self::day_visible_property())
    }

    pub fn set_day_visible(&self, value: bool) {
        self.set_value(Self::day_visible_property(), value)
    }

    /// The maximum year for the picker.
    pub fn max_year(&self) -> DateTimeOffset {
        self.get_value(Self::max_year_property())
    }

    pub fn set_max_year(&self, value: DateTimeOffset) {
        self.set_value(Self::max_year_property(), value)
    }

    fn coerce_max_year(sender: &FerroObject, value: DateTimeOffset) -> DateTimeOffset {
        if value < sender.get_value(Self::min_year_property()) {
            panic!("MaxYear cannot be less than MinYear");
        }

        value
    }

    fn on_max_year_changed(&self, value: DateTimeOffset) {
        if self.selected_date().is_some_and(|selected_date| selected_date > value) {
            self.set_current_value(Self::selected_date_property(), Some(value));
        }
    }

    /// The minimum year for the picker.
    pub fn min_year(&self) -> DateTimeOffset {
        self.get_value(Self::min_year_property())
    }

    pub fn set_min_year(&self, value: DateTimeOffset) {
        self.set_value(Self::min_year_property(), value)
    }

    fn coerce_min_year(sender: &FerroObject, value: DateTimeOffset) -> DateTimeOffset {
        if value > sender.get_value(Self::max_year_property()) {
            panic!("MinYear cannot be greater than MaxYear");
        }

        value
    }

    fn on_min_year_changed(&self, value: DateTimeOffset) {
        if self.selected_date().is_some_and(|selected_date| selected_date < value) {
            self.set_current_value(Self::selected_date_property(), Some(value));
        }
    }

    /// The month format.
    pub fn month_format(&self) -> String {
        self.get_value(Self::month_format_property())
    }

    pub fn set_month_format(&self, value: &str) {
        self.set_value(Self::month_format_property(), value.to_string())
    }

    /// Whether the month is visible.
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

    /// Whether the year is visible.
    pub fn year_visible(&self) -> bool {
        self.get_value(Self::year_visible_property())
    }

    pub fn set_year_visible(&self, value: bool) {
        self.set_value(Self::year_visible_property(), value)
    }

    /// The selected date for the picker; can be `None`.
    pub fn selected_date(&self) -> Option<DateTimeOffset> {
        self.get_value(Self::selected_date_property())
    }

    pub fn set_selected_date(&self, value: Option<DateTimeOffset>) {
        self.set_value(Self::selected_date_property(), value)
    }

    /// Raised when the selected date changes. Disposing the returned handle
    /// removes the handler.
    pub fn selected_date_changed(
        &self,
        handler: impl Fn(&DatePickerSelectedValueChangedEventArgs) + 'static,
    ) -> Rc<dyn IDisposable> {
        let token = self.selected_date_changed.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.selected_date_changed.remove(token);
            }
        })
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
        self.set_current_value(Self::selected_date_property(), Some(presenter.date()));
    }

    fn set_grid(&self) {
        let Some(container) = self.container.borrow().clone() else {
            return;
        };

        let month_text = self.month_text.borrow().clone();
        let year_text = self.year_text.borrow().clone();
        let day_text = self.day_text.borrow().clone();

        let date_time_format = CultureInfo::current_culture().date_time_format();
        let fmt = date_time_format.short_date_pattern();
        let mut columns = [
            (month_text.clone(), if self.month_visible() { index_of_ignore_case(fmt, 'm') } else { -1 }),
            (year_text, if self.year_visible() { index_of_ignore_case(fmt, 'y') } else { -1 }),
            (day_text, if self.day_visible() { index_of_ignore_case(fmt, 'd') } else { -1 }),
        ];

        columns.sort_by_key(|column| column.1);
        container.column_definitions().clear();

        let mut column_index = 0;

        for (text, index) in &columns {
            let Some(text) = text else {
                continue;
            };

            text.set_is_visible(*index != -1);

            if *index != -1 {
                if column_index > 0 {
                    container.column_definitions().add(ColumnDefinition::with_value(0.0, GridUnitType::Auto));
                }

                let is_month = month_text.as_ref().is_some_and(|month_text| month_text.ptr_eq(text));
                container
                    .column_definitions()
                    .add(ColumnDefinition::with_value(if is_month { 138.0 } else { 78.0 }, GridUnitType::Star));

                if text.parent().is_none() {
                    container.children().add(text.clone());
                }

                Grid::set_column(text, column_index * 2);
                column_index += 1;
            }
        }

        let is_spacer1_visible = column_index > 1;
        let is_spacer2_visible = column_index > 2;
        let spacer1 = self.spacer1.borrow().clone().expect("the template has a first spacer");
        let spacer2 = self.spacer2.borrow().clone().expect("the template has a second spacer");
        // The conditional is used to make sure grid cells will be validated
        Grid::set_column(&spacer1, if is_spacer1_visible { 1 } else { 0 });
        Grid::set_column(&spacer2, if is_spacer2_visible { 3 } else { 0 });

        spacer1.set_is_visible(is_spacer1_visible);
        spacer2.set_is_visible(is_spacer2_visible);
    }

    fn set_selected_date_text(&self) {
        if !self.are_controls_available.get() {
            return;
        }

        let month_text = self.month_text.borrow().clone().expect("the template has a month text block");
        let year_text = self.year_text.borrow().clone().expect("the template has a year text block");
        let day_text = self.day_text.borrow().clone().expect("the template has a day text block");

        if let Some(sel_date) = self.selected_date() {
            self.pseudo_classes().set(PC_HAS_NO_DATE, false);
            month_text.set_text(Some(&sel_date.to_string_with(&self.month_format())));
            year_text.set_text(Some(&sel_date.to_string_with(&self.year_format())));
            day_text.set_text(Some(&sel_date.to_string_with(&self.day_format())));
        } else {
            // By clearing local value, we reset text property to the value from the template.
            month_text.clear_value(TextBlock::text_property());
            year_text.clear_value(TextBlock::text_property());
            day_text.clear_value(TextBlock::text_property());
            self.pseudo_classes().set(PC_HAS_NO_DATE, true);
        }
    }

    fn on_flyout_button_clicked(&self) {
        let Some(presenter) = self.presenter.borrow().clone() else {
            panic!("No DatePickerPresenter found.");
        };
        let Some(popup) = self.popup.borrow().clone() else {
            panic!("No Popup found.");
        };

        presenter.set_date(self.selected_date().unwrap_or_else(DateTimeOffset::now));

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

    /// Clears the selected date.
    pub fn clear(&self) {
        self.set_current_value(Self::selected_date_property(), None);
    }
}
