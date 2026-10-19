//! Ported from `Splat.Tests/ServiceLocation/GlobalGenericFirst/RegistrationTests.cs` of Splat.
//! Adapted: an instance is an `Option<T>` (`None` for `null`); a factory is an
//! `Rc<dyn Fn() -> Option<T>>`, compared by identity; `TryGetInstance(out T)` returns
//! `Option<Option<T>>` and `TryGetFactory(out ...)` an `Option` of the factory.

use crate::service_location::common::Registration;
use std::rc::Rc;

const SAMPLE_VALUE: &str = "value";

#[derive(Clone, Debug, PartialEq)]
struct TestClass {
    value: i32,
    name: Option<String>,
}

#[test]
fn from_instance_creates_instance_registration() {
    let instance = "test".to_owned();
    let registration = Registration::from_instance(Some(instance.clone()));
    assert!(!registration.is_factory());
    assert_eq!(registration.get_instance(), Some(instance));
}

#[test]
fn from_factory_creates_factory_registration() {
    let factory: Rc<dyn Fn() -> Option<String>> = Rc::new(|| Some("test".to_owned()));
    let registration = Registration::from_factory(factory.clone());
    assert!(registration.is_factory());
    let result = registration.get_factory().expect("a factory");
    assert!(std::ptr::addr_eq(Rc::as_ptr(&result), Rc::as_ptr(&factory)));
}

#[test]
fn get_instance_returns_correct_instance() {
    let registration = Registration::from_instance(Some(42));
    assert_eq!(registration.get_instance(), Some(42));
}

#[test]
fn get_factory_returns_correct_factory() {
    let expected_value = "factory result";
    let registration: Registration<String> =
        Registration::from_factory(Rc::new(move || Some(expected_value.to_owned())));
    let result_factory = registration.get_factory().expect("a factory");
    assert_eq!(result_factory(), Some(expected_value.to_owned()));
}

#[test]
fn instance_registration_with_null_value_stores_null() {
    let registration: Registration<String> = Registration::from_instance(None);
    assert!(!registration.is_factory());
    assert_eq!(registration.get_instance(), None);
}

#[test]
fn factory_registration_that_returns_null_works_correctly() {
    let registration: Registration<String> = Registration::from_factory(Rc::new(|| None));
    assert!(registration.is_factory());
    let result_factory = registration.try_get_factory();
    assert!(result_factory.is_some());
    assert_eq!(result_factory.unwrap()(), None);
}

#[test]
fn registration_is_value_type_can_be_compared() {
    let reg1 = Registration::from_instance(Some("test".to_owned()));
    let reg2 = Registration::from_instance(Some("test".to_owned()));
    assert!(reg1 == reg2);
}

#[test]
fn factory_registration_with_different_factories_are_not_equal() {
    let reg1: Registration<String> = Registration::from_factory(Rc::new(|| Some("test1".to_owned())));
    let reg2: Registration<String> = Registration::from_factory(Rc::new(|| Some("test2".to_owned())));
    assert!(reg1 != reg2);
}

#[test]
fn registration_with_complex_type_works_correctly() {
    let complex_object = TestClass { value: 42, name: Some("Test".to_owned()) };
    let registration = Registration::from_instance(Some(complex_object.clone()));
    let result = registration.get_instance().expect("an instance");
    assert_eq!(result, complex_object);
    assert_eq!(result.value, 42);
    assert_eq!(result.name.as_deref(), Some("Test"));
}

#[test]
fn factory_registration_with_complex_type_works_correctly() {
    let registration: Registration<TestClass> =
        Registration::from_factory(Rc::new(|| Some(TestClass { value: 100, name: Some("Factory".to_owned()) })));
    let result_factory = registration.get_factory().expect("a factory");
    let result = result_factory().expect("a value");
    assert_eq!(result.value, 100);
    assert_eq!(result.name.as_deref(), Some("Factory"));
}

#[test]
fn get_instance_on_factory_registration_returns_default() {
    let registration: Registration<String> = Registration::from_factory(Rc::new(|| Some(SAMPLE_VALUE.to_owned())));
    assert_eq!(registration.get_instance(), None);
}

#[test]
fn get_factory_on_instance_registration_returns_null() {
    let registration = Registration::from_instance(Some(SAMPLE_VALUE.to_owned()));
    assert!(registration.get_factory().is_none());
}

#[test]
fn try_get_instance_on_instance_registration_returns_true_with_instance() {
    let registration = Registration::from_instance(Some(SAMPLE_VALUE.to_owned()));
    let result = registration.try_get_instance();
    assert_eq!(result, Some(Some(SAMPLE_VALUE.to_owned())));
}

#[test]
fn try_get_instance_on_factory_registration_returns_false() {
    let registration: Registration<String> = Registration::from_factory(Rc::new(|| Some(SAMPLE_VALUE.to_owned())));
    assert_eq!(registration.try_get_instance(), None);
}
