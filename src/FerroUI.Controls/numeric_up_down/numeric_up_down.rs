use super::NumericUpDownValueChangedEventArgs;
use crate::primitives::{TemplateAppliedEventArgs, TemplatedControl, TemplatedControlImpl};
use crate::{
    ButtonSpinner, ContentControl, ControlImpl, Location, SpinDirection, SpinEventArgs, Spinner, TextBox,
    ValidSpinDirections,
};
use ferroui_base::data::converters::IValueConverter;
use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::data::BindingMode;
use ferroui_base::input::{
    FocusChangedEventArgs, InputElement, InputElementImpl, InputElementImplExt, Key, KeyEventArgs, KeyboardNavigation,
    KeyboardNavigationMode, PointerPressedEventArgs,
};
use ferroui_base::interactivity::{
    Interactive, InteractiveImpl, RoutedEvent, RoutedEventHandlerToken, RoutingStrategies,
};
use ferroui_base::layout::{HorizontalAlignment, LayoutableImpl, VerticalAlignment};
use ferroui_base::media::text_formatting::unicode::{Codepoint, GeneralCategory};
use ferroui_base::media::{IBrush, TextAlignment};
use ferroui_base::reactive::{Disposable, IDisposable, ObservableExt};
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::utilities::number_format::format_decimal_composite;
use ferroui_base::utilities::{CultureInfo, Decimal, HandlerList, NumberFormatInfo, NumberParseError, NumberStyles};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, ferro_routed_event, instantiate,
    AttachedProperty, BoxedValue, FerroObject, FerroObjectExtensions, FerroObjectImpl, FerroObjectImplExt,
    FerroProperty, Ref, StyledElementImpl, StyledProperty, StyledPropertyOptions, VisualImpl,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Resets a flag when dropped.
struct Reset<'a>(&'a Cell<bool>);

impl Drop for Reset<'_> {
    fn drop(&mut self) {
        self.0.set(false);
    }
}

/// The failure of a conversion between the text and the value: what the
/// reference reports by throwing.
struct ConversionError(String);

impl ConversionError {
    fn format() -> Self {
        ConversionError("Input string was not in a correct format.".to_string())
    }
}

/// A template part and the handler attached to one of its events.
type Part<T> = (Ref<T>, RoutedEventHandlerToken);

/// Control that represents a text box with button spinners that allow
/// incrementing and decrementing numeric values.
#[repr(C)]
pub struct NumericUpDown {
    base: TemplatedControl,
    text_box_text_changed_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    internal_value_set: Cell<bool>,
    is_syncing_text_and_value_properties: Cell<bool>,
    is_text_changed_from_ui: Cell<bool>,
    is_focused: Cell<bool>,
    // The spinner template part and its spin handler.
    spinner: RefCell<Option<Part<Spinner>>>,
    // The text box template part and its pointer pressed handler.
    text_box: RefCell<Option<Part<TextBox>>>,
    spinned: HandlerList<dyn Fn(&SpinEventArgs)>,
}

ferro_class! {
    NumericUpDown: TemplatedControl, virtuals NumericUpDownImpl: TemplatedControlImpl {
        /// Called when the `NumberFormat` property value changed.
        fn on_number_format_changed(
            this,
            old_value: Option<Rc<NumberFormatInfo>>,
            new_value: Option<Rc<NumberFormatInfo>>
        );
        /// Called when the `FormatString` property value changed.
        fn on_format_string_changed(this, old_value: &str, new_value: &str);
        /// Called when the `Increment` property value changed.
        fn on_increment_changed(this, old_value: Decimal, new_value: Decimal);
        /// Called when the `IsReadOnly` property value changed.
        fn on_is_read_only_changed(this, old_value: bool, new_value: bool);
        /// Called when the `Maximum` property value changed.
        fn on_maximum_changed(this, old_value: Decimal, new_value: Decimal);
        /// Called when the `Minimum` property value changed.
        fn on_minimum_changed(this, old_value: Decimal, new_value: Decimal);
        /// Called when the `Text` property value changed.
        fn on_text_changed(this, old_value: Option<&str>, new_value: Option<&str>);
        /// Called when the `TextConverter` property value changed.
        fn on_text_converter_changed(
            this,
            old_value: Option<Rc<dyn IValueConverter>>,
            new_value: Option<Rc<dyn IValueConverter>>
        );
        /// Called when the `Value` property value changed.
        fn on_value_changed(this, old_value: Option<Decimal>, new_value: Option<Decimal>);
        /// Called when the `Increment` property has to be coerced.
        fn on_coerce_increment(this, base_value: Decimal) -> Decimal;
        /// Called when the `Maximum` property has to be coerced.
        fn on_coerce_maximum(this, base_value: Decimal) -> Decimal;
        /// Called when the `Minimum` property has to be coerced.
        fn on_coerce_minimum(this, base_value: Decimal) -> Decimal;
        /// Called when the `Value` property has to be coerced.
        fn on_coerce_value(this, base_value: Option<Decimal>) -> Option<Decimal>;
        /// Raises the `Spinned` event when spinning is initiated by the
        /// end-user.
        fn on_spin(this, e: &SpinEventArgs);
        /// Raises the `ValueChanged` event.
        fn raise_value_changed_event(this, old_value: Option<Decimal>, new_value: Option<Decimal>);
    }
}

ferro_class_info!(NumericUpDown {
    new: NumericUpDown::new,
    markup: {
        attributes: [
            TemplatePart("PART_Spinner", type(Ref<Spinner>)),
            TemplatePart("PART_TextBox", type(Ref<TextBox>), IsRequired = true),
        ],
    },
});

ferro_impl_classes!(NumericUpDown: StyledElementImpl, VisualImpl, LayoutableImpl, InteractiveImpl);

impl ControlImpl for NumericUpDown {
    fn on_create_automation_peer(this: &Self) -> Ref<crate::automation::peers::AutomationPeer> {
        crate::automation::peers::NumericUpDownAutomationPeer::new(this).upcast()
    }
}

impl FerroObjectImpl for NumericUpDown {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);

        let weak = this.to_ref().downgrade();
        this.initialized(move || {
            if let Some(this) = weak.upgrade() {
                if !this.internal_value_set.get() && this.is_initialized() {
                    this.sync_text_and_value_properties(false, None, true);
                }

                this.set_valid_spin_direction();
            }
        });
    }
}

impl InputElementImpl for NumericUpDown {
    fn on_got_focus(this: &Self, e: &FocusChangedEventArgs) {
        Self::parent_on_got_focus(this, e);
        this.focus_changed(this.is_keyboard_focus_within());
    }

    fn on_lost_focus(this: &Self, e: &FocusChangedEventArgs) {
        this.commit_input(true);
        Self::parent_on_lost_focus(this, e);
        this.focus_changed(this.is_keyboard_focus_within());
    }

    fn on_key_down(this: &Self, e: &KeyEventArgs) {
        if e.key == Key::Enter {
            let commit_success = this.commit_input(false);
            e.set_handled(!commit_success);
        }
    }
}

impl TemplatedControlImpl for NumericUpDown {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        if let Some((text_box, token)) = this.text_box.take() {
            text_box.remove_handler(InputElement::pointer_pressed_event(), token);
            if let Some(subscription) = this.text_box_text_changed_subscription.take() {
                subscription.dispose();
            }
        }

        if let Some(text_box) = e.name_scope().find_as::<TextBox>("PART_TextBox") {
            text_box.set_text(this.text().as_deref());

            let tab_index = InputElement::tab_index_property().as_property();
            text_box.bind_indexer(&tab_index.bind(), &this.indexer(&tab_index.bind()));

            let weak = this.to_ref().downgrade();
            let token = text_box.add_handler(InputElement::pointer_pressed_event(), move |_, e| {
                if let Some(this) = weak.upgrade() {
                    this.text_box_on_pointer_pressed(e);
                }
            });
            *this.text_box.borrow_mut() = Some((text_box.clone(), token));

            let weak = this.to_ref().downgrade();
            let object: &FerroObject = &text_box;
            let subscription = FerroObjectExtensions::get_observable(object, TextBox::text_property())
                .subscribe_fn(move |_| {
                    if let Some(this) = weak.upgrade() {
                        this.text_box_on_text_changed();
                    }
                });
            *this.text_box_text_changed_subscription.borrow_mut() = Some(subscription);
        }

        if let Some((spinner, token)) = this.spinner.take() {
            spinner.remove_handler(Spinner::spin_event(), token);
        }

        if let Some(spinner) = e.name_scope().find_as::<Spinner>("PART_Spinner") {
            let weak = this.to_ref().downgrade();
            let token = spinner.spin(move |_, e| {
                if let Some(this) = weak.upgrade() {
                    this.on_spinner_spin(e);
                }
            });
            *this.spinner.borrow_mut() = Some((spinner, token));
        }

        this.set_valid_spin_direction();
    }
}

impl NumericUpDownImpl for NumericUpDown {
    fn on_number_format_changed(
        this: &Self,
        _old_value: Option<Rc<NumberFormatInfo>>,
        _new_value: Option<Rc<NumberFormatInfo>>,
    ) {
        if this.is_initialized() {
            this.sync_text_and_value_properties(false, None, true);
        }
    }

    fn on_format_string_changed(this: &Self, _old_value: &str, _new_value: &str) {
        if this.is_initialized() {
            this.sync_text_and_value_properties(false, None, true);
        }
    }

    fn on_increment_changed(this: &Self, _old_value: Decimal, _new_value: Decimal) {
        if this.is_initialized() {
            this.set_valid_spin_direction();
        }
    }

    fn on_is_read_only_changed(this: &Self, _old_value: bool, _new_value: bool) {
        this.set_valid_spin_direction();
    }

    fn on_maximum_changed(this: &Self, _old_value: Decimal, _new_value: Decimal) {
        if this.is_initialized() {
            this.set_valid_spin_direction();
        }
        if this.clip_value_to_min_max() {
            if let Some(value) = this.value() {
                this.set_current_value(Self::value_property(), Some(clamp(value, this.minimum(), this.maximum())));
            }
        }
    }

    fn on_minimum_changed(this: &Self, _old_value: Decimal, _new_value: Decimal) {
        if this.is_initialized() {
            this.set_valid_spin_direction();
        }
        if this.clip_value_to_min_max() {
            if let Some(value) = this.value() {
                this.set_current_value(Self::value_property(), Some(clamp(value, this.minimum(), this.maximum())));
            }
        }
    }

    fn on_text_changed(this: &Self, _old_value: Option<&str>, _new_value: Option<&str>) {
        if this.is_initialized() {
            this.sync_text_and_value_properties(true, this.text().as_deref(), false);
        }
    }

    fn on_text_converter_changed(
        this: &Self,
        _old_value: Option<Rc<dyn IValueConverter>>,
        _new_value: Option<Rc<dyn IValueConverter>>,
    ) {
        if this.is_initialized() {
            this.sync_text_and_value_properties(false, None, true);
        }
    }

    fn on_value_changed(this: &Self, old_value: Option<Decimal>, new_value: Option<Decimal>) {
        if !this.internal_value_set.get() && this.is_initialized() {
            this.sync_text_and_value_properties(false, None, true);
        }

        this.set_valid_spin_direction();

        this.raise_value_changed_event(old_value, new_value);
    }

    fn on_coerce_increment(_this: &Self, base_value: Decimal) -> Decimal {
        base_value
    }

    fn on_coerce_maximum(this: &Self, base_value: Decimal) -> Decimal {
        base_value.max(this.minimum())
    }

    fn on_coerce_minimum(this: &Self, base_value: Decimal) -> Decimal {
        base_value.min(this.maximum())
    }

    fn on_coerce_value(_this: &Self, base_value: Option<Decimal>) -> Option<Decimal> {
        base_value
    }

    fn on_spin(this: &Self, e: &SpinEventArgs) {
        if e.direction() == SpinDirection::Increase {
            this.do_increment();
        } else {
            this.do_decrement();
        }

        for (_, handler) in this.spinned.snapshot().iter() {
            handler(e);
        }
    }

    fn raise_value_changed_event(this: &Self, old_value: Option<Decimal>, new_value: Option<Decimal>) {
        let e = NumericUpDownValueChangedEventArgs::new(Some(Self::value_changed_event()), old_value, new_value);
        this.raise_event(&e);
    }
}

ferro_properties! {
    impl NumericUpDown {
        /// Defines the `AllowSpin` property.
        pub fn allow_spin_property() -> StyledProperty<bool> {
            ButtonSpinner::allow_spin_property().add_owner::<NumericUpDown>()
        }

        /// Defines the `ButtonSpinnerLocation` property.
        pub fn button_spinner_location_property() -> StyledProperty<Location> {
            ButtonSpinner::button_spinner_location_property().add_owner::<NumericUpDown>()
        }

        /// Defines the `ShowButtonSpinner` property.
        pub fn show_button_spinner_property() -> StyledProperty<bool> {
            ButtonSpinner::show_button_spinner_property().add_owner::<NumericUpDown>()
        }

        /// Defines the `ClipValueToMinMax` property.
        pub fn clip_value_to_min_max_property() -> StyledProperty<bool> {
            FerroProperty::register::<NumericUpDown, _>("ClipValueToMinMax", false)
        }

        /// Defines the `NumberFormat` property.
        pub fn number_format_property() -> StyledProperty<Option<Rc<NumberFormatInfo>>> {
            FerroProperty::register::<NumericUpDown, _>("NumberFormat", Some(NumberFormatInfo::current_info()))
        }

        /// Defines the `FormatString` property.
        pub fn format_string_property() -> StyledProperty<String> {
            FerroProperty::register::<NumericUpDown, _>("FormatString", String::new())
        }

        /// Defines the `Increment` property.
        pub fn increment_property() -> StyledProperty<Decimal> {
            FerroProperty::register_with::<NumericUpDown, _>(
                "Increment",
                StyledPropertyOptions::new(Decimal::ONE).coerce(|instance, value| {
                    match instance.downcast_ref::<NumericUpDown>() {
                        Some(up_down) => up_down.on_coerce_increment(value),
                        None => value,
                    }
                }),
            )
        }

        /// Defines the `IsReadOnly` property.
        pub fn is_read_only_property() -> StyledProperty<bool> {
            FerroProperty::register::<NumericUpDown, _>("IsReadOnly", false)
        }

        /// Defines the `Maximum` property.
        pub fn maximum_property() -> StyledProperty<Decimal> {
            FerroProperty::register_with::<NumericUpDown, _>(
                "Maximum",
                StyledPropertyOptions::new(Decimal::MAX_VALUE).coerce(|instance, value| {
                    match instance.downcast_ref::<NumericUpDown>() {
                        Some(up_down) => up_down.on_coerce_maximum(value),
                        None => value,
                    }
                }),
            )
        }

        /// Defines the `Minimum` property.
        pub fn minimum_property() -> StyledProperty<Decimal> {
            FerroProperty::register_with::<NumericUpDown, _>(
                "Minimum",
                StyledPropertyOptions::new(Decimal::MIN_VALUE).coerce(|instance, value| {
                    match instance.downcast_ref::<NumericUpDown>() {
                        Some(up_down) => up_down.on_coerce_minimum(value),
                        None => value,
                    }
                }),
            )
        }

        /// Defines the `ParsingNumberStyle` property.
        pub fn parsing_number_style_property() -> StyledProperty<NumberStyles> {
            FerroProperty::register::<NumericUpDown, _>("ParsingNumberStyle", NumberStyles::ANY)
        }

        /// Defines the `Text` property.
        pub fn text_property() -> StyledProperty<Option<String>> {
            FerroProperty::register_with::<NumericUpDown, _>(
                "Text",
                StyledPropertyOptions::new(None)
                    .default_binding_mode(BindingMode::TwoWay)
                    .enable_data_validation(true),
            )
        }

        /// Defines the `TextConverter` property.
        pub fn text_converter_property() -> StyledProperty<Option<Rc<dyn IValueConverter>>> {
            FerroProperty::register_with::<NumericUpDown, _>(
                "TextConverter",
                StyledPropertyOptions::new(None).default_binding_mode(BindingMode::OneWay),
            )
        }

        /// Defines the `Value` property.
        pub fn value_property() -> StyledProperty<Option<Decimal>> {
            FerroProperty::register_with::<NumericUpDown, _>(
                "Value",
                StyledPropertyOptions::new(None)
                    .coerce(|instance, value| {
                        instance
                            .downcast_ref::<NumericUpDown>()
                            .expect("The owner of the Value property is a NumericUpDown.")
                            .on_coerce_value(value)
                    })
                    .default_binding_mode(BindingMode::TwoWay)
                    .enable_data_validation(true),
            )
        }

        /// Defines the `PlaceholderText` property.
        pub fn placeholder_text_property() -> StyledProperty<Option<String>> {
            TextBox::placeholder_text_property().add_owner::<NumericUpDown>()
        }

        /// Defines the `PlaceholderForeground` property.
        pub fn placeholder_foreground_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
            TextBox::placeholder_foreground_property().add_owner::<NumericUpDown>()
        }

        /// Defines the `HorizontalContentAlignment` property.
        pub fn horizontal_content_alignment_property() -> StyledProperty<HorizontalAlignment> {
            ContentControl::horizontal_content_alignment_property().add_owner::<NumericUpDown>()
        }

        /// Defines the `VerticalContentAlignment` property.
        pub fn vertical_content_alignment_property() -> StyledProperty<VerticalAlignment> {
            ContentControl::vertical_content_alignment_property().add_owner::<NumericUpDown>()
        }

        /// Defines the `TextAlignment` property.
        pub fn text_alignment_property() -> AttachedProperty<TextAlignment> {
            TextBox::text_alignment_property().add_owner::<NumericUpDown>()
        }

        /// Defines the `InnerLeftContent` property.
        pub fn inner_left_content_property() -> StyledProperty<Option<BoxedValue>> {
            TextBox::inner_left_content_property().add_owner::<NumericUpDown>()
        }

        /// Defines the `InnerRightContent` property.
        pub fn inner_right_content_property() -> StyledProperty<Option<BoxedValue>> {
            TextBox::inner_right_content_property().add_owner::<NumericUpDown>()
        }
    }
}

impl NumericUpDown {
    /// Defines the `Watermark` property.
    #[deprecated(note = "Use placeholder_text_property instead.")]
    pub fn watermark_property() -> &'static StyledProperty<Option<String>> {
        Self::placeholder_text_property()
    }

    /// Defines the `WatermarkForeground` property.
    #[deprecated(note = "Use placeholder_foreground_property instead.")]
    pub fn watermark_foreground_property() -> &'static StyledProperty<Option<Rc<dyn IBrush>>> {
        Self::placeholder_foreground_property()
    }

    ferro_routed_event!(
        /// Defines the `ValueChanged` event.
        pub fn value_changed_event() -> RoutedEvent<NumericUpDownValueChangedEventArgs> {
            RoutedEvent::register::<NumericUpDown, _>("ValueChanged", RoutingStrategies::BUBBLE)
        }
    );

    /// Initializes the static members of the class.
    fn static_constructor() {
        Self::number_format_property().changed().add_class_handler::<NumericUpDown>(|up_down, e| {
            let (old_value, new_value) = e.get_old_and_new_value::<Option<Rc<NumberFormatInfo>>>();
            up_down.on_number_format_changed(old_value, new_value);
        });
        Self::format_string_property().changed().add_class_handler::<NumericUpDown>(|up_down, e| {
            let (old_value, new_value) = e.get_old_and_new_value::<String>();
            up_down.on_format_string_changed(&old_value, &new_value);
        });
        Self::increment_property().changed().add_class_handler::<NumericUpDown>(|up_down, e| {
            let (old_value, new_value) = e.get_old_and_new_value::<Decimal>();
            up_down.on_increment_changed(old_value, new_value);
        });
        Self::is_read_only_property().changed().add_class_handler::<NumericUpDown>(|up_down, e| {
            let (old_value, new_value) = e.get_old_and_new_value::<bool>();
            up_down.on_is_read_only_changed(old_value, new_value);
        });
        Self::maximum_property().changed().add_class_handler::<NumericUpDown>(|up_down, e| {
            let (old_value, new_value) = e.get_old_and_new_value::<Decimal>();
            up_down.on_maximum_changed(old_value, new_value);
        });
        Self::minimum_property().changed().add_class_handler::<NumericUpDown>(|up_down, e| {
            let (old_value, new_value) = e.get_old_and_new_value::<Decimal>();
            up_down.on_minimum_changed(old_value, new_value);
        });
        Self::text_property().changed().add_class_handler::<NumericUpDown>(|up_down, e| {
            let (old_value, new_value) = e.get_old_and_new_value::<Option<String>>();
            up_down.on_text_changed(old_value.as_deref(), new_value.as_deref());
        });
        Self::text_converter_property().changed().add_class_handler::<NumericUpDown>(|up_down, e| {
            let (old_value, new_value) = e.get_old_and_new_value::<Option<Rc<dyn IValueConverter>>>();
            up_down.on_text_converter_changed(old_value, new_value);
        });
        Self::value_property().changed().add_class_handler::<NumericUpDown>(|up_down, e| {
            let (old_value, new_value) = e.get_old_and_new_value::<Option<Decimal>>();
            up_down.on_value_changed(old_value, new_value);
        });

        InputElement::focusable_property().override_default_value::<NumericUpDown>(true);
        InputElement::is_tab_stop_property().override_default_value::<NumericUpDown>(false);
        KeyboardNavigation::tab_navigation_property()
            .override_default_value::<NumericUpDown>(KeyboardNavigationMode::Local);
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: TemplatedControl::construct(),
            text_box_text_changed_subscription: RefCell::new(None),
            internal_value_set: Cell::new(false),
            is_syncing_text_and_value_properties: Cell::new(false),
            is_text_changed_from_ui: Cell::new(false),
            is_focused: Cell::new(false),
            spinner: RefCell::new(None),
            text_box: RefCell::new(None),
            spinned: HandlerList::new(),
        }
    }

    /// Creates a numeric up-down control.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The spinner template part.
    fn spinner(&self) -> Option<Ref<Spinner>> {
        self.spinner.borrow().as_ref().map(|(spinner, _)| spinner.clone())
    }

    /// The text box template part.
    fn text_box(&self) -> Option<Ref<TextBox>> {
        self.text_box.borrow().as_ref().map(|(text_box, _)| text_box.clone())
    }

    /// Whether increment/decrement operations via the keyboard, the button
    /// spinners or the mouse wheel are possible.
    pub fn allow_spin(&self) -> bool {
        self.get_value(Self::allow_spin_property())
    }

    pub fn set_allow_spin(&self, value: bool) {
        self.set_value(Self::allow_spin_property(), value)
    }

    /// The current location of the button spinner.
    pub fn button_spinner_location(&self) -> Location {
        self.get_value(Self::button_spinner_location_property())
    }

    pub fn set_button_spinner_location(&self, value: Location) {
        self.set_value(Self::button_spinner_location_property(), value)
    }

    /// Whether the spin buttons should be shown.
    pub fn show_button_spinner(&self) -> bool {
        self.get_value(Self::show_button_spinner_property())
    }

    pub fn set_show_button_spinner(&self, value: bool) {
        self.set_value(Self::show_button_spinner_property(), value)
    }

    /// Whether the value should be clipped when the minimum/maximum is
    /// reached.
    pub fn clip_value_to_min_max(&self) -> bool {
        self.get_value(Self::clip_value_to_min_max_property())
    }

    pub fn set_clip_value_to_min_max(&self, value: bool) {
        self.set_value(Self::clip_value_to_min_max_property(), value)
    }

    /// The current number format information.
    pub fn number_format(&self) -> Option<Rc<NumberFormatInfo>> {
        self.get_value(Self::number_format_property())
    }

    pub fn set_number_format(&self, value: Option<Rc<NumberFormatInfo>>) {
        self.set_value(Self::number_format_property(), value)
    }

    /// The display format of the `Value`.
    pub fn format_string(&self) -> String {
        self.get_value(Self::format_string_property())
    }

    pub fn set_format_string(&self, value: &str) {
        self.set_value(Self::format_string_property(), value.to_string())
    }

    /// The amount in which to increment the `Value`.
    pub fn increment(&self) -> Decimal {
        self.get_value(Self::increment_property())
    }

    pub fn set_increment(&self, value: Decimal) {
        self.set_value(Self::increment_property(), value)
    }

    /// Whether the control is read only.
    pub fn is_read_only(&self) -> bool {
        self.get_value(Self::is_read_only_property())
    }

    pub fn set_is_read_only(&self, value: bool) {
        self.set_value(Self::is_read_only_property(), value)
    }

    /// The maximum allowed value.
    pub fn maximum(&self) -> Decimal {
        self.get_value(Self::maximum_property())
    }

    pub fn set_maximum(&self, value: Decimal) {
        self.set_value(Self::maximum_property(), value)
    }

    /// The minimum allowed value.
    pub fn minimum(&self) -> Decimal {
        self.get_value(Self::minimum_property())
    }

    pub fn set_minimum(&self, value: Decimal) {
        self.set_value(Self::minimum_property(), value)
    }

    /// The parsing style (`ALLOW_LEADING_WHITE`, `FLOAT`, ...). By default,
    /// `ANY`. Note that the hexadecimal styles do not work with decimals:
    /// for a hexadecimal display, use the `TextConverter`.
    pub fn parsing_number_style(&self) -> NumberStyles {
        self.get_value(Self::parsing_number_style_property())
    }

    pub fn set_parsing_number_style(&self, value: NumberStyles) {
        self.set_value(Self::parsing_number_style_property(), value)
    }

    /// The formatted string representation of the value.
    pub fn text(&self) -> Option<String> {
        self.get_value(Self::text_property())
    }

    pub fn set_text(&self, value: Option<&str>) {
        self.set_value(Self::text_property(), value.map(str::to_string))
    }

    /// The custom bidirectional text-value converter. A converter overrides
    /// the `ParsingNumberStyle`, providing finer control over the string
    /// representation of the underlying value.
    pub fn text_converter(&self) -> Option<Rc<dyn IValueConverter>> {
        self.get_value(Self::text_converter_property())
    }

    pub fn set_text_converter(&self, value: Option<Rc<dyn IValueConverter>>) {
        self.set_value(Self::text_converter_property(), value)
    }

    /// The value.
    pub fn value(&self) -> Option<Decimal> {
        self.get_value(Self::value_property())
    }

    pub fn set_numeric_value(&self, value: Option<Decimal>) {
        self.set_value(Self::value_property(), value)
    }

    /// The text to use as a placeholder if the `Value` is null.
    pub fn placeholder_text(&self) -> Option<String> {
        self.get_value(Self::placeholder_text_property())
    }

    pub fn set_placeholder_text(&self, value: Option<&str>) {
        self.set_value(Self::placeholder_text_property(), value.map(str::to_string))
    }

    /// The text to use as a placeholder if the `Value` is null.
    #[deprecated(note = "Use placeholder_text instead.")]
    pub fn watermark(&self) -> Option<String> {
        self.placeholder_text()
    }

    #[deprecated(note = "Use set_placeholder_text instead.")]
    pub fn set_watermark(&self, value: Option<&str>) {
        self.set_placeholder_text(value)
    }

    /// The brush used for the foreground color of the placeholder text.
    pub fn placeholder_foreground(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::placeholder_foreground_property())
    }

    pub fn set_placeholder_foreground(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::placeholder_foreground_property(), value)
    }

    /// The brush used for the foreground color of the placeholder text.
    #[deprecated(note = "Use placeholder_foreground instead.")]
    pub fn watermark_foreground(&self) -> Option<Rc<dyn IBrush>> {
        self.placeholder_foreground()
    }

    #[deprecated(note = "Use set_placeholder_foreground instead.")]
    pub fn set_watermark_foreground(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_placeholder_foreground(value)
    }

    /// The horizontal alignment of the content within the control.
    pub fn horizontal_content_alignment(&self) -> HorizontalAlignment {
        self.get_value(Self::horizontal_content_alignment_property())
    }

    pub fn set_horizontal_content_alignment(&self, value: HorizontalAlignment) {
        self.set_value(Self::horizontal_content_alignment_property(), value)
    }

    /// The vertical alignment of the content within the control.
    pub fn vertical_content_alignment(&self) -> VerticalAlignment {
        self.get_value(Self::vertical_content_alignment_property())
    }

    pub fn set_vertical_content_alignment(&self, value: VerticalAlignment) {
        self.set_value(Self::vertical_content_alignment_property(), value)
    }

    /// The text alignment of the control.
    pub fn text_alignment(&self) -> TextAlignment {
        self.get_value(Self::text_alignment_property())
    }

    pub fn set_text_alignment(&self, value: TextAlignment) {
        self.set_value(Self::text_alignment_property(), value)
    }

    /// Custom content that is positioned on the left side of the text
    /// layout box.
    pub fn inner_left_content(&self) -> Option<BoxedValue> {
        self.get_value(Self::inner_left_content_property())
    }

    pub fn set_inner_left_content(&self, value: Option<BoxedValue>) {
        self.set_value(Self::inner_left_content_property(), value)
    }

    /// Custom content that is positioned on the right side of the text
    /// layout box.
    pub fn inner_right_content(&self) -> Option<BoxedValue> {
        self.get_value(Self::inner_right_content_property())
    }

    pub fn set_inner_right_content(&self, value: Option<BoxedValue>) {
        self.set_value(Self::inner_right_content_property(), value)
    }

    /// Occurs when spinning is initiated by the end-user.
    pub fn spinned(&self, handler: impl Fn(&SpinEventArgs) + 'static) -> Rc<dyn IDisposable> {
        let token = self.spinned.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.spinned.remove(token);
            }
        })
    }

    /// Raised when the `Value` changes.
    pub fn value_changed(
        &self,
        handler: impl Fn(&Interactive, &NumericUpDownValueChangedEventArgs) + 'static,
    ) -> RoutedEventHandlerToken {
        self.add_handler(Self::value_changed_event(), handler)
    }

    /// Converts the formatted text to a value.
    fn convert_text_to_value(&self, text: Option<&str>) -> Result<Option<Decimal>, ConversionError> {
        let Some(text) = text.filter(|text| !text.is_empty()) else {
            return Ok(None);
        };

        // Since the conversion from the value to text using a format string
        // may not be parsable, we verify that the already existing text is
        // not the exact same value.
        let current_value_text = self.convert_value_to_text()?;
        if current_value_text.as_deref() == Some(text) {
            return Ok(self.value());
        }

        let result = self.convert_text_to_value_core(current_value_text.as_deref(), text)?;

        if self.clip_value_to_min_max() {
            if let Some(result) = result {
                return Ok(Some(clamp(result, self.minimum(), self.maximum())));
            }
        }

        self.validate_min_max(result)?;

        Ok(result)
    }

    /// Converts the value to formatted text.
    fn convert_value_to_text(&self) -> Result<Option<String>, ConversionError> {
        let value = self.value();

        if let Some(converter) = self.text_converter() {
            let boxed = value.map(|value| Rc::new(value) as BoxedValue);
            let text = converter
                .convert_back(boxed.as_ref(), ValueType::of::<String>(), None, &CultureInfo::current_culture())
                .map_err(|error| ConversionError(error.to_string()))?;
            return Ok(text.map(|text| ValueTypes::to_display_string(Some(&text))));
        }

        let number_format = NumberFormatInfo::get_instance(self.number_format().as_ref());
        let format_string = self.format_string();

        // Manage a format string of type "{0:N2} °".
        if format_string.contains("{0") {
            return format_decimal_composite(&format_string, &number_format, value)
                .map(Some)
                .map_err(|error| ConversionError(error.to_string()));
        }

        match value {
            Some(value) => value
                .to_string_with(&format_string, Some(&number_format))
                .map(Some)
                .map_err(|error| ConversionError(error.to_string())),
            None => Ok(None),
        }
    }

    /// Called by `on_spin` when the spin direction is
    /// `SpinDirection::Increase`.
    fn on_increment(&self) {
        let result = match self.value() {
            Some(value) => value.checked_add(self.increment()).unwrap_or(saturated(self.increment())),
            // If the minimum is set we set the value to the minimum on
            // increment, otherwise to 0. It will be clamped to be between
            // the minimum and the maximum later, so we don't need to do it
            // here.
            None if self.is_set(Self::minimum_property().as_property()) => self.minimum(),
            None => Decimal::ZERO,
        };

        self.set_current_value(Self::value_property(), Some(clamp(result, self.minimum(), self.maximum())));
    }

    /// Called by `on_spin` when the spin direction is
    /// `SpinDirection::Decrease`.
    fn on_decrement(&self) {
        let result = match self.value() {
            Some(value) => value.checked_sub(self.increment()).unwrap_or(saturated(-self.increment())),
            // If the maximum is set we set the value to the maximum on
            // decrement, otherwise to 0. It will be clamped to be between
            // the minimum and the maximum later, so we don't need to do it
            // here.
            None if self.is_set(Self::maximum_property().as_property()) => self.maximum(),
            None => Decimal::ZERO,
        };

        self.set_current_value(Self::value_property(), Some(clamp(result, self.minimum(), self.maximum())));
    }

    /// Sets the valid spin directions.
    fn set_valid_spin_direction(&self) {
        let mut valid_directions = ValidSpinDirections::NONE;

        // Zero increment always prevents spin.
        if !self.increment().is_zero() && !self.is_read_only() {
            let value = self.value();
            if value.is_none() {
                valid_directions = ValidSpinDirections::INCREASE | ValidSpinDirections::DECREASE;
            }

            if value.is_some_and(|value| value < self.maximum()) {
                valid_directions |= ValidSpinDirections::INCREASE;
            }

            if value.is_some_and(|value| value > self.minimum()) {
                valid_directions |= ValidSpinDirections::DECREASE;
            }
        }

        if let Some(spinner) = self.spinner() {
            spinner.set_valid_spin_direction(valid_directions);
        }
    }

    fn set_value_internal(&self, value: Option<Decimal>) {
        self.internal_value_set.set(true);
        let _reset = Reset(&self.internal_value_set);
        self.set_current_value(Self::value_property(), value);
    }

    fn text_box_on_text_changed(&self) {
        self.is_text_changed_from_ui.set(true);
        let _reset = Reset(&self.is_text_changed_from_ui);
        if let Some(text_box) = self.text_box() {
            self.set_current_value(Self::text_property(), text_box.text());
        }
    }

    fn on_spinner_spin(&self, e: &SpinEventArgs) {
        if self.allow_spin() && !self.is_read_only() {
            let mut spin = !e.using_mouse_wheel();
            spin |= self.text_box().is_some_and(|text_box| text_box.is_focused());

            if spin {
                e.set_handled(true);
                self.on_spin(e);
            }
        }
    }

    fn do_decrement(&self) {
        if self.spinner().is_none_or(|spinner| spinner.valid_spin_direction().contains(ValidSpinDirections::DECREASE)) {
            self.on_decrement();
        }
    }

    fn do_increment(&self) {
        if self.spinner().is_none_or(|spinner| spinner.valid_spin_direction().contains(ValidSpinDirections::INCREASE)) {
            self.on_increment();
        }
    }

    fn text_box_on_pointer_pressed(&self, e: &PointerPressedEventArgs) {
        let captured = e.pointer().captured();
        let is_spinner = match (&captured, self.spinner()) {
            (Some(captured), Some(spinner)) => captured.ptr_eq(&spinner),
            (None, None) => true,
            _ => false,
        };
        if !is_spinner {
            let pointer = e.pointer().clone();
            let this = self.to_ref();
            Dispatcher::ui_thread().invoke_async_local_with_priority(
                move || {
                    let spinner = this.spinner().map(|spinner| spinner.upcast::<InputElement>());
                    pointer.capture(spinner.as_ref());
                },
                DispatcherPriority::INPUT,
            );
        }
    }

    fn commit_input(&self, force_text_update: bool) -> bool {
        self.sync_text_and_value_properties(true, self.text().as_deref(), force_text_update)
    }

    /// Synchronizes the `Text` and `Value` properties: optionally updates
    /// the value from the text, and optionally forces the text to be the
    /// text of the value. Returns whether the text was a valid value.
    fn sync_text_and_value_properties(
        &self,
        update_value_from_text: bool,
        text: Option<&str>,
        force_text_update: bool,
    ) -> bool {
        if self.is_syncing_text_and_value_properties.get() {
            return true;
        }

        self.is_syncing_text_and_value_properties.set(true);
        let _reset = Reset(&self.is_syncing_text_and_value_properties);
        let mut parsed_text_is_valid = true;

        if update_value_from_text {
            match self.convert_text_to_value(text) {
                Ok(new_value) => {
                    if new_value != self.value() {
                        self.set_value_internal(new_value);
                    }
                }
                Err(_) => parsed_text_is_valid = false,
            }
        }

        // Do not touch the ongoing text input from the user.
        if !self.is_text_changed_from_ui.get() {
            if force_text_update {
                // A format string that is not valid is an error of the
                // developer: the reference throws here.
                let new_text = self.convert_value_to_text().unwrap_or_else(|error| panic!("{}", error.0));
                if self.text() != new_text {
                    self.set_current_value(Self::text_property(), new_text);
                }
            }

            // Sync the text and the text box.
            if let Some(text_box) = self.text_box() {
                text_box.set_text(self.text().as_deref());
            }
        }

        if self.is_text_changed_from_ui.get() && !parsed_text_is_valid {
            // The text input was made by the user and the text represents
            // an invalid value. Disable the spinner in this case.
            if let Some(spinner) = self.spinner() {
                spinner.set_valid_spin_direction(ValidSpinDirections::NONE);
            }
        } else {
            self.set_valid_spin_direction();
        }

        parsed_text_is_valid
    }

    fn convert_text_to_value_core(
        &self,
        current_value_text: Option<&str>,
        text: &str,
    ) -> Result<Option<Decimal>, ConversionError> {
        if text.is_empty() {
            return Ok(None);
        }

        if let Some(converter) = self.text_converter() {
            let boxed: BoxedValue = Rc::new(text.to_string());
            let value_from_text = converter
                .convert(Some(&boxed), ValueType::of::<Option<Decimal>>(), None, &CultureInfo::current_culture())
                .map_err(|error| ConversionError(error.to_string()))?;
            return match value_from_text {
                None => Ok(None),
                Some(value) => {
                    if let Some(value) = value.downcast_ref::<Decimal>() {
                        Ok(Some(*value))
                    } else if let Some(value) = value.downcast_ref::<Option<Decimal>>() {
                        Ok(*value)
                    } else {
                        Err(ConversionError("Specified cast is not valid.".to_string()))
                    }
                }
            };
        }

        let number_format = self.number_format();
        let number_format = number_format.as_deref();

        if self.is_percent(&self.format_string()) {
            return parse_percent(text, number_format).map(Some);
        }

        let style = self.parsing_number_style();
        let parse = |text: &str| match Decimal::parse_with(text, style, number_format) {
            Ok(value) => Ok(Some(value)),
            // The styles are not valid for decimals: the reference throws.
            Err(NumberParseError::InvalidStyle) => Err(ConversionError(NumberParseError::InvalidStyle.to_string())),
            Err(_) => Ok(None),
        };

        if let Some(value) = parse(text)? {
            return Ok(Some(value));
        }

        // There was a problem while converting the new text. Check if the
        // current value text is also failing: it then also contains special
        // characters, e.g. "90°".
        if let Some(current_value_text) = current_value_text.filter(|text| !text.is_empty()) {
            if parse(current_value_text)?.is_none() {
                // The same non-digit characters are in the current value
                // text and in the new text: remove them from the new text
                // to parse it again.
                let same_special_characters = current_value_text
                    .chars()
                    .filter(|c| !is_digit(*c))
                    .all(|c| text.chars().any(|other| other == c));
                if same_special_characters {
                    let digits: String = text.chars().filter(|c| is_digit(*c)).collect();
                    // If without the special characters the parsing is
                    // good, do not fail.
                    if let Some(value) = parse(&digits)? {
                        return Ok(Some(value));
                    }
                }
            }
        }

        Err(ConversionError::format())
    }

    fn validate_min_max(&self, value: Option<Decimal>) -> Result<(), ConversionError> {
        let Some(value) = value else {
            return Ok(());
        };

        let minimum = self.minimum();
        let maximum = self.maximum();
        if value < minimum {
            Err(ConversionError(format!("Value must be greater than Minimum value of {minimum}")))
        } else if value > maximum {
            Err(ConversionError(format!("Value must be less than Maximum value of {maximum}")))
        } else {
            Ok(())
        }
    }

    fn is_percent(&self, string_to_test: &str) -> bool {
        if let Some(p_index) = string_to_test.find('P') {
            // A "P" between two "'" is considered as text, not percent.
            let is_text = string_to_test[..p_index].contains('\'') && string_to_test[p_index..].contains('\'');

            return !is_text;
        }
        false
    }

    fn focus_changed(&self, has_focus: bool) {
        // The got focus and lost focus notifications are asynchronous and
        // cannot reliably tell that the control has the focus: all they do
        // is let it know that the focus changed sometime in the past.

        let was_focused = self.is_focused.replace(has_focus);

        if has_focus && !was_focused {
            if let Some(text_box) = self.text_box() {
                text_box.focus();
                text_box.select_all();
            }
        }
    }
}

/// Whether the character is a decimal digit of any script (the general
/// category Nd), as the reference tests the characters of the text. Outside
/// the basic plane the reference sees surrogates, which are not digits.
fn is_digit(c: char) -> bool {
    if c.is_ascii() {
        return c.is_ascii_digit();
    }

    let value = c as u32;
    value <= 0xFFFF && Codepoint::new(value).general_category() == GeneralCategory::DecimalNumber
}

/// The value restricted to the range of `min` to `max` (the decimal form of
/// the reference's clamp utility).
fn clamp(value: Decimal, min: Decimal, max: Decimal) -> Decimal {
    assert!(min <= max, "{min} cannot be greater than {max}.");

    if value < min {
        min
    } else if value > max {
        max
    } else {
        value
    }
}

/// The end of the value range in the direction of `step`: the result of a
/// spin that would leave the range of decimals.
fn saturated(step: Decimal) -> Decimal {
    if step.is_sign_negative() {
        Decimal::MIN_VALUE
    } else {
        Decimal::MAX_VALUE
    }
}

/// Parses percent format text.
fn parse_percent(text: &str, number_format: Option<&NumberFormatInfo>) -> Result<Decimal, ConversionError> {
    let current;
    let info = match number_format {
        Some(info) => info,
        None => {
            current = CultureInfo::current_culture().number_format();
            &*current
        }
    };

    if info.percent_symbol().is_empty() {
        return Err(ConversionError("String cannot be of zero length.".to_string()));
    }
    let text = text.replace(info.percent_symbol(), "");
    let result = Decimal::parse_with(&text, NumberStyles::ANY, Some(info))
        .map_err(|error| ConversionError(error.to_string()))?;
    Ok(result / Decimal::from(100))
}
