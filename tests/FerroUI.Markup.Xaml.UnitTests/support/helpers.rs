//! Small helpers the suites share: untyped values and the assertion of a
//! XAML failure.

use std::rc::Rc;

use ferroui_base::metadata::from_markup_value;
use ferroui_base::{BoxedValue, PropertyValue};
use ferroui_markup_xaml::XamlLoadException;

use super::loader::{describe, xaml_error, XamlResult};

/// Boxes a value as an untyped value (`object`).
pub fn boxed<T: PropertyValue>(value: T) -> BoxedValue {
    Rc::new(value)
}

/// An untyped string value.
pub fn boxed_str(value: &str) -> Option<BoxedValue> {
    Some(Rc::new(value.to_string()))
}

/// The value of type `T` an untyped value holds (`value as T`), with the
/// assignability casts of untyped values (base and interface handles,
/// nullable forms).
pub fn value_of<T: Clone + 'static>(value: &Option<BoxedValue>) -> Option<T> {
    from_markup_value::<T>(value)
}

/// The string an untyped value holds, if it holds one.
pub fn string_of(value: &Option<BoxedValue>) -> Option<String> {
    let value = value.as_ref()?;
    if let Some(text) = value.downcast_ref::<String>() {
        return Some(text.clone());
    }
    value.downcast_ref::<Option<String>>().cloned().flatten()
}

/// `Assert.Equal("text", value)` for an untyped value.
#[track_caller]
pub fn assert_string(expected: &str, value: &Option<BoxedValue>) {
    assert_eq!(Some(expected.to_string()), string_of(value), "the value is not the string {expected:?}");
}

/// `Assert.Equal(expected, value)` for an untyped value that must hold a
/// `T`.
#[track_caller]
pub fn assert_value<T: Clone + PartialEq + std::fmt::Debug + 'static>(expected: T, value: &Option<BoxedValue>) {
    assert_eq!(Some(expected), value_of::<T>(value));
}

/// `Assert.IsType<T>(value)`: the untyped value holds exactly a `T`.
pub fn is_type<T: 'static>(value: &Option<BoxedValue>) -> bool {
    value.as_ref().is_some_and(|value| value.downcast_ref::<T>().is_some())
}

/// The name of the type an untyped value holds ("null" for null), for the
/// message of a failed assertion.
pub fn value_type_name(value: &Option<BoxedValue>) -> &'static str {
    value.as_ref().map_or("null", |value| value.type_name())
}

/// `Assert.IsType<T>(value)` for an untyped value that must hold exactly
/// the Rust type `T` (a value type, or the handle of a plain class):
/// returns the value.
#[track_caller]
pub fn assert_value_is<T: Clone + 'static>(value: &Option<BoxedValue>) -> T {
    match value.as_ref().and_then(|value| value.downcast_ref::<T>()) {
        Some(value) => value.clone(),
        None => panic!("expected a value of type '{}', found '{}'", std::any::type_name::<T>(), value_type_name(value)),
    }
}

/// `XamlTestHelpers.AssertThrowsXamlException(cb)`: the load must fail with
/// an XML exception (which the XAML parse, transform and load exceptions
/// derive from).
#[track_caller]
pub fn assert_throws_xaml_exception<T>(result: XamlResult<T>) -> XamlLoadException {
    match result {
        Ok(_) => panic!("Expected to throw xaml exception"),
        Err(error) if xaml_error(&error).is_some_and(|inner| inner.is_xml_exception()) => error,
        Err(error) => panic!("Expected to throw xaml exception: {}", describe(&error)),
    }
}

/// The diagnostic code the compiler reports for a failed load
/// (`FRN2100`, ...): the code of the error of the compiler, as the
/// diagnostics of the loader carry it.
pub fn diagnostic_code(error: &XamlLoadException) -> Option<String> {
    use ferroui_markup_xaml_loader::compiler_extensions::FerroXamlDiagnosticCodes;
    use xamlx::transform::XamlDiagnosticCodeSource;

    xaml_error(error).map(|inner| {
        FerroXamlDiagnosticCodes::xaml_x_diagnostic_code_to_ferro(&XamlDiagnosticCodeSource::Exception(inner))
    })
}

/// `Assert.ThrowsAny<XmlException>(..)` for a load whose failure is pinned:
/// the load must fail with an XML exception that carries the diagnostic
/// `code` and whose message (the text followed by the position) is
/// `message`.
#[track_caller]
pub fn assert_throws_xaml_diagnostic<T>(result: XamlResult<T>, code: &str, message: &str) -> XamlLoadException {
    let error = assert_throws_xaml_exception(result);
    let inner = xaml_error(&error).expect("an XML exception is an error of the compiler");
    assert_eq!(
        (Some(code.to_string()), message.to_string()),
        (diagnostic_code(&error), inner.message()),
        "the load failed with another diagnostic: {}",
        describe(&error)
    );
    error
}

/// `Assert.ThrowsAny<XmlException>(..)`.
#[track_caller]
pub fn assert_throws_xml_exception<T>(result: XamlResult<T>) -> XamlLoadException {
    assert_throws_xaml_exception(result)
}

/// `Assert.ThrowsAny<XamlParseException>(..)`.
#[track_caller]
pub fn assert_throws_xaml_parse_exception<T>(result: XamlResult<T>) -> XamlLoadException {
    match result {
        Ok(_) => panic!("Expected a XamlParseException"),
        Err(error) if xaml_error(&error).is_some_and(|inner| inner.is_xaml_parse_exception()) => error,
        Err(error) => panic!("Expected a XamlParseException: {}", describe(&error)),
    }
}

/// Gives a shared plain test type identity equality (the reference equality
/// of a class instance), so that it can be held in untyped values.
#[macro_export]
macro_rules! test_identity_eq {
    ($($type_:ty),* $(,)?) => {$(
        impl PartialEq for $type_ {
            fn eq(&self, other: &Self) -> bool {
                ::std::ptr::addr_eq(self, other)
            }
        }
    )*};
}

/// `Assert.IsType<T>(object)` for an object of the object model: the
/// object is an instance of exactly the class `T` (not of a class deriving
/// from it).
#[track_caller]
pub fn assert_is_type<T: ferroui_base::ObjectType>(object: &ferroui_base::FerroObject) {
    assert!(
        std::ptr::eq(object.get_type(), T::TYPE),
        "expected an object of type '{}', found '{}'",
        T::TYPE.name(),
        object.get_type().name()
    );
}

/// `Assert.IsType<T>(value)` for an untyped value (a resource, a setter
/// value, a parent of the parent stack): the value is an instance of
/// exactly the class `T`. Returns the instance.
#[track_caller]
pub fn assert_value_is_type<T: ferroui_base::ObjectType>(value: &Option<BoxedValue>) -> ferroui_base::Ref<T> {
    let Some(object) = value.as_ref().and_then(|value| ferroui_base::data::core::ValueTypes::as_object(&**value)) else {
        panic!(
            "expected an object of type '{}', found '{}'",
            T::TYPE.name(),
            value.as_ref().map_or("null", |value| value.type_name())
        )
    };
    assert_is_type::<T>(&object);
    object.cast::<T>().expect("an object of exactly the class is an instance of it")
}

/// `value as T` for an untyped value that boxes a handle of the object
/// model (`Ref<Control>`, `Ref<StyledElement>`, the handle of any class):
/// the object as a `T`, or `None` when the value is null, is not an object
/// of the object model or is not a `T`.
pub fn try_object_of<T: ferroui_base::ObjectType>(value: &Option<BoxedValue>) -> Option<ferroui_base::Ref<T>> {
    value.as_ref().and_then(try_boxed_object_of::<T>)
}

/// [`try_object_of`] for a value that is not null.
pub fn try_boxed_object_of<T: ferroui_base::ObjectType>(value: &BoxedValue) -> Option<ferroui_base::Ref<T>> {
    ferroui_base::data::core::ValueTypes::as_object(&**value).and_then(|object| object.cast::<T>())
}

/// The cast `(T)value` (`(Button)window.Content`) for an untyped value that
/// boxes a handle of the object model. Panics, as the cast of the managed
/// original throws, when the value is null or not a `T`, naming the
/// expected and the actual type.
#[track_caller]
pub fn object_of<T: ferroui_base::ObjectType>(value: &Option<BoxedValue>) -> ferroui_base::Ref<T> {
    match value {
        Some(value) => boxed_object_of::<T>(value),
        None => panic!("Unable to cast null to type '{}'.", T::TYPE.name()),
    }
}

/// [`object_of`] for a value that is not null.
#[track_caller]
pub fn boxed_object_of<T: ferroui_base::ObjectType>(value: &BoxedValue) -> ferroui_base::Ref<T> {
    let Some(object) = ferroui_base::data::core::ValueTypes::as_object(&**value) else {
        panic!("Unable to cast object of type '{}' to type '{}'.", value.type_name(), T::TYPE.name())
    };
    let actual = object.get_type().name();
    object
        .cast::<T>()
        .unwrap_or_else(|| panic!("Unable to cast object of type '{actual}' to type '{}'.", T::TYPE.name()))
}

/// The cast `(Setter)setter` of an item of the setters of a style.
#[track_caller]
pub fn as_setter(setter: &Rc<dyn ferroui_base::styling::SetterBase>) -> &ferroui_base::styling::Setter {
    setter
        .as_any()
        .and_then(|setter| setter.downcast_ref::<ferroui_base::styling::Setter>())
        .unwrap_or_else(|| panic!("Unable to cast the setter to type 'Setter'."))
}

/// The cast `(Setter)setter` of an item of the setters of a key frame.
#[track_caller]
pub fn animation_setter_as_setter(
    setter: &Rc<dyn ferroui_base::animation::IAnimationSetter>,
) -> &ferroui_base::styling::Setter {
    setter
        .as_any()
        .and_then(|setter| setter.downcast_ref::<ferroui_base::styling::Setter>())
        .unwrap_or_else(|| panic!("Unable to cast the animation setter to type 'Setter'."))
}

/// How the value of a setter is held, for the message of a failed
/// assertion.
fn describe_setter_value(value: &Option<ferroui_base::styling::SetterValue>) -> String {
    use ferroui_base::styling::SetterValue;

    match value {
        None => "null".to_string(),
        Some(SetterValue::Value(value)) => format!("the plain value '{}'", value.type_name()),
        Some(SetterValue::Binding(_)) => "a source of values".to_string(),
        Some(SetterValue::BindingBase(_)) => "a binding".to_string(),
        Some(SetterValue::Template(_)) => "a template".to_string(),
    }
}

/// `setter.Value` of a setter whose value is a plain value (not a binding
/// and not a template that is built per control): the value, or `None` for
/// null. Panics when the setter holds anything else.
#[track_caller]
pub fn setter_plain_value(setter: &ferroui_base::styling::Setter) -> Option<BoxedValue> {
    match setter.value() {
        None => None,
        Some(ferroui_base::styling::SetterValue::Value(value)) => Some(value),
        other => panic!("expected the setter to hold a plain value, found {}", describe_setter_value(&other)),
    }
}

/// `setter.Value` of a setter whose value is a binding. Panics when the
/// setter holds anything else (a binding that was applied or boxed as a
/// plain value instead of assigned).
#[track_caller]
pub fn setter_binding(setter: &ferroui_base::styling::Setter) -> Rc<dyn ferroui_base::data::BindingBase> {
    match setter.value() {
        Some(ferroui_base::styling::SetterValue::BindingBase(binding)) => binding,
        other => panic!("expected the setter to hold a binding, found {}", describe_setter_value(&other)),
    }
}

/// `Assert.IsType<T>(binding)` for a binding known by its contract: the
/// binding is an instance of exactly the class `T`. Returns it as a `T`.
#[track_caller]
pub fn assert_binding_is_type<T: 'static>(binding: &Rc<dyn ferroui_base::data::BindingBase>) -> &T {
    binding
        .as_any()
        .and_then(|binding| binding.downcast_ref::<T>())
        .unwrap_or_else(|| panic!("expected a binding of type '{}'", std::any::type_name::<T>()))
}

/// `(setter.Value as TemplateBinding)?.Property`: the property a setter
/// whose value is a template binding binds to; `None` when the value is
/// not a template binding (or the binding has no property).
pub fn setter_template_binding_property(
    setter: &ferroui_base::styling::Setter,
) -> Option<&'static ferroui_base::FerroProperty> {
    match setter.value() {
        Some(ferroui_base::styling::SetterValue::BindingBase(binding)) => binding
            .as_any()
            .and_then(|binding| binding.downcast_ref::<ferroui_base::data::TemplateBinding>())
            .and_then(|binding| binding.property()),
        _ => None,
    }
}

/// `setter.Value` of a setter of a control template property: the plain
/// value of the setter, held in the value type of the property.
#[track_caller]
pub fn setter_control_template(
    setter: &ferroui_base::styling::Setter,
) -> Rc<dyn ferroui_controls::templates::IControlTemplate> {
    // The plain value of a setter holds the value type of its property.
    assert_value_is::<Option<Rc<dyn ferroui_controls::templates::IControlTemplate>>>(&setter_plain_value(setter))
        .expect("the value of the setter is null")
}

/// `Assert.IsType<T>(template)` for a control template known by its
/// contract. Returns it as a `T`.
#[track_caller]
pub fn assert_control_template_is_type<T: 'static>(
    template: &Rc<dyn ferroui_controls::templates::IControlTemplate>,
) -> &T {
    template
        .as_any()
        .and_then(|template| template.downcast_ref::<T>())
        .unwrap_or_else(|| panic!("expected a control template of type '{}'", std::any::type_name::<T>()))
}
