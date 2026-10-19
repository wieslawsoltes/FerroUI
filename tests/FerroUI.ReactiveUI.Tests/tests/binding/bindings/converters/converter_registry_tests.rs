//! `ConverterRegistryTests` (`ReactiveUI.Binding.Tests/Bindings/Converters/ConverterRegistryTests.cs`).

use crate::tests::binding::bindings::type_converters::same_object;
use ferroui_reactiveui::*;
use std::any::Any;
use std::marker::PhantomData;
use std::rc::Rc;

const NEGATIVE_AFFINITY: i32 = -5;
const DEFAULT_AFFINITY: i32 = 5;
const ABOVE_DEFAULT_AFFINITY: i32 = 8;
const HIGH_AFFINITY: i32 = 10;
const EXPECTED_THREE_CONVERTERS: i32 = 3;
const EXPECTED_TWO_CONVERTERS: i32 = 2;

fn int_type() -> ConverterType {
    ConverterType::of::<i32>()
}

fn string_type() -> ConverterType {
    ConverterType::of::<String>()
}

// Not ported: ConcurrentReads_DuringRegistration_ShouldBeThreadSafe - real threads (the registries are per thread)

#[test]
fn converter_with_negative_affinity_should_be_ignored() {
    let registry = BindingTypeConverterRegistry::new();
    let negative_affinity = TestConverter::<i32, String>::new(NEGATIVE_AFFINITY);
    let valid_affinity = TestConverter::<i32, String>::new(EXPECTED_TWO_CONVERTERS);

    registry.register(negative_affinity);
    registry.register(valid_affinity.clone());
    let selected = registry.try_get_converter(int_type(), string_type()).expect("a converter");

    assert!(same_object(&selected, &valid_affinity));
}

#[test]
fn converter_with_zero_affinity_should_be_ignored() {
    let registry = BindingTypeConverterRegistry::new();
    let zero_affinity = TestConverter::<i32, String>::new(0);
    let valid_affinity = TestConverter::<i32, String>::new(EXPECTED_TWO_CONVERTERS);

    registry.register(zero_affinity);
    registry.register(valid_affinity.clone());
    let selected = registry.try_get_converter(int_type(), string_type()).expect("a converter");

    assert!(same_object(&selected, &valid_affinity));
}

#[test]
fn empty_registry_should_return_null() {
    let registry = BindingTypeConverterRegistry::new();
    assert!(registry.try_get_converter(int_type(), string_type()).is_none());
}

#[test]
fn fallback_registry_should_select_highest_affinity() {
    let registry = BindingFallbackConverterRegistry::new();
    let low_affinity = TestFallbackConverter::new(EXPECTED_TWO_CONVERTERS);
    let high_affinity = TestFallbackConverter::new(HIGH_AFFINITY);

    registry.register(low_affinity);
    registry.register(high_affinity.clone());
    let selected = registry.try_get_converter(int_type(), string_type()).expect("a converter");

    assert!(same_object(&selected, &high_affinity));
}

#[test]
fn get_all_converters_should_return_all_registered() {
    let registry = BindingTypeConverterRegistry::new();
    let converter1 = TestConverter::<i32, String>::new(DEFAULT_AFFINITY);
    let converter2 = TestConverter::<f64, bool>::new(EXPECTED_THREE_CONVERTERS);
    let converter3 = TestConverter::<String, i32>::new(EXPECTED_TWO_CONVERTERS);

    registry.register(converter1.clone());
    registry.register(converter2.clone());
    registry.register(converter3.clone());
    let all_converters = registry.get_all_converters();

    assert_eq!(all_converters.len(), EXPECTED_THREE_CONVERTERS as usize);
    assert!(all_converters.iter().any(|converter| same_object(converter, &converter1)));
    assert!(all_converters.iter().any(|converter| same_object(converter, &converter2)));
    assert!(all_converters.iter().any(|converter| same_object(converter, &converter3)));
}

#[test]
fn multiple_converters_should_select_highest_affinity() {
    let registry = BindingTypeConverterRegistry::new();
    let low_affinity = TestConverter::<i32, String>::new(EXPECTED_TWO_CONVERTERS);
    let medium_affinity = TestConverter::<i32, String>::new(DEFAULT_AFFINITY);
    let high_affinity = TestConverter::<i32, String>::new(HIGH_AFFINITY);

    registry.register(medium_affinity);
    registry.register(low_affinity);
    registry.register(high_affinity.clone());
    let selected = registry.try_get_converter(int_type(), string_type()).expect("a converter");

    assert!(same_object(&selected, &high_affinity));
}

#[test]
fn non_existent_type_pair_should_return_null() {
    let registry = BindingTypeConverterRegistry::new();
    registry.register(TestConverter::<i32, String>::new(DEFAULT_AFFINITY));

    let result = registry.try_get_converter(ConverterType::of::<f64>(), ConverterType::of::<bool>());

    assert!(result.is_none());
}

#[test]
fn register_and_retrieve_should_return_converter() {
    let registry = BindingTypeConverterRegistry::new();
    let converter = TestConverter::<i32, String>::new(DEFAULT_AFFINITY);

    registry.register(converter.clone());
    let retrieved = registry.try_get_converter(int_type(), string_type());

    assert!(retrieved.is_some_and(|retrieved| same_object(&retrieved, &converter)));
}

#[test]
fn set_method_registry_should_select_highest_affinity() {
    let registry = SetMethodBindingConverterRegistry::new();
    let low_affinity = TestSetMethodConverter::new(EXPECTED_TWO_CONVERTERS);
    let high_affinity = TestSetMethodConverter::new(ABOVE_DEFAULT_AFFINITY);

    registry.register(low_affinity);
    registry.register(high_affinity.clone());
    let selected = registry.try_get_converter(Some(int_type()), Some(string_type())).expect("a converter");

    assert!(same_object(&selected, &high_affinity));
}

#[test]
fn set_method_registry_empty_try_get_converter_returns_null() {
    let registry = SetMethodBindingConverterRegistry::new();
    assert!(registry.try_get_converter(Some(int_type()), Some(string_type())).is_none());
}

#[test]
fn set_method_registry_empty_get_all_converters_returns_empty() {
    let registry = SetMethodBindingConverterRegistry::new();
    assert_eq!(registry.get_all_converters().len(), 0);
}

// Not ported: SetMethodRegistry_Register_Null_ThrowsArgumentNullException - argument validation

#[test]
fn set_method_registry_get_all_converters_returns_all_registered() {
    let registry = SetMethodBindingConverterRegistry::new();
    let converter1 = TestSetMethodConverter::new(DEFAULT_AFFINITY);
    let converter2 = TestSetMethodConverter::new(EXPECTED_THREE_CONVERTERS);

    registry.register(converter1.clone());
    registry.register(converter2.clone());
    let all = registry.get_all_converters();

    assert_eq!(all.len(), EXPECTED_TWO_CONVERTERS as usize);
    assert!(all.iter().any(|converter| same_object(converter, &converter1)));
    assert!(all.iter().any(|converter| same_object(converter, &converter2)));
}

#[test]
fn type_converter_registry_fresh_get_all_converters_returns_empty() {
    let registry = BindingTypeConverterRegistry::new();
    assert_eq!(registry.get_all_converters().len(), 0);
}

#[test]
fn fallback_registry_fresh_get_all_converters_returns_empty() {
    let registry = BindingFallbackConverterRegistry::new();
    assert_eq!(registry.get_all_converters().len(), 0);
}

/// A typed converter of the given affinity that never converts (`TestConverter<TFrom, TTo>`).
struct TestConverter<F, T> {
    affinity: i32,
    types: PhantomData<fn(F) -> T>,
}

impl<F: 'static, T: 'static> TestConverter<F, T> {
    fn new(affinity: i32) -> Rc<Self> {
        Rc::new(Self { affinity, types: PhantomData })
    }
}

impl<F: 'static, T: 'static> BindingTypeConverter for TestConverter<F, T> {
    type From = F;
    type To = T;

    fn affinity(&self) -> i32 {
        self.affinity
    }

    fn convert(&self, _from: Option<&F>, _conversion_hint: Option<&dyn Any>) -> Option<T> {
        None
    }
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
