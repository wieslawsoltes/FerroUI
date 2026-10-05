//! Ported from the upstream `BindingTests_DataValidation`.
//!
//! The upstream test base class is generic over the kind of the target
//! property; the two instantiations are the `direct_property_tests` and
//! `styled_property_tests` modules.

use super::test_support::*;
use crate::data::Binding;
use ferroui_base::data::core::Value;
use ferroui_base::data::model::{Event, INotifyDataErrorInfo, Model};
use ferroui_base::data::{BindingError, BindingMode, BindingValueType, DataValidationException};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::styling::{Selectors, Setter, Style};
use ferroui_base::*;
use std::cell::Cell;
use std::fmt;
use std::rc::Rc;

#[repr(C)]
struct DataValidationTestControl {
    base: Control,
    state: DataValidationState,
}

ferro_class!(DataValidationTestControl: Control);
ferro_impl_classes!(
    DataValidationTestControl: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl
);

impl DataValidationTestControl {
    fn construct() -> Self {
        Self { base: Control::construct(), state: DataValidationState::default() }
    }

    fn data_validation_error(&self) -> Option<BindingError> {
        self.state.error()
    }
}

#[repr(C)]
struct ValidatedStyledPropertyClass {
    base: DataValidationTestControl,
}

ferro_class!(ValidatedStyledPropertyClass: DataValidationTestControl);
ferro_impl_classes!(
    ValidatedStyledPropertyClass: StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl
);

impl FerroObjectImpl for ValidatedStyledPropertyClass {
    fn update_data_validation(
        this: &Self,
        property: &'static FerroProperty,
        state: BindingValueType,
        error: Option<&BindingError>,
    ) {
        if property == Self::value_property().as_property() {
            this.base.state.update(state, error);
        }
    }
}

impl ValidatedStyledPropertyClass {
    ferro_property!(fn value_property() -> StyledProperty<i32> {
        FerroProperty::register_with::<ValidatedStyledPropertyClass, _>(
            "Value",
            StyledPropertyOptions::new(0).enable_data_validation(true),
        )
    });

    fn new() -> Ref<Self> {
        Control::class_init();
        Self::value_property();
        instantiate(Self { base: DataValidationTestControl::construct() })
    }
}

#[repr(C)]
struct ValidatedDirectPropertyClass {
    base: DataValidationTestControl,
    value: Cell<i32>,
}

ferro_class!(ValidatedDirectPropertyClass: DataValidationTestControl);
ferro_impl_classes!(
    ValidatedDirectPropertyClass: StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl
);

impl FerroObjectImpl for ValidatedDirectPropertyClass {
    fn update_data_validation(
        this: &Self,
        property: &'static FerroProperty,
        state: BindingValueType,
        error: Option<&BindingError>,
    ) {
        if property == Self::value_property().as_property() {
            this.base.state.update(state, error);
        }
    }
}

impl ValidatedDirectPropertyClass {
    ferro_property!(fn value_property() -> DirectProperty<ValidatedDirectPropertyClass, i32> {
        FerroProperty::register_direct_with::<ValidatedDirectPropertyClass, _>(
            "Value",
            |o| o.value.get(),
            Some(|o, v| o.set_value_field(v)),
            DirectPropertyMetadata::new(None).with_enable_data_validation(true),
        )
    });

    fn new() -> Ref<Self> {
        Control::class_init();
        Self::value_property();
        instantiate(Self { base: DataValidationTestControl::construct(), value: Cell::new(0) })
    }

    fn set_value_field(&self, value: i32) {
        self.set_and_raise_cell(Self::value_property(), &self.value, value);
    }
}

/// The error of a setter that rejects a value out of range.
#[derive(Debug)]
struct ArgumentOutOfRangeError;

impl fmt::Display for ArgumentOutOfRangeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Specified argument was out of the range of valid values. (Parameter 'value')")
    }
}

impl std::error::Error for ArgumentOutOfRangeError {}

struct ExceptionValidatingModel {
    value: Cell<i32>,
}

impl ExceptionValidatingModel {
    const MAX_VALUE: i32 = 100;

    fn new() -> Rc<Self> {
        Model::new_model(Self { value: Cell::new(20) })
    }

    fn set_value(&self, value: i32) -> Result<(), BindingError> {
        if value > Self::MAX_VALUE {
            return Err(BindingError::new(ArgumentOutOfRangeError));
        }
        self.value.set(value);
        Ok(())
    }
}

ferro_model!(ExceptionValidatingModel, |b| b
    .validated_property::<Value<i32>>("Value", |o| o.value.get(), |o, v| o.set_value(v)));

struct IndeiValidatingModel {
    has_errors: Cell<bool>,
    value: Cell<i32>,
    errors_changed: Event<str>,
}

impl IndeiValidatingModel {
    const MAX_VALUE: i32 = 100;

    fn new() -> Rc<Self> {
        Model::new_model(Self { has_errors: Cell::new(false), value: Cell::new(20), errors_changed: Event::new() })
    }

    fn with_value(value: i32) -> Rc<Self> {
        let result = Self::new();
        result.set_value(value);
        result
    }

    fn set_value(&self, value: i32) {
        self.value.set(value);
        self.set_has_errors(value > Self::MAX_VALUE);
    }

    fn set_has_errors(&self, value: bool) {
        if self.has_errors.get() != value {
            self.has_errors.set(value);
            self.errors_changed.raise("Value");
        }
    }
}

impl INotifyDataErrorInfo for IndeiValidatingModel {
    fn has_errors(&self) -> bool {
        self.has_errors.get()
    }

    fn get_errors(&self, property_name: Option<&str>) -> Vec<BoxedValue> {
        if property_name == Some("Value") && self.value.get() > Self::MAX_VALUE {
            vec![boxed(format!("Invalid value: {}.", self.value.get()))]
        } else {
            Vec::new()
        }
    }

    fn errors_changed(&self) -> &Event<str> {
        &self.errors_changed
    }
}

ferro_model!(IndeiValidatingModel, |b| b
    .notify_data_error_info()
    .property::<Value<i32>>("Value", |o| o.value.get(), |o, v| o.set_value(v)));

/// A target control and its validated property.
struct Target {
    control: Ref<DataValidationTestControl>,
    property: &'static FerroProperty,
    get: Box<dyn Fn() -> i32>,
    set: Box<dyn Fn(i32)>,
}

impl Target {
    fn error(&self) -> Option<BindingError> {
        self.control.data_validation_error()
    }

    fn is_data_validation_error(&self) -> bool {
        self.error().is_some_and(|e| e.inner().is::<DataValidationException>())
    }

    fn error_message(&self) -> Option<String> {
        self.error().map(|e| e.to_string())
    }
}

fn create_direct_target() -> Target {
    let control = ValidatedDirectPropertyClass::new();
    let (getter, setter) = (control.clone(), control.clone());
    Target {
        control: control.upcast(),
        property: ValidatedDirectPropertyClass::value_property().as_property(),
        get: Box::new(move || getter.value.get()),
        set: Box::new(move |v| setter.set_value_field(v)),
    }
}

fn create_styled_target() -> Target {
    let control = ValidatedStyledPropertyClass::new();
    let (getter, setter) = (control.clone(), control.clone());
    Target {
        control: control.upcast(),
        property: ValidatedStyledPropertyClass::value_property().as_property(),
        get: Box::new(move || getter.get_value(ValidatedStyledPropertyClass::value_property())),
        set: Box::new(move |v| setter.set_value(ValidatedStyledPropertyClass::value_property(), v)),
    }
}

fn two_way_binding() -> Rc<Binding> {
    Binding::with_path_and_mode("Value", BindingMode::TwoWay)
}

fn setter_exception_causes_data_validation_error(target: Target) {
    let binding = two_way_binding();

    target.control.set_data_context(Some(ExceptionValidatingModel::new()));
    target.control.bind_binding(target.property, &binding);

    assert_eq!((target.get)(), 20);

    (target.set)(200);

    assert_eq!((target.get)(), 200);
    assert!(target.error().is_some_and(|e| e.inner().is::<ArgumentOutOfRangeError>()));

    (target.set)(10);

    assert_eq!((target.get)(), 10);
    assert!(target.error().is_none());
}

fn indei_error_causes_data_validation_error(target: Target) {
    let binding = two_way_binding();

    target.control.set_data_context(Some(IndeiValidatingModel::new()));
    target.control.bind_binding(target.property, &binding);

    assert_eq!((target.get)(), 20);

    (target.set)(200);

    assert_eq!((target.get)(), 200);
    assert!(target.is_data_validation_error());
    assert_eq!(target.error_message().as_deref(), Some("Invalid value: 200."));

    (target.set)(10);

    assert_eq!((target.get)(), 10);
    assert!(target.error().is_none());
}

fn disposing_binding_subscription_clears_data_validation(target: Target) {
    let binding = two_way_binding();

    target.control.set_data_context(Some(IndeiValidatingModel::with_value(200)));

    let sub = target.control.bind_binding(target.property, &binding);

    assert_eq!((target.get)(), 200);
    assert!(target.is_data_validation_error());

    sub.dispose();

    assert!(target.error().is_none());
}

mod direct_property_tests {
    use super::*;

    #[test]
    fn setter_exception_causes_data_validation_error() {
        super::setter_exception_causes_data_validation_error(create_direct_target());
    }

    #[test]
    fn indei_error_causes_data_validation_error() {
        super::indei_error_causes_data_validation_error(create_direct_target());
    }

    #[test]
    fn disposing_binding_subscription_clears_data_validation() {
        super::disposing_binding_subscription_clears_data_validation(create_direct_target());
    }
}

mod styled_property_tests {
    use super::*;

    #[test]
    fn setter_exception_causes_data_validation_error() {
        super::setter_exception_causes_data_validation_error(create_styled_target());
    }

    #[test]
    fn indei_error_causes_data_validation_error() {
        super::indei_error_causes_data_validation_error(create_styled_target());
    }

    #[test]
    fn disposing_binding_subscription_clears_data_validation() {
        super::disposing_binding_subscription_clears_data_validation(create_styled_target());
    }

    fn style(class: Option<&str>, property: &'static FerroProperty, binding: Rc<Binding>) -> Ref<Style> {
        let selector = Selectors::is::<DataValidationTestControl>();
        let selector = match class {
            Some(class) => selector.class(class),
            None => selector,
        };
        Style::with_setters(selector, [Setter::new_binding_base(property, binding)])
    }

    #[test]
    fn style_binding_supports_data_validation() {
        let target = create_styled_target();
        let binding = two_way_binding();

        let model = IndeiValidatingModel::new();
        let root = TestRoot::new();
        root.set_data_context(Some(model.clone()));
        root.styles().add(style(None, target.property, binding));
        root.set_child(Some(target.control.clone().upcast()));

        root.execute_initial_layout_pass();

        assert_eq!((target.get)(), 20);

        model.set_value(200);

        assert_eq!((target.get)(), 200);
        assert!(target.is_data_validation_error());
        assert_eq!(target.error_message().as_deref(), Some("Invalid value: 200."));

        model.set_value(10);

        assert_eq!((target.get)(), 10);
        assert!(target.error().is_none());
    }

    #[test]
    fn style_with_activator_binding_supports_data_validation() {
        let target = create_styled_target();
        let binding = two_way_binding();

        let model = IndeiValidatingModel::with_value(200);

        let root = TestRoot::new();
        root.set_data_context(Some(model.clone()));
        root.styles().add(style(Some("foo"), target.property, binding));
        root.set_child(Some(target.control.clone().upcast()));

        root.execute_initial_layout_pass();
        target.control.classes().add("foo");

        assert_eq!((target.get)(), 200);
        assert!(target.is_data_validation_error());
        assert_eq!(target.error_message().as_deref(), Some("Invalid value: 200."));

        target.control.classes().remove("foo");

        assert_eq!((target.get)(), 0);
        assert!(target.error().is_none());

        target.control.classes().add("foo");

        assert!(target.is_data_validation_error());
        assert_eq!(target.error_message().as_deref(), Some("Invalid value: 200."));

        model.set_value(10);

        assert_eq!((target.get)(), 10);
        assert!(target.error().is_none());
    }

    #[test]
    fn data_validation_can_switch_between_style_and_local_value_binding() {
        let target = create_styled_target();
        let model1 = IndeiValidatingModel::with_value(200);
        let model2 = IndeiValidatingModel::with_value(300);
        let binding1 = Binding::with_path("Value");
        let binding2 = Binding::with_path("Value");
        binding2.set_source(Some(model2.clone()));

        let root = TestRoot::new();
        root.set_data_context(Some(model1.clone()));
        root.styles().add(style(None, target.property, binding1));
        root.set_child(Some(target.control.clone().upcast()));

        root.execute_initial_layout_pass();

        assert_eq!((target.get)(), 200);
        assert!(target.is_data_validation_error());
        assert_eq!(target.error_message().as_deref(), Some("Invalid value: 200."));

        let sub = target.control.bind_binding(target.property, &binding2);

        assert_eq!((target.get)(), 300);
        assert_eq!(target.error_message().as_deref(), Some("Invalid value: 300."));

        sub.dispose();

        assert_eq!((target.get)(), 200);
        assert!(target.is_data_validation_error());
        assert_eq!(target.error_message().as_deref(), Some("Invalid value: 200."));
    }

    #[test]
    fn data_validation_can_switch_between_style_and_style_trigger_binding() {
        let target = create_styled_target();
        let model1 = IndeiValidatingModel::with_value(200);
        let model2 = IndeiValidatingModel::with_value(300);
        let binding1 = Binding::with_path("Value");
        let binding2 = Binding::with_path("Value");
        binding2.set_source(Some(model2.clone()));

        let root = TestRoot::new();
        root.set_data_context(Some(model1.clone()));
        root.styles().add(style(None, target.property, binding1));
        root.styles().add(style(Some("foo"), target.property, binding2));
        root.set_child(Some(target.control.clone().upcast()));

        root.execute_initial_layout_pass();

        assert_eq!((target.get)(), 200);
        assert!(target.is_data_validation_error());
        assert_eq!(target.error_message().as_deref(), Some("Invalid value: 200."));

        target.control.classes().add("foo");

        assert_eq!((target.get)(), 300);
        assert_eq!(target.error_message().as_deref(), Some("Invalid value: 300."));

        target.control.classes().remove("foo");

        assert_eq!((target.get)(), 200);
        assert!(target.is_data_validation_error());
        assert_eq!(target.error_message().as_deref(), Some("Invalid value: 200."));
    }
}
