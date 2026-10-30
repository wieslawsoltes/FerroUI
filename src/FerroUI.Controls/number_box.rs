//! WinUI's `NumberBox` (`controls/dev/NumberBox/NumberBox.cpp` of microsoft-ui-xaml, commit
//! `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT, Copyright (c) Microsoft Corporation, see
//! NOTICE.md).
//!
//! A separate control from the upstream `NumericUpDown` (docs/porting/winui-controls.md, section
//! 3): its value is an `f64` that is NaN when the box is empty, it raises a plain event, coerces
//! the value into its range unless validation is disabled, and hides its spin buttons by default.

use crate::automation::peers::{AutomationPeer, NumberBoxAutomationPeer};
use crate::automation::AutomationProperties;
use crate::presenters::ContentPresenter;
use crate::primitives::{Popup, TemplateAppliedEventArgs, TemplatedControl, TemplatedControlImpl, TemplatedControlImplExt};
use crate::templates::IDataTemplate;
use crate::utils::number_expression::{evaluate_number_expression, NumberExpressionErrorKind};
use crate::{Button, ControlImpl, RepeatButton, TextBox};
use ferroui_base::input::{
    FocusChangedEventArgs, InputElement, InputElementImpl, InputElementImplExt, Key, KeyEventArgs,
    PointerWheelEventArgs,
};
use ferroui_base::interactivity::{InteractiveImpl, RoutingStrategies};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::TextAlignment;
use ferroui_base::metadata::{into_markup_value, MarkupDelegate};
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_markup_enum, ferro_markup_type, ferro_property, instantiate, BoxedValue,
    FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Ref, StyledElementImpl,
    StyledProperty, VisualImpl,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Where the spin buttons of a [`NumberBox`] are (WinUI's `NumberBoxSpinButtonPlacementMode`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum NumberBoxSpinButtonPlacementMode {
    /// No spin buttons.
    #[default]
    Hidden = 0,
    /// The spin buttons are in a popup shown while the box has the focus.
    Compact = 1,
    /// The spin buttons are beside the text.
    Inline = 2,
}

/// What a [`NumberBox`] does with text that is not a number and with values out of its range
/// (WinUI's `NumberBoxValidationMode`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum NumberBoxValidationMode {
    /// Invalid text is replaced with the text of the value, and the value is coerced into the
    /// range.
    #[default]
    InvalidInputOverwritten = 0,
    /// Invalid text stays, and the value is not coerced.
    Disabled = 1,
}

ferro_markup_enum!(NumberBoxSpinButtonPlacementMode { Hidden, Compact, Inline }, { namespace: "FerroUI.Controls" });
ferro_markup_enum!(NumberBoxValidationMode { InvalidInputOverwritten, Disabled }, { namespace: "FerroUI.Controls" });

/// Formats the value of a [`NumberBox`] and parses its text (WinUI's `NumberFormatter`, an
/// `INumberFormatter2` that is also an `INumberParser`).
///
/// Deviation (docs/porting/winui-controls.md, Input): WinUI's default formatter is the regional
/// `DecimalFormatter` of the user; the port's default is
/// [`InvariantNumberBoxNumberFormatter`], and an application supplies a regional one.
pub trait INumberBoxNumberFormatter {
    /// The text of `value` (`INumberFormatter2::FormatDouble`).
    fn format_double(&self, value: f64) -> String;
    /// The number `text` is, or `None` (`INumberParser::ParseDouble`).
    fn parse_double(&self, text: &str) -> Option<f64>;
}

impl PartialEq for dyn INumberBoxNumberFormatter {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self, other)
    }
}

/// The invariant formatter: the shortest text that reads back as the same number, and Rust's
/// parsing of numbers.
pub struct InvariantNumberBoxNumberFormatter;

impl INumberBoxNumberFormatter for InvariantNumberBoxNumberFormatter {
    fn format_double(&self, value: f64) -> String {
        value.to_string()
    }

    fn parse_double(&self, text: &str) -> Option<f64> {
        text.parse().ok()
    }
}

/// The arguments of the `ValueChanged` event of a [`NumberBox`] (WinUI's
/// `NumberBoxValueChangedEventArgs`): the value before the change and the coerced new value.
#[derive(Clone, Debug, PartialEq)]
pub struct NumberBoxValueChangedEventArgs {
    /// The value before the change.
    pub old_value: f64,
    /// The new value.
    pub new_value: f64,
}

impl NumberBoxValueChangedEventArgs {
    /// The value before the change.
    pub fn old_value(&self) -> f64 {
        self.old_value
    }

    /// The new value.
    pub fn new_value(&self) -> f64 {
        self.new_value
    }
}

ferro_markup_type!(class NumberBoxValueChangedEventArgs {
    namespace: "FerroUI.Controls",
    handles: [NumberBoxValueChangedEventArgs],
    properties: [
        OldValue: f64 { get: NumberBoxValueChangedEventArgs::old_value },
        NewValue: f64 { get: NumberBoxValueChangedEventArgs::new_value },
    ],
});

/// Resets a flag when a scope ends (WinUI's `gsl::finally`).
struct Reset<'a>(&'a Cell<bool>);

impl Drop for Reset<'_> {
    fn drop(&mut self) {
        self.0.set(false);
    }
}

#[derive(Default)]
struct Parts {
    text_box: Option<Ref<TextBox>>,
    header_presenter: Option<Ref<ContentPresenter>>,
    popup: Option<Ref<Popup>>,
}

/// A text box for numbers (WinUI's `NumberBox`): a value of type `f64`, NaN when the box is
/// empty, with a range, steps, wrapping, expressions and spin buttons.
///
/// The text is committed when the box loses the focus, on Enter and before a step: it is parsed
/// with the [`NumberFormatter`](Self::number_formatter_property), or evaluated as an expression
/// (`utils::number_expression`) when [`AcceptsExpression`](Self::accepts_expression_property) is
/// set. Up and Down (and the spin buttons and the mouse wheel) step by `SmallChange`, PageUp and
/// PageDown by `LargeChange`; Escape restores the text of the value.
///
/// The template is the control theme of the WinUI theme (`Controls/NumberBox.xaml`, from WinUI's
/// `NumberBox.xaml`), with WinUI's parts: the text box `InputBox`, the spin buttons `UpSpinButton`
/// and `DownSpinButton`, the popup `UpDownPopup` with `PopupUpSpinButton` and
/// `PopupDownSpinButton`, and `HeaderContentPresenter`. The control sets WinUI's visual states as
/// pseudo-classes: `:spinbuttonsvisible` and `:spinbuttonspopup` (SpinButtonStates; neither is
/// SpinButtonsCollapsed), `:upspinbuttondisabled` and `:downspinbuttondisabled`. The Fluent and
/// Simple themes have no template for it (docs/porting/winui-controls.md, section 5.1).
///
/// Missing (docs/porting/winui-controls.md, Input): `InputScope`, `SelectionFlyout`,
/// `SelectionHighlightColor`, `TextReadingOrder` and `PreventKeyboardDisplayOnProgrammaticFocus`.
#[repr(C)]
pub struct NumberBox {
    base: TemplatedControl,
    value_updating: Cell<bool>,
    text_updating: Cell<bool>,
    parts: RefCell<Parts>,
    template_subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
    value_changed: HandlerList<dyn Fn(&NumberBoxValueChangedEventArgs)>,
}

ferro_class!(NumberBox: TemplatedControl);
ferroui_base::ferro_class_info!(NumberBox {
    new: NumberBox::new,
    markup: {
        namespace: "FerroUI.Controls",
        events: [
            ValueChanged(Option<BoxedValue>, NumberBoxValueChangedEventArgs) => |this: &Ref<NumberBox>, handler: MarkupDelegate| {
                let sender = this.downgrade();
                this.value_changed(move |args: &NumberBoxValueChangedEventArgs| {
                    handler.invoke(&[into_markup_value(sender.upgrade()), into_markup_value(args.clone())]);
                })
            },
        ],
        attributes: [
            TemplatePart("InputBox", type(Ref<TextBox>)),
            TemplatePart("UpSpinButton", type(Ref<RepeatButton>)),
            TemplatePart("DownSpinButton", type(Ref<RepeatButton>)),
            TemplatePart("UpDownPopup", type(Ref<Popup>)),
            TemplatePart("PopupUpSpinButton", type(Ref<RepeatButton>)),
            TemplatePart("PopupDownSpinButton", type(Ref<RepeatButton>)),
            TemplatePart("HeaderContentPresenter", type(Ref<ContentPresenter>)),
            PseudoClasses(":spinbuttonsvisible", ":spinbuttonspopup", ":upspinbuttondisabled", ":downspinbuttondisabled"),
        ],
    },
});
ferro_impl_classes!(NumberBox: StyledElementImpl, VisualImpl, LayoutableImpl, InteractiveImpl);

impl FerroObjectImpl for NumberBox {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        // NumberBox listens to PreviewKeyDown of its text box, "because textbox eats the down arrow
        // key in some circumstances": the tunnel route of the key down event.
        let weak = this.to_ref().downgrade();
        this.add_handler_with(
            InputElement::key_down_event(),
            move |_, args| {
                if let Some(this) = weak.upgrade() {
                    this.on_number_box_key_down(args);
                }
            },
            RoutingStrategies::TUNNEL,
            false,
        );
    }

    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        let property = change.property();
        if property == Self::value_property().as_property() {
            this.on_value_property_changed(change.get_old_value::<f64>().unwrap_or(f64::NAN));
        } else if property == Self::minimum_property().as_property() {
            // OnMinimumPropertyChanged
            this.coerce_maximum();
            this.coerce_value();
            this.update_spin_button_enabled();
            this.reevaluate_forwarded_uia_properties();
        } else if property == Self::maximum_property().as_property() {
            // OnMaximumPropertyChanged
            this.coerce_minimum();
            this.coerce_value();
            this.update_spin_button_enabled();
            this.reevaluate_forwarded_uia_properties();
        } else if property == Self::small_change_property().as_property()
            || property == Self::is_wrap_enabled_property().as_property()
        {
            this.update_spin_button_enabled();
        } else if property == Self::number_formatter_property().as_property() {
            this.update_text_to_value();
        } else if property == Self::spin_button_placement_mode_property().as_property() {
            this.update_spin_button_placement();
        } else if property == Self::text_property().as_property() {
            if !this.text_updating.get() {
                this.update_value_to_text();
            }
        } else if property == Self::header_property().as_property()
            || property == Self::header_template_property().as_property()
        {
            this.update_header_presenter_state();
        } else if property == Self::validation_mode_property().as_property() {
            this.validate_input();
            this.update_spin_button_enabled();
        } else if property == AutomationProperties::name_property().as_property()
            || property == AutomationProperties::labeled_by_property().as_property()
        {
            this.reevaluate_forwarded_uia_properties();
        }
    }
}

impl ControlImpl for NumberBox {
    fn on_create_automation_peer(this: &Self) -> Ref<AutomationPeer> {
        NumberBoxAutomationPeer::new(this).upcast()
    }
}

impl InputElementImpl for NumberBox {
    fn on_key_up(this: &Self, args: &KeyEventArgs) {
        Self::parent_on_key_up(this, args);
        if args.handled() {
            return;
        }
        // OnNumberBoxKeyUp. GamepadA and GamepadB are Enter and Escape in FerroUI's key events.
        match args.key {
            Key::Enter => {
                this.validate_input();
                args.set_handled(true);
            }
            Key::Escape => {
                this.update_text_to_value();
                args.set_handled(true);
            }
            _ => {}
        }
    }

    fn on_got_focus(this: &Self, args: &FocusChangedEventArgs) {
        Self::parent_on_got_focus(this, args);
        // OnNumberBoxGotFocus: select the text, and open the popup of the compact spin buttons.
        let (text_box, popup) = {
            let parts = this.parts.borrow();
            (parts.text_box.clone(), parts.popup.clone())
        };
        if let Some(text_box) = text_box {
            text_box.select_all();
        }
        if this.spin_button_placement_mode() == NumberBoxSpinButtonPlacementMode::Compact {
            if let Some(popup) = popup {
                popup.set_is_open(true);
            }
        }
    }

    fn on_lost_focus(this: &Self, args: &FocusChangedEventArgs) {
        Self::parent_on_lost_focus(this, args);
        // OnNumberBoxLostFocus
        this.validate_input();
        let popup = this.parts.borrow().popup.clone();
        if let Some(popup) = popup {
            popup.set_is_open(false);
        }
    }

    fn on_pointer_wheel_changed(this: &Self, args: &PointerWheelEventArgs) {
        Self::parent_on_pointer_wheel_changed(this, args);
        if args.handled() {
            return;
        }
        // OnNumberBoxScroll: only while the text box has the focus.
        let text_box = this.parts.borrow().text_box.clone();
        if text_box.is_some_and(|text_box| text_box.is_keyboard_focus_within()) {
            let delta = args.delta().y;
            if delta > 0.0 {
                this.step_value(this.small_change());
            } else if delta < 0.0 {
                this.step_value(-this.small_change());
            }
            args.set_handled(true);
        }
    }
}

impl TemplatedControlImpl for NumberBox {
    fn on_apply_template(this: &Self, args: &TemplateAppliedEventArgs) {
        Self::parent_on_apply_template(this, args);
        this.unhook_events();
        let scope = args.name_scope();

        let mut subscriptions = Vec::new();
        // The localized names of the spin buttons, when the template gives them none
        // (SR_NumberBoxDownSpinButtonName and SR_NumberBoxUpSpinButtonName of the English
        // resources).
        for (name, sign, automation_name) in
            [("DownSpinButton", -1.0, Some("Decrease")), ("UpSpinButton", 1.0, Some("Increase")), ("PopupDownSpinButton", -1.0, None), ("PopupUpSpinButton", 1.0, None)]
        {
            let Some(button) = scope.find_as::<RepeatButton>(name) else { continue };
            if let Some(automation_name) = automation_name {
                if AutomationProperties::get_name(&button).unwrap_or_default().is_empty() {
                    AutomationProperties::set_name(&button, Some(automation_name));
                }
            }
            let weak = this.to_ref().downgrade();
            // OnSpinDownClick and OnSpinUpClick.
            let token = button.click(move |_, _| {
                if let Some(this) = weak.upgrade() {
                    this.step_value(sign * this.small_change());
                }
            });
            let button = button.downgrade();
            subscriptions.push(Disposable::create(move || {
                if let Some(button) = button.upgrade() {
                    button.remove_handler(Button::click_event(), token);
                }
            }));
        }
        *this.template_subscriptions.borrow_mut() = subscriptions;
        *this.parts.borrow_mut() = Parts {
            text_box: scope.find_as::<TextBox>("InputBox"),
            header_presenter: scope.find_as::<ContentPresenter>("HeaderContentPresenter"),
            popup: scope.find_as::<Popup>("UpDownPopup"),
        };
        this.update_header_presenter_state();

        this.update_spin_button_placement();
        this.update_spin_button_enabled();
        this.reevaluate_forwarded_uia_properties();

        // If Text has been set, but Value hasn't, update Value based on Text.
        if !this.is_set(Self::value_property().as_property()) && this.is_set(Self::text_property().as_property()) {
            this.update_value_to_text();
        } else {
            this.update_text_to_value();
        }
    }
}

ferroui_base::ferro_properties! { impl NumberBox {
    ferro_property!(
        /// Defines the `Minimum` property: the smallest value. The largest negative `f64` by
        /// default.
        pub fn minimum_property() -> StyledProperty<f64> {
            FerroProperty::register::<NumberBox, _>("Minimum", -f64::MAX)
        }
    );

    ferro_property!(
        /// Defines the `Maximum` property: the largest value. The largest `f64` by default.
        pub fn maximum_property() -> StyledProperty<f64> {
            FerroProperty::register::<NumberBox, _>("Maximum", f64::MAX)
        }
    );

    ferro_property!(
        /// Defines the `Value` property: the number, NaN when the box is empty (the default).
        pub fn value_property() -> StyledProperty<f64> {
            FerroProperty::register::<NumberBox, _>("Value", f64::NAN)
        }
    );

    ferro_property!(
        /// Defines the `SmallChange` property: the step of the arrow keys, the spin buttons and
        /// the mouse wheel. 1 by default.
        pub fn small_change_property() -> StyledProperty<f64> {
            FerroProperty::register::<NumberBox, _>("SmallChange", 1.0)
        }
    );

    ferro_property!(
        /// Defines the `LargeChange` property: the step of PageUp and PageDown. 10 by default.
        pub fn large_change_property() -> StyledProperty<f64> {
            FerroProperty::register::<NumberBox, _>("LargeChange", 10.0)
        }
    );

    ferro_property!(
        /// Defines the `Text` property: the text of the value. Setting it commits it as input.
        pub fn text_property() -> StyledProperty<String> {
            FerroProperty::register::<NumberBox, _>("Text", String::new())
        }
    );

    ferro_property!(
        /// Defines the `Header` property: the content above the box.
        pub fn header_property() -> StyledProperty<Option<BoxedValue>> {
            FerroProperty::register::<NumberBox, _>("Header", None)
        }
    );

    ferro_property!(
        /// Defines the `HeaderTemplate` property: the template of the header.
        pub fn header_template_property() -> StyledProperty<Option<Rc<dyn IDataTemplate>>> {
            FerroProperty::register::<NumberBox, _>("HeaderTemplate", None)
        }
    );

    ferro_property!(
        /// Defines the `PlaceholderText` property: the text shown while the box is empty.
        pub fn placeholder_text_property() -> StyledProperty<String> {
            FerroProperty::register::<NumberBox, _>("PlaceholderText", String::new())
        }
    );

    ferro_property!(
        /// Defines the `TextAlignment` property: the alignment of the text. Left by default.
        pub fn text_alignment_property() -> StyledProperty<TextAlignment> {
            FerroProperty::register::<NumberBox, _>("TextAlignment", TextAlignment::Left)
        }
    );

    ferro_property!(
        /// Defines the `Description` property: the content below the box.
        pub fn description_property() -> StyledProperty<Option<BoxedValue>> {
            FerroProperty::register::<NumberBox, _>("Description", None)
        }
    );

    ferro_property!(
        /// Defines the `ValidationMode` property. `InvalidInputOverwritten` by default.
        pub fn validation_mode_property() -> StyledProperty<NumberBoxValidationMode> {
            FerroProperty::register::<NumberBox, _>("ValidationMode", NumberBoxValidationMode::InvalidInputOverwritten)
        }
    );

    ferro_property!(
        /// Defines the `SpinButtonPlacementMode` property. `Hidden` by default.
        pub fn spin_button_placement_mode_property() -> StyledProperty<NumberBoxSpinButtonPlacementMode> {
            FerroProperty::register::<NumberBox, _>("SpinButtonPlacementMode", NumberBoxSpinButtonPlacementMode::Hidden)
        }
    );

    ferro_property!(
        /// Defines the `IsWrapEnabled` property: whether a step past one end of the range goes
        /// to the other end. `false` by default.
        pub fn is_wrap_enabled_property() -> StyledProperty<bool> {
            FerroProperty::register::<NumberBox, _>("IsWrapEnabled", false)
        }
    );

    ferro_property!(
        /// Defines the `AcceptsExpression` property: whether the text is evaluated as an
        /// arithmetic expression. `false` by default.
        pub fn accepts_expression_property() -> StyledProperty<bool> {
            FerroProperty::register::<NumberBox, _>("AcceptsExpression", false)
        }
    );

    ferro_property!(
        /// Defines the `NumberFormatter` property: the formatter of the value and parser of the
        /// text. [`InvariantNumberBoxNumberFormatter`] by default.
        pub fn number_formatter_property() -> StyledProperty<Rc<dyn INumberBoxNumberFormatter>> {
            FerroProperty::register::<NumberBox, _>(
                "NumberFormatter",
                Rc::new(InvariantNumberBoxNumberFormatter) as Rc<dyn INumberBoxNumberFormatter>,
            )
        }
    );
} }

impl NumberBox {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: TemplatedControl::construct(),
            value_updating: Cell::new(false),
            text_updating: Cell::new(false),
            parts: RefCell::new(Parts::default()),
            template_subscriptions: RefCell::new(Vec::new()),
            value_changed: HandlerList::new(),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets the smallest value.
    pub fn minimum(&self) -> f64 {
        self.get_value(Self::minimum_property())
    }

    /// Sets the smallest value.
    pub fn set_minimum(&self, value: f64) {
        self.set_value(Self::minimum_property(), value);
    }

    /// Gets the largest value.
    pub fn maximum(&self) -> f64 {
        self.get_value(Self::maximum_property())
    }

    /// Sets the largest value.
    pub fn set_maximum(&self, value: f64) {
        self.set_value(Self::maximum_property(), value);
    }

    /// Gets the value: NaN when the box is empty.
    pub fn value(&self) -> f64 {
        self.get_value(Self::value_property())
    }

    /// Sets the value (WinUI's `Value` setter). Setting NaN while the value is NaN does nothing:
    /// "we are using nan as a value to represent value not set (cleared)", and NaN is not equal to
    /// itself.
    pub fn set_numeric_value(&self, value: f64) {
        if !value.is_nan() || !self.value().is_nan() {
            self.set_value(Self::value_property(), value);
        }
    }

    /// Gets the step of the arrow keys, the spin buttons and the mouse wheel.
    pub fn small_change(&self) -> f64 {
        self.get_value(Self::small_change_property())
    }

    /// Sets the step of the arrow keys, the spin buttons and the mouse wheel.
    pub fn set_small_change(&self, value: f64) {
        self.set_value(Self::small_change_property(), value);
    }

    /// Gets the step of PageUp and PageDown.
    pub fn large_change(&self) -> f64 {
        self.get_value(Self::large_change_property())
    }

    /// Sets the step of PageUp and PageDown.
    pub fn set_large_change(&self, value: f64) {
        self.set_value(Self::large_change_property(), value);
    }

    /// Gets the text of the value.
    pub fn text(&self) -> String {
        self.get_value(Self::text_property())
    }

    /// Sets the text; it is committed as input.
    pub fn set_text(&self, value: impl Into<String>) {
        self.set_value(Self::text_property(), value.into());
    }

    /// Gets the header.
    pub fn header(&self) -> Option<BoxedValue> {
        self.get_value(Self::header_property())
    }

    /// Sets the header.
    pub fn set_header(&self, value: Option<BoxedValue>) {
        self.set_value(Self::header_property(), value);
    }

    /// Gets the template of the header.
    pub fn header_template(&self) -> Option<Rc<dyn IDataTemplate>> {
        self.get_value(Self::header_template_property())
    }

    /// Sets the template of the header.
    pub fn set_header_template(&self, value: Option<Rc<dyn IDataTemplate>>) {
        self.set_value(Self::header_template_property(), value);
    }

    /// Gets the text shown while the box is empty.
    pub fn placeholder_text(&self) -> String {
        self.get_value(Self::placeholder_text_property())
    }

    /// Sets the text shown while the box is empty.
    pub fn set_placeholder_text(&self, value: impl Into<String>) {
        self.set_value(Self::placeholder_text_property(), value.into());
    }

    /// Gets the alignment of the text.
    pub fn text_alignment(&self) -> TextAlignment {
        self.get_value(Self::text_alignment_property())
    }

    /// Sets the alignment of the text.
    pub fn set_text_alignment(&self, value: TextAlignment) {
        self.set_value(Self::text_alignment_property(), value);
    }

    /// Gets the content below the box.
    pub fn description(&self) -> Option<BoxedValue> {
        self.get_value(Self::description_property())
    }

    /// Sets the content below the box.
    pub fn set_description(&self, value: Option<BoxedValue>) {
        self.set_value(Self::description_property(), value);
    }

    /// Gets the validation mode.
    pub fn validation_mode(&self) -> NumberBoxValidationMode {
        self.get_value(Self::validation_mode_property())
    }

    /// Sets the validation mode.
    pub fn set_validation_mode(&self, value: NumberBoxValidationMode) {
        self.set_value(Self::validation_mode_property(), value);
    }

    /// Gets where the spin buttons are.
    pub fn spin_button_placement_mode(&self) -> NumberBoxSpinButtonPlacementMode {
        self.get_value(Self::spin_button_placement_mode_property())
    }

    /// Sets where the spin buttons are.
    pub fn set_spin_button_placement_mode(&self, value: NumberBoxSpinButtonPlacementMode) {
        self.set_value(Self::spin_button_placement_mode_property(), value);
    }

    /// Gets whether a step past one end of the range goes to the other end.
    pub fn is_wrap_enabled(&self) -> bool {
        self.get_value(Self::is_wrap_enabled_property())
    }

    /// Sets whether a step past one end of the range goes to the other end.
    pub fn set_is_wrap_enabled(&self, value: bool) {
        self.set_value(Self::is_wrap_enabled_property(), value);
    }

    /// Gets whether the text is evaluated as an arithmetic expression.
    pub fn accepts_expression(&self) -> bool {
        self.get_value(Self::accepts_expression_property())
    }

    /// Sets whether the text is evaluated as an arithmetic expression.
    pub fn set_accepts_expression(&self, value: bool) {
        self.set_value(Self::accepts_expression_property(), value);
    }

    /// Gets the formatter of the value and parser of the text.
    pub fn number_formatter(&self) -> Rc<dyn INumberBoxNumberFormatter> {
        self.get_value(Self::number_formatter_property())
    }

    /// Sets the formatter of the value and parser of the text.
    pub fn set_number_formatter(&self, value: Rc<dyn INumberBoxNumberFormatter>) {
        self.set_value(Self::number_formatter_property(), value);
    }

    /// Adds a handler of the `ValueChanged` event, raised after a change of the value with the
    /// value before it and the coerced new value. Disposing the result removes the handler.
    pub fn value_changed(&self, handler: impl Fn(&NumberBoxValueChangedEventArgs) + 'static) -> Rc<dyn IDisposable> {
        let token = self.value_changed.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.value_changed.remove(token);
            }
        })
    }

    /// Commits the text of the box (WinUI's `ValidateInput`): empty text clears the value; text
    /// that is a number (or, with `AcceptsExpression`, an expression) becomes the value and is
    /// formatted again; other text is replaced with the text of the value unless validation is
    /// disabled. Returns whether the text was a number or empty.
    ///
    /// Deviation (docs/porting/winui-controls.md, Input): without a template (no text box) the
    /// text committed is the `Text` property.
    pub fn validate_input(&self) -> bool {
        let text_box = self.parts.borrow().text_box.clone();
        let text = match &text_box {
            Some(text_box) => text_box.text().unwrap_or_default(),
            None => self.text(),
        };
        let text = text.trim_matches([' ', '\n', '\r', '\t', '\u{000C}', '\u{000B}']);
        if text.is_empty() {
            self.set_value_coerced(f64::NAN);
            return true;
        }
        let formatter = self.number_formatter();
        let value = if self.accepts_expression() {
            match evaluate_number_expression(text, |number| formatter.parse_double(number)) {
                Ok(value) => Some(value),
                // NumberBoxParser computes NaN for a division by zero.
                Err(error) if error.kind == NumberExpressionErrorKind::DivisionByZero => Some(f64::NAN),
                Err(_) => None,
            }
        } else {
            formatter.parse_double(text)
        };
        match value {
            None => {
                if self.validation_mode() == NumberBoxValidationMode::InvalidInputOverwritten {
                    // Override text to current value.
                    self.update_text_to_value();
                }
                false
            }
            Some(value) => {
                if value == self.value() {
                    // Even if the value hasn't changed, we still want to update the text (e.g.
                    // Value is 3, user types 1 + 2, we want to replace the text with 3).
                    self.update_text_to_value();
                } else {
                    self.set_value_coerced(value);
                }
                true
            }
        }
    }

    /// Commits the text and adds `change` to the value (WinUI's `StepValue`), wrapping past the
    /// ends of the range when `IsWrapEnabled` is set. Nothing happens while the box is empty.
    pub fn step_value(&self, change: f64) {
        // Before adjusting the value, validate the contents of the textbox so we don't override it.
        self.validate_input();
        let mut new_value = self.value();
        if new_value.is_nan() {
            return;
        }
        new_value += change;
        if self.is_wrap_enabled() {
            let (max, min) = (self.maximum(), self.minimum());
            if new_value > max {
                new_value = min;
            } else if new_value < min {
                new_value = max;
            }
        }
        self.set_value_coerced(new_value);
        // We don't want the caret to move to the front of the text for example when using the
        // up/down arrows to change the numberbox value.
        self.move_caret_to_text_end();
    }

    /// The value the control sets: a current value, which keeps a binding of `Value`; NaN over
    /// NaN does nothing, as the `Value` setter.
    fn set_value_coerced(&self, value: f64) {
        if !value.is_nan() || !self.value().is_nan() {
            self.set_current_value(Self::value_property(), value);
        }
    }

    fn unhook_events(&self) {
        let subscriptions = self.template_subscriptions.take();
        for subscription in subscriptions {
            subscription.dispose();
        }
        let old = self.parts.take();
        if let Some(popup) = old.popup {
            popup.set_is_open(false);
        }
    }

    fn on_value_property_changed(&self, old_value: f64) {
        // This handler may change Value; don't send extra events in that case.
        if self.value_updating.replace(true) {
            return;
        }
        let _reset = Reset(&self.value_updating);
        self.coerce_value();
        let new_value = self.value();
        if new_value != old_value && !(new_value.is_nan() && old_value.is_nan()) {
            let args = NumberBoxValueChangedEventArgs { old_value, new_value };
            for (_, handler) in self.value_changed.snapshot().iter() {
                handler(&args);
            }
            // Fire value property change for UIA.
            if let Some(peer) = self.get_automation_peer().and_then(|peer| peer.cast::<NumberBoxAutomationPeer>()) {
                peer.raise_value_changed_event(old_value, new_value);
            }
        }
        self.update_text_to_value();
        self.update_spin_button_enabled();
    }

    fn update_value_to_text(&self) {
        let text_box = self.parts.borrow().text_box.clone();
        if let Some(text_box) = text_box {
            text_box.set_text(Some(&self.text()));
        }
        self.validate_input();
    }

    fn coerce_minimum(&self) {
        let max = self.maximum();
        if self.minimum() > max {
            self.set_current_value(Self::minimum_property(), max);
        }
    }

    fn coerce_maximum(&self) {
        let min = self.minimum();
        if self.maximum() < min {
            self.set_current_value(Self::maximum_property(), min);
        }
    }

    fn coerce_value(&self) {
        // Validate that the value is in bounds.
        let value = self.value();
        if !value.is_nan()
            && !self.is_in_bounds(value)
            && self.validation_mode() == NumberBoxValidationMode::InvalidInputOverwritten
        {
            // Coerce value to be within range.
            let max = self.maximum();
            if value > max {
                self.set_current_value(Self::value_property(), max);
            } else {
                self.set_current_value(Self::value_property(), self.minimum());
            }
        }
    }

    fn is_in_bounds(&self, value: f64) -> bool {
        value >= self.minimum() && value <= self.maximum()
    }

    /// Updates the text of the box and `Text` with the formatted value (WinUI's
    /// `UpdateTextToValue`).
    ///
    /// Deviation (docs/porting/winui-controls.md, Input): without a template `Text` is updated
    /// too.
    fn update_text_to_value(&self) {
        if self.text_updating.replace(true) {
            return;
        }
        let _reset = Reset(&self.text_updating);
        let value = self.value();
        let new_text = if value.is_nan() {
            String::new()
        } else {
            // Rounding the value here will prevent displaying digits caused by floating point
            // imprecision.
            self.number_formatter().format_double(round_to_significant_digits(value))
        };
        let text_box = self.parts.borrow().text_box.clone();
        if let Some(text_box) = text_box {
            text_box.set_text(Some(&new_text));
        }
        self.set_current_value(Self::text_property(), new_text);
    }

    fn update_spin_button_placement(&self) {
        let mode = self.spin_button_placement_mode();
        self.pseudo_classes().set(":spinbuttonsvisible", mode == NumberBoxSpinButtonPlacementMode::Inline);
        self.pseudo_classes().set(":spinbuttonspopup", mode == NumberBoxSpinButtonPlacementMode::Compact);
        if mode != NumberBoxSpinButtonPlacementMode::Compact {
            let popup = self.parts.borrow().popup.clone();
            if let Some(popup) = popup {
                popup.set_is_open(false);
            }
        }
    }

    fn update_spin_button_enabled(&self) {
        let value = self.value();
        let mut is_up_button_enabled = false;
        let mut is_down_button_enabled = false;
        if !value.is_nan() {
            if self.is_wrap_enabled() || self.validation_mode() != NumberBoxValidationMode::InvalidInputOverwritten {
                // If wrapping is enabled, or invalid values are allowed, then the buttons should
                // be enabled.
                is_up_button_enabled = true;
                is_down_button_enabled = true;
            } else {
                is_up_button_enabled = value < self.maximum();
                is_down_button_enabled = value > self.minimum();
            }
        }
        self.pseudo_classes().set(":upspinbuttondisabled", !is_up_button_enabled);
        self.pseudo_classes().set(":downspinbuttondisabled", !is_down_button_enabled);
    }

    /// The name of the text box for UI Automation: the name of the box or its text header, and
    /// its range; and the label of the box.
    fn reevaluate_forwarded_uia_properties(&self) {
        let Some(text_box) = self.parts.borrow().text_box.clone() else { return };
        // UIA Name (SR_NumberBoxMinimumValueStatus and SR_NumberBoxMaximumValueStatus of the
        // English resources).
        let minimum =
            if self.minimum() == -f64::MAX { String::new() } else { format!(" Minimum{}", self.minimum()) };
        let maximum =
            if self.maximum() == f64::MAX { String::new() } else { format!(" Maximum{}", self.maximum()) };
        let name = AutomationProperties::get_name(self).unwrap_or_default();
        if !name.is_empty() {
            // AutomationProperties.Name is a non empty string, we will use that value.
            AutomationProperties::set_name(&text_box, Some(&format!("{name}{minimum}{maximum}")));
        } else if let Some(header) = self.header().and_then(|header| header.downcast_ref::<String>().cloned()) {
            // Header is a string, we can use that as our UIA name.
            AutomationProperties::set_name(&text_box, Some(&format!("{header}{minimum}{maximum}")));
        }
        // UIA LabeledBy
        if let Some(labeled_by) = AutomationProperties::get_labeled_by(self) {
            AutomationProperties::set_labeled_by(&text_box, Some(labeled_by));
        }
    }

    /// Shows the header presenter only for a header that is a non-empty string or another object,
    /// or a header template (WinUI's `UpdateHeaderPresenterState`).
    fn update_header_presenter_state(&self) {
        let mut should_show_header = false;
        if let Some(header) = self.header() {
            match header.downcast_ref::<String>() {
                Some(text) => should_show_header = !text.is_empty(),
                None => {
                    should_show_header = true;
                    // When our header isn't a string, we use the NumberBox's UIA name for the
                    // textbox's UIA name.
                    let text_box = self.parts.borrow().text_box.clone();
                    if let Some(text_box) = text_box {
                        AutomationProperties::set_name(&text_box, AutomationProperties::get_name(self).as_deref());
                    }
                }
            }
        }
        if self.header_template().is_some() {
            should_show_header = true;
        }
        let presenter = self.parts.borrow().header_presenter.clone();
        if let Some(presenter) = presenter {
            presenter.set_is_visible(should_show_header);
        }
        self.reevaluate_forwarded_uia_properties();
    }

    fn move_caret_to_text_end(&self) {
        let text_box = self.parts.borrow().text_box.clone();
        if let Some(text_box) = text_box {
            // This places the caret at the end of the text.
            let end = text_box.text().map_or(0, |text| text.encode_utf16().count() as i32);
            text_box.set_selection_start(end);
            text_box.set_selection_end(end);
            text_box.set_caret_index(end);
        }
    }

    /// OnNumberBoxKeyDown: the steps of the arrow and page keys, on key down so that they repeat.
    fn on_number_box_key_down(&self, args: &KeyEventArgs) {
        if args.handled() {
            return;
        }
        let text_box = self.parts.borrow().text_box.clone();
        if !text_box.is_some_and(|text_box| text_box.is_keyboard_focus_within()) {
            return;
        }
        let change = match args.key {
            Key::Up => self.small_change(),
            Key::Down => -self.small_change(),
            Key::PageUp => self.large_change(),
            Key::PageDown => -self.large_change(),
            _ => return,
        };
        self.step_value(change);
        args.set_handled(true);
    }
}

/// The value rounded to ten significant digits for display (the `IncrementNumberRounder` of
/// NumberBox with `SignificantDigits(10)`, whose default rounding, RoundHalfUp, resolves a tie
/// toward positive infinity). The value itself is not changed. The rounding works on the decimal
/// digits of the value, so that scaling never overflows or underflows.
///
/// Deviation (docs/porting/winui-controls.md, Input): a port of the rounding, not of the WinRT
/// rounder.
fn round_to_significant_digits(value: f64) -> f64 {
    if !value.is_finite() || value == 0.0 {
        return value;
    }
    let scientific = format!("{:.16e}", value.abs());
    let Some((mantissa, exponent)) = scientific.split_once('e') else { return value };
    let Ok(exponent) = exponent.parse::<i32>() else { return value };
    let digits: Vec<u8> = mantissa.bytes().filter(u8::is_ascii_digit).map(|digit| digit - b'0').collect();
    if digits.len() < 11 {
        return value;
    }
    let mut retained = digits[..10].iter().fold(0u64, |number, digit| number * 10 + u64::from(*digit));
    let next = digits[10];
    let greater_than_half = next > 5 || next == 5 && digits[11..].iter().any(|digit| *digit != 0);
    if greater_than_half || next == 5 && value.is_sign_positive() {
        retained += 1;
    }
    let sign = if value.is_sign_negative() { "-" } else { "" };
    format!("{sign}{retained}e{}", exponent - 9).parse().unwrap_or(value)
}
