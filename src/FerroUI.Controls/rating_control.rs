// Copyright (c) Microsoft Corporation. Licensed under the MIT License. See NOTICE.md.
// Adapted from the pinned controls/dev/RatingControl sources. Rendering, layout,
// input and automation use FerroUI's native services and replaceable control template.

use super::rating_item_info::{
    RatingItemFontInfo, RatingItemImageInfo, RatingItemInfo, RatingItemState,
};
use crate::automation::peers::{
    AutomationPeer, ControlAutomationPeer, RatingControlAutomationPeer,
};
use crate::automation::{
    AccessibilityView, AutomationProperties,
    RangeValuePatternIdentifiers, ValuePatternIdentifiers,
};
use crate::primitives::{
    TemplateAppliedEventArgs, TemplatedControl, TemplatedControlImpl, TemplatedControlImplExt,
};
use crate::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions};
use crate::{Control, ControlImpl, Grid, Image, StackPanel, TextBlock};
use ferroui_base::controls::ResourceKey;
use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::input::{
    FocusChangedEventArgs, IPointer, InputElement, InputElementImpl, InputElementImplExt, Key,
    KeyDeviceType, KeyEventArgs, MouseButton, NavigationMethod, PointerCaptureLostEventArgs,
    PointerEventArgs, PointerPressedEventArgs, PointerReleasedEventArgs, PointerType,
};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{HorizontalAlignment, LayoutableImpl, Orientation, VerticalAlignment};
use ferroui_base::media::{
    Brushes, FlowDirection, FontFamily, IBrush, RectangleGeometry, ScaleTransform,
};
use ferroui_base::metadata::{into_markup_value, MarkupDelegate};
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate, BoxedValue,
    FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Rect, Ref,
    StyledElementImpl, StyledProperty, StyledPropertyOptions, VisualImpl,
};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

#[derive(Clone)]
struct ItemVisual {
    visual: Ref<Control>,
    scale: Ref<ScaleTransform>,
    clip: Option<Ref<RectangleGeometry>>,
}

#[derive(Clone, Default)]
struct Parts {
    background: Option<Ref<StackPanel>>,
    foreground: Option<Ref<StackPanel>>,
    caption: Option<Ref<TextBlock>>,
    items: Vec<(ItemVisual, ItemVisual)>,
}

/// A native rating editor with WinUI's unset sentinel, fractional placeholder,
/// clear-on-repeat selection, and user-only ValueChanged event semantics.
///
/// The default template uses independently instantiated font/image items and
/// per-item clipping. It supports a native replacement template exposing
/// RatingBackgroundStackPanel, RatingForegroundStackPanel and Caption parts.
#[repr(C)]
pub struct RatingControl {
    base: TemplatedControl,
    parts: RefCell<Parts>,
    item_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
    pointer: RefCell<Option<Weak<dyn IPointer>>>,
    preview: Cell<Option<f64>>,
    pointer_x: Cell<f64>,
    pointer_scalar: Cell<f64>,
    rendering_size: Cell<f64>,
    spacing: Cell<f64>,
    stable_value: Cell<f64>,
    stable_maximum: Cell<i32>,
    engagement_value: Cell<Option<f64>>,
    rebuilding: Cell<bool>,
    value_changed: HandlerList<dyn Fn(&RatingControl)>,
}

ferro_class!(RatingControl: TemplatedControl);
ferro_class_info!(RatingControl {
    new: RatingControl::new,
    markup: {
        namespace: "FerroUI.Controls",
        events: [
            ValueChanged(Option<BoxedValue>, Option<BoxedValue>) => |this: &Ref<RatingControl>, handler: MarkupDelegate| {
                this.value_changed(move |sender| {
                    handler.invoke(&[into_markup_value(Some(sender.to_ref())), into_markup_value(None::<BoxedValue>)]);
                })
            },
        ],
    },
});
ferro_impl_classes!(RatingControl: StyledElementImpl, VisualImpl, LayoutableImpl, InteractiveImpl);

impl FerroObjectImpl for RatingControl {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        // The original default style supplies this instance. Keep it per-control
        // so editing a custom glyph description cannot mutate another control.
        let info = RatingItemFontInfo::new();
        info.set_glyph("\u{e735}");
        info.set_unset_glyph("\u{e734}");
        this.set_current_value(Self::item_info_property(), Some(info.upcast()));
        let weak = this.to_ref().downgrade();
        let resources = this.resources_changed(move |_| {
            if let Some(owner) = weak.upgrade() {
                owner.rebuild_items();
            }
        });
        let weak = this.to_ref().downgrade();
        let theme = this.actual_theme_variant_changed(move || {
            if let Some(owner) = weak.upgrade() {
                owner.update_appearance();
            }
        });
        this.subscriptions.borrow_mut().extend([resources, theme]);
    }

    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if !change.is_effective_value_change() {
            return;
        }
        let property = change.property();
        if property == Self::max_rating_property().as_property() {
            let maximum = this.max_rating().max(1);
            if maximum != this.max_rating() {
                this.set_current_value(Self::max_rating_property(), maximum);
                return;
            }
            if this.value() > maximum as f64 {
                this.set_current_value(Self::value_property(), maximum as f64);
            }
            if this.placeholder_value() > maximum as f64 {
                this.set_current_value(Self::placeholder_value_property(), maximum as f64);
            }
            let old_maximum = this.stable_maximum.replace(maximum);
            this.rebuild_items();
            if let Some(peer) = ControlAutomationPeer::from_element(this)
                .and_then(|p| p.cast::<RatingControlAutomationPeer>())
            {
                if old_maximum != maximum {
                    peer.raise_property_changed_event(
                        RangeValuePatternIdentifiers::maximum_property(),
                        Some(Rc::new(old_maximum as f64) as BoxedValue),
                        Some(Rc::new(maximum as f64) as BoxedValue),
                    );
                }
                peer.description_changed();
            }
        } else if property == Self::value_property().as_property()
            || property == Self::placeholder_value_property().as_property()
        {
            let placeholder = property == Self::placeholder_value_property().as_property();
            let value = if placeholder {
                this.placeholder_value()
            } else {
                this.value()
            };
            let coerced = if value < 0.0 {
                -1.0
            } else if !placeholder && value <= 1.0 {
                1.0
            } else {
                value.min(this.max_rating().max(1) as f64)
            };
            if value != coerced {
                if placeholder {
                    this.set_current_value(Self::placeholder_value_property(), coerced);
                } else {
                    this.set_current_value(Self::value_property(), coerced);
                }
                return;
            }
            let old = if placeholder {
                this.stable_value.get()
            } else {
                this.stable_value.replace(value)
            };
            this.update_appearance();
            if let Some(peer) = ControlAutomationPeer::from_element(this)
                .and_then(|p| p.cast::<RatingControlAutomationPeer>())
            {
                peer.rating_changed(old, this.value(), placeholder);
            }
        } else if property == Self::item_info_property().as_property() {
            let old = this.item_subscription.borrow_mut().take();
            if let Some(old) = old {
                old.dispose();
            }
            if let Some(info) = this.item_info() {
                let weak = this.to_ref().downgrade();
                let subscription = info.property_changed(move |_| {
                    if let Some(owner) = weak.upgrade() {
                        owner.update_appearance();
                    }
                });
                *this.item_subscription.borrow_mut() = Some(subscription);
            }
            this.rebuild_items();
        } else if property == Self::is_read_only_property().as_property()
            || property == InputElement::is_effectively_enabled_property().as_property()
        {
            if !this.can_interact() {
                this.cancel_pointer();
                this.end_focus_engagement(false);
            }
            this.update_appearance();
            if property == Self::is_read_only_property().as_property() {
                if let Some(peer) = ControlAutomationPeer::from_element(this) {
                    let old = change.get_old_value::<bool>().unwrap_or(false);
                    for identifier in [
                        RangeValuePatternIdentifiers::is_read_only_property(),
                        ValuePatternIdentifiers::is_read_only_property(),
                    ] {
                        peer.raise_property_changed_event(
                            identifier,
                            Some(Rc::new(old) as BoxedValue),
                            Some(Rc::new(this.is_read_only()) as BoxedValue),
                        );
                    }
                }
            }
        } else if [
            Self::caption_property().as_property(),
            TemplatedControl::font_family_property().as_property(),
            TemplatedControl::foreground_property().as_property(),
        ]
        .contains(&property)
        {
            this.update_appearance();
        }
    }
}

impl ControlImpl for RatingControl {
    fn on_create_automation_peer(this: &Self) -> Ref<AutomationPeer> {
        RatingControlAutomationPeer::new(this).upcast()
    }
}

impl TemplatedControlImpl for RatingControl {
    fn on_apply_template(this: &Self, args: &TemplateAppliedEventArgs) {
        this.cancel_pointer();
        Self::parent_on_apply_template(this, args);
        *this.parts.borrow_mut() = Parts {
            background: args
                .name_scope()
                .find_as::<StackPanel>("RatingBackgroundStackPanel"),
            foreground: args
                .name_scope()
                .find_as::<StackPanel>("RatingForegroundStackPanel"),
            caption: args.name_scope().find_as::<TextBlock>("Caption"),
            items: Vec::new(),
        };
        this.rebuild_items();
    }
}

impl InputElementImpl for RatingControl {
    fn on_key_down(this: &Self, args: &KeyEventArgs) {
        Self::parent_on_key_down(this, args);
        if args.handled() || !this.can_interact() {
            return;
        }
        if args.key_device_type == KeyDeviceType::Gamepad {
            match args.key {
                Key::Enter => {
                    if this.is_focus_engaged() {
                        this.end_focus_engagement(true);
                    } else {
                        this.begin_focus_engagement();
                    }
                    args.set_handled(true);
                    return;
                }
                Key::Escape if this.is_focus_engaged() => {
                    this.end_focus_engagement(false);
                    args.set_handled(true);
                    return;
                }
                Key::Left | Key::Right if this.is_focus_engaged() => {}
                _ => return,
            }
        }
        let direction = if this.flow_direction() == FlowDirection::RightToLeft {
            -1.0
        } else {
            1.0
        };
        match args.key {
            Key::Left => this.change_rating_by(-direction),
            Key::Right => this.change_rating_by(direction),
            Key::Up => this.change_rating_by(1.0),
            Key::Down => this.change_rating_by(-1.0),
            Key::Home => this.select_rating(0.0, false),
            Key::End => this.select_rating(this.max_rating() as f64, false),
            _ => return,
        }
        args.set_handled(true);
    }

    fn on_lost_focus(this: &Self, args: &FocusChangedEventArgs) {
        Self::parent_on_lost_focus(this, args);
        this.end_focus_engagement(false);
    }

    fn on_pointer_entered(this: &Self, args: &PointerEventArgs) {
        Self::parent_on_pointer_entered(this, args);
        this.update_pointer_preview(args);
    }
    fn on_pointer_moved(this: &Self, args: &PointerEventArgs) {
        Self::parent_on_pointer_moved(this, args);
        if this.update_pointer_preview(args) {
            args.set_handled(true);
        }
    }
    fn on_pointer_exited(this: &Self, args: &PointerEventArgs) {
        Self::parent_on_pointer_exited(this, args);
        if this.active_pointer().is_none() {
            this.preview.set(None);
            this.update_appearance();
        }
    }
    fn on_pointer_pressed(this: &Self, args: &PointerPressedEventArgs) {
        Self::parent_on_pointer_pressed(this, args);
        if args.handled()
            || !this.can_interact()
            || this.active_pointer().is_some()
            || !args
                .get_current_point(Some(this))
                .properties
                .is_left_button_pressed
        {
            return;
        }
        if !this.update_pointer_preview(args) {
            return;
        }
        let pointer = args.pointer().clone();
        *this.pointer.borrow_mut() = Some(Rc::downgrade(&pointer));
        pointer.capture(Some(&this.to_ref().upcast::<InputElement>()));
        if pointer.captured() != Some(this.to_ref().upcast::<InputElement>())
            || !this.can_interact()
        {
            this.cancel_pointer();
            this.update_appearance();
            return;
        }
        this.focus_with(NavigationMethod::Pointer, args.key_modifiers());
        this.update_appearance();
        args.set_handled(true);
    }
    fn on_pointer_released(this: &Self, args: &PointerReleasedEventArgs) {
        Self::parent_on_pointer_released(this, args);
        let owns_pointer = this
            .pointer
            .borrow()
            .as_ref()
            .and_then(Weak::upgrade)
            .is_some_and(|p| p.id() == args.pointer().id());
        if !owns_pointer || args.initial_press_mouse_button() != MouseButton::Left {
            return;
        }
        if this.can_interact() {
            if let Some(value) = this.rating_at(args, true) {
                this.select_rating(value, true);
            }
        }
        let pointer = this.pointer.borrow_mut().take().and_then(|p| p.upgrade());
        if let Some(pointer) = pointer {
            pointer.capture(None);
        }
        this.update_pointer_preview(args);
        this.update_appearance();
        args.set_handled(true);
    }
    fn on_pointer_capture_lost(this: &Self, args: &PointerCaptureLostEventArgs) {
        Self::parent_on_pointer_capture_lost(this, args);
        if !this
            .active_pointer()
            .is_some_and(|pointer| pointer.id() == args.pointer().id())
        {
            return;
        }
        this.pointer.borrow_mut().take();
        this.preview.set(None);
        this.update_appearance();
    }
}

ferro_properties! { impl RatingControl {
    pub fn caption_property() -> StyledProperty<String> { FerroProperty::register::<RatingControl, _>("Caption", String::new()) }
    pub fn initial_set_value_property() -> StyledProperty<i32> { FerroProperty::register::<RatingControl, _>("InitialSetValue", 1) }
    pub fn is_clear_enabled_property() -> StyledProperty<bool> { FerroProperty::register::<RatingControl, _>("IsClearEnabled", true) }
    pub fn is_read_only_property() -> StyledProperty<bool> { FerroProperty::register::<RatingControl, _>("IsReadOnly", false) }
    pub fn max_rating_property() -> StyledProperty<i32> { FerroProperty::register::<RatingControl, _>("MaxRating", 5) }
    pub fn placeholder_value_property() -> StyledProperty<f64> { FerroProperty::register_with::<RatingControl, _>("PlaceholderValue", StyledPropertyOptions::new(-1.0_f64).validate(|v| !v.is_nan())) }
    pub fn item_info_property() -> StyledProperty<Option<Ref<RatingItemInfo>>> { FerroProperty::register::<RatingControl, _>("ItemInfo", None) }
    pub fn value_property() -> StyledProperty<f64> { FerroProperty::register_with::<RatingControl, _>("Value", StyledPropertyOptions::new(-1.0_f64).validate(|v| !v.is_nan())) }
} }

impl RatingControl {
    fn static_constructor() {
        InputElement::focusable_property().override_default_value::<RatingControl>(true);
        TemplatedControl::font_family_property()
            .override_default_value::<RatingControl>(FontFamily::new("Segoe Fluent Icons"));
        let template = FuncControlTemplate::for_type::<RatingControl>(|_, scope| {
            let background = StackPanel::new();
            background.set_name(Some("RatingBackgroundStackPanel".to_owned()));
            background.set_orientation(Orientation::Horizontal);
            background.set_background(Some(Brushes::transparent()));
            let foreground = StackPanel::new();
            foreground.set_name(Some("RatingForegroundStackPanel".to_owned()));
            foreground.set_orientation(Orientation::Horizontal);
            foreground.set_is_hit_test_visible(false);
            let stars = Grid::new();
            stars.children().add(
                background
                    .register_in_name_scope(&**scope)
                    .upcast::<Control>(),
            );
            stars.children().add(
                foreground
                    .register_in_name_scope(&**scope)
                    .upcast::<Control>(),
            );
            let caption = TextBlock::new();
            caption.set_name(Some("Caption".to_owned()));
            caption.set_vertical_alignment(VerticalAlignment::Center);
            // Caption text remains ordinary UI text rather than an icon-font glyph.
            caption.set_font_family(FontFamily::default());
            let root = StackPanel::new();
            root.set_orientation(Orientation::Horizontal);
            root.set_spacing(12.0);
            root.children().add(stars.upcast::<Control>());
            root.children()
                .add(caption.register_in_name_scope(&**scope).upcast::<Control>());
            root.upcast()
        });
        TemplatedControl::template_property()
            .override_default_value::<RatingControl>(Some(template));
    }

    pub fn construct() -> Self {
        Self {
            base: TemplatedControl::construct(),
            parts: RefCell::new(Parts::default()),
            item_subscription: RefCell::new(None),
            subscriptions: RefCell::new(Vec::new()),
            pointer: RefCell::new(None),
            preview: Cell::new(None),
            pointer_x: Cell::new(0.0),
            pointer_scalar: Cell::new(0.8),
            rendering_size: Cell::new(32.0),
            spacing: Cell::new(8.0),
            stable_value: Cell::new(-1.0),
            stable_maximum: Cell::new(5),
            engagement_value: Cell::new(None),
            rebuilding: Cell::new(false),
            value_changed: HandlerList::new(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn caption(&self) -> String {
        self.get_value(Self::caption_property())
    }
    pub fn set_caption(&self, value: impl Into<String>) {
        self.base.set_value(Self::caption_property(), value.into());
    }
    pub fn initial_set_value(&self) -> i32 {
        self.get_value(Self::initial_set_value_property())
    }
    pub fn set_initial_set_value(&self, value: i32) {
        self.base
            .set_value(Self::initial_set_value_property(), value);
    }
    pub fn is_clear_enabled(&self) -> bool {
        self.get_value(Self::is_clear_enabled_property())
    }
    pub fn set_is_clear_enabled(&self, value: bool) {
        self.base
            .set_value(Self::is_clear_enabled_property(), value);
    }
    pub fn is_read_only(&self) -> bool {
        self.get_value(Self::is_read_only_property())
    }
    pub fn set_is_read_only(&self, value: bool) {
        self.base.set_value(Self::is_read_only_property(), value);
    }
    pub fn max_rating(&self) -> i32 {
        self.get_value(Self::max_rating_property())
    }
    pub fn set_max_rating(&self, value: i32) {
        self.base.set_value(Self::max_rating_property(), value);
    }
    pub fn placeholder_value(&self) -> f64 {
        self.get_value(Self::placeholder_value_property())
    }
    pub fn set_placeholder_value(&self, value: f64) {
        self.base
            .set_value(Self::placeholder_value_property(), value);
    }
    pub fn item_info(&self) -> Option<Ref<RatingItemInfo>> {
        self.get_value(Self::item_info_property())
    }
    pub fn set_item_info(&self, value: Option<Ref<RatingItemInfo>>) {
        self.base.set_value(Self::item_info_property(), value);
    }
    pub fn value(&self) -> f64 {
        self.get_value(Self::value_property())
    }
    pub fn set_rating_value(&self, value: f64) {
        self.base.set_value(Self::value_property(), value);
    }

    /// Subscribes to user edits. Assigning Value or changing MaxRating never
    /// raises this event, matching the WinUI control's public event contract.
    pub fn value_changed(&self, handler: impl Fn(&RatingControl) + 'static) -> Rc<dyn IDisposable> {
        let token = self.value_changed.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(owner) = weak.upgrade() {
                owner.value_changed.remove(token);
            }
        })
    }

    /// Applies a user selection. Pointer selection of the current rating clears
    /// it when enabled; repeated keyboard increments at MaxRating remain set.
    pub fn select_rating(&self, value: f64, from_pointer: bool) {
        if !self.can_interact() || value.is_nan() {
            return;
        }
        let old = self.value();
        let value = value.clamp(0.0, self.max_rating().max(1) as f64);
        if old <= -1.0 && value == 0.0 {
            return;
        }
        let next = if !self.is_clear_enabled() && value <= 0.0 {
            1.0
        } else if value == old
            && self.is_clear_enabled()
            && (value != self.max_rating() as f64 || from_pointer)
        {
            -1.0
        } else if value > 0.0 {
            value
        } else {
            -1.0
        };
        self.set_current_value(Self::value_property(), next);
        for (_, handler) in self.value_changed.snapshot().iter() {
            handler(self);
        }
    }

    /// Moves to the next integral rating, using InitialSetValue when unset.
    pub fn change_rating_by(&self, delta: f64) {
        if delta == 0.0 || !delta.is_finite() {
            return;
        }
        let value = self.value();
        let next = if value == -1.0 {
            self.initial_set_value() as f64
        } else if value.fract() != 0.0 && delta == -1.0 {
            value.floor()
        } else {
            value.floor() + delta
        };
        self.select_rating(next, false);
    }

    /// Whether normalized gamepad input currently edits the value rather than moving focus.
    pub fn is_focus_engaged(&self) -> bool {
        self.engagement_value.get().is_some()
    }

    /// Starts gamepad editing, preserving the value for cancel/focus-loss rollback.
    pub fn begin_focus_engagement(&self) {
        if !self.can_interact() || self.is_focus_engaged() {
            return;
        }
        let previous = self.value();
        self.engagement_value.set(Some(previous));
        if previous == -1.0 {
            self.set_current_value(Self::value_property(), self.initial_set_value() as f64);
            for (_, handler) in self.value_changed.snapshot().iter() {
                handler(self);
            }
        }
        self.pseudo_classes()
            .set(":focus-engaged", self.is_focus_engaged());
    }

    /// Ends gamepad editing. Cancel and focus loss restore the pre-engagement value.
    pub fn end_focus_engagement(&self, commit: bool) {
        let Some(previous) = self.engagement_value.take() else {
            return;
        };
        self.pseudo_classes().set(":focus-engaged", false);
        if !commit && self.value() != previous {
            self.set_current_value(Self::value_property(), previous);
            for (_, handler) in self.value_changed.snapshot().iter() {
                handler(self);
            }
        }
    }

    fn active_pointer(&self) -> Option<Rc<dyn IPointer>> {
        self.pointer.borrow().as_ref().and_then(Weak::upgrade)
    }

    fn can_interact(&self) -> bool {
        self.is_effectively_enabled() && !self.is_read_only()
    }

    fn cancel_pointer(&self) {
        let pointer = self.pointer.borrow_mut().take().and_then(|p| p.upgrade());
        if let Some(pointer) = pointer {
            pointer.capture(None);
        }
        self.preview.set(None);
    }

    fn rating_at(&self, args: &PointerEventArgs, captured: bool) -> Option<f64> {
        let background = self.parts.borrow().background.clone()?;
        let point = args.get_position(Some(&background));
        let width = background.bounds().width;
        if width <= 0.0 || !width.is_finite() {
            return None;
        }
        if !captured
            && (point.x < 0.0
                || point.x > width
                || point.y < 0.0
                || point.y > background.bounds().height)
        {
            return None;
        }
        // Coordinates are relative to the native panel. Its inherited RTL
        // transform already mirrors them; reversing again would invert twice.
        self.pointer_x.set(point.x);
        Some(
            (point.x / width * self.max_rating() as f64)
                .ceil()
                .clamp(0.0, self.max_rating() as f64),
        )
    }

    fn update_pointer_preview(&self, args: &PointerEventArgs) -> bool {
        if !self.can_interact() {
            return false;
        }
        let active = self.active_pointer();
        if active
            .as_ref()
            .is_some_and(|p| p.id() != args.pointer().id())
        {
            return false;
        }
        let captured = active.is_some();
        let value = self.rating_at(args, captured);
        self.preview.set(value);
        self.pointer_scalar
            .set(if args.pointer().type_() == PointerType::Touch {
                1.0
            } else {
                0.8
            });
        self.update_appearance();
        value.is_some()
    }

    fn resource_number(&self, key: &str, fallback: f64, allow_zero: bool) -> f64 {
        self.try_find_resource(
            &ResourceKey::from(key),
            self.actual_theme_variant().as_ref(),
        )
        .flatten()
        .and_then(|v| v.downcast_ref::<f64>().copied())
        .filter(|v| v.is_finite() && (*v > 0.0 || allow_zero && *v == 0.0))
        .unwrap_or(fallback)
    }

    fn brush(&self, key: &str, fallback: &str) -> Option<Rc<dyn IBrush>> {
        for key in [key, fallback] {
            if let Some(value) = self
                .try_find_resource(
                    &ResourceKey::from(key),
                    self.actual_theme_variant().as_ref(),
                )
                .flatten()
            {
                if let Some(value) = ValueTypes::try_cast(&value, ValueType::of::<Rc<dyn IBrush>>())
                {
                    if let Some(brush) = value.downcast_ref::<Rc<dyn IBrush>>() {
                        return Some(brush.clone());
                    }
                }
            }
        }
        self.foreground()
    }

    fn rebuild_items(&self) {
        let parts = self.parts.borrow().clone();
        let (Some(background), Some(foreground)) = (parts.background, parts.foreground) else {
            return;
        };
        if self.rebuilding.replace(true) {
            return;
        }
        struct Reset<'a>(&'a Cell<bool>);
        impl Drop for Reset<'_> {
            fn drop(&mut self) {
                self.0.set(false);
            }
        }
        let _reset = Reset(&self.rebuilding);
        let size = self.resource_number("RatingControlFontSizeForRendering", 32.0, false);
        let spacing = self.resource_number("RatingControlItemSpacing", 8.0, true);
        self.rendering_size.set(size);
        self.spacing.set(spacing);
        background.children().clear();
        foreground.children().clear();
        background.set_spacing(spacing);
        foreground.set_spacing(spacing);
        let info = self.item_info();
        let font = info.as_ref().and_then(|i| i.cast::<RatingItemFontInfo>());
        let image = info.as_ref().and_then(|i| i.cast::<RatingItemImageInfo>());
        let mut items = Vec::new();
        if font.is_some() || image.is_some() {
            for _ in 0..self.max_rating().max(1) {
                let build = |is_foreground: bool, parent: &StackPanel| {
                    let visual: Ref<Control> = if font.is_some() {
                        let text = TextBlock::new();
                        text.set_font_size(size);
                        text.upcast()
                    } else {
                        Image::new().upcast()
                    };
                    visual.set_width(size);
                    visual.set_height(size);
                    visual.set_horizontal_alignment(HorizontalAlignment::Center);
                    visual.set_vertical_alignment(VerticalAlignment::Center);
                    visual.set_is_hit_test_visible(false);
                    AutomationProperties::set_accessibility_view(&visual, AccessibilityView::Raw);
                    let scale = ScaleTransform::with_scale(0.5, 0.5);
                    visual.set_render_transform(Some(scale.clone().into()));
                    let clip = is_foreground
                        .then(|| RectangleGeometry::with_rect(Rect::new(0.0, 0.0, size, size)));
                    if let Some(clip) = &clip {
                        visual.set_clip(clip.clone());
                    }
                    let container = Grid::new();
                    container.set_width(size / 2.0);
                    container.set_height(size);
                    container.children().add(visual.clone());
                    parent.children().add(container.upcast::<Control>());
                    ItemVisual {
                        visual,
                        scale,
                        clip,
                    }
                };
                items.push((build(false, &background), build(true, &foreground)));
            }
        }
        self.parts.borrow_mut().items = items;
        self.update_appearance();
    }

    fn update_appearance(&self) {
        let parts = self.parts.borrow().clone();
        let value = self.value();
        let placeholder = self.placeholder_value();
        let preview = self.preview.get();
        let display = preview
            .unwrap_or(if value >= 0.0 { value } else { placeholder })
            .max(0.0);
        let state = if !self.is_effectively_enabled() {
            RatingItemState::Disabled
        } else if preview.is_some() && value >= 0.0 {
            RatingItemState::PointerOver
        } else if preview.is_some() {
            RatingItemState::PointerOverPlaceholder
        } else if value >= 0.0 {
            RatingItemState::Set
        } else {
            RatingItemState::Placeholder
        };
        self.pseudo_classes().set(":set", value >= 0.0);
        self.pseudo_classes()
            .set(":placeholder", value < 0.0 && placeholder >= 0.0);
        self.pseudo_classes().set(":readonly", self.is_read_only());
        self.pseudo_classes()
            .set(":pressed", self.active_pointer().is_some());
        if let Some(caption) = parts.caption {
            caption.set_text(Some(&self.caption()));
            caption.set_is_visible(!self.caption().is_empty());
            caption.set_foreground(self.brush(
                "RatingControlCaptionForeground",
                "TextFillColorSecondaryBrush",
            ));
        }
        let selected_key = match state {
            RatingItemState::Disabled => (
                "RatingControlDisabledSelectedForeground",
                "TextFillColorDisabledBrush",
            ),
            RatingItemState::PointerOver => (
                "RatingControlPointerOverSelectedForeground",
                "AccentFillColorDefaultBrush",
            ),
            RatingItemState::PointerOverPlaceholder => (
                "RatingControlPointerOverPlaceholderForeground",
                "ControlAltFillColorTertiaryBrush",
            ),
            RatingItemState::Placeholder => (
                "RatingControlPlaceholderForeground",
                "TextFillColorPrimaryBrush",
            ),
            _ => (
                "RatingControlSelectedForeground",
                "AccentFillColorDefaultBrush",
            ),
        };
        let background_brush = self.brush(
            "RatingControlUnselectedForeground",
            "TextFillColorSecondaryBrush",
        );
        let foreground_brush = self.brush(selected_key.0, selected_key.1);
        let info = self.item_info();
        let font = info.as_ref().and_then(|i| i.cast::<RatingItemFontInfo>());
        let image = info.as_ref().and_then(|i| i.cast::<RatingItemImageInfo>());
        let size = self.rendering_size.get();
        for (index, (background, foreground)) in parts.items.iter().enumerate() {
            for (item, item_state, brush) in [
                (background, RatingItemState::Unset, &background_brush),
                (foreground, state, &foreground_brush),
            ] {
                if let (Some(info), Some(text)) = (&font, item.visual.cast::<TextBlock>()) {
                    text.set_text(Some(&info.glyph_for(item_state)));
                    text.set_font_family(self.font_family());
                    text.set_foreground(brush.clone());
                } else if let (Some(info), Some(image)) = (&image, item.visual.cast::<Image>()) {
                    image.set_source(info.image_for(item_state));
                }
                let scale = if preview.is_some() {
                    let center =
                        size / 2.0 * (index as f64 + 0.5) + index as f64 * self.spacing.get();
                    let distance = center - self.pointer_x.get();
                    (self.pointer_scalar.get() * (1.0 - 0.0005 * distance * distance)).max(0.5)
                } else {
                    0.5
                };
                item.scale.set_scale_x(scale);
                item.scale.set_scale_y(scale);
            }
            if let Some(clip) = &foreground.clip {
                clip.set_rect(Rect::new(
                    0.0,
                    0.0,
                    size * (display - index as f64).clamp(0.0, 1.0),
                    size,
                ));
            }
        }
    }
}

impl Drop for RatingControl {
    fn drop(&mut self) {
        if let Some(subscription) = self.item_subscription.get_mut().take() {
            subscription.dispose();
        }
        for subscription in self.subscriptions.get_mut().drain(..) {
            subscription.dispose();
        }
    }
}

