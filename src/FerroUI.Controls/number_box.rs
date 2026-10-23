//! WinUI's floating-point number editor, using the framework's text, input and template services.

use super::number_box_expression::evaluate_number_expression;
use crate::automation::AutomationProperties;
use crate::presenters::ContentPresenter;
use crate::primitives::{
    Popup, TemplateAppliedEventArgs, TemplatedControl, TemplatedControlImpl,
    TemplatedControlImplExt,
};
use crate::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions, IDataTemplate};
use crate::{Button, Control, ControlImpl, Grid, PlacementMode, RepeatButton, StackPanel, TextBox};
use ferroui_base::input::{
    FocusChangedEventArgs, InputElementImpl, InputElementImplExt, Key, KeyEventArgs,
    PointerWheelEventArgs,
};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{LayoutableImpl, Orientation};
use ferroui_base::media::TextAlignment;
use ferroui_base::metadata::{into_markup_value, MarkupDelegate, MarkupType, MarkupTyped};
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_markup_enum, ferro_properties,
    instantiate, BoxedValue, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    FerroPropertyChangedEventArgs, Ref, StyledElementImpl, StyledProperty, VisualImpl,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Visibility and placement of the native spin buttons.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum NumberBoxSpinButtonPlacementMode {
    #[default]
    Hidden = 0,
    Compact = 1,
    Inline = 2,
}

/// Whether invalid text and values outside the range are replaced.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum NumberBoxValidationMode {
    #[default]
    InvalidInputOverwritten = 0,
    Disabled = 1,
}

ferro_markup_enum!(NumberBoxSpinButtonPlacementMode { Hidden, Compact, Inline }, { namespace: "FerroUI.Controls" });
ferro_markup_enum!(NumberBoxValidationMode { InvalidInputOverwritten, Disabled }, { namespace: "FerroUI.Controls" });

/// A formatter must also parse the values it produces, as the native contract requires.
/// Host applications can supply their existing localization/number-format service here.
pub trait INumberBoxNumberFormatter {
    fn format_double(&self, value: f64) -> String;
    fn parse_double(&self, text: &str) -> Option<f64>;
}

impl PartialEq for dyn INumberBoxNumberFormatter {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self, other)
    }
}

/// Portable invariant formatting. Native regional formatting is an explicit host adapter.
pub struct InvariantNumberBoxNumberFormatter;

impl INumberBoxNumberFormatter for InvariantNumberBoxNumberFormatter {
    fn format_double(&self, value: f64) -> String {
        value.to_string()
    }
    fn parse_double(&self, text: &str) -> Option<f64> {
        text.parse().ok()
    }
}

/// The actual old and coerced new values of one numeric edit.
#[derive(Clone, Debug, PartialEq)]
pub struct NumberBoxValueChangedEventArgs {
    pub old_value: f64,
    pub new_value: f64,
}

ferroui_base::ferro_markup_type!(class NumberBoxValueChangedEventArgs {
    namespace: "FerroUI.Controls",
    handles: [NumberBoxValueChangedEventArgs],
    properties: [
        OldValue: f64 { get: |e: &NumberBoxValueChangedEventArgs| e.old_value },
        NewValue: f64 { get: |e: &NumberBoxValueChangedEventArgs| e.new_value },
    ],
});

struct Reset<'a>(&'a Cell<bool>);
impl Drop for Reset<'_> {
    fn drop(&mut self) {
        self.0.set(false);
    }
}

/// A double-precision numeric editor with empty/NaN, expression, range and wrapping semantics.
///
/// This is a separate control because the existing decimal editor intentionally has a
/// different value domain. Its visual parts are ordinary native framework controls.
#[repr(C)]
pub struct NumberBox {
    base: TemplatedControl,
    updating_value: Cell<bool>,
    updating_text: Cell<bool>,
    parts: RefCell<Parts>,
    subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
    value_changed: HandlerList<dyn Fn(&NumberBoxValueChangedEventArgs)>,
}

#[derive(Default)]
struct Parts {
    text: Option<Ref<TextBox>>,
    up: Option<Ref<RepeatButton>>,
    down: Option<Ref<RepeatButton>>,
    popup_up: Option<Ref<RepeatButton>>,
    popup_down: Option<Ref<RepeatButton>>,
    popup: Option<Ref<Popup>>,
    header: Option<Ref<ContentPresenter>>,
    description: Option<Ref<ContentPresenter>>,
}

ferro_class!(NumberBox: TemplatedControl);
ferro_class_info!(NumberBox {
    new: NumberBox::new,
    markup: {
        events: [
            ValueChanged(Option<BoxedValue>, NumberBoxValueChangedEventArgs) => |this: &Ref<NumberBox>, handler: MarkupDelegate| {
                let weak = this.downgrade();
                this.value_changed(move |args| {
                    if let Some(sender) = weak.upgrade() {
                        handler.invoke(&[into_markup_value(Some(sender)), into_markup_value(args.clone())]);
                    }
                })
            },
        ],
    },
});
ferro_impl_classes!(NumberBox: StyledElementImpl, VisualImpl, LayoutableImpl, InteractiveImpl);

impl FerroObjectImpl for NumberBox {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        let property = change.property();
        if property == Self::value_property().as_property() {
            this.on_value_changed(change.get_old_value::<f64>().unwrap_or(f64::NAN));
        } else if property == Self::text_property().as_property() {
            if !this.updating_text.get() {
                this.set_editor_text(&this.text());
                this.commit_input();
            }
        } else if property == Self::minimum_property().as_property() {
            if this.minimum() > this.maximum() {
                this.set_current_value(Self::maximum_property(), this.minimum());
            }
            this.coerce_numeric_value();
            this.update_parts();
        } else if property == Self::maximum_property().as_property() {
            if this.maximum() < this.minimum() {
                this.set_current_value(Self::minimum_property(), this.maximum());
            }
            this.coerce_numeric_value();
            this.update_parts();
        } else if property == Self::number_formatter_property().as_property() {
            this.update_text_from_value();
        } else {
            if property == Self::validation_mode_property().as_property() {
                this.coerce_numeric_value();
            }
            this.update_parts();
        }
    }
}

impl ControlImpl for NumberBox {
    fn on_create_automation_peer(this: &Self) -> Ref<crate::automation::peers::AutomationPeer> {
        crate::automation::peers::NumberBoxAutomationPeer::new(this).upcast()
    }
}

impl InputElementImpl for NumberBox {
    fn on_key_down(this: &Self, e: &KeyEventArgs) {
        Self::parent_on_key_down(this, e);
        if e.handled() || !this.is_effectively_enabled() {
            return;
        }
        let change = match e.key {
            Key::Up => this.small_change(),
            Key::Down => -this.small_change(),
            Key::PageUp => this.large_change(),
            Key::PageDown => -this.large_change(),
            _ => return,
        };
        this.step_value(change);
        e.set_handled(true);
    }
    fn on_key_up(this: &Self, e: &KeyEventArgs) {
        Self::parent_on_key_up(this, e);
        if e.handled() {
            return;
        }
        match e.key {
            Key::Enter => {
                this.commit_input();
                e.set_handled(true);
            }
            Key::Escape => {
                this.update_text_from_value();
                e.set_handled(true);
            }
            _ => {}
        }
    }
    fn on_got_focus(this: &Self, e: &FocusChangedEventArgs) {
        Self::parent_on_got_focus(this, e);
        let text = this.parts.borrow().text.clone();
        if let Some(text) = text {
            text.select_all();
        }
        let popup = this.parts.borrow().popup.clone();
        if this.spin_button_placement_mode() == NumberBoxSpinButtonPlacementMode::Compact {
            if let Some(popup) = popup {
                popup.set_is_open(true);
            }
        }
    }
    fn on_lost_focus(this: &Self, e: &FocusChangedEventArgs) {
        Self::parent_on_lost_focus(this, e);
        this.commit_input();
        let popup = this.parts.borrow().popup.clone();
        if let Some(popup) = popup {
            popup.set_is_open(false);
        }
    }
    fn on_pointer_wheel_changed(this: &Self, e: &PointerWheelEventArgs) {
        Self::parent_on_pointer_wheel_changed(this, e);
        if !e.handled()
            && this.is_keyboard_focus_within()
            && this.is_effectively_enabled()
            && e.delta().y != 0.0
        {
            this.step_value(this.small_change() * e.delta().y.signum());
            e.set_handled(true);
        }
    }
}

impl TemplatedControlImpl for NumberBox {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        Self::parent_on_apply_template(this, e);
        let subscriptions = this.subscriptions.replace(Vec::new());
        for subscription in subscriptions {
            subscription.dispose();
        }
        let old = this.parts.replace(Parts::default());
        if let Some(popup) = old.popup {
            popup.set_is_open(false);
        }
        let scope = e.name_scope();
        let text = scope.find_as::<TextBox>("PART_TextBox");
        let up = scope.find_as::<RepeatButton>("PART_SpinUp");
        let down = scope.find_as::<RepeatButton>("PART_SpinDown");
        let popup_up = scope.find_as::<RepeatButton>("PART_PopupSpinUp");
        let popup_down = scope.find_as::<RepeatButton>("PART_PopupSpinDown");
        *this.parts.borrow_mut() = Parts {
            text: text.clone(),
            up: up.clone(),
            down: down.clone(),
            popup_up: popup_up.clone(),
            popup_down: popup_down.clone(),
            popup: scope.find_as::<Popup>("PART_SpinPopup"),
            header: scope.find_as::<ContentPresenter>("PART_HeaderPresenter"),
            description: scope.find_as::<ContentPresenter>("PART_DescriptionPresenter"),
        };
        if let Some(text) = text {
            let weak = this.to_ref().downgrade();
            let weak_text = text.downgrade();
            let subscription = text.property_changed(move |change| {
                if change.property() == TextBox::text_property().as_property() {
                    if let (Some(owner), Some(text)) = (weak.upgrade(), weak_text.upgrade()) {
                        if !owner.updating_text.get() {
                            owner.updating_text.set(true);
                            let _reset = Reset(&owner.updating_text);
                            owner.set_current_value(
                                Self::text_property(),
                                text.text().unwrap_or_default(),
                            );
                        }
                    }
                }
            });
            this.subscriptions.borrow_mut().push(subscription);
        }
        for (button, sign) in [(up, 1.0), (down, -1.0), (popup_up, 1.0), (popup_down, -1.0)] {
            if let Some(button) = button {
                let weak = this.to_ref().downgrade();
                let token = button.click(move |_, _| {
                    if let Some(owner) = weak.upgrade() {
                        owner.step_value(sign * owner.small_change());
                    }
                });
                let weak_button = button.downgrade();
                this.subscriptions
                    .borrow_mut()
                    .push(Disposable::create(move || {
                        if let Some(button) = weak_button.upgrade() {
                            button.remove_handler(Button::click_event(), token);
                        }
                    }));
            }
        }
        this.update_text_from_value();
        this.update_parts();
    }
}

ferro_properties! { impl NumberBox {
    pub fn minimum_property() -> StyledProperty<f64> { FerroProperty::register::<NumberBox, _>("Minimum", -f64::MAX) }
    pub fn maximum_property() -> StyledProperty<f64> { FerroProperty::register::<NumberBox, _>("Maximum", f64::MAX) }
    pub fn value_property() -> StyledProperty<f64> { FerroProperty::register::<NumberBox, _>("Value", f64::NAN) }
    pub fn small_change_property() -> StyledProperty<f64> { FerroProperty::register::<NumberBox, _>("SmallChange", 1.0) }
    pub fn large_change_property() -> StyledProperty<f64> { FerroProperty::register::<NumberBox, _>("LargeChange", 10.0) }
    pub fn text_property() -> StyledProperty<String> { FerroProperty::register::<NumberBox, _>("Text", String::new()) }
    pub fn header_property() -> StyledProperty<Option<BoxedValue>> { FerroProperty::register::<NumberBox, _>("Header", None) }
    pub fn header_template_property() -> StyledProperty<Option<Rc<dyn IDataTemplate>>> { FerroProperty::register::<NumberBox, _>("HeaderTemplate", None) }
    pub fn description_property() -> StyledProperty<Option<BoxedValue>> { FerroProperty::register::<NumberBox, _>("Description", None) }
    pub fn placeholder_text_property() -> StyledProperty<String> { FerroProperty::register::<NumberBox, _>("PlaceholderText", String::new()) }
    pub fn text_alignment_property() -> StyledProperty<TextAlignment> { FerroProperty::register::<NumberBox, _>("TextAlignment", TextAlignment::Left) }
    pub fn validation_mode_property() -> StyledProperty<NumberBoxValidationMode> { FerroProperty::register::<NumberBox, _>("ValidationMode", NumberBoxValidationMode::InvalidInputOverwritten) }
    pub fn spin_button_placement_mode_property() -> StyledProperty<NumberBoxSpinButtonPlacementMode> { FerroProperty::register::<NumberBox, _>("SpinButtonPlacementMode", NumberBoxSpinButtonPlacementMode::Hidden) }
    pub fn is_wrap_enabled_property() -> StyledProperty<bool> { FerroProperty::register::<NumberBox, _>("IsWrapEnabled", false) }
    pub fn accepts_expression_property() -> StyledProperty<bool> { FerroProperty::register::<NumberBox, _>("AcceptsExpression", false) }
    pub fn number_formatter_property() -> StyledProperty<Rc<dyn INumberBoxNumberFormatter>> {
        FerroProperty::register::<NumberBox, _>("NumberFormatter", Rc::new(InvariantNumberBoxNumberFormatter) as Rc<dyn INumberBoxNumberFormatter>)
    }
} }

impl NumberBox {
    fn static_constructor() {
        let template = FuncControlTemplate::for_type::<NumberBox>(|_, scope| {
            let root = StackPanel::new();
            root.set_spacing(4.0);
            let header = ContentPresenter::new();
            header.set_name(Some("PART_HeaderPresenter".into()));
            root.children()
                .add(header.register_in_name_scope(&**scope).upcast::<Control>());
            let row = Grid::new();
            row.set_column_definitions("*,Auto,Auto".parse().expect("constant grid definition"));
            let text = TextBox::new();
            text.set_name(Some("PART_TextBox".into()));
            row.children().add(
                text.clone()
                    .register_in_name_scope(&**scope)
                    .upcast::<Control>(),
            );
            let make_button = |name: &str, label: &str| {
                let button = RepeatButton::new();
                button.set_name(Some(name.into()));
                button.set_content(Some(Rc::new(label.to_owned())));
                button.set_focusable(false);
                AutomationProperties::set_name(
                    &button,
                    Some(if label == "+" {
                        "Increase value"
                    } else {
                        "Decrease value"
                    }),
                );
                button.register_in_name_scope(&**scope)
            };
            let down = make_button("PART_SpinDown", "−");
            Grid::set_column(&down, 1);
            let up = make_button("PART_SpinUp", "+");
            Grid::set_column(&up, 2);
            row.children().add(down.upcast::<Control>());
            row.children().add(up.upcast::<Control>());
            let popup_buttons = StackPanel::new();
            popup_buttons.set_orientation(Orientation::Horizontal);
            popup_buttons
                .children()
                .add(make_button("PART_PopupSpinDown", "−").upcast::<Control>());
            popup_buttons
                .children()
                .add(make_button("PART_PopupSpinUp", "+").upcast::<Control>());
            let popup = Popup::new();
            popup.set_name(Some("PART_SpinPopup".into()));
            popup.set_child(popup_buttons);
            popup.set_placement_target(text);
            popup.set_placement(PlacementMode::RightEdgeAlignedTop);
            row.children()
                .add(popup.register_in_name_scope(&**scope).upcast::<Control>());
            root.children().add(row.upcast::<Control>());
            let description = ContentPresenter::new();
            description.set_name(Some("PART_DescriptionPresenter".into()));
            root.children().add(
                description
                    .register_in_name_scope(&**scope)
                    .upcast::<Control>(),
            );
            root.upcast()
        });
        TemplatedControl::template_property().override_default_value::<NumberBox>(Some(template));
    }

    pub fn construct() -> Self {
        Self {
            base: TemplatedControl::construct(),
            updating_value: Cell::new(false),
            updating_text: Cell::new(false),
            parts: RefCell::new(Parts::default()),
            subscriptions: RefCell::new(Vec::new()),
            value_changed: HandlerList::new(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn minimum(&self) -> f64 {
        self.get_value(Self::minimum_property())
    }
    pub fn set_minimum(&self, value: f64) {
        self.set_value(Self::minimum_property(), value);
    }
    pub fn maximum(&self) -> f64 {
        self.get_value(Self::maximum_property())
    }
    pub fn set_maximum(&self, value: f64) {
        self.set_value(Self::maximum_property(), value);
    }
    pub fn value(&self) -> f64 {
        self.get_value(Self::value_property())
    }
    pub fn set_numeric_value(&self, value: f64) {
        self.set_value(Self::value_property(), value);
    }
    pub fn small_change(&self) -> f64 {
        self.get_value(Self::small_change_property())
    }
    pub fn set_small_change(&self, value: f64) {
        self.set_value(Self::small_change_property(), value);
    }
    pub fn large_change(&self) -> f64 {
        self.get_value(Self::large_change_property())
    }
    pub fn set_large_change(&self, value: f64) {
        self.set_value(Self::large_change_property(), value);
    }
    pub fn text(&self) -> String {
        self.get_value(Self::text_property())
    }
    pub fn set_text(&self, value: &str) {
        self.set_value(Self::text_property(), value.to_owned());
    }
    pub fn header(&self) -> Option<BoxedValue> {
        self.get_value(Self::header_property())
    }
    pub fn set_header(&self, value: Option<BoxedValue>) {
        self.set_value(Self::header_property(), value);
    }
    pub fn header_template(&self) -> Option<Rc<dyn IDataTemplate>> {
        self.get_value(Self::header_template_property())
    }
    pub fn set_header_template(&self, value: Option<Rc<dyn IDataTemplate>>) {
        self.set_value(Self::header_template_property(), value);
    }
    pub fn description(&self) -> Option<BoxedValue> {
        self.get_value(Self::description_property())
    }
    pub fn set_description(&self, value: Option<BoxedValue>) {
        self.set_value(Self::description_property(), value);
    }
    pub fn placeholder_text(&self) -> String {
        self.get_value(Self::placeholder_text_property())
    }
    pub fn set_placeholder_text(&self, value: &str) {
        self.set_value(Self::placeholder_text_property(), value.to_owned());
    }
    pub fn text_alignment(&self) -> TextAlignment {
        self.get_value(Self::text_alignment_property())
    }
    pub fn set_text_alignment(&self, value: TextAlignment) {
        self.set_value(Self::text_alignment_property(), value);
    }
    pub fn validation_mode(&self) -> NumberBoxValidationMode {
        self.get_value(Self::validation_mode_property())
    }
    pub fn set_validation_mode(&self, value: NumberBoxValidationMode) {
        self.set_value(Self::validation_mode_property(), value);
    }
    pub fn spin_button_placement_mode(&self) -> NumberBoxSpinButtonPlacementMode {
        self.get_value(Self::spin_button_placement_mode_property())
    }
    pub fn set_spin_button_placement_mode(&self, value: NumberBoxSpinButtonPlacementMode) {
        self.set_value(Self::spin_button_placement_mode_property(), value);
    }
    pub fn is_wrap_enabled(&self) -> bool {
        self.get_value(Self::is_wrap_enabled_property())
    }
    pub fn set_is_wrap_enabled(&self, value: bool) {
        self.set_value(Self::is_wrap_enabled_property(), value);
    }
    pub fn accepts_expression(&self) -> bool {
        self.get_value(Self::accepts_expression_property())
    }
    pub fn set_accepts_expression(&self, value: bool) {
        self.set_value(Self::accepts_expression_property(), value);
    }
    pub fn number_formatter(&self) -> Rc<dyn INumberBoxNumberFormatter> {
        self.get_value(Self::number_formatter_property())
    }
    pub fn set_number_formatter(&self, value: Rc<dyn INumberBoxNumberFormatter>) {
        self.set_value(Self::number_formatter_property(), value);
    }

    pub fn value_changed(
        &self,
        handler: impl Fn(&NumberBoxValueChangedEventArgs) + 'static,
    ) -> Rc<dyn IDisposable> {
        let token = self.value_changed.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(owner) = weak.upgrade() {
                owner.value_changed.remove(token);
            }
        })
    }

    /// Commits a text edit. Invalid text remains visible only when validation is disabled.
    pub fn commit_input(&self) -> bool {
        let text = self.text();
        let text = text.trim_matches([' ', '\t', '\r', '\n', '\u{000B}', '\u{000C}']);
        if text.is_empty() {
            self.set_current_value(Self::value_property(), f64::NAN);
            return true;
        }
        let formatter = self.number_formatter();
        let value = if self.accepts_expression() {
            evaluate_number_expression(text, |number| formatter.parse_double(number)).ok()
        } else {
            formatter.parse_double(text)
        };
        match value {
            Some(value) => {
                self.set_current_value(Self::value_property(), value);
                self.update_text_from_value();
                true
            }
            None => {
                if self.validation_mode() == NumberBoxValidationMode::InvalidInputOverwritten {
                    self.update_text_from_value();
                }
                false
            }
        }
    }

    /// Applies a small/large step after committing the in-progress edit.
    pub fn step_value(&self, change: f64) {
        self.commit_input();
        if self.value().is_nan() {
            return;
        }
        let mut value = self.value() + change;
        if self.is_wrap_enabled() {
            if value > self.maximum() {
                value = self.minimum();
            } else if value < self.minimum() {
                value = self.maximum();
            }
        }
        self.set_current_value(Self::value_property(), value);
        let text = self.parts.borrow().text.clone();
        if let Some(text) = text {
            text.set_caret_index(
                text.text()
                    .map_or(0, |text| text.encode_utf16().count() as i32),
            );
        }
    }

    fn coerce_numeric_value(&self) {
        let value = self.value();
        if !value.is_nan()
            && self.validation_mode() == NumberBoxValidationMode::InvalidInputOverwritten
        {
            if value > self.maximum() {
                self.set_current_value(Self::value_property(), self.maximum());
            } else if value < self.minimum() {
                self.set_current_value(Self::value_property(), self.minimum());
            }
        }
    }

    fn on_value_changed(&self, old_value: f64) {
        if self.updating_value.replace(true) {
            return;
        }
        let _reset = Reset(&self.updating_value);
        self.coerce_numeric_value();
        let new_value = self.value();
        if old_value != new_value && !(old_value.is_nan() && new_value.is_nan()) {
            let args = NumberBoxValueChangedEventArgs {
                old_value,
                new_value,
            };
            for (_, handler) in self.value_changed.snapshot().iter() {
                handler(&args);
            }
            if let Some(peer) = self
                .get_automation_peer()
                .and_then(|peer| peer.cast::<crate::automation::peers::NumberBoxAutomationPeer>())
            {
                peer.raise_value_changed(old_value, new_value);
            }
        }
        self.update_text_from_value();
        self.update_parts();
    }

    fn set_editor_text(&self, value: &str) {
        let text = self.parts.borrow().text.clone();
        if let Some(text) = text {
            text.set_text(Some(value));
        }
    }

    fn update_text_from_value(&self) {
        if self.updating_text.replace(true) {
            return;
        }
        let _reset = Reset(&self.updating_text);
        let value = self.value();
        let text = if value.is_nan() {
            String::new()
        } else {
            self.number_formatter()
                .format_double(round_for_display(value))
        };
        self.set_current_value(Self::text_property(), text.clone());
        self.set_editor_text(&text);
    }

    fn update_parts(&self) {
        let (text, buttons, popup, header, description) = {
            let parts = self.parts.borrow();
            (
                parts.text.clone(),
                [
                    parts.up.clone(),
                    parts.down.clone(),
                    parts.popup_up.clone(),
                    parts.popup_down.clone(),
                ],
                parts.popup.clone(),
                parts.header.clone(),
                parts.description.clone(),
            )
        };
        if let Some(text) = text {
            text.set_placeholder_text(Some(&self.placeholder_text()));
            text.set_text_alignment(self.text_alignment());
            let name = AutomationProperties::get_name(self);
            if !name.as_deref().unwrap_or_default().is_empty() {
                AutomationProperties::set_name(&text, name.as_deref());
            }
        }
        let mode = self.spin_button_placement_mode();
        let value = self.value();
        let unlimited =
            self.is_wrap_enabled() || self.validation_mode() == NumberBoxValidationMode::Disabled;
        for (index, button) in buttons.into_iter().enumerate() {
            if let Some(button) = button {
                button
                    .set_is_visible(index >= 2 || mode == NumberBoxSpinButtonPlacementMode::Inline);
                button.set_is_enabled(
                    !value.is_nan()
                        && (unlimited
                            || if index % 2 == 0 {
                                value < self.maximum()
                            } else {
                                value > self.minimum()
                            }),
                );
            }
        }
        if mode != NumberBoxSpinButtonPlacementMode::Compact {
            if let Some(popup) = popup {
                popup.set_is_open(false);
            }
        }
        if let Some(header) = header {
            header.set_content(self.header());
            header.set_content_template(self.header_template());
            header.set_is_visible(self.header().is_some());
        }
        if let Some(description) = description {
            description.set_content(self.description());
            description.set_is_visible(self.description().is_some());
        }
    }
}

pub(crate) fn register_markup() {
    MarkupType::register_all(TYPES);
}

const TYPES: &[&MarkupType] = &[
    <NumberBoxSpinButtonPlacementMode as MarkupTyped>::MARKUP,
    <NumberBoxValidationMode as MarkupTyped>::MARKUP,
    <NumberBoxValueChangedEventArgs as MarkupTyped>::MARKUP,
];

// NumberBox.cpp configures a ten-significant-digit display rounder. WinRT's
// default RoundHalfUp resolves an exact tie toward positive infinity. Work in
// decimal scientific notation so scaling never overflows or underflows for a
// finite double. Rounding is for display only; the stored Value is unchanged.
fn round_for_display(value: f64) -> f64 {
    if !value.is_finite() || value == 0.0 {
        return value;
    }
    let scientific = format!("{:.16e}", value.abs());
    let (mantissa, exponent) = scientific.split_once('e').expect("scientific double");
    let exponent: i32 = exponent.parse().expect("scientific exponent");
    let digits: Vec<u8> = mantissa
        .bytes()
        .filter(|byte| byte.is_ascii_digit())
        .collect();
    let mut retained: u64 = digits[..10]
        .iter()
        .fold(0, |number, digit| number * 10 + (digit - b'0') as u64);
    let next = digits[10];
    let greater_than_half =
        next > b'5' || next == b'5' && digits[11..].iter().any(|digit| *digit != b'0');
    if greater_than_half || next == b'5' && value.is_sign_positive() {
        retained += 1;
    }
    format!(
        "{}{retained}e{}",
        if value.is_sign_negative() { "-" } else { "" },
        exponent - 9
    )
    .parse()
    .expect("rounded decimal double")
}
