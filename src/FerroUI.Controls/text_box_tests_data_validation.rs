//! The data validation tests of the text box.
//!
//! A setter of the reference models that throws is a validated property
//! here: its setter returns the error.

use crate::presenters::TextPresenter;
use crate::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions, IControlTemplate};
use crate::test_support::test_scope;
use crate::{Control, DataValidationErrors, ErrorConverter, TextBox};
use ferroui_base::controls::NameScopeRef;
use ferroui_base::data::core::plugins::PropertyInfoAccessorFactory;
use ferroui_base::data::core::{ClrPropertyInfo, Maybe, Value};
use ferroui_base::data::model::{Event, INotifyDataErrorInfo, INotifyPropertyChanged, Model};
use ferroui_base::data::{
    BindingError, BindingMode, BindingPriority, CompiledBinding, CompiledBindingPathBuilder, ReflectionBinding,
    RelativeSource, RelativeSourceMode,
};
use ferroui_base::{ferro_model, AnyValue, BoxedValue, Ref};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;

fn build_template(_control: &Ref<TextBox>, scope: &NameScopeRef) -> Ref<Control> {
    let binding = ReflectionBinding::new("Text").with_mode(BindingMode::TwoWay);
    binding.set_priority(BindingPriority::Template);
    binding.set_relative_source(Some(RelativeSource::new(RelativeSourceMode::TemplatedParent)));

    let presenter = TextPresenter::new();
    presenter.set_name(Some("PART_TextPresenter".to_string()));
    presenter.bind_binding(TextPresenter::text_property().as_property(), &binding);

    presenter.register_in_name_scope(&**scope).upcast()
}

fn create_template() -> Option<Rc<dyn IControlTemplate>> {
    Some(FuncControlTemplate::for_type::<TextBox>(build_template))
}

/// The error of a setter that rejects a value.
#[derive(Debug)]
struct InvalidOperationError(&'static str);

impl fmt::Display for InvalidOperationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

impl std::error::Error for InvalidOperationError {}

struct ExceptionTest {
    less_than_10: Cell<i32>,
    property_changed: Event<str>,
}

impl ExceptionTest {
    fn new() -> Rc<Self> {
        Model::new_model(Self { less_than_10: Cell::new(0), property_changed: Event::new() })
    }

    fn less_than_10(&self) -> i32 {
        self.less_than_10.get()
    }

    fn set_less_than_10(&self, value: i32) -> Result<(), BindingError> {
        if value < 10 {
            self.less_than_10.set(value);
            Ok(())
        } else {
            Err(BindingError::new(InvalidOperationError("More than 10.")))
        }
    }
}

impl INotifyPropertyChanged for ExceptionTest {
    fn property_changed(&self) -> &Event<str> {
        &self.property_changed
    }
}

ferro_model!(ExceptionTest, |b| b.notify_property_changed().validated_property::<Value<i32>>(
    "LessThan10",
    |o| o.less_than_10(),
    |o, v| o.set_less_than_10(v)
));

struct IndeiStringTest {
    errors: RefCell<HashMap<String, Vec<String>>>,
    value: RefCell<Option<String>>,
    errors_changed: Event<str>,
    property_changed: Event<str>,
}

impl IndeiStringTest {
    fn new() -> Rc<Self> {
        Model::new_model(Self {
            errors: RefCell::new(HashMap::new()),
            value: RefCell::new(None),
            errors_changed: Event::new(),
            property_changed: Event::new(),
        })
    }

    fn value(&self) -> Option<String> {
        self.value.borrow().clone()
    }

    fn set_value(&self, value: Option<String>) {
        let bad = value.as_deref() == Some("bad");
        *self.value.borrow_mut() = value;
        if bad {
            self.errors.borrow_mut().insert("Value".to_string(), vec!["Invalid".to_string()]);
        } else {
            self.errors.borrow_mut().remove("Value");
        }
        self.errors_changed.raise("Value");
    }
}

impl INotifyPropertyChanged for IndeiStringTest {
    fn property_changed(&self) -> &Event<str> {
        &self.property_changed
    }
}

impl INotifyDataErrorInfo for IndeiStringTest {
    fn has_errors(&self) -> bool {
        !self.errors.borrow().is_empty()
    }

    fn get_errors(&self, property_name: Option<&str>) -> Vec<BoxedValue> {
        match property_name.and_then(|name| self.errors.borrow().get(name).cloned()) {
            Some(errors) => errors
                .into_iter()
                .map(|error| {
                    let error: BoxedValue = Rc::new(error);
                    error
                })
                .collect(),
            None => Vec::new(),
        }
    }

    fn errors_changed(&self) -> &Event<str> {
        &self.errors_changed
    }
}

ferro_model!(IndeiStringTest, |b| b
    .notify_property_changed()
    .notify_data_error_info()
    .property::<Maybe<String>>("Value", |o| o.value(), |o, v| o.set_value(v)));

/// A text box whose text is bound both ways to `LessThan10` of a new model.
fn exception_test_target() -> Ref<TextBox> {
    let target = TextBox::new();
    target.set_data_context(Some(ExceptionTest::new()));
    target.bind_binding(
        TextBox::text_property().as_property(),
        &ReflectionBinding::new("LessThan10").with_mode(BindingMode::TwoWay),
    );
    target.set_template(create_template());
    target
}

/// The error of a setter, if an error of a control is one.
fn as_invalid_operation(error: &BoxedValue) -> Option<String> {
    let error: &dyn AnyValue = &**error;
    let error = error.downcast_ref::<BindingError>()?;
    error.inner().downcast_ref::<InvalidOperationError>().map(|error| error.to_string())
}

#[test]
fn setter_exceptions_should_set_error_pseudoclass() {
    let _scope = test_scope();
    let target = exception_test_target();

    target.apply_template();

    assert!(!target.classes().contains(":error"));
    target.set_text(Some("20"));
    assert!(target.classes().contains(":error"));
    target.set_text(Some("1"));
    assert!(!target.classes().contains(":error"));
}

#[test]
fn setter_exceptions_should_set_data_validation_errors_errors() {
    let _scope = test_scope();
    let target = exception_test_target();

    target.apply_template();

    assert!(DataValidationErrors::get_errors(&target).is_none());
    target.set_text(Some("20"));

    let errors = DataValidationErrors::get_errors(&target).expect("the errors");
    assert_eq!(1, errors.len());
    assert_eq!(Some("More than 10."), as_invalid_operation(&errors[0]).as_deref());
    target.set_text(Some("1"));
    assert!(DataValidationErrors::get_errors(&target).is_none());
}

#[test]
fn setter_exceptions_should_be_converter_if_error_converter_set() {
    let _scope = test_scope();
    let target = exception_test_target();
    DataValidationErrors::set_error_converter(
        &target,
        Some(ErrorConverter::new(|err| {
            let converted: BoxedValue =
                Rc::new(format!("Error: {}", as_invalid_operation(err).unwrap_or_default()));
            Some(converted)
        })),
    );

    target.apply_template();

    target.set_text(Some("20"));

    let errors = DataValidationErrors::get_errors(&target).expect("the errors");
    assert_eq!(1, errors.len());
    let error: &dyn AnyValue = &*errors[0];
    let error = error.downcast_ref::<String>().expect("a string");
    assert!(error.starts_with("Error: "));
}

#[test]
fn setter_exceptions_should_set_data_validation_errors_has_errors() {
    let _scope = test_scope();
    let target = exception_test_target();

    target.apply_template();

    assert!(!DataValidationErrors::get_has_errors(&target));
    target.set_text(Some("20"));
    assert!(DataValidationErrors::get_has_errors(&target));
    target.set_text(Some("1"));
    assert!(!DataValidationErrors::get_has_errors(&target));
}

#[test]
fn compiled_bindings_type_converter_exceptions_should_set_data_validation_errors_has_errors() {
    let _scope = test_scope();

    let path = CompiledBindingPathBuilder::new()
        .property(
            Rc::new(ClrPropertyInfo::read_write_validated::<ExceptionTest, Value<i32>>(
                "LessThan10",
                |target| target.less_than_10(),
                |target, value| target.set_less_than_10(value),
            )),
            PropertyInfoAccessorFactory::create_inpc_property_accessor::<ExceptionTest>(),
        )
        .build();

    let target = TextBox::new();
    target.set_data_context(Some(ExceptionTest::new()));
    target.bind_binding(
        TextBox::text_property().as_property(),
        &CompiledBinding::new(path).with_source(Some(ExceptionTest::new())).with_mode(BindingMode::TwoWay),
    );
    target.set_template(create_template());

    target.apply_template();

    target.set_text(Some("a"));
    assert!(DataValidationErrors::get_has_errors(&target));
}

#[test]
fn compiled_binding_to_data_validation_property_reports_data_validation_errors() {
    let _scope = test_scope();

    // This binding is shape-eligible for the typed binding expression (a directly
    // assignable single-property data context binding), which does not support data
    // validation. Because the text of a text box enables data validation it must fall back to the
    // untyped binding expression and still surface validation errors.
    let path = CompiledBindingPathBuilder::new()
        .typed_property::<IndeiStringTest, Option<String>>(
            "Value",
            |o| o.value(),
            Some(Rc::new(|o: &IndeiStringTest, v| o.set_value(v))),
        )
        .build();

    let target = TextBox::new();
    target.set_data_context(Some(IndeiStringTest::new()));
    target.bind_binding(
        TextBox::text_property().as_property(),
        &CompiledBinding::new(path).with_mode(BindingMode::TwoWay),
    );
    target.set_template(create_template());

    target.apply_template();

    assert!(!DataValidationErrors::get_has_errors(&target));
    target.set_text(Some("bad"));
    assert!(DataValidationErrors::get_has_errors(&target));
    target.set_text(Some("good"));
    assert!(!DataValidationErrors::get_has_errors(&target));
}
