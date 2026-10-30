// Copyright 2023 The Android Open Source Project (reference behavior).
// Copyright 2026 FerroUI contributors (Rust adaptation).
// SPDX-License-Identifier: Apache-2.0
//! Material clock and keyboard time selection. Reference: AndroidX Material3
//! TimePicker.kt at e4d933c98df7baa7ea2b83eea6e086915961e0ff (1.5.0-beta02).
//! This is the reusable picker content; applications own their dialog title/actions.

use crate::motion::MaterialMotionScheme;
use crate::time_picker_dial::MaterialTimePickerDial;
use crate::time_picker_state::{
    MaterialTimeDisplayMode, MaterialTimeSelection, TimePickerMetrics, display_hour, parse_input,
};
use ferroui_base::animation::TimeSpan;
use ferroui_base::data::BindingMode;
use ferroui_base::input::platform::{ClipboardErrorKind, ClipboardExtensions};
use ferroui_base::input::text_input::{
    TextInputContentType, TextInputOptions, TextInputReturnKeyType,
};
use ferroui_base::input::{
    FocusChangedEventArgs, InputElement, InputElementImpl, InputElementImplExt, Key, KeyEventArgs,
    KeyModifiers, KeyboardNavigationHandler, NavigationDirection, NavigationMethod,
    TextInputEventArgs,
};
use ferroui_base::interactivity::{
    InteractiveImpl, RoutedEvent, RoutedEventArgs, RoutingStrategies,
};
use ferroui_base::layout::{LayoutableImpl, Orientation};
use ferroui_base::media::FlowDirection;
use ferroui_base::reactive::IDisposable;
use ferroui_base::threading::Dispatcher;
use ferroui_base::{
    DirectProperty, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    FerroPropertyChangedEventArgs, Rect, Ref, Size, StyledElementImpl, StyledProperty,
    StyledPropertyOptions, Visual, VisualImpl, VisualImplExt, VisualTreeAttachmentEventArgs,
    WeakRef, ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties,
    ferro_routed_event, ferro_routed_event_args, instantiate,
};
use ferroui_controls::automation::peers::{
    AutomationControlType, AutomationPeer, AutomationPeerImpl, ControlAutomationPeer,
    ControlAutomationPeerImpl,
};
use ferroui_controls::automation::provider::{IValueProvider, ProviderError};
use ferroui_controls::automation::{
    AutomationLiveSetting, AutomationProperties, ElementNotEnabledException,
};
use ferroui_controls::primitives::{
    TemplateAppliedEventArgs, TemplatedControl, TemplatedControlImpl, TemplatedControlImplExt,
    ToggleButton,
};
use ferroui_controls::{
    ControlImpl, Panel, PanelImpl, PastingFromClipboardEventArgs, RadioButton, TextBlock, TextBox,
    TextBoxImpl, TextBoxImplExt,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};

static GROUP_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Default)]
pub struct MaterialTimeChangedEventArgs {
    base: RoutedEventArgs,
    old_time: TimeSpan,
    new_time: TimeSpan,
}
ferro_routed_event_args!(MaterialTimeChangedEventArgs: RoutedEventArgs);
impl MaterialTimeChangedEventArgs {
    pub fn old_time(&self) -> TimeSpan {
        self.old_time
    }
    pub fn new_time(&self) -> TimeSpan {
        self.new_time
    }
}

#[derive(Clone, Default)]
struct Parts {
    layout: Option<Ref<MaterialTimePickerLayout>>,
    dial: Option<Ref<MaterialTimePickerDial>>,
    hour: Option<Ref<RadioButton>>,
    minute: Option<Ref<RadioButton>>,
    am: Option<Ref<RadioButton>>,
    pm: Option<Ref<RadioButton>>,
    hour_field: Option<Ref<MaterialTimePickerField>>,
    minute_field: Option<Ref<MaterialTimePickerField>>,
    hour_label: Option<Ref<TextBlock>>,
    minute_label: Option<Ref<TextBlock>>,
    separator: Option<Ref<TextBlock>>,
}

/// A persistent, minute-precision time editor with an analog dial and numeric input.
/// Hour is always 0–23, even when the display uses AM/PM. Invalid intermediate text
/// is retained separately; SelectedTime and TimeChanged expose only valid values.
#[repr(C)]
pub struct MaterialTimePicker {
    base: TemplatedControl,
    parts: RefCell<Parts>,
    subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
    synchronizing: Cell<bool>,
    editing: Cell<bool>,
    selected_time: Cell<TimeSpan>,
    hour_valid: Cell<bool>,
    minute_valid: Cell<bool>,
    input_valid: Cell<bool>,
    hour_draft: RefCell<String>,
    minute_draft: RefCell<String>,
    group_id: u64,
}
ferro_class!(MaterialTimePicker: TemplatedControl);
ferro_class_info!(MaterialTimePicker {
    new: MaterialTimePicker::new
});
ferro_impl_classes!(MaterialTimePicker: StyledElementImpl, InteractiveImpl, InputElementImpl, LayoutableImpl);
impl ControlImpl for MaterialTimePicker {
    fn on_create_automation_peer(this: &Self) -> Ref<AutomationPeer> {
        MaterialTimePickerAutomationPeer::new(this).upcast()
    }
}
impl FerroObjectImpl for MaterialTimePicker {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.synchronize(true);
    }
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if this.synchronizing.get() {
            return;
        }
        let p = change.property();
        let value_changed =
            p == Self::hour_property().as_property() || p == Self::minute_property().as_property();
        let reset_draft = value_changed || p == Self::is_24_hour_property().as_property();
        if value_changed
            || p == Self::is_24_hour_property().as_property()
            || p == Self::selection_property().as_property()
            || p == Self::display_mode_property().as_property()
            || p == Self::orientation_property().as_property()
            || p == Self::is_vibrant_property().as_property()
            || p == Self::is_read_only_property().as_property()
            || p == Self::auto_advance_property().as_property()
            || p == Self::reduced_motion_property().as_property()
            || p == Self::motion_scheme_property().as_property()
            || p == Self::hour_label_property().as_property()
            || p == Self::minute_label_property().as_property()
            || p == Self::am_label_property().as_property()
            || p == Self::pm_label_property().as_property()
            || p == Self::hour_error_text_property().as_property()
            || p == Self::minute_error_text_property().as_property()
            || p == Visual::flow_direction_property().as_property()
            || p == InputElement::is_effectively_enabled_property().as_property()
        {
            this.synchronize(reset_draft && !this.editing.get());
        }
        if value_changed {
            this.publish_time();
        }
        if (p == Self::display_mode_property().as_property()
            || p == Self::selection_property().as_property())
            && this.display_mode() == MaterialTimeDisplayMode::Input
            && this.is_attached_to_visual_tree()
            && this.can_edit()
        {
            this.focus_field(this.selection());
        }
    }
}
impl VisualImpl for MaterialTimePicker {
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);
        this.assign_radio_groups();
        this.synchronize(false);
    }
    fn on_detached_from_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        if let Some(dial) = this.parts.borrow().dial.clone() {
            dial.cancel_interaction();
        }
        Self::parent_on_detached_from_visual_tree(this, e);
    }
}
impl TemplatedControlImpl for MaterialTimePicker {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        this.release_parts();
        Self::parent_on_apply_template(this, e);
        let scope = e.name_scope();
        let parts = Parts {
            layout: scope.find_as("PART_Layout"),
            dial: scope.find_as("PART_Dial"),
            hour: scope.find_as("PART_Hour"),
            minute: scope.find_as("PART_Minute"),
            am: scope.find_as("PART_Am"),
            pm: scope.find_as("PART_Pm"),
            hour_field: scope.find_as("PART_HourField"),
            minute_field: scope.find_as("PART_MinuteField"),
            hour_label: scope.find_as("PART_HourLabel"),
            minute_label: scope.find_as("PART_MinuteLabel"),
            separator: scope.find_as("PART_Separator"),
        };
        let weak = this.to_ref().downgrade();
        if let Some(layout) = &parts.layout {
            *layout.owner.borrow_mut() = Some(weak.clone());
        }
        if let Some(dial) = &parts.dial {
            dial.set_owner(Some(weak.clone()));
        }
        for (button, role) in [
            (&parts.hour, 0),
            (&parts.minute, 1),
            (&parts.am, 2),
            (&parts.pm, 3),
        ] {
            if let Some(button) = button {
                // Runtime identities are installed only after attachment. This keeps
                // independently loaded, unattached markup trees deterministic.
                if this.is_attached_to_visual_tree() {
                    button.set_group_name(Some(format!(
                        "MaterialTimePicker{}-{}",
                        this.group_id,
                        role / 2
                    )));
                }
                let owner = weak.clone();
                let weak_button = button.downgrade();
                let subscription = button.property_changed(move |e| {
                    if e.property() != ToggleButton::is_checked_property().as_property() {
                        return;
                    }
                    if let (Some(owner), Some(button)) = (owner.upgrade(), weak_button.upgrade()) {
                        if !owner.synchronizing.get()
                            && button.is_checked() == Some(true)
                            && owner.can_edit()
                        {
                            match role {
                                0 => {
                                    owner.set_selection(MaterialTimeSelection::Hour);
                                    owner.focus_selection();
                                }
                                1 => {
                                    owner.set_selection(MaterialTimeSelection::Minute);
                                    owner.focus_selection();
                                }
                                2 => owner.set_period(false),
                                _ => owner.set_period(true),
                            }
                        }
                    }
                });
                this.subscriptions.borrow_mut().push(subscription);
                if role < 2 {
                    let owner = weak.clone();
                    let subscription = button.add_disposable_handler_with(
                        ferroui_controls::Button::click_event(),
                        move |_, _| {
                            if let Some(owner) = owner.upgrade().filter(|o| o.can_edit()) {
                                owner.focus_selection();
                            }
                        },
                        RoutingStrategies::BUBBLE,
                        false,
                    );
                    this.subscriptions.borrow_mut().push(subscription);
                }
            }
        }
        for (field, selection) in [
            (&parts.hour_field, MaterialTimeSelection::Hour),
            (&parts.minute_field, MaterialTimeSelection::Minute),
        ] {
            if let Some(field) = field {
                *field.owner.borrow_mut() = Some(weak.clone());
                field.selection.set(selection);
                TextInputOptions::set_content_type(field, TextInputContentType::Digits);
                TextInputOptions::set_return_key_type(
                    field,
                    if selection == MaterialTimeSelection::Hour {
                        TextInputReturnKeyType::Next
                    } else {
                        TextInputReturnKeyType::Done
                    },
                );
            }
        }
        for label in [&parts.hour_label, &parts.minute_label]
            .into_iter()
            .flatten()
        {
            AutomationProperties::set_live_setting(label, AutomationLiveSetting::Polite);
        }
        *this.parts.borrow_mut() = parts;
        this.synchronize(false);
        if this.display_mode() == MaterialTimeDisplayMode::Input
            && this.is_attached_to_visual_tree()
            && this.can_edit()
        {
            this.focus_field(this.selection());
        }
    }
}

ferro_properties! { impl MaterialTimePicker {
    pub fn hour_property() -> StyledProperty<i32> { FerroProperty::register_with::<Self,_>("Hour", StyledPropertyOptions::new(0).validate(|v: &i32| (0..24).contains(v)).default_binding_mode(BindingMode::TwoWay)) }
    pub fn minute_property() -> StyledProperty<i32> { FerroProperty::register_with::<Self,_>("Minute", StyledPropertyOptions::new(0).validate(|v: &i32| (0..60).contains(v)).default_binding_mode(BindingMode::TwoWay)) }
    pub fn is_24_hour_property() -> StyledProperty<bool> { FerroProperty::register::<Self,_>("Is24Hour", false) }
    pub fn selection_property() -> StyledProperty<MaterialTimeSelection> { FerroProperty::register::<Self,_>("Selection", MaterialTimeSelection::Hour) }
    pub fn display_mode_property() -> StyledProperty<MaterialTimeDisplayMode> { FerroProperty::register::<Self,_>("DisplayMode", MaterialTimeDisplayMode::Dial) }
    pub fn orientation_property() -> StyledProperty<Orientation> { FerroProperty::register::<Self,_>("Orientation", Orientation::Vertical) }
    pub fn is_vibrant_property() -> StyledProperty<bool> { FerroProperty::register::<Self,_>("IsVibrant", false) }
    pub fn is_read_only_property() -> StyledProperty<bool> { FerroProperty::register::<Self,_>("IsReadOnly", false) }
    pub fn auto_advance_property() -> StyledProperty<bool> { FerroProperty::register::<Self,_>("AutoAdvance", true) }
    pub fn reduced_motion_property() -> StyledProperty<bool> { FerroProperty::register::<Self,_>("ReducedMotion", false) }
    pub fn motion_scheme_property() -> StyledProperty<MaterialMotionScheme> { FerroProperty::register::<Self,_>("MotionScheme", MaterialMotionScheme::Expressive) }
    pub fn hour_label_property() -> StyledProperty<String> { FerroProperty::register::<Self,_>("HourLabel", "Hour".to_owned()) }
    pub fn minute_label_property() -> StyledProperty<String> { FerroProperty::register::<Self,_>("MinuteLabel", "Minute".to_owned()) }
    pub fn am_label_property() -> StyledProperty<String> { FerroProperty::register::<Self,_>("AmLabel", "AM".to_owned()) }
    pub fn pm_label_property() -> StyledProperty<String> { FerroProperty::register::<Self,_>("PmLabel", "PM".to_owned()) }
    pub fn hour_error_text_property() -> StyledProperty<String> { FerroProperty::register::<Self,_>("HourErrorText", "Enter a valid hour".to_owned()) }
    pub fn minute_error_text_property() -> StyledProperty<String> { FerroProperty::register::<Self,_>("MinuteErrorText", "Enter a minute from 0 to 59".to_owned()) }
    pub fn selected_time_property() -> DirectProperty<MaterialTimePicker, TimeSpan> { FerroProperty::register_direct::<Self,_>("SelectedTime", |o| o.selected_time(), Some(|o,v| o.set_selected_time(v)), TimeSpan::ZERO) }
    pub fn is_hour_input_valid_property() -> DirectProperty<MaterialTimePicker,bool> { FerroProperty::register_direct::<Self,_>("IsHourInputValid", |o| o.is_hour_input_valid(), None, true) }
    pub fn is_minute_input_valid_property() -> DirectProperty<MaterialTimePicker,bool> { FerroProperty::register_direct::<Self,_>("IsMinuteInputValid", |o| o.is_minute_input_valid(), None, true) }
    pub fn is_input_valid_property() -> DirectProperty<MaterialTimePicker,bool> { FerroProperty::register_direct::<Self,_>("IsInputValid", |o| o.is_input_valid(), None, true) }
}}

macro_rules! accessors {
    ($get:ident, $set:ident, $property:ident, $ty:ty) => {
        pub fn $get(&self) -> $ty {
            self.get_value(Self::$property())
        }
        pub fn $set(&self, value: $ty) {
            self.set_value(Self::$property(), value);
        }
    };
}
impl MaterialTimePicker {
    ferro_routed_event!(
        pub fn time_changed_event() -> RoutedEvent<MaterialTimeChangedEventArgs> {
            RoutedEvent::register::<Self, _>("TimeChanged", RoutingStrategies::BUBBLE)
        }
    );
    ferro_routed_event!(
        pub fn time_change_finished_event() -> RoutedEvent<RoutedEventArgs> {
            RoutedEvent::register::<Self, _>("TimeChangeFinished", RoutingStrategies::BUBBLE)
        }
    );
    pub fn construct() -> Self {
        Self {
            base: TemplatedControl::construct(),
            parts: RefCell::new(Parts::default()),
            subscriptions: RefCell::new(Vec::new()),
            synchronizing: Cell::new(false),
            editing: Cell::new(false),
            selected_time: Cell::new(TimeSpan::ZERO),
            hour_valid: Cell::new(true),
            minute_valid: Cell::new(true),
            input_valid: Cell::new(true),
            hour_draft: RefCell::new("12".into()),
            minute_draft: RefCell::new("00".into()),
            group_id: GROUP_ID.fetch_add(1, Ordering::Relaxed),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    accessors!(hour, set_hour, hour_property, i32);
    accessors!(minute, set_minute, minute_property, i32);
    accessors!(is_24_hour, set_is_24_hour, is_24_hour_property, bool);
    accessors!(
        selection,
        set_selection,
        selection_property,
        MaterialTimeSelection
    );
    accessors!(
        display_mode,
        set_display_mode,
        display_mode_property,
        MaterialTimeDisplayMode
    );
    accessors!(
        orientation,
        set_orientation,
        orientation_property,
        Orientation
    );
    accessors!(is_vibrant, set_is_vibrant, is_vibrant_property, bool);
    accessors!(is_read_only, set_is_read_only, is_read_only_property, bool);
    /// Whether pointer and numeric input can advance automatically to minutes.
    /// Hosts with touch exploration enabled should set this to false. Clock keyboard
    /// activation and accessibility selection never auto-advance.
    pub fn auto_advance(&self) -> bool {
        self.get_value(Self::auto_advance_property())
    }
    pub fn set_auto_advance(&self, value: bool) {
        self.set_value(Self::auto_advance_property(), value);
    }
    accessors!(
        reduced_motion,
        set_reduced_motion,
        reduced_motion_property,
        bool
    );
    accessors!(
        motion_scheme,
        set_motion_scheme,
        motion_scheme_property,
        MaterialMotionScheme
    );
    accessors!(hour_label, set_hour_label, hour_label_property, String);
    accessors!(
        minute_label,
        set_minute_label,
        minute_label_property,
        String
    );
    accessors!(am_label, set_am_label, am_label_property, String);
    accessors!(pm_label, set_pm_label, pm_label_property, String);
    accessors!(
        hour_error_text,
        set_hour_error_text,
        hour_error_text_property,
        String
    );
    accessors!(
        minute_error_text,
        set_minute_error_text,
        minute_error_text_property,
        String
    );
    pub fn selected_time(&self) -> TimeSpan {
        self.selected_time.get()
    }
    pub fn is_hour_input_valid(&self) -> bool {
        self.hour_valid.get()
    }
    pub fn is_minute_input_valid(&self) -> bool {
        self.minute_valid.get()
    }
    pub fn is_input_valid(&self) -> bool {
        self.input_valid.get()
    }
    pub fn hour_input(&self) -> String {
        self.hour_draft.borrow().clone()
    }
    pub fn minute_input(&self) -> String {
        self.minute_draft.borrow().clone()
    }
    /// Sets a valid 24-hour time atomically, emitting at most one TimeChanged event.
    pub fn set_time(&self, hour: i32, minute: i32) {
        assert!(
            (0..24).contains(&hour) && (0..60).contains(&minute),
            "time must be within 00:00..23:59"
        );
        self.synchronizing.set(true);
        self.set_current_value(Self::hour_property(), hour);
        self.set_current_value(Self::minute_property(), minute);
        self.synchronizing.set(false);
        self.synchronize(!self.editing.get());
        self.publish_time();
    }
    pub fn set_selected_time(&self, value: TimeSpan) {
        let minutes = value.total_minutes();
        assert!(
            minutes.is_finite() && (0.0..1440.0).contains(&minutes),
            "time must be within one day"
        );
        let minutes = minutes.floor() as i32;
        self.set_time(minutes / 60, minutes % 60);
    }
    pub(crate) fn can_edit(&self) -> bool {
        self.is_effectively_enabled() && !self.is_read_only()
    }
    pub(crate) fn set_period(&self, pm: bool) {
        if self.can_edit() && self.is_hour_input_valid() && (self.hour() >= 12) != pm {
            self.set_time(self.hour() % 12 + if pm { 12 } else { 0 }, self.minute());
            self.finish_edit();
        }
    }
    pub(crate) fn select_number(&self, value: i32) {
        if !self.can_edit() {
            return;
        }
        if self.selection() == MaterialTimeSelection::Minute {
            self.set_time(self.hour(), value);
        } else {
            self.set_time(
                if self.is_24_hour() {
                    value
                } else {
                    value % 12 + if self.hour() >= 12 { 12 } else { 0 }
                },
                self.minute(),
            );
        }
    }
    pub(crate) fn finish_edit(&self) {
        if self.is_input_valid() {
            self.raise_event(&RoutedEventArgs::with_event(
                Self::time_change_finished_event(),
            ));
        }
    }
    fn publish_time(&self) {
        let time = TimeSpan::from_minutes((self.hour() * 60 + self.minute()) as f64);
        let old = self.selected_time();
        if old == time {
            return;
        }
        self.set_and_raise_cell(Self::selected_time_property(), &self.selected_time, time);
        let e = MaterialTimeChangedEventArgs {
            base: RoutedEventArgs::with_event(Self::time_changed_event()),
            old_time: old,
            new_time: time,
        };
        self.raise_event(&e);
    }
    fn release_parts(&self) {
        for subscription in self.subscriptions.borrow_mut().drain(..) {
            subscription.dispose();
        }
        let old = self.parts.replace(Parts::default());
        if let Some(layout) = old.layout {
            layout.owner.borrow_mut().take();
        }
        if let Some(dial) = old.dial {
            dial.cancel_interaction();
            dial.set_owner(None);
        }
        for field in [old.hour_field, old.minute_field].into_iter().flatten() {
            field.owner.borrow_mut().take();
        }
    }
    fn assign_radio_groups(&self) {
        let parts = self.parts.borrow().clone();
        let was_syncing = self.synchronizing.replace(true);
        for (button, group) in [
            (&parts.hour, 0),
            (&parts.minute, 0),
            (&parts.am, 1),
            (&parts.pm, 1),
        ] {
            if let Some(button) = button {
                button.set_group_name(Some(format!("MaterialTimePicker{}-{group}", self.group_id)));
            }
        }
        self.synchronizing.set(was_syncing);
    }
    fn synchronize(&self, reset_drafts: bool) {
        if self.synchronizing.replace(true) {
            return;
        }
        if reset_drafts {
            *self.hour_draft.borrow_mut() =
                format!("{:02}", display_hour(self.hour(), self.is_24_hour()));
            *self.minute_draft.borrow_mut() = format!("{:02}", self.minute());
            self.set_validity(true, true);
        }
        let parts = self.parts.borrow().clone();
        let input = self.display_mode() == MaterialTimeDisplayMode::Input;
        self.pseudo_classes().set(":input", input);
        self.pseudo_classes().set(":vibrant", self.is_vibrant());
        self.pseudo_classes()
            .set(":invalid", !self.is_input_valid());
        for (button, selected, text, label) in [
            (
                &parts.hour,
                self.selection() == MaterialTimeSelection::Hour,
                if self.is_hour_input_valid() {
                    format!("{:02}", display_hour(self.hour(), self.is_24_hour()))
                } else {
                    self.hour_input()
                },
                self.hour_label(),
            ),
            (
                &parts.minute,
                self.selection() == MaterialTimeSelection::Minute,
                if self.is_minute_input_valid() {
                    format!("{:02}", self.minute())
                } else {
                    self.minute_input()
                },
                self.minute_label(),
            ),
            (
                &parts.am,
                self.hour() < 12,
                self.am_label(),
                self.am_label(),
            ),
            (
                &parts.pm,
                self.hour() >= 12,
                self.pm_label(),
                self.pm_label(),
            ),
        ] {
            if let Some(button) = button {
                button.set_content(Some(Rc::new(text)));
                button.set_is_checked(Some(selected));
                button.set_is_enabled(self.can_edit());
                button.pseudo_classes().set(":vibrant", self.is_vibrant());
                button.pseudo_classes().set(":input", input);
                button.pseudo_classes().set(
                    ":horizontal",
                    !input && self.orientation() == Orientation::Horizontal,
                );
                AutomationProperties::set_name(button, Some(&label));
            }
        }
        for (button, selection, valid) in [
            (
                &parts.hour,
                MaterialTimeSelection::Hour,
                self.is_hour_input_valid(),
            ),
            (
                &parts.minute,
                MaterialTimeSelection::Minute,
                self.is_minute_input_valid(),
            ),
        ] {
            if let Some(button) = button {
                button.set_is_visible(!input || self.selection() != selection);
                button.pseudo_classes().set(":invalid", !valid);
            }
        }
        for button in [&parts.am, &parts.pm].into_iter().flatten() {
            button.set_is_visible(!self.is_24_hour());
            button.set_is_enabled(self.can_edit() && self.is_hour_input_valid());
        }
        for (field, draft, valid, label, error) in [
            (
                &parts.hour_field,
                self.hour_input(),
                self.is_hour_input_valid(),
                self.hour_label(),
                self.hour_error_text(),
            ),
            (
                &parts.minute_field,
                self.minute_input(),
                self.is_minute_input_valid(),
                self.minute_label(),
                self.minute_error_text(),
            ),
        ] {
            if let Some(field) = field {
                field.set_is_visible(input && self.selection() == field.selection.get());
                field.set_is_read_only(self.is_read_only());
                field.pseudo_classes().set(":vibrant", self.is_vibrant());
                field.pseudo_classes().set(":invalid", !valid);
                if field.text().as_deref() != Some(&draft) {
                    field.set_text(Some(&draft));
                }
                AutomationProperties::set_name(field, Some(&label));
                AutomationProperties::set_help_text(
                    field,
                    Some(if valid { &label } else { &error }),
                );
            }
        }
        for (label, valid, text, error) in [
            (
                &parts.hour_label,
                self.is_hour_input_valid(),
                self.hour_label(),
                self.hour_error_text(),
            ),
            (
                &parts.minute_label,
                self.is_minute_input_valid(),
                self.minute_label(),
                self.minute_error_text(),
            ),
        ] {
            if let Some(label) = label {
                label.set_is_visible(input && (!self.is_vibrant() || !valid));
                label.set_text(Some(if valid { &text } else { &error }));
                label.pseudo_classes().set(":invalid", !valid);
            }
        }
        if let Some(dial) = &parts.dial {
            dial.set_is_visible(!input);
            dial.set_is_enabled(self.can_edit());
            dial.synchronize();
        }
        if let Some(layout) = &parts.layout {
            layout.invalidate_measure();
        }
        self.synchronizing.set(false);
    }
    fn set_validity(&self, hour: bool, minute: bool) {
        self.set_and_raise_cell(Self::is_hour_input_valid_property(), &self.hour_valid, hour);
        self.set_and_raise_cell(
            Self::is_minute_input_valid_property(),
            &self.minute_valid,
            minute,
        );
        self.set_and_raise_cell(
            Self::is_input_valid_property(),
            &self.input_valid,
            hour && minute,
        );
    }
    /// Edits an input field without discarding an invalid numeric draft. Returns false
    /// for rejected syntax; such a replacement leaves both text and value unchanged.
    pub fn set_input(&self, selection: MaterialTimeSelection, text: &str) -> bool {
        if !self.can_edit() {
            return false;
        }
        let Ok(value) = parse_input(text, selection, self.is_24_hour(), self.hour() >= 12) else {
            return false;
        };
        self.editing.set(true);
        match selection {
            MaterialTimeSelection::Hour => {
                *self.hour_draft.borrow_mut() = text.into();
                self.set_validity(value.is_some(), self.is_minute_input_valid());
            }
            MaterialTimeSelection::Minute => {
                *self.minute_draft.borrow_mut() = text.into();
                self.set_validity(self.is_hour_input_valid(), value.is_some());
            }
        }
        if let Some(value) = value {
            if selection == MaterialTimeSelection::Hour {
                self.set_time(value, self.minute());
            } else {
                self.set_time(self.hour(), value);
            }
        } else {
            self.synchronize(false);
        }
        self.editing.set(false);
        true
    }
    pub(crate) fn focus_field(&self, selection: MaterialTimeSelection) {
        self.set_selection(selection);
        let parts = self.parts.borrow().clone();
        let field = if selection == MaterialTimeSelection::Hour {
            parts.hour_field
        } else {
            parts.minute_field
        };
        if let Some(field) = field {
            field.focus();
            field.select_all();
        }
    }
    pub(crate) fn focus_dial(&self) {
        if let Some(dial) = self.parts.borrow().dial.clone() {
            dial.focus_selected();
        }
    }
    fn focus_selection(&self) {
        if self.display_mode() == MaterialTimeDisplayMode::Input {
            self.focus_field(self.selection());
        } else {
            self.focus_dial();
        }
    }
    pub(crate) fn exit_dial(&self, backwards: bool) {
        let parts = self.parts.borrow().clone();
        let target = if backwards {
            if self.selection() == MaterialTimeSelection::Hour {
                parts.hour.clone()
            } else {
                parts.minute.clone()
            }
        } else if self.selection() == MaterialTimeSelection::Hour {
            parts.minute.clone()
        } else if !self.is_24_hour() {
            parts.am
        } else {
            None
        };
        if let Some(target) = target {
            target.focus_with(NavigationMethod::Tab, KeyModifiers::empty());
        } else if let Some(minute) = parts.minute {
            if let Some(next) =
                KeyboardNavigationHandler::get_next(&minute.upcast(), NavigationDirection::Next)
            {
                next.focus_with(NavigationMethod::Tab, KeyModifiers::empty());
            }
        }
    }
}
impl Drop for MaterialTimePicker {
    fn drop(&mut self) {
        for subscription in self.subscriptions.get_mut().drain(..) {
            subscription.dispose();
        }
    }
}

/// TextBox subclass keeps the platform editor, IME, clipboard, selection, and value provider.
#[repr(C)]
pub struct MaterialTimePickerField {
    base: TextBox,
    owner: RefCell<Option<WeakRef<MaterialTimePicker>>>,
    selection: Cell<MaterialTimeSelection>,
}
ferro_class!(MaterialTimePickerField: TextBox);
ferro_class_info!(MaterialTimePickerField {
    new: MaterialTimePickerField::new
});
ferro_impl_classes!(MaterialTimePickerField: StyledElementImpl, VisualImpl, LayoutableImpl, InteractiveImpl, ControlImpl, TemplatedControlImpl);
impl FerroObjectImpl for MaterialTimePickerField {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        let weak = this.to_ref().downgrade();
        this.pasting_from_clipboard(move |_, event| {
            let Some(args) = event.downcast_ref::<PastingFromClipboardEventArgs>() else {
                return;
            };
            let Some(clipboard) = args.clipboard() else {
                return;
            };
            event.set_handled(true);
            let weak = weak.clone();
            // Paste uses the same input transformation as typing and IME commits. The
            // ordinary TextBox paste path bypasses on_text_input and would update its
            // presenter/caret after a rejected Text property replacement.
            drop(
                Dispatcher::ui_thread()
                    .to_task_scheduler()
                    .start_local(async move {
                        match clipboard.try_get_text_async().await {
                            Ok(Some(text)) => {
                                if let Some(field) =
                                    weak.upgrade().filter(|f| f.is_attached_to_visual_tree())
                                {
                                    let mut input = TextInputEventArgs::new();
                                    input.text = Some(text);
                                    input.set_routed_event(Some(InputElement::text_input_event()));
                                    field.raise_event(&input);
                                }
                            }
                            Ok(None) => {}
                            Err(error) => {
                                if !matches!(
                                    error.kind(),
                                    ClipboardErrorKind::Timeout
                                        | ClipboardErrorKind::Canceled
                                        | ClipboardErrorKind::AccessDenied
                                        | ClipboardErrorKind::Platform
                                ) {
                                    panic!("Unhandled time-input clipboard error: {error}");
                                }
                            }
                        }
                    }),
            );
        });
    }
    fn on_property_changed(this: &Self, e: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, e);
        if e.property() == TextBox::text_property().as_property() {
            if let Some(owner) = this.owner() {
                if !owner.synchronizing.get() {
                    if !owner.set_input(this.selection.get(), this.text().as_deref().unwrap_or(""))
                    {
                        owner.synchronize(false);
                    }
                }
            }
        }
    }
}
impl TextBoxImpl for MaterialTimePickerField {
    fn coerce_text(this: &Self, text: Option<String>) -> Option<String> {
        if let Some(owner) = this.owner() {
            if !owner.synchronizing.get()
                && parse_input(
                    text.as_deref().unwrap_or(""),
                    this.selection.get(),
                    owner.is_24_hour(),
                    owner.hour() >= 12,
                )
                .is_err()
            {
                return this.text();
            }
        }
        Self::parent_coerce_text(this, text)
    }
}
impl InputElementImpl for MaterialTimePickerField {
    fn on_got_focus(this: &Self, e: &FocusChangedEventArgs) {
        Self::parent_on_got_focus(this, e);
        if let Some(owner) = this.owner() {
            owner.set_selection(this.selection.get());
        }
    }
    fn on_text_input(this: &Self, e: &TextInputEventArgs) {
        let Some(owner) = this.owner() else {
            Self::parent_on_text_input(this, e);
            return;
        };
        if !owner.can_edit() {
            e.set_handled(true);
            return;
        }
        let text = e.text.as_deref().unwrap_or("");
        if !text.bytes().all(|c| c.is_ascii_digit()) {
            e.set_handled(true);
            return;
        }
        let current = this.text().unwrap_or_default();
        if text.len() == 1 && current.len() == 2 && this.selection_start() == this.selection_end() {
            this.select_all();
        } else {
            let retained = current.len().saturating_sub(
                (this.selection_start() - this.selection_end()).unsigned_abs() as usize,
            );
            if retained + text.len() > 2 {
                e.set_handled(true);
                return;
            }
        }
        Self::parent_on_text_input(this, e);
        let value = this.text().unwrap_or_default();
        if this.selection.get() == MaterialTimeSelection::Hour
            && owner.auto_advance()
            && owner.is_hour_input_valid()
            && (value.len() == 2
                || (!owner.is_24_hour()
                    && value.parse::<i32>().is_ok_and(|v| (2..=9).contains(&v))))
        {
            owner.focus_field(MaterialTimeSelection::Minute);
        }
    }
    fn on_key_down(this: &Self, e: &KeyEventArgs) {
        if e.key == Key::Enter {
            if let Some(owner) = this.owner() {
                if owner.is_input_valid() {
                    if this.selection.get() == MaterialTimeSelection::Hour {
                        owner.focus_field(MaterialTimeSelection::Minute);
                    } else {
                        owner.finish_edit();
                    }
                }
                e.set_handled(true);
                return;
            }
        }
        Self::parent_on_key_down(this, e);
    }
}
impl MaterialTimePickerField {
    pub fn construct() -> Self {
        Self {
            base: TextBox::construct(),
            owner: RefCell::new(None),
            selection: Cell::new(MaterialTimeSelection::Hour),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    fn owner(&self) -> Option<Ref<MaterialTimePicker>> {
        self.owner.borrow().as_ref().and_then(WeakRef::upgrade)
    }
}

/// Native measured layout, including compact landscape dial sizing and RTL outer layout.
#[repr(C)]
pub struct MaterialTimePickerLayout {
    base: Panel,
    owner: RefCell<Option<WeakRef<MaterialTimePicker>>>,
    metrics: Cell<TimePickerMetrics>,
}
ferro_class!(MaterialTimePickerLayout: Panel);
ferro_class_info!(MaterialTimePickerLayout {
    new: MaterialTimePickerLayout::new
});
ferro_impl_classes!(MaterialTimePickerLayout: FerroObjectImpl, StyledElementImpl, InputElementImpl, InteractiveImpl, ControlImpl, PanelImpl);
impl VisualImpl for MaterialTimePickerLayout {
    fn bypass_flow_direction_policies(_: &Self) -> bool {
        true
    }
}
impl MaterialTimePickerLayout {
    pub fn construct() -> Self {
        Self {
            base: Panel::construct(),
            owner: RefCell::new(None),
            metrics: Cell::new(TimePickerMetrics::new(
                false,
                false,
                MaterialTimeDisplayMode::Dial,
                Orientation::Vertical,
                f64::INFINITY,
            )),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    fn owner(&self) -> Option<Ref<MaterialTimePicker>> {
        self.owner.borrow().as_ref().and_then(WeakRef::upgrade)
    }
}
impl LayoutableImpl for MaterialTimePickerLayout {
    fn measure_override(this: &Self, available: Size) -> Size {
        let Some(owner) = this.owner() else {
            return Size::default();
        };
        let mut metrics = TimePickerMetrics::new(
            owner.is_vibrant(),
            owner.is_24_hour(),
            owner.display_mode(),
            owner.orientation(),
            available.height,
        );
        if metrics.input && !owner.is_input_valid() {
            metrics.support_height = 39.0;
        }
        this.metrics.set(metrics);
        let parts = owner.parts.borrow().clone();
        for field in [parts.hour_field, parts.minute_field].into_iter().flatten() {
            field.measure(Size::new(metrics.field_width, metrics.field_height));
        }
        for button in [parts.hour, parts.minute].into_iter().flatten() {
            button.measure(Size::new(metrics.field_width, metrics.field_height));
        }
        for label in [parts.hour_label, parts.minute_label].into_iter().flatten() {
            label.measure(Size::new(metrics.field_width, metrics.support_height));
        }
        if let Some(separator) = parts.separator {
            separator.measure(Size::new(metrics.separator, metrics.field_height));
        }
        for button in [parts.am, parts.pm].into_iter().flatten() {
            button.measure(if metrics.horizontal {
                Size::new(
                    (metrics.fields_width() - metrics.period_inner_gap) / 2.0,
                    if owner.is_vibrant() { 40.0 } else { 38.0 },
                )
            } else {
                Size::new(
                    metrics.period_width,
                    (metrics.field_height - metrics.period_inner_gap) / 2.0,
                )
            });
        }
        if let Some(dial) = parts.dial {
            dial.measure(Size::new(metrics.dial_size, metrics.dial_size));
        }
        metrics.desired_size(owner.is_24_hour(), owner.is_vibrant())
    }
    fn arrange_override(this: &Self, size: Size) -> Size {
        let Some(owner) = this.owner() else {
            return size;
        };
        let m = this.metrics.get();
        let parts = owner.parts.borrow().clone();
        let display = m.display_size(owner.is_24_hour(), owner.is_vibrant());
        let desired = m.desired_size(owner.is_24_hour(), owner.is_vibrant());
        let rtl = owner.flow_direction() == FlowDirection::RightToLeft;
        let ox = (size.width - desired.width).max(0.0) / 2.0;
        let oy = (size.height - desired.height).max(0.0) / 2.0;
        let dx = if m.horizontal {
            if rtl {
                ox + m.dial_size + m.dial_gap
            } else {
                ox
            }
        } else {
            ox + (desired.width - display.width) / 2.0
        };
        let dy = if m.horizontal {
            oy + (desired.height - display.height) / 2.0
        } else {
            oy
        };
        let fx = dx
            + if rtl && !m.horizontal && !owner.is_24_hour() {
                m.period_width + m.period_gap
            } else {
                0.0
            };
        let hour = Rect::new(fx, dy, m.field_width, m.field_height);
        let minute = Rect::new(
            fx + m.field_width + m.separator,
            dy,
            m.field_width,
            m.field_height,
        );
        if let Some(c) = parts.hour {
            c.arrange(hour);
        }
        if let Some(c) = parts.hour_field {
            c.arrange(hour);
        }
        if let Some(c) = parts.minute {
            c.arrange(minute);
        }
        if let Some(c) = parts.minute_field {
            c.arrange(minute);
        }
        if let Some(c) = parts.separator {
            c.arrange(Rect::new(
                fx + m.field_width,
                dy - 4.0,
                m.separator,
                m.field_height,
            ));
        }
        if let Some(c) = parts.hour_label {
            c.arrange(Rect::new(
                hour.x,
                dy + m.field_height + 7.0,
                m.field_width,
                32.0,
            ));
        }
        if let Some(c) = parts.minute_label {
            c.arrange(Rect::new(
                minute.x,
                dy + m.field_height + 7.0,
                m.field_width,
                32.0,
            ));
        }
        for (button, index) in [(parts.am, 0), (parts.pm, 1)] {
            if let Some(button) = button {
                let rect = if m.horizontal {
                    let w = (m.fields_width() - m.period_inner_gap) / 2.0;
                    // HorizontalPeriodToggle uses absolute Placeable.place:
                    // AM stays left of PM even when the outer groups mirror.
                    Rect::new(
                        dx + index as f64 * (w + m.period_inner_gap),
                        dy + m.field_height + m.period_gap,
                        w,
                        if owner.is_vibrant() { 40.0 } else { 38.0 },
                    )
                } else {
                    let h = (m.field_height - m.period_inner_gap) / 2.0;
                    Rect::new(
                        if rtl {
                            dx
                        } else {
                            dx + m.fields_width() + m.period_gap
                        },
                        dy + index as f64 * (h + m.period_inner_gap),
                        m.period_width,
                        h,
                    )
                };
                button.arrange(rect);
            }
        }
        if let Some(dial) = parts.dial {
            let (x, y) = if m.horizontal {
                (
                    if rtl {
                        ox
                    } else {
                        ox + display.width + m.dial_gap
                    },
                    oy + (desired.height - m.dial_size) / 2.0,
                )
            } else {
                (
                    ox + (desired.width - m.dial_size) / 2.0,
                    oy + display.height + m.dial_gap,
                )
            };
            dial.arrange(Rect::new(x, y, m.dial_size, m.dial_size));
        }
        size
    }
}

#[repr(C)]
pub struct MaterialTimePickerAutomationPeer {
    base: ControlAutomationPeer,
}
ferro_class!(MaterialTimePickerAutomationPeer: ControlAutomationPeer);
ferro_class_info!(MaterialTimePickerAutomationPeer { interfaces: [Rc<dyn IValueProvider> => MaterialTimePickerAutomationPeer::provider] });
ferro_impl_classes!(MaterialTimePickerAutomationPeer: FerroObjectImpl, ControlAutomationPeerImpl);
impl AutomationPeerImpl for MaterialTimePickerAutomationPeer {
    fn get_class_name_core(_: &Self) -> String {
        "MaterialTimePicker".into()
    }
    fn get_automation_control_type_core(_: &Self) -> AutomationControlType {
        AutomationControlType::Custom
    }
}
impl MaterialTimePickerAutomationPeer {
    pub fn new(owner: &MaterialTimePicker) -> Ref<Self> {
        instantiate(Self {
            base: ControlAutomationPeer::construct(owner),
        })
    }
    fn provider(peer: Ref<Self>) -> Rc<dyn IValueProvider> {
        Rc::new(TimeValueProvider(peer))
    }
    fn picker(&self) -> Ref<MaterialTimePicker> {
        ControlAutomationPeer::owner(self)
            .cast()
            .expect("time picker owner")
    }
}
struct TimeValueProvider(Ref<MaterialTimePickerAutomationPeer>);
impl IValueProvider for TimeValueProvider {
    fn peer(&self) -> Ref<AutomationPeer> {
        self.0.clone().upcast()
    }
    fn is_read_only(&self) -> bool {
        self.0.picker().is_read_only()
    }
    fn value(&self) -> Option<String> {
        let p = self.0.picker();
        Some(format!("{:02}:{:02}", p.hour(), p.minute()))
    }
    fn set_value(&self, value: Option<&str>) -> Result<(), ProviderError> {
        let owner = self.0.picker();
        if !owner.can_edit() {
            return Err(Box::new(ElementNotEnabledException::with_message(
                "Time picker is disabled or read-only.",
            )));
        }
        let time = value
            .and_then(|s| s.split_once(':'))
            .and_then(|(h, m)| Some((h.parse::<i32>().ok()?, m.parse::<i32>().ok()?)))
            .filter(|(h, m)| (0..24).contains(h) && (0..60).contains(m));
        let Some((hour, minute)) = time else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Expected a time between 00:00 and 23:59.",
            )
            .into());
        };
        owner.set_time(hour, minute);
        owner.finish_edit();
        Ok(())
    }
}
