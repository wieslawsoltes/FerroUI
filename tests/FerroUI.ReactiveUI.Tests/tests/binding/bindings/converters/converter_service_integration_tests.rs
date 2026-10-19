//! `ConverterServiceIntegrationTests` (`ReactiveUI.Binding.Tests/Bindings/Converters/ConverterServiceIntegrationTests.cs`).

use crate::tests::binding::bindings::type_converters::same_object;
use ferroui_reactiveui::*;
use std::any::Any;
use std::marker::PhantomData;
use std::rc::Rc;

const LOW_AFFINITY: i32 = 2;
const BELOW_DEFAULT_AFFINITY: i32 = 3;
const DEFAULT_AFFINITY: i32 = 5;
const ABOVE_DEFAULT_AFFINITY: i32 = 8;
const HIGH_AFFINITY: i32 = 10;
const HIGHEST_AFFINITY: i32 = 100;

fn int_type() -> ConverterType {
    ConverterType::of::<i32>()
}

fn string_type() -> ConverterType {
    ConverterType::of::<String>()
}

/// Whether `resolved` is the typed converter `expected`.
fn is_typed<T: ?Sized>(resolved: &Option<ResolvedConverter>, expected: &Rc<T>) -> bool {
    resolved.as_ref().and_then(ResolvedConverter::as_typed).is_some_and(|typed| same_object(typed, expected))
}

/// Whether `resolved` is the fallback converter `expected`.
fn is_fallback<T: ?Sized>(resolved: &Option<ResolvedConverter>, expected: &Rc<T>) -> bool {
    resolved
        .as_ref()
        .and_then(ResolvedConverter::as_fallback)
        .is_some_and(|fallback| same_object(fallback, expected))
}

#[test]
fn converter_service_should_expose_all_registries() {
    let service = ConverterService::new();

    assert!(service.typed_converters().get_all_converters().is_empty());
    assert!(service.fallback_converters().get_all_converters().is_empty());
    assert!(service.set_method_converters().get_all_converters().is_empty());
}

#[test]
fn custom_high_affinity_converter_should_override_default() {
    let service = ConverterService::new();
    let default_converter = TestTypedConverter::<i32, String>::new(LOW_AFFINITY);
    let custom_converter = TestTypedConverter::<i32, String>::new(HIGHEST_AFFINITY);
    service.typed_converters().register(default_converter);
    service.typed_converters().register(custom_converter.clone());

    let result = service.resolve_converter(int_type(), string_type());

    assert!(is_typed(&result, &custom_converter));
}

#[test]
fn fallback_converter_should_be_used_when_no_typed_match() {
    let service = ConverterService::new();
    let typed_converter = TestTypedConverter::<i32, String>::new(DEFAULT_AFFINITY);
    let fallback_converter = TestFallbackConverter::new(BELOW_DEFAULT_AFFINITY);
    service.typed_converters().register(typed_converter);
    service.fallback_converters().register(fallback_converter.clone());

    // Another pair than i32 to String.
    let result = service.resolve_converter(ConverterType::of::<f64>(), ConverterType::of::<bool>());

    assert!(is_fallback(&result, &fallback_converter));
}

#[test]
fn multiple_fallback_converters_should_select_highest_affinity() {
    let service = ConverterService::new();
    let low_affinity = TestFallbackConverter::new(LOW_AFFINITY);
    let medium_affinity = TestFallbackConverter::new(DEFAULT_AFFINITY);
    let high_affinity = TestFallbackConverter::new(HIGH_AFFINITY);
    service.fallback_converters().register(medium_affinity);
    service.fallback_converters().register(low_affinity);
    service.fallback_converters().register(high_affinity.clone());

    let result = service.resolve_converter(int_type(), string_type());

    assert!(is_fallback(&result, &high_affinity));
}

#[test]
fn no_converter_should_return_null() {
    let service = ConverterService::new();
    service.typed_converters().register(TestTypedConverter::<i32, String>::new(DEFAULT_AFFINITY));

    let result = service.resolve_converter(ConverterType::of::<f64>(), ConverterType::of::<bool>());

    assert!(result.is_none());
}

#[test]
fn real_converters_should_resolve_correctly() {
    let service = ConverterService::new();
    let int_to_string = Rc::new(IntegerToStringTypeConverter);
    let string_to_int = Rc::new(StringToIntegerTypeConverter);
    let equality = Rc::new(EqualityTypeConverter);
    service.typed_converters().register(int_to_string.clone());
    service.typed_converters().register(string_to_int.clone());
    service.typed_converters().register(equality);

    let result1 = service.resolve_converter(int_type(), string_type());
    let result2 = service.resolve_converter(string_type(), int_type());

    assert!(is_typed(&result1, &int_to_string));
    assert!(is_typed(&result2, &string_to_int));
}

#[test]
fn binding_converters_current_should_be_accessible() {
    let service = Rc::new(ConverterService::new());
    let converter = TestTypedConverter::<i32, String>::new(DEFAULT_AFFINITY);
    service.typed_converters().register(converter.clone());

    BindingConverters::set_service(service);
    let result = BindingConverters::current().resolve_converter(int_type(), string_type());

    assert!(is_typed(&result, &converter));

    // Cleanup - reset to default
    BindingConverters::set_service(Rc::new(ConverterService::new()));
}

#[test]
fn set_method_converter_should_be_retrievable() {
    let service = ConverterService::new();
    let set_method_converter = TestSetMethodConverter::new(ABOVE_DEFAULT_AFFINITY);
    service.set_method_converters().register(set_method_converter.clone());

    let result = service.set_method_converters().try_get_converter(Some(int_type()), Some(string_type()));

    assert!(result.is_some_and(|result| same_object(&result, &set_method_converter)));
}

#[test]
fn typed_converter_should_be_preferred_over_fallback() {
    let service = ConverterService::new();
    let typed_converter = TestTypedConverter::<i32, String>::new(LOW_AFFINITY);
    // A higher affinity, which still loses to the typed converter.
    let fallback_converter = TestFallbackConverter::new(HIGH_AFFINITY);
    service.typed_converters().register(typed_converter.clone());
    service.fallback_converters().register(fallback_converter);

    let result = service.resolve_converter(int_type(), string_type());

    assert!(is_typed(&result, &typed_converter));
}

#[test]
fn zero_affinity_converter_should_be_ignored_in_resolution() {
    let service = ConverterService::new();
    let zero_affinity = TestTypedConverter::<i32, String>::new(0);
    let valid_affinity = TestTypedConverter::<i32, String>::new(LOW_AFFINITY);
    service.typed_converters().register(zero_affinity);
    service.typed_converters().register(valid_affinity.clone());

    let result = service.resolve_converter(int_type(), string_type());

    assert!(is_typed(&result, &valid_affinity));
}

/// A fallback converter of the given affinity that never converts (`TestFallbackConverter`).
struct TestFallbackConverter {
    base_affinity: i32,
}

impl TestFallbackConverter {
    fn new(base_affinity: i32) -> Rc<Self> {
        Rc::new(Self { base_affinity })
    }
}

impl IBindingFallbackConverter for TestFallbackConverter {
    fn get_affinity_for_objects(&self, _from_type: ConverterType, _to_type: ConverterType) -> i32 {
        self.base_affinity
    }

    fn try_convert(
        &self,
        _from_type: ConverterType,
        _from: &dyn Any,
        _to_type: ConverterType,
        _conversion_hint: Option<&dyn Any>,
    ) -> Option<Rc<dyn Any>> {
        None
    }
}

/// A set-method converter of the given affinity that returns the new value
/// (`TestSetMethodConverter`).
struct TestSetMethodConverter {
    base_affinity: i32,
}

impl TestSetMethodConverter {
    fn new(base_affinity: i32) -> Rc<Self> {
        Rc::new(Self { base_affinity })
    }
}

impl ISetMethodBindingConverter for TestSetMethodConverter {
    fn get_affinity_for_objects(&self, _from_type: Option<ConverterType>, _to_type: Option<ConverterType>) -> i32 {
        self.base_affinity
    }

    fn perform_set(
        &self,
        _to_target: Option<Rc<dyn Any>>,
        new_value: Option<Rc<dyn Any>>,
        _arguments: Option<&[Option<Rc<dyn Any>>]>,
    ) -> Option<Rc<dyn Any>> {
        new_value
    }
}

/// A typed converter of the given affinity that never converts (`TestTypedConverter<TFrom, TTo>`).
struct TestTypedConverter<F, T> {
    affinity: i32,
    types: PhantomData<fn(F) -> T>,
}

impl<F: 'static, T: 'static> TestTypedConverter<F, T> {
    fn new(affinity: i32) -> Rc<Self> {
        Rc::new(Self { affinity, types: PhantomData })
    }
}

impl<F: 'static, T: 'static> BindingTypeConverter for TestTypedConverter<F, T> {
    type From = F;
    type To = T;

    fn affinity(&self) -> i32 {
        self.affinity
    }

    fn convert(&self, _from: Option<&F>, _conversion_hint: Option<&dyn Any>) -> Option<T> {
        None
    }
}
