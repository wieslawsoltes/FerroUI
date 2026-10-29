//! WinUI's `RatingControl` (`controls/dev/RatingControl/RatingControl.cpp` of microsoft-ui-xaml,
//! commit `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT, Copyright (c) Microsoft Corporation, see
//! NOTICE.md).

use crate::automation::peers::{AutomationPeer, ControlAutomationPeer, RatingControlAutomationPeer};
use crate::automation::{AccessibilityView, AutomationProperties, RangeValuePatternIdentifiers, ValuePatternIdentifiers};
use crate::documents::TextElement;
use crate::primitives::{TemplateAppliedEventArgs, TemplatedControl, TemplatedControlImpl, TemplatedControlImplExt};
use crate::rating_item_info::{RatingItemFontInfo, RatingItemImageInfo, RatingItemInfo, RatingItemState};
use crate::{Control, ControlImpl, Grid, Image, StackPanel, TextBlock};
use ferroui_base::input::{
    IPointer, InputElement, InputElementImpl, InputElementImplExt, Key, KeyEventArgs,
    MouseButton, NavigationMethod, PointerCaptureLostEventArgs, PointerEventArgs, PointerPressedEventArgs,
    PointerReleasedEventArgs, PointerType,
};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{HorizontalAlignment, LayoutableImpl, VerticalAlignment};
use ferroui_base::logging::{LogArea, LogEventLevel, Logger};
use ferroui_base::media::{FlowDirection, RectangleGeometry, ScaleTransform};
use ferroui_base::metadata::{into_markup_value, MarkupDelegate};
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_property, instantiate, BoxedValue, FerroObjectImpl, FerroObjectImplExt,
    FerroProperty, FerroPropertyChangedEventArgs, Rect, Ref, StyledElementImpl, StyledProperty, StyledPropertyOptions,
    VisualImpl,
};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// The value of a rating that is not set (`c_noValueSetSentinel`).
const NO_VALUE_SET_SENTINEL: f64 = -1.0;

/// The scale of the items under the mouse and under a touch (`c_mouseOverScale`,
/// `c_touchOverScale`).
const MOUSE_OVER_SCALE: f64 = 0.8;
const TOUCH_OVER_SCALE: f64 = 1.0;

#[derive(Clone)]
struct ItemVisual {
    visual: Ref<Control>,
    scale: Ref<ScaleTransform>,
    clip: Option<Ref<RectangleGeometry>>,
}

#[derive(Clone, Default)]
struct Parts {
    background_stack_panel: Option<Ref<StackPanel>>,
    foreground_stack_panel: Option<Ref<StackPanel>>,
    items: Vec<(ItemVisual, ItemVisual)>,
}

/// A control to rate with stars (WinUI's `RatingControl`): a value from 1 to `MaxRating`, or -1
/// while it is not set, a placeholder value (a community rating) shown while it is not set, and a
/// caption.
///
/// The items are drawn twice, in the background stack panel in the unset state and in the
/// foreground stack panel clipped to the value, from the glyphs of a [`RatingItemFontInfo`] (the
/// stars E735 and E734 of the symbol font by default, in the font of the control) or the images of
/// a [`RatingItemImageInfo`]. A click or a tap sets the rating under the pointer, and clears it
/// when it is the rating (with `IsClearEnabled`); the arrow keys, Home and End change it. Only the
/// changes of the user raise `ValueChanged`.
///
/// The template is the control theme of the WinUI theme (`Controls/RatingControl.xaml`, from
/// WinUI's `RatingControl.xaml`), with WinUI's parts `RatingBackgroundStackPanel` and
/// `RatingForegroundStackPanel`; the items take their size from the `FontSize` of the stack
/// panels (`RatingControlFontSizeForRendering`) and their brushes from the theme. The control
/// sets WinUI's visual states (CommonStates) as the pseudo-classes `:placeholder`,
/// `:pointeroverplaceholder`, `:pointeroverunselected`, `:set` and `:pointeroverset` (Disabled is
/// `:disabled`). The Fluent and Simple themes have no template for it
/// (docs/porting/winui-controls.md, section 5.1).
///
/// Missing (docs/porting/winui-controls.md, Input): the composition animations of the scale of the
/// items (the scale is set when the pointer moves), and the focus engagement of the gamepad
/// (`IsFocusEngagementEnabled`, GamepadA and GamepadB).
#[repr(C)]
pub struct RatingControl {
    base: TemplatedControl,
    parts: RefCell<Parts>,
    item_info_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    template_subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
    pointer: RefCell<Option<Weak<dyn IPointer>>>,
    /// The rating under the pointer while it is over the items (`m_isPointerOver` with
    /// `m_mousePercentage`).
    preview: Cell<Option<f64>>,
    pointer_x: Cell<f64>,
    pointer_scalar: Cell<f64>,
    rendering_size: Cell<f64>,
    item_spacing: Cell<f64>,
    stable_value: Cell<f64>,
    stable_maximum: Cell<i32>,
    stamping_out: Cell<bool>,
    value_changed: HandlerList<dyn Fn(&RatingControl)>,
}

ferro_class!(RatingControl: TemplatedControl);
ferroui_base::ferro_class_info!(RatingControl {
    new: RatingControl::new,
    markup: {
        namespace: "FerroUI.Controls",
        events: [
            ValueChanged(Option<BoxedValue>, Option<BoxedValue>) => |this: &Ref<RatingControl>, handler: MarkupDelegate| {
                this.value_changed(move |sender: &RatingControl| {
                    handler.invoke(&[into_markup_value(Some(sender.to_ref())), into_markup_value(None::<BoxedValue>)]);
                })
            },
        ],
        attributes: [
            TemplatePart("RatingBackgroundStackPanel", type(Ref<StackPanel>)),
            TemplatePart("RatingForegroundStackPanel", type(Ref<StackPanel>)),
            PseudoClasses(":placeholder", ":pointeroverplaceholder", ":pointeroverunselected", ":set", ":pointeroverset"),
        ],
    },
});
ferro_impl_classes!(RatingControl: StyledElementImpl, VisualImpl, LayoutableImpl, InteractiveImpl);

/// A NaN rating is not set: WinUI's coercion keeps NaN, which no state draws.
fn coerce_rating(value: f64) -> f64 {
    if !value.is_nan() {
        return value;
    }
    if let Some(logger) = Logger::try_get(LogEventLevel::Warning, LogArea::CONTROL) {
        logger.log(None, "A rating of RatingControl is NaN: it is not set (-1)");
    }
    NO_VALUE_SET_SENTINEL
}

impl FerroObjectImpl for RatingControl {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        // The default style of WinUI sets the item info MUX_RatingControlDefaultFontInfo; the
        // control creates its own, so that a change of the glyphs of one control changes no other.
        let info = RatingItemFontInfo::new();
        info.set_glyph("\u{E735}");
        info.set_unset_glyph("\u{E734}");
        this.set_current_value(Self::item_info_property(), Some(info.upcast()));
    }

    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if !change.is_effective_value_change() {
            return;
        }
        let property = change.property();
        if property == Self::max_rating_property().as_property() {
            this.on_max_rating_changed();
        } else if property == Self::value_property().as_property() || property == Self::placeholder_value_property().as_property() {
            this.on_value_changed(property == Self::placeholder_value_property().as_property());
        } else if property == Self::item_info_property().as_property() {
            this.on_item_info_changed();
        } else if property == Self::is_read_only_property().as_property()
            || property == InputElement::is_effectively_enabled_property().as_property()
        {
            if !this.can_interact() {
                this.cancel_pointer();
            }
            this.update_rating_items_appearance();
            if property == Self::is_read_only_property().as_property() {
                if let Some(peer) = ControlAutomationPeer::from_element(this) {
                    let old = change.get_old_value::<bool>().unwrap_or(false);
                    for identifier in
                        [RangeValuePatternIdentifiers::is_read_only_property(), ValuePatternIdentifiers::is_read_only_property()]
                    {
                        peer.raise_property_changed_event(
                            identifier,
                            Some(Rc::new(old) as BoxedValue),
                            Some(Rc::new(this.is_read_only()) as BoxedValue),
                        );
                    }
                }
            }
        } else if property == TemplatedControl::font_family_property().as_property() {
            // OnFontFamilyChanged
            this.stamp_out_rating_items();
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
        for subscription in this.template_subscriptions.take() {
            subscription.dispose();
        }
        let scope = args.name_scope();
        let background = scope.find_as::<StackPanel>("RatingBackgroundStackPanel");
        *this.parts.borrow_mut() = Parts {
            background_stack_panel: background.clone(),
            foreground_stack_panel: scope.find_as::<StackPanel>("RatingForegroundStackPanel"),
            items: Vec::new(),
        };
        // The size and the spacing of the items are the font size and the spacing the template
        // gives the background stack panel (RatingControlFontSizeForRendering and
        // RatingControlItemSpacing).
        if let Some(background) = background {
            let weak = this.to_ref().downgrade();
            let subscription = background.property_changed(move |change| {
                let property = change.property();
                if property == TextElement::font_size_property().as_property()
                    || property == StackPanel::spacing_property().as_property()
                {
                    if let Some(this) = weak.upgrade() {
                        this.stamp_out_rating_items();
                    }
                }
            });
            this.template_subscriptions.borrow_mut().push(subscription);
        }
        this.stamp_out_rating_items();
    }
}

impl InputElementImpl for RatingControl {
    fn on_key_down(this: &Self, args: &KeyEventArgs) {
        Self::parent_on_key_down(this, args);
        if args.handled() || !this.can_interact() {
            return;
        }
        // OnKeyDown: Up and Down are Right and Left whatever the flow direction.
        let direction = if this.flow_direction() == FlowDirection::RightToLeft { -1.0 } else { 1.0 };
        match args.key {
            Key::Left => this.change_rating_by(-direction, false),
            Key::Right => this.change_rating_by(direction, false),
            Key::Up => this.change_rating_by(1.0, false),
            Key::Down => this.change_rating_by(-1.0, false),
            Key::Home => this.set_rating_to(0.0, false),
            Key::End => this.set_rating_to(this.max_rating() as f64, false),
            _ => return,
        }
        args.set_handled(true);
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
            this.update_rating_items_appearance();
        }
    }

    fn on_pointer_pressed(this: &Self, args: &PointerPressedEventArgs) {
        Self::parent_on_pointer_pressed(this, args);
        if args.handled()
            || !this.can_interact()
            || this.active_pointer().is_some()
            || !args.get_current_point(Some(this)).properties.is_left_button_pressed
        {
            return;
        }
        if !this.update_pointer_preview(args) {
            return;
        }
        let pointer = args.pointer().clone();
        *this.pointer.borrow_mut() = Some(Rc::downgrade(&pointer));
        pointer.capture(Some(&this.to_ref().upcast::<InputElement>()));
        if pointer.captured() != Some(this.to_ref().upcast::<InputElement>()) || !this.can_interact() {
            this.cancel_pointer();
            this.update_rating_items_appearance();
            return;
        }
        this.focus_with(NavigationMethod::Pointer, args.key_modifiers());
        this.update_rating_items_appearance();
        args.set_handled(true);
    }

    fn on_pointer_released(this: &Self, args: &PointerReleasedEventArgs) {
        Self::parent_on_pointer_released(this, args);
        let owns_pointer =
            this.pointer.borrow().as_ref().and_then(Weak::upgrade).is_some_and(|pointer| pointer.id() == args.pointer().id());
        if !owns_pointer || args.initial_press_mouse_button() != MouseButton::Left {
            return;
        }
        // OnPointerReleasedBackgroundStackPanel
        if this.can_interact() {
            if let Some(value) = this.rating_at(args, true) {
                this.set_rating_to(value, true);
            }
        }
        let pointer = this.pointer.borrow_mut().take().and_then(|pointer| pointer.upgrade());
        if let Some(pointer) = pointer {
            pointer.capture(None);
        }
        this.update_pointer_preview(args);
        this.update_rating_items_appearance();
        args.set_handled(true);
    }

    fn on_pointer_capture_lost(this: &Self, args: &PointerCaptureLostEventArgs) {
        Self::parent_on_pointer_capture_lost(this, args);
        if !this.active_pointer().is_some_and(|pointer| pointer.id() == args.pointer().id()) {
            return;
        }
        this.pointer.borrow_mut().take();
        this.preview.set(None);
        this.update_rating_items_appearance();
    }
}

ferroui_base::ferro_properties! { impl RatingControl {
    ferro_property!(
        /// Defines the `Caption` property: the text beside the items.
        pub fn caption_property() -> StyledProperty<String> {
            FerroProperty::register::<RatingControl, _>("Caption", String::new())
        }
    );

    ferro_property!(
        /// Defines the `InitialSetValue` property: the rating the keys start from while the rating
        /// is not set. 1 by default.
        pub fn initial_set_value_property() -> StyledProperty<i32> {
            FerroProperty::register::<RatingControl, _>("InitialSetValue", 1)
        }
    );

    ferro_property!(
        /// Defines the `IsClearEnabled` property: whether choosing the rating again clears it.
        /// `true` by default.
        pub fn is_clear_enabled_property() -> StyledProperty<bool> {
            FerroProperty::register::<RatingControl, _>("IsClearEnabled", true)
        }
    );

    ferro_property!(
        /// Defines the `IsReadOnly` property: whether the user cannot change the rating.
        pub fn is_read_only_property() -> StyledProperty<bool> {
            FerroProperty::register::<RatingControl, _>("IsReadOnly", false)
        }
    );

    ferro_property!(
        /// Defines the `MaxRating` property: the number of items. 5 by default; a value less than 1
        /// is 1.
        pub fn max_rating_property() -> StyledProperty<i32> {
            FerroProperty::register::<RatingControl, _>("MaxRating", 5)
        }
    );

    ferro_property!(
        /// Defines the `PlaceholderValue` property: the rating shown while the rating is not set
        /// (a community rating), -1 for none (the default). From 0 to `MaxRating`.
        pub fn placeholder_value_property() -> StyledProperty<f64> {
            FerroProperty::register_with::<RatingControl, _>(
                "PlaceholderValue",
                StyledPropertyOptions::new(NO_VALUE_SET_SENTINEL).coerce(|_, value: f64| coerce_rating(value)),
            )
        }
    );

    ferro_property!(
        /// Defines the `ItemInfo` property: the glyphs or the images of the items.
        pub fn item_info_property() -> StyledProperty<Option<Ref<RatingItemInfo>>> {
            FerroProperty::register::<RatingControl, _>("ItemInfo", None)
        }
    );

    ferro_property!(
        /// Defines the `Value` property: the rating, from 1 to `MaxRating`, or -1 while it is not
        /// set (the default).
        pub fn value_property() -> StyledProperty<f64> {
            FerroProperty::register_with::<RatingControl, _>(
                "Value",
                StyledPropertyOptions::new(NO_VALUE_SET_SENTINEL).coerce(|_, value: f64| coerce_rating(value)),
            )
        }
    );
} }

impl RatingControl {
    fn static_constructor() {
        InputElement::focusable_property().override_default_value::<RatingControl>(true);
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: TemplatedControl::construct(),
            parts: RefCell::new(Parts::default()),
            item_info_subscription: RefCell::new(None),
            template_subscriptions: RefCell::new(Vec::new()),
            pointer: RefCell::new(None),
            preview: Cell::new(None),
            pointer_x: Cell::new(0.0),
            pointer_scalar: Cell::new(MOUSE_OVER_SCALE),
            rendering_size: Cell::new(0.0),
            item_spacing: Cell::new(0.0),
            stable_value: Cell::new(NO_VALUE_SET_SENTINEL),
            stable_maximum: Cell::new(5),
            stamping_out: Cell::new(false),
            value_changed: HandlerList::new(),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets the text beside the items.
    pub fn caption(&self) -> String {
        self.get_value(Self::caption_property())
    }

    /// Sets the text beside the items.
    pub fn set_caption(&self, value: impl Into<String>) {
        self.set_value(Self::caption_property(), value.into());
    }

    /// Gets the rating the keys start from while the rating is not set.
    pub fn initial_set_value(&self) -> i32 {
        self.get_value(Self::initial_set_value_property())
    }

    /// Sets the rating the keys start from while the rating is not set.
    pub fn set_initial_set_value(&self, value: i32) {
        self.set_value(Self::initial_set_value_property(), value);
    }

    /// Gets whether choosing the rating again clears it.
    pub fn is_clear_enabled(&self) -> bool {
        self.get_value(Self::is_clear_enabled_property())
    }

    /// Sets whether choosing the rating again clears it.
    pub fn set_is_clear_enabled(&self, value: bool) {
        self.set_value(Self::is_clear_enabled_property(), value);
    }

    /// Gets whether the user cannot change the rating.
    pub fn is_read_only(&self) -> bool {
        self.get_value(Self::is_read_only_property())
    }

    /// Sets whether the user cannot change the rating.
    pub fn set_is_read_only(&self, value: bool) {
        self.set_value(Self::is_read_only_property(), value);
    }

    /// Gets the number of items.
    pub fn max_rating(&self) -> i32 {
        self.get_value(Self::max_rating_property())
    }

    /// Sets the number of items.
    pub fn set_max_rating(&self, value: i32) {
        self.set_value(Self::max_rating_property(), value);
    }

    /// Gets the rating shown while the rating is not set.
    pub fn placeholder_value(&self) -> f64 {
        self.get_value(Self::placeholder_value_property())
    }

    /// Sets the rating shown while the rating is not set.
    pub fn set_placeholder_value(&self, value: f64) {
        self.set_value(Self::placeholder_value_property(), value);
    }

    /// Gets the glyphs or the images of the items.
    pub fn item_info(&self) -> Option<Ref<RatingItemInfo>> {
        self.get_value(Self::item_info_property())
    }

    /// Sets the glyphs or the images of the items.
    pub fn set_item_info(&self, value: Option<Ref<RatingItemInfo>>) {
        self.set_value(Self::item_info_property(), value);
    }

    /// Gets the rating: -1 while it is not set.
    pub fn value(&self) -> f64 {
        self.get_value(Self::value_property())
    }

    /// Sets the rating. A change made here does not raise `ValueChanged`.
    pub fn set_rating_value(&self, value: f64) {
        self.set_value(Self::value_property(), value);
    }

    /// Adds a handler of the `ValueChanged` event, raised for the changes of the user only.
    /// Disposing the result removes the handler.
    pub fn value_changed(&self, handler: impl Fn(&RatingControl) + 'static) -> Rc<dyn IDisposable> {
        let token = self.value_changed.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.value_changed.remove(token);
            }
        })
    }

    /// Sets a rating chosen by the user (WinUI's `SetRatingTo`): choosing the rating again clears
    /// it with `IsClearEnabled` (by the pointer; the keys do not clear the largest rating), and a
    /// rating of 0 clears it, or is 1 without `IsClearEnabled`. Raises `ValueChanged`.
    pub(crate) fn set_rating_to(&self, value: f64, original_input_from_pointer: bool) {
        if !self.can_interact() || value.is_nan() {
            return;
        }
        let old_rating = self.value();
        let value = value.clamp(0.0, self.max_rating().max(1) as f64);
        if old_rating <= NO_VALUE_SET_SENTINEL && value == 0.0 {
            return;
        }
        let new_rating = if !self.is_clear_enabled() && value <= 0.0 {
            1.0
        } else if value == old_rating
            && self.is_clear_enabled()
            && (value != self.max_rating() as f64 || original_input_from_pointer)
        {
            NO_VALUE_SET_SENTINEL
        } else if value > 0.0 {
            value
        } else {
            NO_VALUE_SET_SENTINEL
        };
        self.set_current_value(Self::value_property(), new_rating);
        self.raise_value_changed();
    }

    /// Moves to the next whole rating (WinUI's `ChangeRatingBy`): from `InitialSetValue` while the
    /// rating is not set, and down from a fraction to its whole part.
    pub(crate) fn change_rating_by(&self, change: f64, original_input_from_pointer: bool) {
        if change == 0.0 || !change.is_finite() {
            return;
        }
        let value = self.value();
        let new_rating = if value == NO_VALUE_SET_SENTINEL {
            self.initial_set_value() as f64
        } else if value.fract() != 0.0 && change == -1.0 {
            value.floor()
        } else {
            value.floor() + change
        };
        self.set_rating_to(new_rating, original_input_from_pointer);
    }

    fn raise_value_changed(&self) {
        for (_, handler) in self.value_changed.snapshot().iter() {
            handler(self);
        }
    }

    fn on_max_rating_changed(&self) {
        let maximum = self.max_rating().max(1);
        if maximum != self.max_rating() {
            self.set_current_value(Self::max_rating_property(), maximum);
            return;
        }
        if self.value() > maximum as f64 {
            self.set_current_value(Self::value_property(), maximum as f64);
        }
        if self.placeholder_value() > maximum as f64 {
            self.set_current_value(Self::placeholder_value_property(), maximum as f64);
        }
        let old_maximum = self.stable_maximum.replace(maximum);
        self.stamp_out_rating_items();
        if let Some(peer) = ControlAutomationPeer::from_element(self).and_then(|peer| peer.cast::<RatingControlAutomationPeer>()) {
            if old_maximum != maximum {
                peer.raise_property_changed_event(
                    RangeValuePatternIdentifiers::maximum_property(),
                    Some(Rc::new(old_maximum as f64) as BoxedValue),
                    Some(Rc::new(maximum as f64) as BoxedValue),
                );
            }
            peer.description_changed();
        }
    }

    /// OnValueChanged and OnPlaceholderValueChanged, with WinUI's coercions
    /// (CoerceValueBetweenMinAndMax: a value is -1 or from 1 to `MaxRating`;
    /// CoercePlaceholderValueBetweenMinAndMax: a placeholder is -1 or from 0 to `MaxRating`).
    fn on_value_changed(&self, placeholder: bool) {
        let value = if placeholder { self.placeholder_value() } else { self.value() };
        let coerced = if value < 0.0 {
            NO_VALUE_SET_SENTINEL
        } else if !placeholder && value <= 1.0 {
            1.0
        } else {
            value.min(self.max_rating().max(1) as f64)
        };
        if value != coerced {
            if placeholder {
                self.set_current_value(Self::placeholder_value_property(), coerced);
            } else {
                self.set_current_value(Self::value_property(), coerced);
            }
            return;
        }
        let old = if placeholder { self.stable_value.get() } else { self.stable_value.replace(value) };
        self.update_rating_items_appearance();
        if let Some(peer) = ControlAutomationPeer::from_element(self).and_then(|peer| peer.cast::<RatingControlAutomationPeer>()) {
            peer.rating_changed(old, self.value(), placeholder);
        }
    }

    fn on_item_info_changed(&self) {
        let old = self.item_info_subscription.borrow_mut().take();
        if let Some(old) = old {
            old.dispose();
        }
        if let Some(info) = self.item_info() {
            let weak = self.to_ref().downgrade();
            let subscription = info.property_changed(move |_| {
                if let Some(this) = weak.upgrade() {
                    this.update_rating_items_appearance();
                }
            });
            *self.item_info_subscription.borrow_mut() = Some(subscription);
        }
        self.stamp_out_rating_items();
    }

    fn active_pointer(&self) -> Option<Rc<dyn IPointer>> {
        self.pointer.borrow().as_ref().and_then(Weak::upgrade)
    }

    fn can_interact(&self) -> bool {
        self.is_effectively_enabled() && !self.is_read_only()
    }

    fn cancel_pointer(&self) {
        let pointer = self.pointer.borrow_mut().take().and_then(|pointer| pointer.upgrade());
        if let Some(pointer) = pointer {
            pointer.capture(None);
        }
        self.preview.set(None);
    }

    /// The rating under the pointer: the item it is over, counted from the start of the background
    /// stack panel. While the pointer is captured it may be outside the panel.
    fn rating_at(&self, args: &PointerEventArgs, captured: bool) -> Option<f64> {
        let background = self.parts.borrow().background_stack_panel.clone()?;
        let point = args.get_position(Some(&background));
        let width = background.bounds().width;
        if width <= 0.0 || !width.is_finite() {
            return None;
        }
        if !captured && (point.x < 0.0 || point.x > width || point.y < 0.0 || point.y > background.bounds().height) {
            return None;
        }
        // The coordinates are those of the panel, which a right-to-left flow already mirrors.
        self.pointer_x.set(point.x);
        Some((point.x / width * self.max_rating() as f64).ceil().clamp(0.0, self.max_rating() as f64))
    }

    fn update_pointer_preview(&self, args: &PointerEventArgs) -> bool {
        if !self.can_interact() {
            return false;
        }
        let active = self.active_pointer();
        if active.as_ref().is_some_and(|pointer| pointer.id() != args.pointer().id()) {
            return false;
        }
        let value = self.rating_at(args, active.is_some());
        self.preview.set(value);
        self.pointer_scalar.set(if args.pointer().type_() == PointerType::Touch { TOUCH_OVER_SCALE } else { MOUSE_OVER_SCALE });
        self.update_rating_items_appearance();
        value.is_some()
    }

    /// Creates the items of the two stack panels (WinUI's `StampOutRatingItems` with
    /// `PopulateStackPanelWithItems`): each item, as large as the font size of the panels, is
    /// drawn at half its size in a cell of half its width.
    fn stamp_out_rating_items(&self) {
        let parts = self.parts.borrow().clone();
        let (Some(background), Some(foreground)) = (parts.background_stack_panel, parts.foreground_stack_panel) else {
            // OnApplyTemplate() hasn't executed yet.
            return;
        };
        if self.stamping_out.replace(true) {
            return;
        }
        struct Reset<'a>(&'a Cell<bool>);
        impl Drop for Reset<'_> {
            fn drop(&mut self) {
                self.0.set(false);
            }
        }
        let _reset = Reset(&self.stamping_out);
        let size = TextElement::get_font_size(&background);
        self.rendering_size.set(size);
        self.item_spacing.set(background.spacing());
        background.children().clear();
        foreground.children().clear();
        let info = self.item_info();
        let is_font = info.as_ref().is_some_and(|info| info.cast::<RatingItemFontInfo>().is_some());
        let is_image = info.as_ref().is_some_and(|info| info.cast::<RatingItemImageInfo>().is_some());
        let mut items = Vec::new();
        if is_font || is_image {
            for _ in 0..self.max_rating().max(1) {
                let build = |is_foreground: bool, panel: &StackPanel| {
                    let visual: Ref<Control> = if is_font { TextBlock::new().upcast() } else { Image::new().upcast() };
                    visual.set_width(size);
                    visual.set_height(size);
                    visual.set_horizontal_alignment(HorizontalAlignment::Center);
                    visual.set_vertical_alignment(VerticalAlignment::Center);
                    visual.set_is_hit_test_visible(false);
                    AutomationProperties::set_accessibility_view(&visual, AccessibilityView::Raw);
                    let scale = ScaleTransform::with_scale(0.5, 0.5);
                    visual.set_render_transform(Some(scale.clone().into()));
                    let clip = is_foreground.then(|| RectangleGeometry::with_rect(Rect::new(0.0, 0.0, size, size)));
                    if let Some(clip) = &clip {
                        visual.set_clip(clip.clone());
                    }
                    let cell = Grid::new();
                    cell.set_width(size / 2.0);
                    cell.set_height(size);
                    cell.children().add(visual.clone());
                    panel.children().add(cell.upcast::<Control>());
                    ItemVisual { visual, scale, clip }
                };
                items.push((build(false, &background), build(true, &foreground)));
            }
        }
        self.parts.borrow_mut().items = items;
        self.update_rating_items_appearance();
    }

    /// The state, the glyphs or images, the scale and the clip of the items (WinUI's
    /// `UpdateRatingItemsAppearance` with `CustomizeStackPanel`).
    fn update_rating_items_appearance(&self) {
        let parts = self.parts.borrow().clone();
        let value = self.value();
        let placeholder = self.placeholder_value();
        let preview = self.preview.get();
        let display = preview.unwrap_or(if value >= 0.0 { value } else { placeholder }).max(0.0);
        // The visual states of CommonStates; Disabled is :disabled.
        let state = if !self.is_effectively_enabled() {
            RatingItemState::Disabled
        } else if preview.is_some() && value > NO_VALUE_SET_SENTINEL {
            RatingItemState::PointerOver
        } else if preview.is_some() {
            RatingItemState::PointerOverPlaceholder
        } else if value > NO_VALUE_SET_SENTINEL {
            RatingItemState::Set
        } else {
            RatingItemState::Placeholder
        };
        let pointer_over = preview.is_some();
        let set = value > NO_VALUE_SET_SENTINEL;
        let pseudo_classes = self.pseudo_classes();
        pseudo_classes.set(":pointeroverset", pointer_over && set);
        pseudo_classes.set(":pointeroverplaceholder", pointer_over && !set && placeholder == NO_VALUE_SET_SENTINEL);
        pseudo_classes.set(":pointeroverunselected", pointer_over && !set && placeholder != NO_VALUE_SET_SENTINEL);
        pseudo_classes.set(":set", !pointer_over && set);
        pseudo_classes.set(":placeholder", !pointer_over && !set && placeholder > NO_VALUE_SET_SENTINEL);

        let info = self.item_info();
        let font = info.as_ref().and_then(|info| info.cast::<RatingItemFontInfo>());
        let image = info.as_ref().and_then(|info| info.cast::<RatingItemImageInfo>());
        let size = self.rendering_size.get();
        for (index, (background, foreground)) in parts.items.iter().enumerate() {
            for (item, item_state) in [(background, RatingItemState::Unset), (foreground, state)] {
                if let (Some(info), Some(text)) = (&font, item.visual.cast::<TextBlock>()) {
                    text.set_text(Some(&info.glyph_for(item_state)));
                } else if let (Some(info), Some(image)) = (&image, item.visual.cast::<Image>()) {
                    image.set_source(info.image_for(item_state));
                }
                // ApplyScaleExpressionAnimation: the items grow toward the pointer.
                let scale = if pointer_over {
                    let center = size / 2.0 * (index as f64 + 0.5) + index as f64 * self.item_spacing.get();
                    let distance = center - self.pointer_x.get();
                    (self.pointer_scalar.get() * (1.0 - 0.0005 * distance * distance)).max(0.5)
                } else {
                    0.5
                };
                item.scale.set_scale_x(scale);
                item.scale.set_scale_y(scale);
            }
            // The clips of the stars: whole, partial or empty.
            if let Some(clip) = &foreground.clip {
                clip.set_rect(Rect::new(0.0, 0.0, size * (display - index as f64).clamp(0.0, 1.0), size));
            }
        }
    }
}

impl Drop for RatingControl {
    fn drop(&mut self) {
        if let Some(subscription) = self.item_info_subscription.get_mut().take() {
            subscription.dispose();
        }
        for subscription in self.template_subscriptions.get_mut().drain(..) {
            subscription.dispose();
        }
    }
}
