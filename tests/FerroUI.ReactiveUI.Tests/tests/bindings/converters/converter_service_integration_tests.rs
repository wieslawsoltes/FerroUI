//! `ConverterServiceIntegrationTests` (`ReactiveUI.Tests/Bindings/Converters/ConverterServiceIntegrationTests.cs`).
//!
//! A registry hands back the converter it holds, so the identity of the converter found is
//! compared with the one registered.

use ferroui_reactiveui::*;
use std::any::Any;
use std::marker::PhantomData;
use std::rc::Rc;

const LOW_AFFINITY: i32 = 2;

const MEDIUM_AFFINITY: i32 = 5;

const HIGH_AFFINITY: i32 = 10;

/// A fallback converter of a fixed affinity that converts nothing (`TestFallbackConverter`).
struct TestFallbackConverter(i32);

impl IBindingFallbackConverter for TestFallbackConverter {
    fn get_affinity_for_objects(&self, _from_type: ConverterType, _to_type: ConverterType) -> i32 {
        self.0
    }

    fn try_convert(&self, _from_type: ConverterType, _from: &dyn Any, _to_type: ConverterType, _conversion_hint: Option<&dyn Any>) -> Option<Rc<dyn Any>> {
        None
    }
}

/// A set-method converter of a fixed affinity that sets the value as it is (`TestSetMethodConverter`).
struct TestSetMethodConverter(i32);

impl ISetMethodBindingConverter for TestSetMethodConverter {
    fn get_affinity_for_objects(&self, _from_type: Option<ConverterType>, _to_type: Option<ConverterType>) -> i32 {
        self.0
    }

    fn perform_set(&self, _to_target: Option<Rc<dyn Any>>, new_value: Option<Rc<dyn Any>>, _arguments: Option<&[Option<Rc<dyn Any>>]>) -> Option<Rc<dyn Any>> {
        new_value
    }
}

/// A typed converter from `F` to `T` of a fixed affinity that converts nothing (`TestTypedConverter<TFrom, TTo>`).
struct TestTypedConverter<F, T> {
    affinity: i32,
    _types: PhantomData<fn(F) -> T>,
}

impl<F: 'static, T: 'static> TestTypedConverter<F, T> {
    fn new(affinity: i32) -> Rc<dyn IBindingTypeConverter> {
        Rc::new(Self { affinity, _types: PhantomData })
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

fn same<T: ?Sized, U: ?Sized>(first: &Rc<T>, second: &Rc<U>) -> bool {
    std::ptr::addr_eq(Rc::as_ptr(first), Rc::as_ptr(second))
}

/// The typed converter `service` resolves from `F` to `T`, if it resolves a typed one.
fn resolved_typed<F: 'static, T: 'static>(service: &ConverterService) -> Option<Rc<dyn IBindingTypeConverter>> {
    service.resolve_converter(ConverterType::of::<F>(), ConverterType::of::<T>())?.as_typed().cloned()
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
    const CUSTOM_AFFINITY: i32 = 100;
    let service = ConverterService::new();
    let default_converter = TestTypedConverter::<i32, String>::new(LOW_AFFINITY);
    let custom_converter = TestTypedConverter::<i32, String>::new(CUSTOM_AFFINITY);

    service.typed_converters().register(default_converter);
    service.typed_converters().register(custom_converter.clone());

    let result = resolved_typed::<i32, String>(&service);

    assert!(result.is_some_and(|result| same(&result, &custom_converter)));
}

#[test]
fn fallback_converter_should_be_used_when_no_typed_match() {
    const FALLBACK_AFFINITY: i32 = 3;
    let service = ConverterService::new();
    let typed_converter = TestTypedConverter::<i32, String>::new(MEDIUM_AFFINITY);
    let fallback_converter: Rc<dyn IBindingFallbackConverter> = Rc::new(TestFallbackConverter(FALLBACK_AFFINITY));

    service.typed_converters().register(typed_converter);
    service.fallback_converters().register(fallback_converter.clone());

    // Another pair of types than i32 to String.
    let result = service.resolve_converter(ConverterType::of::<f64>(), ConverterType::of::<bool>());

    assert!(result.as_ref().and_then(ResolvedConverter::as_fallback).is_some_and(|result| same(result, &fallback_converter)));
}

#[test]
fn multiple_fallback_converters_should_select_highest_affinity() {
    let service = ConverterService::new();
    let low_affinity: Rc<dyn IBindingFallbackConverter> = Rc::new(TestFallbackConverter(LOW_AFFINITY));
    let medium_affinity: Rc<dyn IBindingFallbackConverter> = Rc::new(TestFallbackConverter(MEDIUM_AFFINITY));
    let high_affinity: Rc<dyn IBindingFallbackConverter> = Rc::new(TestFallbackConverter(HIGH_AFFINITY));

    service.fallback_converters().register(medium_affinity);
    service.fallback_converters().register(low_affinity);
    service.fallback_converters().register(high_affinity.clone());

    let result = service.resolve_converter(ConverterType::of::<i32>(), ConverterType::of::<String>());

    assert!(result.as_ref().and_then(ResolvedConverter::as_fallback).is_some_and(|result| same(result, &high_affinity)));
}

#[test]
fn no_converter_should_return_null() {
    let service = ConverterService::new();
    service.typed_converters().register(TestTypedConverter::<i32, String>::new(MEDIUM_AFFINITY));

    let result = service.resolve_converter(ConverterType::of::<f64>(), ConverterType::of::<bool>());

    assert!(result.is_none());
}

#[test]
fn real_converters_should_resolve_correctly() {
    let service = ConverterService::new();
    let int_to_string: Rc<dyn IBindingTypeConverter> = Rc::new(IntegerToStringTypeConverter);
    let string_to_int: Rc<dyn IBindingTypeConverter> = Rc::new(StringToIntegerTypeConverter);
    let equality: Rc<dyn IBindingTypeConverter> = Rc::new(EqualityTypeConverter);

    service.typed_converters().register(int_to_string.clone());
    service.typed_converters().register(string_to_int.clone());
    service.typed_converters().register(equality);

    let result1 = resolved_typed::<i32, String>(&service);
    let result2 = resolved_typed::<String, i32>(&service);

    assert!(result1.is_some_and(|result| same(&result, &int_to_string)));
    assert!(result2.is_some_and(|result| same(&result, &string_to_int)));
}

#[test]
fn binding_converters_current_should_be_accessible() {
    let service = Rc::new(ConverterService::new());
    let converter = TestTypedConverter::<i32, String>::new(MEDIUM_AFFINITY);
    service.typed_converters().register(converter.clone());

    BindingConverters::set_service(service);
    let result = resolved_typed::<i32, String>(&BindingConverters::current());

    assert!(result.is_some_and(|result| same(&result, &converter)));

    // Back to an empty service.
    BindingConverters::set_service(Rc::new(ConverterService::new()));
}

#[test]
fn set_method_converter_should_be_retrievable() {
    const SET_METHOD_AFFINITY: i32 = 8;
    let service = ConverterService::new();
    let set_method_converter: Rc<dyn ISetMethodBindingConverter> = Rc::new(TestSetMethodConverter(SET_METHOD_AFFINITY));

    service.set_method_converters().register(set_method_converter.clone());

    let result = service.set_method_converters().try_get_converter(Some(ConverterType::of::<i32>()), Some(ConverterType::of::<String>()));

    assert!(result.is_some_and(|result| same(&result, &set_method_converter)));
}

#[test]
fn typed_converter_should_be_preferred_over_fallback() {
    let service = ConverterService::new();
    let typed_converter = TestTypedConverter::<i32, String>::new(LOW_AFFINITY);
    // A higher affinity, which loses to the typed converter all the same.
    let fallback_converter: Rc<dyn IBindingFallbackConverter> = Rc::new(TestFallbackConverter(HIGH_AFFINITY));

    service.typed_converters().register(typed_converter.clone());
    service.fallback_converters().register(fallback_converter);

    let result = resolved_typed::<i32, String>(&service);

    assert!(result.is_some_and(|result| same(&result, &typed_converter)));
}

#[test]
fn zero_affinity_converter_should_be_ignored_in_resolution() {
    let service = ConverterService::new();
    let zero_affinity = TestTypedConverter::<i32, String>::new(0);
    let valid_affinity = TestTypedConverter::<i32, String>::new(LOW_AFFINITY);

    service.typed_converters().register(zero_affinity);
    service.typed_converters().register(valid_affinity.clone());

    let result = resolved_typed::<i32, String>(&service);

    assert!(result.is_some_and(|result| same(&result, &valid_affinity)));
}
