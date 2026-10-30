// Copyright 2022-2026 The Android Open Source Project (reference behavior).
// Copyright 2026 FerroUI contributors (Rust adaptation).
// SPDX-License-Identifier: Apache-2.0
//! Material single-date/range picker, with its own calendar, year and input surfaces.
//! Reference: AndroidX Material3 DatePicker.kt, DateRangePicker.kt and DateInput.kt,
//! revision e4d933c98df7baa7ea2b83eea6e086915961e0ff. Calendar is used only for its
//! existing blackout-date engine; its visual template is never placed in this control.

use crate::date_picker_calendar::{
    MaterialDateCell, MaterialDateMonthView, MaterialDateMonthsPanel,
};
use crate::date_picker_state::{
    MaterialDateInputFormat, MaterialDatePickerMode, MaterialDatePickerState,
    MaterialDateSelectionMode, day_start, month_start,
};
use crate::motion::{
    MaterialMotionRole, MaterialMotionScheme, MaterialSpringState, animations_enabled,
};
use crate::text_field::MaterialTextField;
use ferroui_base::animation::{IGlobalClock, TimeSpan};
use ferroui_base::data::BindingMode;
use ferroui_base::input::text_input::{
    TextInputContentType, TextInputOptions, TextInputReturnKeyType,
};
use ferroui_base::input::{
    InputElement, InputElementImpl, Key, KeyEventArgs, KeyModifiers, KeyboardNavigation,
    NavigationMethod, PointerEventArgs, PointerPressedEventArgs, PointerReleasedEventArgs,
    PointerType,
};
use ferroui_base::interactivity::{
    InteractiveImpl, RoutedEvent, RoutedEventArgs, RoutingStrategies,
};
use ferroui_base::layout::{HorizontalAlignment, LayoutableImpl, VerticalAlignment};
use ferroui_base::media::{FlowDirection, IBrush, TextAlignment, TranslateTransform};
use ferroui_base::reactive::{AnonymousObserver, IDisposable};
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::utilities::{CultureInfo, DateTime, DayOfWeek};
use ferroui_base::{
    BoxedValue, DirectProperty, DirectPropertyMetadata, FerroLocator, FerroObjectImpl,
    FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, LocatorExtensions, Point,
    Ref, StyledElementImpl, StyledProperty, StyledPropertyOptions, Thickness, Vector, VisualImpl,
    VisualImplExt, VisualTreeAttachmentEventArgs, ferro_class, ferro_class_info,
    ferro_impl_classes, ferro_properties, ferro_routed_event, instantiate,
};
use ferroui_controls::automation::peers::{
    AutomationControlType, AutomationPeer, AutomationPeerImpl, ControlAutomationPeer,
    ControlAutomationPeerImpl,
};
use ferroui_controls::automation::provider::ISelectionProvider;
use ferroui_controls::automation::{
    AccessibilityView, AutomationLiveSetting, AutomationProperties,
};
use ferroui_controls::primitives::{
    TemplateAppliedEventArgs, TemplatedControl, TemplatedControlImpl, TemplatedControlImplExt,
    UniformGrid,
};
use ferroui_controls::{
    Button, Control, ControlImpl, Grid, Panel, ScrollViewer, TextBlock, TextBox,
};
use ferroui_controls::{Calendar, CalendarBlackoutDatesCollection};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

struct Parts {
    body: Ref<Panel>,
    calendar: Ref<Control>,
    input: Ref<Grid>,
    calendar_transform: Ref<TranslateTransform>,
    input_transform: Ref<TranslateTransform>,
    title: Ref<TextBlock>,
    headline: Ref<TextBlock>,
    range_headline: Ref<Panel>,
    range_headline_start: Ref<TextBlock>,
    range_headline_end: Ref<TextBlock>,
    header: Ref<Grid>,
    toggle: Ref<Button>,
    edit_icon: Ref<Control>,
    calendar_icon: Ref<Control>,
    navigation: Ref<Control>,
    month_label: Ref<TextBlock>,
    year_toggle: Ref<Button>,
    previous: Ref<Button>,
    next: Ref<Button>,
    weekdays: Ref<UniformGrid>,
    month_viewport: Ref<Panel>,
    month_views: [Ref<MaterialDateMonthView>; 3],
    month_transforms: [Ref<TranslateTransform>; 3],
    years: Ref<ScrollViewer>,
    year_grid: Ref<UniformGrid>,
    range: Ref<ScrollViewer>,
    range_months: Ref<MaterialDateMonthsPanel>,
    start_input: Ref<TextBox>,
    end_input: Ref<TextBox>,
    actions: Ref<Control>,
    confirm: Ref<Button>,
    cancel: Ref<Button>,
}
struct Motion {
    mode: MaterialSpringState,
    calendar_alpha: MaterialSpringState,
    input_alpha: MaterialSpringState,
    year_size: MaterialSpringState,
    year_alpha: MaterialSpringState,
    month_offset: MaterialSpringState,
}
impl Motion {
    fn new(scheme: MaterialMotionScheme) -> Self {
        let spring = |role, value| MaterialSpringState::new(scheme.spring(role), value);
        Self {
            mode: spring(MaterialMotionRole::DefaultSpatial, 0.0),
            calendar_alpha: spring(MaterialMotionRole::DefaultEffects, 1.0),
            input_alpha: spring(MaterialMotionRole::DefaultEffects, 0.0),
            year_size: spring(MaterialMotionRole::DefaultEffects, 0.0),
            year_alpha: spring(MaterialMotionRole::DefaultEffects, 0.0),
            month_offset: spring(MaterialMotionRole::DefaultEffects, 0.0),
        }
    }
    fn springs(&mut self) -> [&mut MaterialSpringState; 6] {
        [
            &mut self.mode,
            &mut self.calendar_alpha,
            &mut self.input_alpha,
            &mut self.year_size,
            &mut self.year_alpha,
            &mut self.month_offset,
        ]
    }
}
#[derive(Clone, Copy)]
struct Swipe {
    id: i32,
    origin: Point,
    previous: Point,
    timestamp: u64,
    velocity: f64,
    active: bool,
}

/// Embed directly, or enable `ShowActions` in a dialog/sheet. `SelectedDate` aliases
/// `SelectedStartDate`; end dates are inclusive. SelectionChanged is raised for edits;
/// Confirmed/Dismissed let the application own modal lifetime and committed values.
#[repr(C)]
pub struct MaterialDatePicker {
    base: TemplatedControl,
    state: Cell<MaterialDatePickerState>,
    year_open: Cell<bool>,
    engine: Ref<Calendar>,
    blackout_handler: Cell<Option<u64>>,
    date_policy: RefCell<Option<Rc<dyn Fn(DateTime) -> bool>>>,
    year_policy: RefCell<Option<Rc<dyn Fn(i32) -> bool>>>,
    parts: RefCell<Option<Rc<Parts>>>,
    subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
    clock_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    visibility_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    motion: RefCell<Motion>,
    last_frame: Cell<Option<f64>>,
    updating_input: Cell<bool>,
    input_digits: RefCell<[String; 2]>,
    input_errors: RefCell<[Option<String>; 2]>,
    changing_range_offset: Cell<bool>,
    range_position_pending: Cell<bool>,
    complete_notified: Cell<bool>,
    input_height: Cell<f64>,
    swipe: Cell<Option<Swipe>>,
}
ferro_class!(MaterialDatePicker: TemplatedControl);
ferro_class_info!(MaterialDatePicker { new: MaterialDatePicker::new,
    markup: { attributes: [PseudoClasses(":range", ":input", ":years")] },
});
ferro_impl_classes!(MaterialDatePicker: StyledElementImpl, InteractiveImpl, LayoutableImpl, InputElementImpl);
ferro_properties! { impl MaterialDatePicker {
    pub fn selected_date_property() -> DirectProperty<MaterialDatePicker, Option<DateTime>> { FerroProperty::register_direct_with::<Self, _>("SelectedDate", |o| o.selected_date(), Some(|o,v| o.set_selected_date(v)), DirectPropertyMetadata::new(Some(None)).with_default_binding_mode(BindingMode::TwoWay)) }
    pub fn selected_start_date_property() -> DirectProperty<MaterialDatePicker, Option<DateTime>> { FerroProperty::register_direct_with::<Self, _>("SelectedStartDate", |o| o.selected_start_date(), Some(|o,v| o.set_selected_start_date(v)), DirectPropertyMetadata::new(Some(None)).with_default_binding_mode(BindingMode::TwoWay)) }
    pub fn selected_end_date_property() -> DirectProperty<MaterialDatePicker, Option<DateTime>> { FerroProperty::register_direct_with::<Self, _>("SelectedEndDate", |o| o.selected_end_date(), Some(|o,v| o.set_selected_end_date(v)), DirectPropertyMetadata::new(Some(None)).with_default_binding_mode(BindingMode::TwoWay)) }
    pub fn displayed_month_property() -> DirectProperty<MaterialDatePicker, DateTime> { FerroProperty::register_direct::<Self, _>("DisplayedMonth", |o| o.displayed_month(), Some(|o,v| o.set_displayed_month(v)), DateTime::MIN_VALUE) }
    pub fn is_selection_complete_property() -> DirectProperty<MaterialDatePicker, bool> { FerroProperty::register_direct::<Self, _>("IsSelectionComplete", |o| o.is_selection_complete(), None, false) }
    pub fn is_year_picker_open_property() -> DirectProperty<MaterialDatePicker, bool> { FerroProperty::register_direct::<Self, _>("IsYearPickerOpen", |o| o.is_year_picker_open(), Some(|o,v| o.set_is_year_picker_open(v)), false) }
    pub fn display_mode_property() -> StyledProperty<MaterialDatePickerMode> { FerroProperty::register::<Self, _>("DisplayMode", MaterialDatePickerMode::Picker) }
    pub fn selection_mode_property() -> StyledProperty<MaterialDateSelectionMode> { FerroProperty::register::<Self, _>("SelectionMode", MaterialDateSelectionMode::Single) }
    pub fn first_year_property() -> StyledProperty<i32> { FerroProperty::register_with::<Self,_>("FirstYear", StyledPropertyOptions::new(1900).validate(|v: &i32| (1..=9999).contains(v))) }
    pub fn last_year_property() -> StyledProperty<i32> { FerroProperty::register_with::<Self,_>("LastYear", StyledPropertyOptions::new(2100).validate(|v: &i32| (1..=9999).contains(v))) }
    pub fn culture_property() -> StyledProperty<Option<CultureInfo>> { FerroProperty::register::<Self,_>("Culture", None) }
    pub fn first_day_of_week_property() -> StyledProperty<Option<DayOfWeek>> { FerroProperty::register::<Self,_>("FirstDayOfWeek", None) }
    pub fn today_property() -> StyledProperty<DateTime> { FerroProperty::register::<Self,_>("Today", DateTime::today()) }
    pub fn title_property() -> StyledProperty<Option<String>> { FerroProperty::register::<Self,_>("Title", None) }
    pub fn show_mode_toggle_property() -> StyledProperty<bool> { FerroProperty::register::<Self,_>("ShowModeToggle", true) }
    pub fn show_actions_property() -> StyledProperty<bool> { FerroProperty::register::<Self,_>("ShowActions", false) }
    pub fn confirm_text_property() -> StyledProperty<String> { FerroProperty::register::<Self,_>("ConfirmText", "OK".into()) }
    pub fn cancel_text_property() -> StyledProperty<String> { FerroProperty::register::<Self,_>("CancelText", "Cancel".into()) }
    pub fn range_calendar_height_property() -> StyledProperty<f64> { FerroProperty::register_with::<Self,_>("RangeCalendarHeight", StyledPropertyOptions::new(384.0_f64).validate(|v| v.is_finite() && *v >= 144.0)) }
    pub fn range_background_property() -> StyledProperty<Option<Rc<dyn IBrush>>> { FerroProperty::register::<Self,_>("RangeBackground", None) }
    pub fn subhead_foreground_property() -> StyledProperty<Option<Rc<dyn IBrush>>> { FerroProperty::register::<Self,_>("SubheadForeground", None) }
    pub fn reduced_motion_property() -> StyledProperty<bool> { FerroProperty::register::<Self,_>("ReducedMotion", false) }
    pub fn motion_scheme_property() -> StyledProperty<MaterialMotionScheme> { FerroProperty::register::<Self,_>("MotionScheme", MaterialMotionScheme::Expressive) }
} }
impl MaterialDatePicker {
    ferro_routed_event! { pub fn selection_changed_event() -> RoutedEvent<RoutedEventArgs> { RoutedEvent::register::<Self, _>("SelectionChanged", RoutingStrategies::BUBBLE) } }
    ferro_routed_event! { pub fn confirmed_event() -> RoutedEvent<RoutedEventArgs> { RoutedEvent::register::<Self, _>("Confirmed", RoutingStrategies::BUBBLE) } }
    ferro_routed_event! { pub fn dismissed_event() -> RoutedEvent<RoutedEventArgs> { RoutedEvent::register::<Self, _>("Dismissed", RoutingStrategies::BUBBLE) } }
}
impl FerroObjectImpl for MaterialDatePicker {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        let weak = this.to_ref().downgrade();
        this.blackout_handler
            .set(Some(this.blackout_dates().add_collection_changed(Rc::new(
                move |_| {
                    if let Some(this) = weak.upgrade() {
                        this.refresh_selectable_dates();
                    }
                },
            ))));
    }
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        match change.property().name() {
            "FirstYear" | "LastYear" => {
                let mut state = this.state();
                state.set_year_range(this.first_year(), this.last_year());
                this.publish(state, true);
                this.rebuild_years();
                if let Some(p) = this.parts() {
                    p.range_months.set_owner(this);
                }
            }
            "SelectionMode" => {
                let mut state = this.state();
                state.selection_mode = this.selection_mode();
                state.set_selection(state.selected_start, state.selected_end);
                this.set_is_year_picker_open(false);
                this.publish(state, true);
                this.refresh_layout();
            }
            "DisplayMode" => {
                let mut state = this.state();
                state.set_mode(this.display_mode());
                this.set_is_year_picker_open(false);
                this.publish(state, true);
                this.start_mode_motion();
                if this.display_mode() == MaterialDatePickerMode::Input {
                    this.queue_focus_input();
                }
            }
            "Culture" | "FirstDayOfWeek" => {
                this.rebuild_weekdays();
                this.refresh_calendar();
                this.sync_inputs();
                this.refresh_header();
            }
            "Today" | "RangeBackground" | "SubheadForeground" => this.refresh_calendar(),
            "Title"
            | "ShowModeToggle"
            | "ShowActions"
            | "ConfirmText"
            | "CancelText"
            | "RangeCalendarHeight" => this.refresh_layout(),
            "ReducedMotion" | "MotionScheme" => {
                this.start_mode_motion();
                this.advance(0.0);
            }
            _ => {}
        }
    }
}
impl ControlImpl for MaterialDatePicker {
    fn on_create_automation_peer(this: &Self) -> Ref<AutomationPeer> {
        MaterialDatePickerAutomationPeer::new(this).upcast()
    }
}
impl VisualImpl for MaterialDatePicker {
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);
        let weak = this.to_ref().downgrade();
        *this.visibility_subscription.borrow_mut() =
            Some(this.is_effectively_visible_changed(move || {
                if let Some(this) = weak.upgrade() {
                    if this.is_effectively_visible() {
                        this.update_clock();
                    } else {
                        this.finish_motion();
                        this.stop_clock();
                    }
                }
            }));
        this.update_clock();
    }
    fn on_detached_from_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        this.swipe.set(None);
        this.finish_motion();
        this.stop_clock();
        if let Some(subscription) = this.visibility_subscription.borrow_mut().take() {
            subscription.dispose();
        }
        Self::parent_on_detached_from_visual_tree(this, e);
    }
}
impl TemplatedControlImpl for MaterialDatePicker {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        Self::parent_on_apply_template(this, e);
        for subscription in this.subscriptions.borrow_mut().drain(..) {
            subscription.dispose();
        }
        this.parts.borrow_mut().take();
        macro_rules! part {
            ($ty:ty,$name:literal) => {
                match e.name_scope().find_as::<$ty>($name) {
                    Some(v) => v,
                    None => return,
                }
            };
        }
        let body = part!(Panel, "PART_Body");
        let calendar = part!(Control, "PART_Calendar");
        let input = part!(Grid, "PART_Input");
        let calendar_transform = TranslateTransform::new();
        let input_transform = TranslateTransform::new();
        calendar.set_render_transform(Some(calendar_transform.clone().into()));
        input.set_render_transform(Some(input_transform.clone().into()));
        let month_views = [
            part!(MaterialDateMonthView, "PART_PreviousMonth"),
            part!(MaterialDateMonthView, "PART_Month"),
            part!(MaterialDateMonthView, "PART_NextMonth"),
        ];
        let month_transforms = [
            TranslateTransform::new(),
            TranslateTransform::new(),
            TranslateTransform::new(),
        ];
        for (view, transform) in month_views.iter().zip(month_transforms.iter()) {
            view.set_render_transform(Some(transform.clone().into()));
        }
        let p = Rc::new(Parts {
            body,
            calendar,
            input,
            calendar_transform,
            input_transform,
            title: part!(TextBlock, "PART_Title"),
            headline: part!(TextBlock, "PART_Headline"),
            range_headline: part!(Panel, "PART_RangeHeadline"),
            range_headline_start: part!(TextBlock, "PART_RangeHeadlineStart"),
            range_headline_end: part!(TextBlock, "PART_RangeHeadlineEnd"),
            header: part!(Grid, "PART_Header"),
            toggle: part!(Button, "PART_ModeToggle"),
            edit_icon: part!(Control, "PART_EditIcon"),
            calendar_icon: part!(Control, "PART_CalendarIcon"),
            navigation: part!(Control, "PART_Navigation"),
            month_label: part!(TextBlock, "PART_MonthLabel"),
            year_toggle: part!(Button, "PART_YearToggle"),
            previous: part!(Button, "PART_Previous"),
            next: part!(Button, "PART_Next"),
            weekdays: part!(UniformGrid, "PART_Weekdays"),
            month_viewport: part!(Panel, "PART_MonthViewport"),
            month_views,
            month_transforms,
            years: part!(ScrollViewer, "PART_Years"),
            year_grid: part!(UniformGrid, "PART_YearGrid"),
            range: part!(ScrollViewer, "PART_Range"),
            range_months: part!(MaterialDateMonthsPanel, "PART_RangeMonths"),
            start_input: part!(TextBox, "PART_StartInput"),
            end_input: part!(TextBox, "PART_EndInput"),
            actions: part!(Control, "PART_Actions"),
            confirm: part!(Button, "PART_Confirm"),
            cancel: part!(Button, "PART_Cancel"),
        });
        *this.parts.borrow_mut() = Some(p.clone());
        for (button, action) in [
            (&p.toggle, 0),
            (&p.year_toggle, 1),
            (&p.previous, 2),
            (&p.next, 3),
            (&p.confirm, 4),
            (&p.cancel, 5),
        ] {
            let weak = this.to_ref().downgrade();
            this.subscriptions
                .borrow_mut()
                .push(button.add_disposable_handler_with(
                    Button::click_event(),
                    move |_, e| {
                        if let Some(this) = weak.upgrade() {
                            match action {
                                0 => this.set_display_mode(
                                    if this.display_mode() == MaterialDatePickerMode::Picker {
                                        MaterialDatePickerMode::Input
                                    } else {
                                        MaterialDatePickerMode::Picker
                                    },
                                ),
                                1 => this.set_is_year_picker_open(!this.is_year_picker_open()),
                                2 => this.navigate_month(-1),
                                3 => this.navigate_month(1),
                                4 => {
                                    this.confirm();
                                }
                                _ => this.dismiss(),
                            }
                            e.set_handled(true);
                        }
                    },
                    RoutingStrategies::BUBBLE,
                    false,
                ));
        }
        for (index, editor) in [&p.start_input, &p.end_input].iter().enumerate() {
            TextInputOptions::set_content_type(editor, TextInputContentType::Digits);
            TextInputOptions::set_return_key_type(editor, TextInputReturnKeyType::Done);
            TextInputOptions::set_show_suggestions(editor, Some(false));
            let weak = this.to_ref().downgrade();
            this.subscriptions
                .borrow_mut()
                .push(editor.add_disposable_handler_with(
                    InputElement::key_down_event(),
                    move |_, event| {
                        if let Some(this) = weak.upgrade() {
                            this.prepare_input_deletion(index, event);
                        }
                    },
                    RoutingStrategies::TUNNEL,
                    false,
                ));
            this.subscriptions
                .borrow_mut()
                .push(editor.add_disposable_handler_with(
                    InputElement::text_input_event(),
                    move |_, event| {
                        if event
                            .text
                            .as_ref()
                            .is_some_and(|text| text.chars().any(|c| !c.is_ascii_digit()))
                        {
                            event.set_handled(true);
                        }
                    },
                    RoutingStrategies::TUNNEL,
                    false,
                ));
            let weak = this.to_ref().downgrade();
            this.subscriptions
                .borrow_mut()
                .push(editor.property_changed(move |change| {
                    if change.property() == TextBox::text_property().as_property() {
                        if let Some(this) = weak.upgrade() {
                            this.input_changed(index);
                        }
                    }
                }));
        }
        let weak = this.to_ref().downgrade();
        this.subscriptions
            .borrow_mut()
            .push(p.range.add_disposable_handler_with(
                ScrollViewer::scroll_changed_event(),
                move |_, _| {
                    if let Some(this) = weak.upgrade() {
                        this.range_scrolled();
                    }
                },
                RoutingStrategies::BUBBLE,
                false,
            ));
        let weak = this.to_ref().downgrade();
        this.subscriptions
            .borrow_mut()
            .push(p.range.layout_updated(move || {
                if let Some(this) = weak.upgrade() {
                    if this.range_position_pending.get() {
                        this.scroll_range_to_month();
                    }
                }
            }));
        let weak = this.to_ref().downgrade();
        this.subscriptions
            .borrow_mut()
            .push(p.input.layout_updated(move || {
                if let Some(this) = weak.upgrade() {
                    if let Some(parts) = this.parts() {
                        if parts.input.is_visible() {
                            let height = parts.input.desired_size().height.max(90.0);
                            if (height - this.input_height.replace(height)).abs() > 0.01 {
                                this.apply_motion();
                            }
                        }
                    }
                }
            }));
        for kind in 0..3 {
            let weak = this.to_ref().downgrade();
            let subscription = match kind {
                0 => p.month_viewport.add_disposable_handler_with(
                    InputElement::pointer_pressed_event(),
                    move |_, e| {
                        if let Some(this) = weak.upgrade() {
                            this.begin_swipe(e);
                        }
                    },
                    RoutingStrategies::TUNNEL,
                    true,
                ),
                1 => p.month_viewport.add_disposable_handler_with(
                    InputElement::pointer_moved_event(),
                    move |_, e| {
                        if let Some(this) = weak.upgrade() {
                            this.move_swipe(e);
                        }
                    },
                    RoutingStrategies::TUNNEL,
                    true,
                ),
                _ => p.month_viewport.add_disposable_handler_with(
                    InputElement::pointer_released_event(),
                    move |_, e| {
                        if let Some(this) = weak.upgrade() {
                            this.end_swipe(e);
                        }
                    },
                    RoutingStrategies::TUNNEL,
                    true,
                ),
            };
            this.subscriptions.borrow_mut().push(subscription);
        }
        let weak = this.to_ref().downgrade();
        this.subscriptions
            .borrow_mut()
            .push(p.month_viewport.add_disposable_handler_with(
                InputElement::pointer_capture_lost_event(),
                move |_, _| {
                    if let Some(this) = weak.upgrade() {
                        if this.swipe.take().is_some() {
                            this.snap_month(0.0);
                        }
                    }
                },
                RoutingStrategies::BUBBLE,
                true,
            ));
        p.range_months.set_owner(this);
        this.rebuild_weekdays();
        this.refresh_calendar();
        this.sync_inputs();
        this.refresh_layout();
        this.finish_motion();
    }
}

impl MaterialDatePicker {
    pub fn construct() -> Self {
        Self {
            base: TemplatedControl::construct(),
            state: Cell::new(MaterialDatePickerState::default()),
            year_open: Cell::new(false),
            engine: Calendar::new(),
            blackout_handler: Cell::new(None),
            date_policy: RefCell::new(None),
            year_policy: RefCell::new(None),
            parts: RefCell::new(None),
            subscriptions: RefCell::new(Vec::new()),
            clock_subscription: RefCell::new(None),
            visibility_subscription: RefCell::new(None),
            motion: RefCell::new(Motion::new(MaterialMotionScheme::Expressive)),
            last_frame: Cell::new(None),
            updating_input: Cell::new(false),
            input_digits: RefCell::new([String::new(), String::new()]),
            input_errors: RefCell::new([None, None]),
            changing_range_offset: Cell::new(false),
            range_position_pending: Cell::new(true),
            complete_notified: Cell::new(false),
            input_height: Cell::new(90.0),
            swipe: Cell::new(None),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    fn parts(&self) -> Option<Rc<Parts>> {
        self.parts.borrow().clone()
    }
    pub fn state(&self) -> MaterialDatePickerState {
        self.state.get()
    }
    pub fn selected_date(&self) -> Option<DateTime> {
        self.state().selected_start
    }
    pub fn selected_start_date(&self) -> Option<DateTime> {
        self.state().selected_start
    }
    pub fn selected_end_date(&self) -> Option<DateTime> {
        self.state().selected_end
    }
    pub fn displayed_month(&self) -> DateTime {
        self.state().displayed_month
    }
    pub fn is_selection_complete(&self) -> bool {
        self.state().is_complete() && self.input_errors.borrow().iter().all(Option::is_none)
    }
    pub fn is_year_picker_open(&self) -> bool {
        self.year_open.get()
    }
    pub fn display_mode(&self) -> MaterialDatePickerMode {
        self.get_value(Self::display_mode_property())
    }
    pub fn set_display_mode(&self, v: MaterialDatePickerMode) {
        self.set_value(Self::display_mode_property(), v);
    }
    pub fn selection_mode(&self) -> MaterialDateSelectionMode {
        self.get_value(Self::selection_mode_property())
    }
    pub fn set_selection_mode(&self, v: MaterialDateSelectionMode) {
        self.set_value(Self::selection_mode_property(), v);
    }
    pub fn first_year(&self) -> i32 {
        self.get_value(Self::first_year_property())
    }
    pub fn set_first_year(&self, v: i32) {
        self.set_value(Self::first_year_property(), v);
    }
    pub fn last_year(&self) -> i32 {
        self.get_value(Self::last_year_property())
    }
    pub fn set_last_year(&self, v: i32) {
        self.set_value(Self::last_year_property(), v);
    }
    pub fn culture(&self) -> CultureInfo {
        self.get_value(Self::culture_property())
            .unwrap_or_else(CultureInfo::current_culture)
    }
    pub fn set_culture(&self, v: Option<CultureInfo>) {
        self.set_value(Self::culture_property(), v);
    }
    pub fn first_day_of_week(&self) -> DayOfWeek {
        self.get_value(Self::first_day_of_week_property())
            .unwrap_or_else(|| self.culture().date_time_format().first_day_of_week())
    }
    pub fn set_first_day_of_week(&self, v: Option<DayOfWeek>) {
        self.set_value(Self::first_day_of_week_property(), v);
    }
    pub fn today(&self) -> DateTime {
        day_start(self.get_value(Self::today_property()))
    }
    pub fn set_today(&self, v: DateTime) {
        self.set_value(Self::today_property(), day_start(v));
    }
    pub fn title(&self) -> Option<String> {
        self.get_value(Self::title_property())
    }
    pub fn set_title(&self, v: Option<String>) {
        self.set_value(Self::title_property(), v);
    }
    pub fn show_mode_toggle(&self) -> bool {
        self.get_value(Self::show_mode_toggle_property())
    }
    pub fn set_show_mode_toggle(&self, v: bool) {
        self.set_value(Self::show_mode_toggle_property(), v);
    }
    pub fn show_actions(&self) -> bool {
        self.get_value(Self::show_actions_property())
    }
    pub fn set_show_actions(&self, v: bool) {
        self.set_value(Self::show_actions_property(), v);
    }
    pub fn confirm_text(&self) -> String {
        self.get_value(Self::confirm_text_property())
    }
    pub fn set_confirm_text(&self, v: String) {
        self.set_value(Self::confirm_text_property(), v);
    }
    pub fn cancel_text(&self) -> String {
        self.get_value(Self::cancel_text_property())
    }
    pub fn set_cancel_text(&self, v: String) {
        self.set_value(Self::cancel_text_property(), v);
    }
    pub fn range_calendar_height(&self) -> f64 {
        self.get_value(Self::range_calendar_height_property())
    }
    pub fn set_range_calendar_height(&self, v: f64) {
        self.set_value(Self::range_calendar_height_property(), v);
    }
    pub fn range_background(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::range_background_property())
    }
    pub fn set_range_background(&self, v: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::range_background_property(), v);
    }
    pub fn subhead_foreground(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::subhead_foreground_property())
    }
    pub fn set_subhead_foreground(&self, v: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::subhead_foreground_property(), v);
    }
    pub fn reduced_motion(&self) -> bool {
        self.get_value(Self::reduced_motion_property())
    }
    pub fn set_reduced_motion(&self, v: bool) {
        self.set_value(Self::reduced_motion_property(), v);
    }
    pub fn motion_scheme(&self) -> MaterialMotionScheme {
        self.get_value(Self::motion_scheme_property())
    }
    pub fn set_motion_scheme(&self, v: MaterialMotionScheme) {
        self.set_value(Self::motion_scheme_property(), v);
    }
    pub fn blackout_dates(&self) -> CalendarBlackoutDatesCollection {
        self.engine.blackout_dates()
    }
    pub fn set_date_validator(&self, policy: Option<Rc<dyn Fn(DateTime) -> bool>>) {
        *self.date_policy.borrow_mut() = policy;
        self.refresh_selectable_dates();
    }
    pub fn set_year_validator(&self, policy: Option<Rc<dyn Fn(i32) -> bool>>) {
        *self.year_policy.borrow_mut() = policy;
        self.refresh_selectable_dates();
    }
    pub fn is_year_selectable(&self, year: i32) -> bool {
        self.state().contains_year(year)
            && self.year_policy.borrow().as_ref().is_none_or(|p| p(year))
    }
    pub fn is_date_selectable(&self, date: DateTime) -> bool {
        let date = day_start(date);
        self.is_year_selectable(date.year())
            && !self.blackout_dates().contains_date(date)
            && self.date_policy.borrow().as_ref().is_none_or(|p| p(date))
    }
    pub fn refresh_selectable_dates(&self) {
        self.refresh_calendar();
        self.refresh_years();
        self.validate_inputs();
    }
    pub fn set_selected_date(&self, date: Option<DateTime>) {
        self.set_selection(date, None);
    }
    pub fn set_selected_start_date(&self, date: Option<DateTime>) {
        self.set_selection(date, self.selected_end_date());
    }
    pub fn set_selected_end_date(&self, date: Option<DateTime>) {
        self.set_selection(self.selected_start_date(), date);
    }
    pub fn set_selection(&self, start: Option<DateTime>, end: Option<DateTime>) {
        let mut state = self.state();
        state.set_selection(start, end);
        self.publish(state, true);
    }
    pub fn select_date(&self, date: DateTime) -> bool {
        if !self.is_effectively_enabled() || !self.is_date_selectable(date) {
            return false;
        }
        let mut state = self.state();
        state.select_date(date);
        self.publish(state, true);
        true
    }
    pub fn set_displayed_month(&self, date: DateTime) {
        let mut state = self.state();
        state.set_displayed_month(date);
        self.publish(state, false);
    }
    pub fn restore_state(&self, state: MaterialDatePickerState) {
        self.set_first_year(state.first_year);
        self.set_last_year(state.last_year);
        self.set_selection_mode(state.selection_mode);
        self.set_selection(state.selected_start, state.selected_end);
        self.set_displayed_month(state.displayed_month);
        self.set_display_mode(state.mode);
    }
    pub fn confirm(&self) -> bool {
        if !self.is_selection_complete() || !self.is_effectively_enabled() {
            return false;
        }
        self.raise_event(&RoutedEventArgs::with_event(Self::confirmed_event()));
        true
    }
    pub fn dismiss(&self) {
        self.raise_event(&RoutedEventArgs::with_event(Self::dismissed_event()));
    }
    pub fn input_format(&self) -> MaterialDateInputFormat {
        MaterialDateInputFormat::for_culture(&self.culture().date_time_format())
    }
    pub fn input_error(&self, end: bool) -> Option<String> {
        self.input_errors.borrow()[usize::from(end)].clone()
    }
    pub(crate) fn format_date_description(&self, date: DateTime) -> String {
        date.to_string_format("D", &self.culture())
    }
    pub(crate) fn format_month(&self, month: DateTime) -> String {
        month.to_string_format("Y", &self.culture())
    }
    fn format_headline(&self, date: DateTime) -> String {
        let culture = self.culture();
        let info = culture.date_time_format();
        let pattern = if info.short_date_pattern().starts_with('d') {
            "d MMM yyyy"
        } else if info.short_date_pattern().starts_with('y') {
            "yyyy MMM d"
        } else {
            "MMM d, yyyy"
        };
        date.to_string_format(pattern, &culture)
    }
    fn publish(&self, state: MaterialDatePickerState, update_inputs: bool) {
        let old = self.state.replace(state);
        if old.selected_start != state.selected_start {
            self.raise_direct_property_changed(
                Self::selected_date_property(),
                &old.selected_start,
                &state.selected_start,
            );
            self.raise_direct_property_changed(
                Self::selected_start_date_property(),
                &old.selected_start,
                &state.selected_start,
            );
        }
        if old.selected_end != state.selected_end {
            self.raise_direct_property_changed(
                Self::selected_end_date_property(),
                &old.selected_end,
                &state.selected_end,
            );
        }
        if old.displayed_month != state.displayed_month {
            self.raise_direct_property_changed(
                Self::displayed_month_property(),
                &old.displayed_month,
                &state.displayed_month,
            );
            self.refresh_calendar();
            self.scroll_range_to_month();
        } else {
            self.refresh_calendar_selection();
        }
        if update_inputs {
            self.sync_inputs();
        }
        self.refresh_header();
        self.notify_complete();
        if old.selected_start != state.selected_start || old.selected_end != state.selected_end {
            self.raise_event(&RoutedEventArgs::with_event(Self::selection_changed_event()));
        }
    }
    fn refresh_header(&self) {
        let Some(p) = self.parts() else { return };
        let range = self.selection_mode() == MaterialDateSelectionMode::Range;
        let input = self.display_mode() == MaterialDatePickerMode::Input;
        let title = self.title().unwrap_or_else(|| {
            match (range, input) {
                (false, false) => "Select date",
                (false, true) => "Enter date",
                (true, false) => "Select dates",
                (true, true) => "Enter dates",
            }
            .into()
        });
        p.title.set_text(Some(&title));
        let headline = if range {
            format!(
                "{} – {}",
                self.selected_start_date()
                    .map(|d| self.format_headline(d))
                    .unwrap_or_else(|| "Start date".into()),
                self.selected_end_date()
                    .map(|d| self.format_headline(d))
                    .unwrap_or_else(|| "End date".into())
            )
        } else {
            self.selected_date()
                .map(|d| self.format_headline(d))
                .unwrap_or_else(|| {
                    if input {
                        "Entered date"
                    } else {
                        "Selected date"
                    }
                    .into()
                })
        };
        p.headline.set_text(Some(&headline));
        p.range_headline_start.set_text(Some(
            &self
                .selected_start_date()
                .map(|d| self.format_headline(d))
                .unwrap_or_else(|| "Start date".into()),
        ));
        p.range_headline_end.set_text(Some(
            &self
                .selected_end_date()
                .map(|d| self.format_headline(d))
                .unwrap_or_else(|| "End date".into()),
        ));
        AutomationProperties::set_name(
            &p.range_headline,
            Some(&format!(
                "Start date: {}, End date: {}",
                self.selected_start_date()
                    .map(|d| self.format_date_description(d))
                    .unwrap_or_else(|| "No selection".into()),
                self.selected_end_date()
                    .map(|d| self.format_date_description(d))
                    .unwrap_or_else(|| "No selection".into())
            )),
        );
        AutomationProperties::set_live_setting(&p.range_headline, AutomationLiveSetting::Polite);
        AutomationProperties::set_live_setting(&p.headline, AutomationLiveSetting::Polite);
        p.month_label
            .set_text(Some(&self.format_month(self.displayed_month())));
        AutomationProperties::set_name(
            &p.year_toggle,
            Some(&format!(
                "{}, {}",
                self.format_month(self.displayed_month()),
                if self.is_year_picker_open() {
                    "Switch to selecting a day"
                } else {
                    "Switch to selecting a year"
                }
            )),
        );
        p.previous
            .set_is_enabled(self.state().month_index(self.displayed_month()) > 0);
        p.next.set_is_enabled(
            self.state().month_index(self.displayed_month()) < self.state().month_count() - 1,
        );
        p.confirm.set_is_enabled(self.is_selection_complete());
        p.confirm
            .set_content(Some(Rc::new(self.confirm_text()) as BoxedValue));
        p.cancel
            .set_content(Some(Rc::new(self.cancel_text()) as BoxedValue));
        AutomationProperties::set_name(
            &p.toggle,
            Some(if input {
                "Switch to calendar input mode"
            } else {
                "Switch to text input mode"
            }),
        );
    }
    fn refresh_layout(&self) {
        let Some(p) = self.parts() else { return };
        let range = self.selection_mode() == MaterialDateSelectionMode::Range;
        let input = self.display_mode() == MaterialDatePickerMode::Input;
        self.pseudo_classes().set(":range", range);
        self.pseudo_classes().set(":input", input);
        p.header.set_min_height(if range { 68.0 } else { 120.0 });
        p.title.set_margin(Thickness::new(
            if range { 64.0 } else { 24.0 },
            if range { 0.0 } else { 16.0 },
            12.0,
            0.0,
        ));
        p.headline.set_margin(Thickness::new(
            if range { 64.0 } else { 24.0 },
            0.0,
            12.0,
            12.0,
        ));
        p.headline.set_is_visible(!range);
        p.range_headline.set_is_visible(range);
        p.toggle.set_is_visible(self.show_mode_toggle());
        p.edit_icon.set_is_visible(!input);
        p.calendar_icon.set_is_visible(input);
        p.navigation.set_is_visible(!range);
        p.month_viewport.set_is_visible(!range);
        p.range.set_is_visible(range);
        p.range
            .set_height((self.range_calendar_height() - 48.0).max(96.0));
        p.end_input.set_is_visible(range);
        Grid::set_column_span(&p.start_input, if range { 1 } else { 2 });
        p.start_input.set_margin(if range {
            Thickness::new(0.0, 0.0, 4.0, 0.0)
        } else {
            Thickness::default()
        });
        p.end_input.set_margin(Thickness::new(4.0, 0.0, 0.0, 0.0));
        MaterialTextField::set_label(
            &p.start_input,
            Some(if range { "Start date" } else { "Date" }),
        );
        MaterialTextField::set_label(&p.end_input, Some("End date"));
        p.actions.set_is_visible(self.show_actions());
        p.range_months.set_owner(self);
        self.refresh_header();
        self.apply_motion();
        self.scroll_range_to_month();
    }
    fn rebuild_weekdays(&self) {
        let Some(p) = self.parts() else { return };
        p.weekdays.children().clear();
        let culture = self.culture();
        let info = culture.date_time_format();
        for offset in 0..7 {
            let day = ((self.first_day_of_week().to_i32() + offset) % 7) as usize;
            let text = TextBlock::new();
            let narrow = info.shortest_day_names()[day]
                .chars()
                .next()
                .map(|c| c.to_string())
                .unwrap_or_default();
            text.set_text(Some(&narrow));
            text.set_font_size(16.0);
            text.set_line_height(24.0);
            text.set_text_alignment(TextAlignment::Center);
            text.set_vertical_alignment(VerticalAlignment::Center);
            text.set_horizontal_alignment(HorizontalAlignment::Center);
            AutomationProperties::set_name(&text, Some(&info.day_names()[day]));
            p.weekdays.children().add(text.upcast::<Control>());
        }
    }
    fn refresh_calendar(&self) {
        let Some(p) = self.parts() else { return };
        let current = self.displayed_month();
        for (index, view) in p.month_views.iter().enumerate() {
            if let Some(month) = current
                .try_add_months(index as i32 - 1)
                .filter(|m| self.state().contains_year(m.year()))
            {
                view.set_is_visible(true);
                view.set_month(self, month);
                view.set_is_hit_test_visible(index == 1);
                for child in view.children().snapshot().iter() {
                    child.set_focusable(index == 1);
                    AutomationProperties::set_accessibility_view(
                        &child,
                        if index == 1 {
                            AccessibilityView::Default
                        } else {
                            AccessibilityView::Raw
                        },
                    );
                }
            } else {
                view.set_is_visible(false);
            }
        }
        p.range_months.refresh();
        self.refresh_years();
        self.apply_motion();
    }
    fn refresh_calendar_selection(&self) {
        if let Some(p) = self.parts() {
            for view in &p.month_views {
                view.refresh();
            }
            p.range_months.refresh();
        }
    }
    fn rebuild_years(&self) {
        let Some(p) = self.parts() else { return };
        if !self.is_year_picker_open() && p.year_grid.children().is_empty() {
            return;
        }
        p.year_grid.children().clear();
        for year in self.state().first_year..=self.state().last_year {
            p.year_grid
                .children()
                .add(MaterialDateCell::for_year(self, year).upcast::<Control>());
        }
    }
    fn refresh_years(&self) {
        if let Some(p) = self.parts() {
            for child in p.year_grid.children().snapshot().iter() {
                if let Some(cell) = child.cast::<MaterialDateCell>() {
                    cell.refresh();
                }
            }
        }
    }
    pub fn set_is_year_picker_open(&self, open: bool) {
        let open = open
            && self.selection_mode() == MaterialDateSelectionMode::Single
            && self.display_mode() == MaterialDatePickerMode::Picker;
        if !self.set_and_raise_cell(Self::is_year_picker_open_property(), &self.year_open, open) {
            return;
        }
        self.pseudo_classes().set(":years", open);
        if open {
            self.rebuild_years();
        }
        if let Some(p) = self.parts() {
            p.previous.set_is_visible(!open);
            p.next.set_is_visible(!open);
            if open {
                let row = ((self.displayed_month().year() - self.state().first_year - 3).max(0) / 3)
                    as f64;
                p.years.set_offset(Vector::new(0.0, row * 64.0));
                self.queue_focus_year(self.displayed_month().year());
            }
        }
        self.refresh_header();
        let scheme = self.motion_scheme();
        {
            let mut m = self.motion.borrow_mut();
            m.year_size.retarget(if open { 1.0 } else { 0.0 });
            m.year_alpha.set_spec(scheme.spring(if open {
                MaterialMotionRole::DefaultEffects
            } else {
                MaterialMotionRole::FastEffects
            }));
            if open && m.year_alpha.current().value < 0.6 {
                m.year_alpha = MaterialSpringState::new(
                    scheme.spring(MaterialMotionRole::DefaultEffects),
                    0.6,
                );
            }
            m.year_alpha.retarget(if open { 1.0 } else { 0.0 });
        }
        self.advance(0.0);
        self.update_clock();
    }
    pub fn select_year(&self, year: i32) -> bool {
        if !self.is_year_selectable(year) {
            return false;
        }
        self.set_displayed_month(DateTime::new(year, self.displayed_month().month(), 1));
        self.set_is_year_picker_open(false);
        if let Some(p) = self.parts() {
            p.year_toggle
                .focus_with(NavigationMethod::Tab, KeyModifiers::NONE);
        }
        true
    }
    pub fn navigate_month(&self, delta: i32) {
        let state = self.state();
        let old = state.month_index(state.displayed_month);
        let index = (old + delta).clamp(0, state.month_count() - 1);
        if old == index {
            return;
        }
        let width = self.month_width();
        let offset =
            self.motion.borrow().month_offset.current().value + (index - old) as f64 * width;
        self.set_displayed_month(state.month_at(index));
        {
            let mut m = self.motion.borrow_mut();
            m.month_offset = MaterialSpringState::new(
                self.motion_scheme()
                    .spring(MaterialMotionRole::DefaultEffects),
                offset,
            );
            m.month_offset.retarget(0.0);
        }
        self.advance(0.0);
        self.update_clock();
    }
    fn scroll_range_to_month(&self) {
        if self.changing_range_offset.get()
            || self.selection_mode() != MaterialDateSelectionMode::Range
        {
            return;
        }
        let Some(p) = self.parts() else { return };
        let offset = self.state().month_index(self.displayed_month()) as f64 * 336.0;
        if p.range.extent().height <= 0.0 || !p.range.is_effectively_visible() {
            self.range_position_pending.set(true);
            return;
        }
        self.changing_range_offset.set(true);
        p.range_months.realize(offset, p.range.height());
        p.range.set_offset(Vector::new(0.0, offset));
        self.range_position_pending.set(false);
        self.changing_range_offset.set(false);
    }
    fn range_scrolled(&self) {
        if self.changing_range_offset.get()
            || self.range_position_pending.get()
            || self.selection_mode() != MaterialDateSelectionMode::Range
        {
            return;
        }
        let Some(p) = self.parts() else { return };
        self.changing_range_offset.set(true);
        p.range_months
            .realize(p.range.offset().y, p.range.viewport().height);
        let index = (p.range.offset().y / 336.0).floor() as i32;
        self.set_displayed_month(self.state().month_at(index));
        self.changing_range_offset.set(false);
    }
    fn sync_inputs(&self) {
        if self.updating_input.get() {
            return;
        }
        let format = self.input_format();
        let dates = [self.selected_start_date(), self.selected_end_date()];
        self.updating_input.set(true);
        for (index, date) in dates.into_iter().enumerate() {
            let digits = date.map(|d| format.digits(d)).unwrap_or_default();
            self.input_digits.borrow_mut()[index] = digits.clone();
            if let Some(p) = self.parts() {
                let editor = if index == 0 {
                    &p.start_input
                } else {
                    &p.end_input
                };
                editor.set_text(Some(&format.format_digits(&digits)));
                editor.set_caret_index(editor.text().map_or(0, |text| text.len() as i32));
                editor.set_placeholder_text(Some(&format.pattern()));
            }
        }
        self.updating_input.set(false);
        self.validate_inputs();
    }
    fn input_changed(&self, index: usize) {
        if self.updating_input.get() {
            return;
        }
        let Some(p) = self.parts() else { return };
        let editor = if index == 0 {
            &p.start_input
        } else {
            &p.end_input
        };
        let format = self.input_format();
        let text = editor.text().unwrap_or_default();
        let Some(digits) = format.extract_digits(&text) else {
            self.updating_input.set(true);
            editor.set_text(Some(
                &format.format_digits(&self.input_digits.borrow()[index]),
            ));
            self.updating_input.set(false);
            return;
        };
        let before_caret = text
            .chars()
            .take(editor.caret_index().max(0) as usize)
            .filter(char::is_ascii_digit)
            .count();
        let formatted = format.format_digits(&digits);
        self.input_digits.borrow_mut()[index] = digits;
        if text != formatted {
            self.updating_input.set(true);
            editor.set_text(Some(&formatted));
            let mut count = 0;
            let mut caret = 0;
            for c in formatted.chars() {
                if c.is_ascii_digit() {
                    if count == before_caret {
                        break;
                    }
                    count += 1;
                }
                caret += 1;
            }
            editor.set_caret_index(caret);
            self.updating_input.set(false);
        }
        self.validate_inputs();
        let digits = self.input_digits.borrow().clone();
        let errors = self.input_errors.borrow().clone();
        let value = if errors[index].is_none() {
            format.parse_digits(&digits[index])
        } else {
            None
        };
        let start = if index == 0 {
            value
        } else {
            self.selected_start_date()
        };
        let end = if self.selection_mode() == MaterialDateSelectionMode::Range {
            if index == 1 {
                value
            } else {
                self.selected_end_date()
            }
        } else {
            None
        };
        let mut state = self.state();
        state.set_selection(start, end);
        self.updating_input.set(true);
        self.publish(state, false);
        self.updating_input.set(false);
    }
    // Delimiters are display characters, so deletion at one must remove the
    // neighboring underlying digit. The actual mutation still goes through the
    // existing TextBox edit path, preserving selection, undo and IME ownership.
    fn prepare_input_deletion(&self, index: usize, event: &KeyEventArgs) {
        if !matches!(event.key, Key::Back | Key::Delete) {
            return;
        }
        let Some(parts) = self.parts() else { return };
        let editor = if index == 0 {
            &parts.start_input
        } else {
            &parts.end_input
        };
        if editor.selection_start() != editor.selection_end() {
            return;
        }
        let text = editor.text().unwrap_or_default();
        let chars: Vec<char> = text.chars().collect();
        let caret = (editor.caret_index().max(0) as usize).min(chars.len());
        if event.key == Key::Back && caret > 0 && !chars[caret - 1].is_ascii_digit() {
            if let Some(index) = chars[..caret].iter().rposition(char::is_ascii_digit) {
                editor.set_selection_start(index as i32);
                editor.set_selection_end(caret as i32);
            }
        } else if event.key == Key::Delete && caret < chars.len() && !chars[caret].is_ascii_digit()
        {
            if let Some(index) = chars[caret..].iter().position(char::is_ascii_digit) {
                editor.set_selection_start(caret as i32);
                editor.set_selection_end((caret + index + 1) as i32);
            }
        }
    }
    fn validate_inputs(&self) {
        let format = self.input_format();
        let digits = self.input_digits.borrow().clone();
        let parsed = [
            format.parse_digits(&digits[0]),
            format.parse_digits(&digits[1]),
        ];
        let mut errors = [None, None];
        for index in 0..2 {
            if index == 1 && self.selection_mode() == MaterialDateSelectionMode::Single {
                continue;
            }
            if digits[index].len() != 8 {
                continue;
            }
            errors[index] = match parsed[index] {
                None => Some(format!(
                    "Date does not match expected pattern: {}",
                    format.pattern()
                )),
                Some(date) if !self.state().contains_year(date.year()) => Some(format!(
                    "Date must be between {} and {}",
                    self.state().first_year,
                    self.state().last_year
                )),
                Some(date) if !self.is_date_selectable(date) => Some(format!(
                    "Date not allowed: {}",
                    self.format_date_description(date)
                )),
                Some(_) => None,
            };
        }
        if self.selection_mode() == MaterialDateSelectionMode::Range {
            if parsed[0]
                .zip(self.selected_end_date())
                .is_some_and(|(start, end)| start > end)
                && errors[0].is_none()
            {
                errors[0] = Some("Start date must not be after end date".into());
            }
            if parsed[1]
                .zip(self.selected_start_date())
                .is_some_and(|(end, start)| end < start)
                && errors[1].is_none()
            {
                errors[1] = Some("End date must not be before start date".into());
            }
        }
        *self.input_errors.borrow_mut() = errors.clone();
        self.notify_complete();
        if let Some(p) = self.parts() {
            p.input.set_margin(Thickness::new(
                24.0,
                10.0,
                24.0,
                if errors.iter().any(Option::is_some) {
                    12.0
                } else {
                    16.0
                },
            ));
            for (index, editor) in [&p.start_input, &p.end_input].iter().enumerate() {
                MaterialTextField::set_error_text(editor, errors[index].as_deref());
                MaterialTextField::set_is_error(editor, errors[index].is_some());
            }
            p.confirm.set_is_enabled(self.is_selection_complete());
        }
    }
    fn notify_complete(&self) {
        self.set_and_raise_cell(
            Self::is_selection_complete_property(),
            &self.complete_notified,
            self.is_selection_complete(),
        );
    }
    fn queue_focus_input(&self) {
        let weak = self.to_ref().downgrade();
        Dispatcher::ui_thread().post_local(
            move || {
                if let Some(this) = weak.upgrade() {
                    if this.display_mode() == MaterialDatePickerMode::Input {
                        if let Some(p) = this.parts() {
                            p.start_input
                                .focus_with(NavigationMethod::Tab, KeyModifiers::NONE);
                        }
                    }
                }
            },
            DispatcherPriority::BACKGROUND,
        );
    }
    fn queue_focus_year(&self, year: i32) {
        let weak = self.to_ref().downgrade();
        Dispatcher::ui_thread().post_local(
            move || {
                if let Some(this) = weak.upgrade() {
                    this.focus_year(year);
                }
            },
            DispatcherPriority::BACKGROUND,
        );
    }
    fn focus_year(&self, year: i32) -> bool {
        let Some(p) = self.parts() else { return false };
        let found = p
            .year_grid
            .children()
            .snapshot()
            .iter()
            .filter_map(|c| c.cast::<MaterialDateCell>())
            .find(|c| c.year() == Some(year) && c.is_effectively_enabled());
        if let Some(cell) = found {
            KeyboardNavigation::set_tab_once_active_element(
                &p.year_grid,
                Some(cell.clone().upcast()),
            );
            cell.bring_into_view();
            cell.focus_with(NavigationMethod::Directional, KeyModifiers::NONE)
        } else {
            false
        }
    }
    fn focus_date(&self, date: DateTime) -> bool {
        if month_start(date) != self.displayed_month() {
            self.set_displayed_month(date);
        }
        let Some(p) = self.parts() else { return false };
        let root: Ref<Control> = if self.selection_mode() == MaterialDateSelectionMode::Range {
            p.range_months.clone().upcast()
        } else {
            p.month_views[1].clone().upcast()
        };
        let cell = root
            .get_visual_descendants()
            .filter_map(|c| c.cast::<MaterialDateCell>())
            .find(|c| c.date() == Some(date));
        cell.is_some_and(|c| c.focus_with(NavigationMethod::Directional, KeyModifiers::NONE))
    }
    pub(crate) fn cell_key(&self, cell: &MaterialDateCell, e: &KeyEventArgs) -> bool {
        if e.handled() {
            return false;
        }
        let rtl = self.flow_direction() == FlowDirection::RightToLeft;
        if let Some(year) = cell.year() {
            let delta = match e.key {
                Key::Left => {
                    if rtl {
                        1
                    } else {
                        -1
                    }
                }
                Key::Right => {
                    if rtl {
                        -1
                    } else {
                        1
                    }
                }
                Key::Up => -3,
                Key::Down => 3,
                _ => return false,
            };
            let mut next = year + delta;
            while self.state().contains_year(next) {
                if self.is_year_selectable(next) {
                    return self.focus_year(next);
                }
                next += delta;
            }
            return true;
        }
        let Some(date) = cell.date() else {
            return false;
        };
        let delta = match e.key {
            Key::Left => {
                if rtl {
                    1
                } else {
                    -1
                }
            }
            Key::Right => {
                if rtl {
                    -1
                } else {
                    1
                }
            }
            Key::Up => -7,
            Key::Down => 7,
            Key::PageUp => {
                if let Some(d) = date.try_add_months(-1) {
                    return self.focus_date(d);
                }
                return true;
            }
            Key::PageDown => {
                if let Some(d) = date.try_add_months(1) {
                    return self.focus_date(d);
                }
                return true;
            }
            _ => return false,
        };
        let mut target = date;
        while let Some(next) = target.try_add_days(delta as f64) {
            if !self.state().contains_year(next.year()) {
                break;
            }
            if self.is_date_selectable(next) {
                return self.focus_date(next);
            }
            target = next;
        }
        true
    }
    fn start_mode_motion(&self) {
        let input = self.display_mode() == MaterialDatePickerMode::Input;
        let range = self.selection_mode() == MaterialDateSelectionMode::Range;
        let scheme = self.motion_scheme();
        {
            let mut m = self.motion.borrow_mut();
            m.mode.set_spec(scheme.spring(if range {
                MaterialMotionRole::FastEffects
            } else {
                MaterialMotionRole::DefaultSpatial
            }));
            m.mode.retarget(if input { 1.0 } else { 0.0 });
            m.calendar_alpha.set_spec(scheme.spring(if range || input {
                MaterialMotionRole::FastEffects
            } else {
                MaterialMotionRole::DefaultEffects
            }));
            m.calendar_alpha.retarget(if input { 0.0 } else { 1.0 });
            m.input_alpha.set_spec(scheme.spring(if input && !range {
                MaterialMotionRole::DefaultEffects
            } else {
                MaterialMotionRole::FastEffects
            }));
            m.input_alpha.retarget(if input { 1.0 } else { 0.0 });
        }
        self.refresh_layout();
        self.advance(0.0);
        self.update_clock();
    }
    fn month_width(&self) -> f64 {
        self.parts()
            .map_or(336.0, |p| p.month_viewport.bounds().width.max(336.0))
    }
    fn apply_motion(&self) {
        let Some(p) = self.parts() else { return };
        let m = self.motion.borrow();
        let input = m.mode.current().value;
        let calendar_height = if self.selection_mode() == MaterialDateSelectionMode::Range {
            self.range_calendar_height()
        } else {
            392.0
        };
        let input_height = self.input_height.get();
        if self.selection_mode() == MaterialDateSelectionMode::Range {
            // The pinned range picker uses Crossfade(FastEffects), rather than
            // the single picker's spatial AnimatedContent/size transform.
            let keep_calendar = self.display_mode() == MaterialDatePickerMode::Picker
                || m.calendar_alpha.current().value > 0.001;
            p.body.set_height(if keep_calendar {
                calendar_height.max(input_height)
            } else {
                input_height
            });
            p.calendar_transform.set_y(0.0);
            p.input_transform.set_y(0.0);
        } else {
            p.body
                .set_height((calendar_height + (input_height - calendar_height) * input).max(0.0));
            p.calendar_transform.set_y(-48.0 * input);
            p.input_transform.set_y(input_height * (1.0 - input));
        }
        p.calendar
            .set_opacity(m.calendar_alpha.current().value.clamp(0.0, 1.0));
        p.input
            .set_opacity(m.input_alpha.current().value.clamp(0.0, 1.0));
        p.calendar.set_is_visible(
            self.display_mode() == MaterialDatePickerMode::Picker || p.calendar.opacity() > 0.001,
        );
        p.input.set_is_visible(
            self.display_mode() == MaterialDatePickerMode::Input || p.input.opacity() > 0.001,
        );
        p.calendar
            .set_is_hit_test_visible(self.display_mode() == MaterialDatePickerMode::Picker);
        p.input
            .set_is_hit_test_visible(self.display_mode() == MaterialDatePickerMode::Input);
        p.years
            .set_height((336.0 * m.year_size.current().value).clamp(0.0, 336.0));
        p.years
            .set_opacity(m.year_alpha.current().value.clamp(0.0, 1.0));
        p.years
            .set_is_visible(self.is_year_picker_open() || p.years.opacity() > 0.001);
        p.years.set_is_hit_test_visible(self.is_year_picker_open());
        for (index, t) in p.month_transforms.iter().enumerate() {
            t.set_x((index as f64 - 1.0) * self.month_width() + m.month_offset.current().value);
        }
        AutomationProperties::set_accessibility_view(
            &p.calendar,
            if self.display_mode() == MaterialDatePickerMode::Picker {
                AccessibilityView::Default
            } else {
                AccessibilityView::Raw
            },
        );
        AutomationProperties::set_accessibility_view(
            &p.input,
            if self.display_mode() == MaterialDatePickerMode::Input {
                AccessibilityView::Default
            } else {
                AccessibilityView::Raw
            },
        );
    }
    fn advance(&self, seconds: f64) {
        let reduced = !animations_enabled(self.reduced_motion());
        {
            let mut motion = self.motion.borrow_mut();
            for spring in motion.springs() {
                spring.set_reduced_motion(reduced);
                spring.advance(seconds);
                if spring.is_at_rest(0.001, 0.0625) {
                    spring.finish();
                }
            }
        }
        self.apply_motion();
        if !self.motion_pending() {
            self.stop_clock();
        }
    }
    fn finish_motion(&self) {
        for spring in self.motion.borrow_mut().springs() {
            spring.finish();
        }
        self.apply_motion();
    }
    fn motion_pending(&self) -> bool {
        self.motion
            .borrow_mut()
            .springs()
            .iter()
            .any(|s| !s.is_at_rest(0.001, 0.0625))
    }
    fn update_clock(&self) {
        if !self.is_attached_to_visual_tree()
            || !self.is_effectively_visible()
            || !self.motion_pending()
        {
            self.stop_clock();
            return;
        }
        if self.clock_subscription.borrow().is_some() {
            return;
        }
        let Some(clock) = FerroLocator::current().get_service::<dyn IGlobalClock>() else {
            return;
        };
        let weak = self.to_ref().downgrade();
        self.last_frame.set(None);
        *self.clock_subscription.borrow_mut() = Some(clock.subscribe(Rc::new(
            AnonymousObserver::new(move |time: TimeSpan| {
                if let Some(this) = weak.upgrade() {
                    let now = time.total_seconds();
                    if now.is_finite() {
                        let elapsed = this
                            .last_frame
                            .replace(Some(now))
                            .map_or(0.0, |last| (now - last).max(0.0));
                        this.advance(elapsed);
                    }
                }
            }),
        )));
    }
    fn stop_clock(&self) {
        if let Some(s) = self.clock_subscription.borrow_mut().take() {
            s.dispose();
        }
        self.last_frame.set(None);
    }
    fn begin_swipe(&self, e: &PointerPressedEventArgs) {
        if self.is_year_picker_open()
            || self.selection_mode() != MaterialDateSelectionMode::Single
            || !self.is_effectively_enabled()
            || e.pointer().type_() == PointerType::Mouse && !e.properties().is_left_button_pressed
        {
            return;
        }
        let point = e.get_position(Some(self));
        self.swipe.set(Some(Swipe {
            id: e.pointer().id(),
            origin: point,
            previous: point,
            timestamp: e.timestamp(),
            velocity: 0.0,
            active: false,
        }));
    }
    fn move_swipe(&self, e: &PointerEventArgs) {
        let Some(mut swipe) = self.swipe.get().filter(|s| s.id == e.pointer().id()) else {
            return;
        };
        let point = e.get_position(Some(self));
        let diff = point - swipe.origin;
        let mut first = false;
        if !swipe.active {
            if diff.y.abs() > 8.0 && diff.y.abs() > diff.x.abs() {
                self.swipe.set(None);
                return;
            }
            if diff.x.abs() <= 8.0 {
                return;
            }
            swipe.active = true;
            first = true;
            if let Some(p) = self.parts() {
                e.pointer()
                    .capture(Some(&p.month_viewport.clone().upcast()));
            }
            e.prevent_gesture_recognition();
        }
        let movement = point.x - swipe.previous.x;
        let sign = if self.flow_direction() == FlowDirection::RightToLeft {
            -1.0
        } else {
            1.0
        };
        let delta = (movement - if first { 8.0 * movement.signum() } else { 0.0 }) * sign;
        let elapsed = (e.timestamp().saturating_sub(swipe.timestamp)) as f64 / 1000.0;
        if elapsed > 0.0 {
            swipe.velocity = (movement * sign / elapsed).clamp(-8000.0, 8000.0);
        }
        let mut offset = self.motion.borrow().month_offset.current().value + delta;
        let width = self.month_width();
        while offset <= -width
            && self.state().month_index(self.displayed_month()) < self.state().month_count() - 1
        {
            self.set_displayed_month(self.displayed_month().add_months(1));
            offset += width;
        }
        while offset >= width && self.state().month_index(self.displayed_month()) > 0 {
            self.set_displayed_month(self.displayed_month().add_months(-1));
            offset -= width;
        }
        if self.state().month_index(self.displayed_month()) == 0 {
            offset = offset.min(0.0);
        }
        if self.state().month_index(self.displayed_month()) == self.state().month_count() - 1 {
            offset = offset.max(0.0);
        }
        self.motion.borrow_mut().month_offset = MaterialSpringState::new(
            self.motion_scheme()
                .spring(MaterialMotionRole::DefaultEffects),
            offset,
        );
        swipe.previous = point;
        swipe.timestamp = e.timestamp();
        self.swipe.set(Some(swipe));
        self.apply_motion();
        e.set_handled(true);
    }
    fn end_swipe(&self, e: &PointerReleasedEventArgs) {
        if let Some(swipe) = self.swipe.take().filter(|s| s.id == e.pointer().id()) {
            if swipe.active {
                e.pointer().capture(None);
                self.snap_month(swipe.velocity);
                e.set_handled(true);
            }
        }
    }
    fn snap_month(&self, velocity: f64) {
        let offset = self.motion.borrow().month_offset.current().value;
        let page = if velocity.abs() >= 400.0 {
            if velocity < 0.0 { 1 } else { -1 }
        } else if offset.abs() > self.month_width() / 2.0 {
            if offset < 0.0 { 1 } else { -1 }
        } else {
            0
        };
        if page != 0 {
            self.navigate_month(page);
        }
        self.motion.borrow_mut().month_offset.retarget(0.0);
        self.advance(0.0);
        self.update_clock();
    }
    #[cfg(test)]
    pub(crate) fn has_clock_subscription(&self) -> bool {
        self.clock_subscription.borrow().is_some()
    }
}
impl Drop for MaterialDatePicker {
    fn drop(&mut self) {
        if let Some(token) = self.blackout_handler.take() {
            self.blackout_dates().remove_collection_changed(token);
        }
        for subscription in self.subscriptions.get_mut().drain(..) {
            subscription.dispose();
        }
        if let Some(subscription) = self.clock_subscription.get_mut().take() {
            subscription.dispose();
        }
        if let Some(subscription) = self.visibility_subscription.get_mut().take() {
            subscription.dispose();
        }
    }
}

#[repr(C)]
pub struct MaterialDatePickerAutomationPeer {
    base: ControlAutomationPeer,
}
ferro_class!(MaterialDatePickerAutomationPeer:ControlAutomationPeer);
ferro_class_info!(MaterialDatePickerAutomationPeer{interfaces:[Rc<dyn ISelectionProvider> => MaterialDatePickerAutomationPeer::provider]});
ferro_impl_classes!(MaterialDatePickerAutomationPeer:FerroObjectImpl,ControlAutomationPeerImpl);
impl AutomationPeerImpl for MaterialDatePickerAutomationPeer {
    fn get_automation_control_type_core(_: &Self) -> AutomationControlType {
        AutomationControlType::Calendar
    }
    fn get_class_name_core(_: &Self) -> String {
        "MaterialDatePicker".into()
    }
    fn is_control_element_core(_: &Self) -> bool {
        true
    }
    fn is_content_element_core(_: &Self) -> bool {
        true
    }
}
impl MaterialDatePickerAutomationPeer {
    pub fn new(owner: &MaterialDatePicker) -> Ref<Self> {
        instantiate(Self {
            base: ControlAutomationPeer::construct(owner),
        })
    }
    fn picker(&self) -> Ref<MaterialDatePicker> {
        self.owner().cast().expect("date picker peer owner")
    }
    fn provider(peer: Ref<Self>) -> Rc<dyn ISelectionProvider> {
        Rc::new(DatePickerSelectionProvider(peer))
    }
}
struct DatePickerSelectionProvider(Ref<MaterialDatePickerAutomationPeer>);
impl ISelectionProvider for DatePickerSelectionProvider {
    fn peer(&self) -> Ref<AutomationPeer> {
        self.0.clone().upcast()
    }
    fn can_select_multiple(&self) -> bool {
        self.0.picker().selection_mode() == MaterialDateSelectionMode::Range
    }
    fn is_selection_required(&self) -> bool {
        false
    }
    fn get_selection(&self) -> Vec<Ref<AutomationPeer>> {
        let picker = self.0.picker();
        picker
            .get_visual_descendants()
            .filter_map(|v| v.cast::<MaterialDateCell>())
            .filter(|c| c.is_selected() && c.is_effectively_visible() && !c.is_year())
            .map(|c| ControlAutomationPeer::create_peer_for_element(&c))
            .collect()
    }
}
