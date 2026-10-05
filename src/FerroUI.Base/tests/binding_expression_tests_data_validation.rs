//! Port of the upstream `BindingExpressionTests.DataValidation` tests.
//!
//! Not ported: `DataAnnotations_Validation_Updates_Data_Validation_When_Writing_To_Source_OneWayToSource`,
//! `Updates_Data_Validation_For_Required_DataAnnotation` and
//! `Handles_Indei_And_DataAnnotations_On_Same_Class` (validation attributes
//! are not available).

use super::binding_test_support::*;
use super::*;
use crate::data::converters::IValueConverter;
use crate::data::core::{ModelRef, Value, ValueType};
use crate::data::model::{Event, INotifyDataErrorInfo, INotifyPropertyChanged, Model};
use crate::data::{BindingError, BindingErrorType, BindingMode, BindingNotification};
use crate::ferro_model;
use std::collections::HashMap;

/// The message of the managed "argument out of range" error for `value`.
const OUT_OF_RANGE: &str = "Specified argument was out of the range of valid values. (Parameter 'value')";

/// `new { Foo = default(ViewModel) }`.
struct NullFooData;

ferro_model!(NullFooData, |b| b.read_only::<ModelRef<ViewModel>>("Foo", |_| None));

pub struct ExceptionViewModel {
    must_be_positive: Cell<i32>,
    property_changed: Event<str>,
}

impl ExceptionViewModel {
    fn new(must_be_positive: i32) -> Rc<Self> {
        Model::new_model(Self { must_be_positive: Cell::new(must_be_positive), property_changed: Event::new() })
    }

    fn must_be_positive(&self) -> i32 {
        self.must_be_positive.get()
    }

    fn set_must_be_positive(&self, value: i32) -> Result<(), BindingError> {
        if value <= 0 {
            return Err(BindingError::message(OUT_OF_RANGE));
        }
        self.must_be_positive.set(value);
        self.property_changed.raise("MustBePositive");
        Ok(())
    }

    fn must_be_positive_step() -> Step {
        inpc_validated_prop::<ExceptionViewModel, Value<i32>>(
            "MustBePositive",
            Out::Int,
            |o| o.must_be_positive(),
            |o, v| o.set_must_be_positive(v),
        )
    }
}

impl INotifyPropertyChanged for ExceptionViewModel {
    fn property_changed(&self) -> &Event<str> {
        &self.property_changed
    }
}

ferro_model!(ExceptionViewModel, |b| b.notify_property_changed().validated_property::<Value<i32>>(
    "MustBePositive",
    |o| o.must_be_positive(),
    |o, v| o.set_must_be_positive(v)
));

struct IndeiViewModel {
    must_be_positive: Cell<i32>,
    errors: RefCell<HashMap<String, Vec<String>>>,
    property_changed: Event<str>,
    errors_changed: Event<str>,
}

impl IndeiViewModel {
    fn new() -> Rc<Self> {
        Model::new_model(Self {
            must_be_positive: Cell::new(0),
            errors: RefCell::new(HashMap::new()),
            property_changed: Event::new(),
            errors_changed: Event::new(),
        })
    }

    fn with_value(value: i32) -> Rc<Self> {
        let result = Self::new();
        result.set_must_be_positive(value);
        result
    }

    fn errors_changed_subscription_count(&self) -> usize {
        self.errors_changed.handler_count()
    }

    fn must_be_positive(&self) -> i32 {
        self.must_be_positive.get()
    }

    fn set_must_be_positive(&self, value: i32) {
        self.must_be_positive.set(value);
        self.property_changed.raise("MustBePositive");

        if value >= 0 {
            self.errors.borrow_mut().remove("MustBePositive");
            self.errors_changed.raise("MustBePositive");
        } else {
            self.errors.borrow_mut().insert(s("MustBePositive"), vec![s("Must be positive")]);
            self.errors_changed.raise("MustBePositive");
        }
    }

    fn must_be_positive_step() -> Step {
        inpc_prop::<IndeiViewModel, Value<i32>>("MustBePositive", Out::Int, |o| o.must_be_positive(), |o, v| {
            o.set_must_be_positive(v)
        })
    }
}

impl INotifyPropertyChanged for IndeiViewModel {
    fn property_changed(&self) -> &Event<str> {
        &self.property_changed
    }
}

impl INotifyDataErrorInfo for IndeiViewModel {
    fn has_errors(&self) -> bool {
        self.must_be_positive.get() >= 0
    }

    fn get_errors(&self, property_name: Option<&str>) -> Vec<BoxedValue> {
        match property_name.and_then(|name| self.errors.borrow().get(name).cloned()) {
            Some(errors) => errors.into_iter().map(boxed).collect(),
            None => Vec::new(),
        }
    }

    fn errors_changed(&self) -> &Event<str> {
        &self.errors_changed
    }
}

ferro_model!(IndeiViewModel, |b| b
    .notify_property_changed()
    .notify_data_error_info()
    .property::<Value<i32>>("MustBePositive", |o| o.must_be_positive(), |o, v| o.set_must_be_positive(v)));

struct IndeiContainerViewModel {
    inner: RefCell<Option<Rc<IndeiViewModel>>>,
    property_changed: Event<str>,
    errors_changed: Event<str>,
}

impl IndeiContainerViewModel {
    fn new(inner: Option<Rc<IndeiViewModel>>) -> Rc<Self> {
        Model::new_model(Self {
            inner: RefCell::new(inner),
            property_changed: Event::new(),
            errors_changed: Event::new(),
        })
    }

    fn inner(&self) -> Option<Rc<IndeiViewModel>> {
        self.inner.borrow().clone()
    }

    fn set_inner(&self, value: Option<Rc<IndeiViewModel>>) {
        self.inner.replace(value);
        self.property_changed.raise("Inner");
    }

    fn inner_step() -> Step {
        inpc_prop::<IndeiContainerViewModel, ModelRef<IndeiViewModel>>("Inner", Out::Object, |o| o.inner(), |o, v| {
            o.set_inner(v)
        })
    }
}

impl INotifyPropertyChanged for IndeiContainerViewModel {
    fn property_changed(&self) -> &Event<str> {
        &self.property_changed
    }
}

impl INotifyDataErrorInfo for IndeiContainerViewModel {
    fn has_errors(&self) -> bool {
        false
    }

    fn get_errors(&self, _property_name: Option<&str>) -> Vec<BoxedValue> {
        Vec::new()
    }

    fn errors_changed(&self) -> &Event<str> {
        &self.errors_changed
    }
}

ferro_model!(IndeiContainerViewModel, |b| b
    .notify_property_changed()
    .notify_data_error_info()
    .property::<ModelRef<IndeiViewModel>>("Inner", |o| o.inner(), |o, v| o.set_inner(v)));

/// Converts text back to an identifier, reporting text that is not a number
/// as an error of the given type.
struct InvalidIdConverter {
    error_type: BindingErrorType,
}

impl IValueConverter for InvalidIdConverter {
    fn convert(
        &self,
        value: Option<&BoxedValue>,
        _target_type: ValueType,
        _parameter: Option<&BoxedValue>,
    ) -> Result<Option<BoxedValue>, BindingError> {
        Ok(value.cloned())
    }

    fn convert_back(
        &self,
        value: Option<&BoxedValue>,
        _target_type: ValueType,
        _parameter: Option<&BoxedValue>,
    ) -> Result<Option<BoxedValue>, BindingError> {
        if let Some(i) = value.and_then(|v| v.downcast_ref::<i32>()) {
            return Ok(Some(boxed(*i)));
        }
        if let Some(parsed) = value.and_then(|v| v.downcast_ref::<String>()).and_then(|v| v.parse::<i32>().ok()) {
            return Ok(Some(boxed(parsed)));
        }
        Ok(Some(Rc::new(BindingNotification::with_error(
            BindingError::message(format!("'{}' is not a valid ID.", to_text(value).unwrap_or_default())),
            self.error_type,
        ))))
    }
}

struct FuncValueConverter {
    convert_back: Box<dyn Fn(Option<&BoxedValue>) -> Option<BoxedValue>>,
}

impl IValueConverter for FuncValueConverter {
    fn convert(
        &self,
        value: Option<&BoxedValue>,
        _target_type: ValueType,
        _parameter: Option<&BoxedValue>,
    ) -> Result<Option<BoxedValue>, BindingError> {
        Ok(value.cloned())
    }

    fn convert_back(
        &self,
        value: Option<&BoxedValue>,
        _target_type: ValueType,
        _parameter: Option<&BoxedValue>,
    ) -> Result<Option<BoxedValue>, BindingError> {
        Ok((self.convert_back)(value))
    }
}

fn validated() -> Opts {
    Opts::default().with_data_validation(true)
}

/// The message of a failed conversion to the target type.
fn conversion_to_target_message(value: &str, target: &str) -> String {
    format!("Could not convert '{value}' ({}) to '{target}'.", std::any::type_name::<String>())
}

binding_tests! {
    fn root_null_should_update_data_validation(f) {
        let target = create_target_with_source(f, None, &Path::of(ViewModel::string_step()), validated());

        assert_binding_chain_error(
            &target,
            TargetClass::string_property(),
            "Binding Source is null.",
            "StringValue",
            "(source)",
        );
    }

    fn null_value_in_path_should_update_data_validation(f) {
        let data = Model::new_model(NullFooData);
        let path = Path::of(plain_read_only_prop::<NullFooData, ModelRef<ViewModel>>("Foo", Out::Object, |_| None))
            .then(ViewModel::string_step())
            .then(plain_read_only_prop::<String, Value<i32>>("Length", Out::Int, |o| o.len() as i32));
        let target = create_target_with_source(f, src(&data), &path, validated());

        assert_binding_chain_error(
            &target,
            TargetClass::int_property(),
            "Value is null.",
            "Foo.StringValue.Length",
            "Foo",
        );
    }

    fn invalid_double_string_should_update_data_validation(f) {
        let data = ViewModel::with_string("foo");
        let target = create_target_with_source(
            f,
            src(&data),
            &Path::of(ViewModel::string_step()),
            validated().with_property(TargetClass::double_property()),
        );

        assert_binding_error(
            &target,
            TargetClass::double_property(),
            &conversion_to_target_message("foo", "f64"),
            BindingErrorType::Error,
        );
    }

    fn invalid_double_string_should_revert_to_fallback_value(f) {
        let data = ViewModel::with_string("foo");
        let target = create_target_with_source(
            f,
            src(&data),
            &Path::of(ViewModel::string_step()),
            validated().with_fallback_value(boxed(42.0)).with_property(TargetClass::double_property()),
        );

        assert_eq!(target.double(), 42.0);
        assert_binding_error(
            &target,
            TargetClass::double_property(),
            &conversion_to_target_message("foo", "f64"),
            BindingErrorType::Error,
        );
    }

    fn setter_exception_does_not_cause_data_validation_error_when_data_validation_not_enabled(f) {
        let data = ExceptionViewModel::new(5);
        let target = create_target_with_source(
            f,
            src(&data),
            &Path::of(ExceptionViewModel::must_be_positive_step()),
            Opts::mode(BindingMode::TwoWay).with_data_validation(false),
        );

        target.set_int(-5);

        assert_eq!(target.int(), -5);
        assert_eq!(data.must_be_positive(), 5);
        assert_no_error(&target, TargetClass::int_property());
    }

    fn setter_exception_updates_data_validation(f) {
        let data = ExceptionViewModel::new(5);
        let target = create_target_with_source(
            f,
            src(&data),
            &Path::of(ExceptionViewModel::must_be_positive_step()),
            validated().with_mode(BindingMode::TwoWay),
        );

        target.set_int(-5);

        assert_eq!(target.int(), -5);
        assert_eq!(data.must_be_positive(), 5);
        assert_binding_error(
            &target,
            TargetClass::int_property(),
            OUT_OF_RANGE,
            BindingErrorType::DataValidationError,
        );
    }

    fn setter_exception_error_is_cleared_when_reverting_to_same_valid_value(f) {
        // Issue #20534: when a setter fails on an invalid value and the user
        // reverts to the same valid value that was last successfully set,
        // the validation error should be cleared.
        let data = ExceptionViewModel::new(5);
        let target = create_target_with_source(
            f,
            src(&data),
            &Path::of(ExceptionViewModel::must_be_positive_step()),
            validated().with_mode(BindingMode::TwoWay),
        );

        // Step 1: set a valid value.
        target.set_int(10);
        assert_eq!(data.must_be_positive(), 10);
        assert_no_error(&target, TargetClass::int_property());

        // Step 2: set an invalid value: the setter fails, an error appears.
        target.set_int(-5);
        assert_eq!(data.must_be_positive(), 10);
        assert_binding_error(
            &target,
            TargetClass::int_property(),
            OUT_OF_RANGE,
            BindingErrorType::DataValidationError,
        );

        // Step 3: revert to the same valid value (10). The error must clear.
        target.set_int(10);
        assert_eq!(data.must_be_positive(), 10);
        assert_no_error(&target, TargetClass::int_property());
    }

    fn indei_validation_does_not_subscribe_when_data_validation_not_enabled(f) {
        let data = IndeiViewModel::with_value(5);
        let _target = create_target_with_source(
            f,
            src(&data),
            &Path::of(IndeiViewModel::must_be_positive_step()),
            Opts::mode(BindingMode::TwoWay).with_data_validation(false),
        );

        assert_eq!(data.errors_changed_subscription_count(), 0);
    }

    fn indei_validation_subscribes_and_unsubscribes(f) {
        let data = IndeiViewModel::with_value(5);
        let (_target, expression) = create_target_and_expression(
            f,
            &Path::of(IndeiViewModel::must_be_positive_step()),
            validated().with_mode(BindingMode::TwoWay).with_source(src(&data)),
        );

        assert_eq!(data.errors_changed_subscription_count(), 1);
        expression.dispose();
        assert_eq!(data.errors_changed_subscription_count(), 0);
    }

    fn conversion_errors_update_data_validation_when_writing_to_source(f) {
        let data = ViewModel::with_double(5.6);
        let target = create_target_with_source(
            f,
            src(&data),
            &Path::of(ViewModel::double_step()),
            validated().with_mode(BindingMode::TwoWay).with_property(TargetClass::tag_property()),
        );

        // Can write a double value.
        target.set_tag(Some(boxed(1.2)));
        assert_eq!(data.double_value(), 1.2);
        assert_no_error(&target, TargetClass::string_property());

        // Can write a string value and it gets converted to double.
        target.set_tag(Some(boxed(s("3.4"))));
        assert_eq!(data.double_value(), 3.4);
        assert_no_error(&target, TargetClass::string_property());

        // An invalid string value should result in an error. It is reported
        // as a data validation error rather than a binding error, preserving
        // the upstream semantics.
        target.set_tag(Some(boxed(s("bar"))));
        assert_eq!(data.double_value(), 3.4);
        assert_binding_error(
            &target,
            TargetClass::tag_property(),
            &format!("Could not convert 'bar' ({}) to f64.", std::any::type_name::<String>()),
            BindingErrorType::DataValidationError,
        );
    }

    fn indei_validation_updates_data_validation_when_writing_to_source(f) {
        let data = IndeiViewModel::new();
        let target = create_target_with_source(
            f,
            src(&data),
            &Path::of(IndeiViewModel::must_be_positive_step()),
            validated().with_mode(BindingMode::TwoWay),
        );

        assert_eq!(target.int(), 0);
        assert_eq!(data.must_be_positive(), 0);
        assert_no_error(&target, TargetClass::int_property());

        target.set_int(5);

        assert_eq!(target.int(), 5);
        assert_eq!(data.must_be_positive(), 5);
        assert_no_error(&target, TargetClass::int_property());

        target.set_int(-5);

        assert_eq!(target.int(), -5);
        assert_eq!(data.must_be_positive(), -5);
        assert_data_validation_error(&target, TargetClass::int_property(), "Must be positive");

        target.set_int(5);

        assert_eq!(target.int(), 5);
        assert_eq!(data.must_be_positive(), 5);
        assert_no_error(&target, TargetClass::int_property());
    }

    fn indei_validation_updates_data_validation_when_writing_to_source_one_way_to_source(f) {
        // Issue #8235: validation errors should be displayed for
        // one-way-to-source bindings.
        let data = IndeiViewModel::new();
        let target = create_target_with_source(
            f,
            src(&data),
            &Path::of(IndeiViewModel::must_be_positive_step()),
            validated().with_mode(BindingMode::OneWayToSource),
        );

        assert_eq!(data.must_be_positive(), 0);
        assert_no_error(&target, TargetClass::int_property());

        target.set_int(5);

        assert_eq!(data.must_be_positive(), 5);
        assert_no_error(&target, TargetClass::int_property());

        target.set_int(-5);

        assert_eq!(data.must_be_positive(), -5);
        assert_data_validation_error(&target, TargetClass::int_property(), "Must be positive");

        target.set_int(5);

        assert_eq!(data.must_be_positive(), 5);
        assert_no_error(&target, TargetClass::int_property());
    }

    fn conversion_error_is_cleared_when_value_becomes_valid_one_way_to_source(f) {
        // Issue #15378.
        let data = ViewModel::new();
        let target = create_target_with_source(
            f,
            src(&data),
            &Path::of(ViewModel::double_step()),
            validated().with_mode(BindingMode::OneWayToSource).with_property(TargetClass::object_property()),
        );

        target.set_object(Some(boxed(5.0)));

        assert_eq!(data.double_value(), 5.0);
        assert_no_error(&target, TargetClass::object_property());

        target.set_object(None);

        assert_binding_error(
            &target,
            TargetClass::object_property(),
            "Could not convert '(null)' (null) to f64.",
            BindingErrorType::DataValidationError,
        );

        target.set_object(Some(boxed(5.0)));

        assert_eq!(data.double_value(), 5.0);
        assert_no_error(&target, TargetClass::object_property());
    }

    fn does_not_subscribe_to_indei_of_intermediate_object_in_chain(f) {
        let data = IndeiContainerViewModel::new(Some(IndeiViewModel::new()));
        let _target = create_target_with_source(
            f,
            src(&data),
            &Path::of(IndeiContainerViewModel::inner_step()).then(IndeiViewModel::must_be_positive_step()),
            validated().with_mode(BindingMode::TwoWay),
        );

        // Data validation on an intermediate object in a chain is not
        // observed.
        assert_eq!(data.errors_changed.handler_count(), 0);
        assert_eq!(data.inner().unwrap().errors_changed_subscription_count(), 1);
    }

    fn updates_data_validation_for_null_value_in_property_chain(f) {
        let data = IndeiContainerViewModel::new(None);
        let target = create_target_with_source(
            f,
            src(&data),
            &Path::of(IndeiContainerViewModel::inner_step()).then(IndeiViewModel::must_be_positive_step()),
            validated().with_mode(BindingMode::TwoWay),
        );

        assert_binding_chain_error(
            &target,
            TargetClass::int_property(),
            "Value is null.",
            "Inner.MustBePositive",
            "Inner",
        );
    }

    fn setting_valid_value_should_clear_binding_error(f) {
        let data = ViewModel::with_double(5.6);
        let target = create_target_with_source(
            f,
            src(&data),
            &Path::of(ViewModel::double_step()),
            validated().with_mode(BindingMode::TwoWay).with_property(TargetClass::string_property()),
        );

        target.set_string(Some("5.6"));
        target.set_string(Some("5.6a"));
        target.set_string(Some("5.6"));

        assert_no_error(&target, TargetClass::string_property());
    }

    fn convert_back_data_validation_error_updates_data_validation(f) {
        let data = ViewModel::with_int(1);
        let target = create_target_with_source(
            f,
            src(&data),
            &Path::of(ViewModel::int_step()),
            validated()
                .with_converter(Rc::new(InvalidIdConverter { error_type: BindingErrorType::DataValidationError }))
                .with_mode(BindingMode::TwoWay)
                .with_property(TargetClass::tag_property()),
        );

        target.set_tag(Some(boxed(s("42"))));
        assert_no_error(&target, TargetClass::tag_property());

        target.set_tag(Some(boxed(s("0x555g"))));

        assert_eq!(data.int_value(), 42);
        assert_binding_error(
            &target,
            TargetClass::tag_property(),
            "'0x555g' is not a valid ID.",
            BindingErrorType::DataValidationError,
        );
    }

    fn convert_back_error_updates_data_validation(f) {
        let data = ViewModel::with_int(1);
        let target = create_target_with_source(
            f,
            src(&data),
            &Path::of(ViewModel::int_step()),
            validated()
                .with_converter(Rc::new(InvalidIdConverter { error_type: BindingErrorType::Error }))
                .with_mode(BindingMode::TwoWay)
                .with_property(TargetClass::tag_property()),
        );

        target.set_tag(Some(boxed(s("42"))));
        assert_no_error(&target, TargetClass::tag_property());

        target.set_tag(Some(boxed(s("0x555g"))));

        assert_eq!(data.int_value(), 42);
        assert_binding_error(
            &target,
            TargetClass::tag_property(),
            "'0x555g' is not a valid ID.",
            BindingErrorType::Error,
        );
    }

    fn convert_back_notification_with_value_writes_value(f) {
        let data = ViewModel::with_int(1);
        let target = create_target_with_source(
            f,
            src(&data),
            &Path::of(ViewModel::int_step()),
            validated()
                .with_converter(Rc::new(FuncValueConverter {
                    convert_back: Box::new(|_| Some(Rc::new(BindingNotification::new(Some(boxed(42)))))),
                }))
                .with_mode(BindingMode::TwoWay)
                .with_property(TargetClass::tag_property()),
        );

        target.set_tag(Some(boxed(s("foo"))));

        assert_eq!(data.int_value(), 42);
        assert_no_error(&target, TargetClass::tag_property());
    }
}
